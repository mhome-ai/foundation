use crate::{invoke, CancelOnDrop, CancellationToken, InvocationControl, ScriptHost, ScriptLimits};
use mhome_script_api::{ScriptError, ScriptValue};
use std::{
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc, Mutex,
    },
    thread::JoinHandle,
};
use tokio::{
    sync::{mpsc, oneshot, watch, OwnedSemaphorePermit, Semaphore},
    task::{JoinSet, LocalSet},
};

/// A trusted host scheduling classification, never selected by script arguments.
#[derive(Clone, Copy, Debug)]
pub enum Admission {
    Request,
    Hook,
}

#[derive(Clone, Copy, Debug)]
pub struct WorkerConfig {
    pub threads: usize,
    pub requests: usize,
    pub hooks: usize,
}

impl Default for WorkerConfig {
    fn default() -> Self {
        Self {
            threads: 2,
            requests: 4,
            hooks: 2,
        }
    }
}

struct Job {
    source: Arc<str>,
    args: Vec<ScriptValue>,
    host: Arc<dyn ScriptHost>,
    control: InvocationControl,
    limits: ScriptLimits,
    reply: oneshot::Sender<Result<ScriptValue, ScriptError>>,
    _permit: OwnedSemaphorePermit,
}

struct Worker {
    queue: mpsc::Sender<Job>,
    finished: watch::Receiver<bool>,
    thread: Mutex<Option<JoinHandle<()>>>,
}

struct Inner {
    workers: Vec<Worker>,
    requests: Arc<Semaphore>,
    hooks: Arc<Semaphore>,
    next: AtomicUsize,
    shutdown: CancellationToken,
}

impl Drop for Inner {
    fn drop(&mut self) {
        self.shutdown.cancel();
    }
}

/// A small dedicated CPU pool. Waiting host calls yield within a LocalSet;
/// request saturation cannot consume the separate hook admission capacity.
/// Admission is fail-fast, including hooks, so recursive hooks never wait forever
/// for a slot held by their caller. Capacity is released after actual worker cleanup.
#[derive(Clone)]
pub struct ScriptWorkers(Arc<Inner>);

impl ScriptWorkers {
    /// Create at service startup and share across Integration and Automation calls.
    pub fn new(config: WorkerConfig) -> Result<Self, ScriptError> {
        if !(1..=64).contains(&config.threads) || config.requests == 0 || config.hooks == 0 {
            return Err(ScriptError::new(
                "INVALID_CONFIG",
                "Script worker counts must be positive and threads at most 64",
            ));
        }
        let capacity = config
            .requests
            .checked_add(config.hooks)
            .filter(|n| *n <= Semaphore::MAX_PERMITS)
            .ok_or_else(|| ScriptError::new("INVALID_CONFIG", "Script worker capacity overflow"))?;
        let mut inner = Inner {
            workers: Vec::new(),
            requests: Arc::new(Semaphore::new(config.requests)),
            hooks: Arc::new(Semaphore::new(config.hooks)),
            next: AtomicUsize::new(0),
            shutdown: CancellationToken::new(),
        };
        for index in 0..config.threads {
            let runtime = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .map_err(|e| ScriptError::new("WORKER_START_FAILED", e.to_string()))?;
            let (queue, receiver) = mpsc::channel(capacity);
            let (finished_tx, finished) = watch::channel(false);
            let shutdown = inner.shutdown.clone();
            let thread = std::thread::Builder::new()
                .name(format!("script-{index}"))
                .spawn(move || {
                    let _finished = Finished(finished_tx);
                    runtime.block_on(LocalSet::new().run_until(worker(receiver, shutdown)));
                })
                .map_err(|e| ScriptError::new("WORKER_START_FAILED", e.to_string()))?;
            inner.workers.push(Worker {
                queue,
                finished,
                thread: Mutex::new(Some(thread)),
            });
        }
        Ok(Self(Arc::new(inner)))
    }

    pub async fn invoke(
        &self,
        source: Arc<str>,
        args: Vec<ScriptValue>,
        host: Arc<dyn ScriptHost>,
        mut control: InvocationControl,
        limits: ScriptLimits,
        admission: Admission,
    ) -> Result<ScriptValue, ScriptError> {
        control.shutdown = self.0.shutdown.clone();
        control.check()?;
        let permits = match admission {
            Admission::Request => &self.0.requests,
            Admission::Hook => &self.0.hooks,
        };
        let permit = permits.clone().try_acquire_owned().map_err(|_| {
            ScriptError::new("SCRIPT_BUSY", "Script execution capacity is exhausted")
        })?;
        control.cancel = control.cancel.child_token();
        let _cancel_on_drop = CancelOnDrop(control.cancel.clone());
        let (reply, result) = oneshot::channel();
        let job = Job {
            source,
            args,
            host,
            control: control.clone(),
            limits,
            reply,
            _permit: permit,
        };
        let index = self.0.next.fetch_add(1, Ordering::Relaxed) % self.0.workers.len();
        self.0.workers[index]
            .queue
            .try_send(job)
            .map_err(|_| ScriptError::new("WORKER_UNAVAILABLE", "Script worker is unavailable"))?;
        tokio::select! {
            biased;
            _ = control.cancelled() => Err(ScriptError::new("CANCELLED", "Script invocation cancelled")),
            _ = tokio::time::sleep_until(control.deadline.into()) => Err(ScriptError::new("DEADLINE_EXCEEDED", "Script invocation timed out")),
            result = result => result.unwrap_or_else(|_| Err(ScriptError::new("WORKER_FAILED", "Script worker failed"))),
        }
    }

    /// Stop admission, interrupt CPU work, drop host futures and wait for threads.
    /// Idempotent; closing any clone closes the shared service.
    pub async fn shutdown(&self) {
        self.0.shutdown.cancel();
        for worker in &self.0.workers {
            let mut finished = worker.finished.clone();
            let _ = finished.wait_for(|done| *done).await;
            // The completion guard has run; joining cannot wait for script work.
            if let Some(thread) = worker
                .thread
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .take()
            {
                let _ = thread.join();
            }
        }
    }
}

struct Finished(watch::Sender<bool>);
impl Drop for Finished {
    fn drop(&mut self) {
        self.0.send_replace(true);
    }
}

async fn worker(mut queue: mpsc::Receiver<Job>, shutdown: CancellationToken) {
    let mut running = JoinSet::new();
    loop {
        tokio::select! {
            biased;
            _ = shutdown.cancelled() => break,
            job = queue.recv() => match job {
                Some(job) => {
                    running.spawn_local(async move {
                        let result = invoke(&job.source, job.args, job.host, job.control, job.limits).await;
                        let _ = job.reply.send(result);
                        // Permit lives through runtime teardown, including error/cancellation.
                        drop(job._permit);
                    });
                }
                None => break,
            },
            _ = running.join_next(), if !running.is_empty() => {},
        }
    }
    queue.close();
    while queue.try_recv().is_ok() {}
    running.abort_all();
    while running.join_next().await.is_some() {}
}
