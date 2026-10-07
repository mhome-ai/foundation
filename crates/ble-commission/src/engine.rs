//! Shared state, background tasks and session bookkeeping.
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use app_facade_api::host_provision::PROVISION_CHANGED_EVENT;
use app_facade_api::pod::{
    AdapterState, AdapterStatus, CommissionError, CommissionErrorCode, DeviceKind,
    COMMISSION_CHANGED_EVENT, DISCOVERY_CHANGED_EVENT, DISCOVERY_EVENT_INTERVAL_MS,
};
use tokio::sync::{mpsc, Notify};
use tokio::task::JoinHandle;
use tokio::time::Instant;
use tracing::warn;

use crate::discovery::{Discovery, SCAN_RETRY_MS};
use crate::platform::{Advertisement, Platform};
use crate::session::{error, Live, Phase};

const TICK: Duration = Duration::from_secs(1);

/// Wall-clock milliseconds that advance with the runtime's clock.
struct Clock {
    base_ms: i64,
    start: Instant,
}

impl Clock {
    fn new() -> Self {
        let base_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|elapsed| elapsed.as_millis() as i64)
            .unwrap_or_default();
        Self {
            base_ms,
            start: Instant::now(),
        }
    }

    fn now_ms(&self) -> i64 {
        self.base_ms + self.start.elapsed().as_millis() as i64
    }
}

#[derive(Default)]
pub(crate) struct Sessions {
    pub pod: Option<Live>,
    pub host: Option<Live>,
}

impl Sessions {
    pub fn slot(&mut self, kind: DeviceKind) -> &mut Option<Live> {
        match kind {
            DeviceKind::Pod => &mut self.pod,
            DeviceKind::Host => &mut self.host,
        }
    }

    pub fn current(&mut self, kind: DeviceKind, id: &str) -> Option<&mut Live> {
        self.slot(kind).as_mut().filter(|live| live.id == id)
    }

    fn all(&self) -> impl Iterator<Item = &Live> {
        self.pod.iter().chain(self.host.iter())
    }

    pub fn active(&self) -> bool {
        self.all().any(|live| !live.phase.is_terminal())
    }

    pub fn radio_owned(&self) -> bool {
        self.all().any(|live| !live.released)
    }
}

pub(crate) struct Engine {
    pub platform: Platform,
    clock: Clock,
    pub discovery: Mutex<Discovery>,
    pub sessions: Mutex<Sessions>,
    events: mpsc::UnboundedSender<(&'static str, String)>,
    discovery_changed: Notify,
    reconcile: Notify,
    radio: tokio::sync::Mutex<()>,
}

impl Engine {
    pub fn start(platform: Platform) -> (Arc<Self>, Vec<JoinHandle<()>>) {
        let (events, queue) = mpsc::unbounded_channel();
        let engine = Arc::new(Self {
            platform,
            clock: Clock::new(),
            discovery: Mutex::new(Discovery::new()),
            sessions: Mutex::new(Sessions::default()),
            events,
            discovery_changed: Notify::new(),
            reconcile: Notify::new(),
            radio: tokio::sync::Mutex::new(()),
        });
        let tasks = vec![
            tokio::spawn(pump_events(engine.platform.events.clone(), queue)),
            tokio::spawn(publish_discovery(engine.clone())),
            tokio::spawn(reconcile_scanning(engine.clone())),
        ];
        (engine, tasks)
    }

    pub fn now_ms(&self) -> i64 {
        self.clock.now_ms()
    }

    fn emit(&self, target: &'static str, payload: String) {
        let _ = self.events.send((target, payload));
    }

    pub fn emit_session(&self, live: &Live) {
        let target = match live.kind {
            DeviceKind::Pod => COMMISSION_CHANGED_EVENT,
            DeviceKind::Host => PROVISION_CHANGED_EVENT,
        };
        self.emit(target, live.snapshot().to_string());
    }

    pub fn discovery_changed(&self) {
        self.discovery_changed.notify_one();
    }

    pub fn reconcile(&self) {
        self.reconcile.notify_one();
    }

    pub fn on_adapter_state(&self, status: AdapterStatus) {
        if self.discovery.lock().unwrap().set_adapter(status) {
            self.discovery_changed();
        }
        self.reconcile();
    }

    pub fn on_advertisement(&self, advertisement: &Advertisement) {
        let now = self.now_ms();
        if self.discovery.lock().unwrap().observe(advertisement, now) {
            self.discovery_changed();
        }
    }

    /// Applies `change` to the session while it is current and not terminal,
    /// then publishes it. `false` means the session ended or was replaced and
    /// the caller must stop.
    pub fn update(&self, kind: DeviceKind, id: &str, change: impl FnOnce(&mut Live)) -> bool {
        let mut sessions = self.sessions.lock().unwrap();
        let Some(live) = sessions
            .current(kind, id)
            .filter(|live| !live.phase.is_terminal())
        else {
            return false;
        };
        change(live);
        live.revision += 1;
        self.emit_session(live);
        true
    }

    /// Changes bookkeeping that is not part of the snapshot.
    pub fn mutate<R>(
        &self,
        kind: DeviceKind,
        id: &str,
        change: impl FnOnce(&mut Live) -> R,
    ) -> Option<R> {
        self.sessions.lock().unwrap().current(kind, id).map(change)
    }

    pub fn phase(&self, kind: DeviceKind, id: &str) -> Option<Phase> {
        self.mutate(kind, id, |live| live.phase)
    }

    /// Ends the session from outside its task and interrupts what it is doing.
    pub fn terminate(
        &self,
        kind: DeviceKind,
        id: &str,
        phase: Phase,
        failure: Option<CommissionError>,
    ) -> bool {
        let mut sessions = self.sessions.lock().unwrap();
        let Some(live) = sessions
            .current(kind, id)
            .filter(|live| !live.phase.is_terminal())
        else {
            return false;
        };
        live.set(phase, failure);
        live.revision += 1;
        self.emit_session(live);
        let _ = live.cancel.send(true);
        true
    }

    pub fn release(&self, kind: DeviceKind, id: &str) {
        self.mutate(kind, id, |live| live.released = true);
        self.reconcile();
    }

    pub async fn pause_scanning(&self) {
        let _radio = self.radio.lock().await;
        if !self.discovery.lock().unwrap().scanning {
            return;
        }
        let _ = self.platform.radio.set_scanning(false).await;
        if self.discovery.lock().unwrap().set_scanning(false) {
            self.discovery_changed();
        }
    }

    async fn reconcile_once(&self) {
        let _radio = self.radio.lock().await;
        let now = self.now_ms();
        let owned = self.sessions.lock().unwrap().radio_owned();
        let (want, scanning, may_start, pruned) = {
            let mut discovery = self.discovery.lock().unwrap();
            let pruned = discovery.prune(now);
            let want = discovery.has_live_lease(now)
                && !owned
                && discovery.adapter.state == AdapterState::Ready;
            let may_start = discovery
                .scan_refused_at
                .is_none_or(|refused| now - refused >= SCAN_RETRY_MS);
            (want, discovery.scanning, may_start, pruned)
        };
        if pruned {
            self.discovery_changed();
        }
        if want && !scanning && may_start {
            let result = self.platform.radio.set_scanning(true).await;
            let mut discovery = self.discovery.lock().unwrap();
            let changed = match result {
                Ok(()) => {
                    discovery.scan_refused_at = None;
                    discovery.set_scanning(true)
                }
                Err(status) => {
                    let changed = discovery.set_adapter(status);
                    discovery.scan_refused_at = Some(now);
                    changed
                }
            };
            drop(discovery);
            if changed {
                self.discovery_changed();
            }
        } else if !want && scanning {
            let _ = self.platform.radio.set_scanning(false).await;
            if self.discovery.lock().unwrap().set_scanning(false) {
                self.discovery_changed();
            }
        }
    }

    /// Ends sessions whose owner lease or lifetime ran out.
    pub fn spawn_watchdog(self: &Arc<Self>, kind: DeviceKind, id: String) {
        let engine = Arc::downgrade(self);
        tokio::spawn(async move {
            loop {
                tokio::time::sleep(TICK).await;
                let Some(engine) = engine.upgrade() else {
                    return;
                };
                let now = engine.now_ms();
                let step = engine.mutate(kind, &id, |live| {
                    if live.phase.is_terminal() {
                        Watch::Stop
                    } else if live.phase.runs_to_conclusion() {
                        Watch::Wait
                    } else if now >= live.expires_at_ms {
                        Watch::End(
                            Phase::TimedOut,
                            Some(error(
                                CommissionErrorCode::SessionTimeout,
                                "The setup took too long.",
                                None,
                            )),
                        )
                    } else if now >= live.lease_expires_ms {
                        Watch::End(Phase::Cancelled, None)
                    } else {
                        Watch::Wait
                    }
                });
                match step.unwrap_or(Watch::Stop) {
                    Watch::Stop => return,
                    Watch::Wait => {}
                    Watch::End(phase, failure) => {
                        engine.terminate(kind, &id, phase, failure);
                        return;
                    }
                }
            }
        });
    }
}

enum Watch {
    Stop,
    Wait,
    End(Phase, Option<CommissionError>),
}

async fn pump_events(
    sink: Arc<dyn crate::platform::EventSink>,
    mut queue: mpsc::UnboundedReceiver<(&'static str, String)>,
) {
    while let Some((target, payload)) = queue.recv().await {
        let sink = sink.clone();
        if tokio::task::spawn_blocking(move || sink.publish(target, &payload))
            .await
            .is_err()
        {
            warn!(target, "event sink panicked");
        }
    }
}

async fn publish_discovery(engine: Arc<Engine>) {
    let interval = Duration::from_millis(DISCOVERY_EVENT_INTERVAL_MS as u64);
    let mut last: Option<(Instant, u64)> = None;
    loop {
        engine.discovery_changed.notified().await;
        if let Some((at, _)) = last {
            tokio::time::sleep_until(at + interval).await;
        }
        let snapshot = engine.discovery.lock().unwrap().snapshot();
        if last.is_some_and(|(_, revision)| revision == snapshot.revision) {
            continue;
        }
        match serde_json::to_string(&snapshot) {
            Ok(payload) => engine.emit(DISCOVERY_CHANGED_EVENT, payload),
            Err(error) => warn!(%error, "discovery snapshot serialization failed"),
        }
        last = Some((Instant::now(), snapshot.revision));
    }
}

async fn reconcile_scanning(engine: Arc<Engine>) {
    loop {
        engine.reconcile_once().await;
        tokio::select! {
            _ = engine.reconcile.notified() => {}
            _ = tokio::time::sleep(TICK) => {}
        }
    }
}
