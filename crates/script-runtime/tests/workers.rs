use mhome_script_api::{ScriptError, ScriptValue as V};
use mhome_script_runtime::{
    Admission, CancellationToken, HostFuture, InvocationControl, NoHost, ScriptHost, ScriptLimits,
    ScriptWorkers, WorkerConfig,
};
use std::{
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
    time::{Duration, Instant},
};
use tokio::sync::Notify;

fn control() -> InvocationControl {
    InvocationControl::new(
        Instant::now() + Duration::from_secs(3),
        CancellationToken::new(),
    )
}
fn workers() -> ScriptWorkers {
    ScriptWorkers::new(WorkerConfig {
        threads: 1,
        requests: 1,
        hooks: 1,
    })
    .unwrap()
}

struct Nested(ScriptWorkers);
impl ScriptHost for Nested {
    fn call<'a>(
        &'a self,
        operation: &'a str,
        _: Vec<V>,
        control: InvocationControl,
    ) -> HostFuture<'a> {
        Box::pin(async move {
            match operation {
                "channel" => self.0.invoke("export default () => 41".into(), vec![], Arc::new(NoHost), control, ScriptLimits::default(), Admission::Hook).await,
                "recursive" => self.0.invoke("import {call} from '@meowlink/script/host';export default async () => await call('recursive',[])".into(), vec![], Arc::new(Nested(self.0.clone())), control, ScriptLimits::default(), Admission::Hook).await,
                _ => Err(ScriptError::new("UNKNOWN", "unknown operation")),
            }
        })
    }
}

#[tokio::test(flavor = "current_thread")]
async fn channel_hook_reenters_one_thread_with_request_admission_full() {
    let pool = workers();
    let result = pool.invoke("import {call} from '@meowlink/script/host';export default async () => (await call('channel',[]))+1".into(), vec![], Arc::new(Nested(pool.clone())), control(), ScriptLimits::default(), Admission::Request).await.unwrap();
    assert_eq!(result, V::Number(42.0));
    pool.shutdown().await;
}

#[tokio::test(flavor = "current_thread")]
async fn recursive_hooks_fail_fast_instead_of_waiting_for_their_own_slot() {
    let pool = workers();
    let start = Instant::now();
    let result = pool.invoke("import {call} from '@meowlink/script/host';export default async () => await call('recursive',[])".into(), vec![], Arc::new(Nested(pool.clone())), control(), ScriptLimits::default(), Admission::Request).await.unwrap_err();
    assert_eq!(result.code, "SCRIPT_BUSY");
    assert!(start.elapsed() < Duration::from_secs(1));
    pool.shutdown().await;
}

struct WaitHost {
    started: Notify,
    dropped: AtomicUsize,
}
struct Guard<'a>(&'a AtomicUsize);
impl Drop for Guard<'_> {
    fn drop(&mut self) {
        self.0.fetch_add(1, Ordering::SeqCst);
    }
}
impl ScriptHost for WaitHost {
    fn call<'a>(&'a self, _: &'a str, _: Vec<V>, _: InvocationControl) -> HostFuture<'a> {
        Box::pin(async move {
            let _guard = Guard(&self.dropped);
            self.started.notify_one();
            std::future::pending().await
        })
    }
}

#[tokio::test(flavor = "current_thread")]
async fn saturation_is_bounded_and_shutdown_releases_pending_host_work() {
    let pool = workers();
    let host = Arc::new(WaitHost {
        started: Notify::new(),
        dropped: AtomicUsize::new(0),
    });
    let task = tokio::spawn({
        let pool = pool.clone();
        let host = host.clone();
        async move {
            pool.invoke("import {call} from '@meowlink/script/host';export default async () => await call('wait',[])".into(), vec![], host, control(), ScriptLimits::default(), Admission::Request).await
        }
    });
    tokio::time::timeout(Duration::from_secs(1), host.started.notified())
        .await
        .unwrap();
    assert_eq!(
        pool.invoke(
            "export default () => 1".into(),
            vec![],
            Arc::new(NoHost),
            control(),
            ScriptLimits::default(),
            Admission::Request
        )
        .await
        .unwrap_err()
        .code,
        "SCRIPT_BUSY"
    );
    pool.shutdown().await;
    assert_eq!(task.await.unwrap().unwrap_err().code, "CANCELLED");
    assert_eq!(host.dropped.load(Ordering::SeqCst), 1);
    assert_eq!(
        pool.invoke(
            "export default () => 1".into(),
            vec![],
            Arc::new(NoHost),
            control(),
            ScriptLimits::default(),
            Admission::Request
        )
        .await
        .unwrap_err()
        .code,
        "CANCELLED"
    );
    pool.shutdown().await;
}

#[tokio::test(flavor = "current_thread")]
async fn shutdown_interrupts_cpu_without_waiting_for_request_deadline() {
    let pool = workers();
    let task = tokio::spawn({
        let pool = pool.clone();
        async move {
            pool.invoke(
                "export default () => {while(true){}}".into(),
                vec![],
                Arc::new(NoHost),
                control(),
                ScriptLimits::default(),
                Admission::Request,
            )
            .await
        }
    });
    tokio::time::sleep(Duration::from_millis(30)).await;
    let start = Instant::now();
    pool.shutdown().await;
    assert!(start.elapsed() < Duration::from_secs(1));
    assert_eq!(task.await.unwrap().unwrap_err().code, "CANCELLED");
}

#[tokio::test(flavor = "current_thread")]
async fn caller_abort_cancels_worker_and_recovers_admission() {
    let pool = workers();
    let host = Arc::new(WaitHost {
        started: Notify::new(),
        dropped: AtomicUsize::new(0),
    });
    let task = tokio::spawn({
        let pool = pool.clone();
        let host = host.clone();
        async move {
            pool.invoke("import {call} from '@meowlink/script/host';export default async () => await call('wait',[])".into(), vec![], host, control(), ScriptLimits::default(), Admission::Request).await
        }
    });
    tokio::time::timeout(Duration::from_secs(1), host.started.notified())
        .await
        .unwrap();
    task.abort();
    assert!(task.await.unwrap_err().is_cancelled());
    tokio::time::timeout(Duration::from_secs(1), async {
        loop {
            match pool
                .invoke(
                    "export default () => 42".into(),
                    vec![],
                    Arc::new(NoHost),
                    control(),
                    ScriptLimits::default(),
                    Admission::Request,
                )
                .await
            {
                Ok(value) => {
                    assert_eq!(value, V::Number(42.0));
                    break;
                }
                Err(error) => {
                    assert_eq!(error.code, "SCRIPT_BUSY");
                    tokio::time::sleep(Duration::from_millis(1)).await;
                }
            }
        }
    })
    .await
    .unwrap();
    assert_eq!(host.dropped.load(Ordering::SeqCst), 1);
    pool.shutdown().await;
}
