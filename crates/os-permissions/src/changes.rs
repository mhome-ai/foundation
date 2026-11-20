//! In-process invalidation only. A subscription neither queries nor requests access.
use std::sync::{Arc, Mutex, Weak};

type Callback = dyn Fn() + Send + Sync;

#[derive(Default)]
pub(crate) struct Changes(Mutex<Vec<Weak<Callback>>>);

/// Keep this guard alive to receive native request/monitor notifications.
/// Callbacks run on the native caller's thread and must not block or query the OS.
pub struct PermissionSubscription {
    _callback: Arc<Callback>,
}

impl Changes {
    pub(crate) fn subscribe(
        &self,
        callback: impl Fn() + Send + Sync + 'static,
    ) -> PermissionSubscription {
        let callback: Arc<Callback> = Arc::new(callback);
        let mut callbacks = self.0.lock().unwrap_or_else(|e| e.into_inner());
        callbacks.retain(|item| item.strong_count() != 0);
        callbacks.push(Arc::downgrade(&callback));
        PermissionSubscription {
            _callback: callback,
        }
    }

    pub(crate) fn notify(&self) {
        let callbacks: Vec<_> = {
            let mut callbacks = self.0.lock().unwrap_or_else(|e| e.into_inner());
            callbacks.retain(|item| item.strong_count() != 0);
            callbacks.iter().filter_map(Weak::upgrade).collect()
        };
        for callback in callbacks {
            // A consumer must not unwind across a native callback's FFI boundary.
            let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| callback()));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    #[test]
    fn subscriptions_are_passive_and_drop_releases_the_callback() {
        let changes = Changes::default();
        let calls = Arc::new(AtomicUsize::new(0));
        let count = calls.clone();
        let guard = changes.subscribe(move || {
            count.fetch_add(1, Ordering::SeqCst);
        });
        assert_eq!(calls.load(Ordering::SeqCst), 0);
        changes.notify();
        assert_eq!(calls.load(Ordering::SeqCst), 1);
        drop(guard);
        changes.notify();
        assert_eq!(calls.load(Ordering::SeqCst), 1);
        assert_eq!(Arc::strong_count(&calls), 1);
    }

    #[test]
    fn callbacks_may_subscribe_without_deadlocking_and_panics_are_isolated() {
        let changes = Arc::new(Changes::default());
        let bus = changes.clone();
        let _first = changes.subscribe(move || {
            let _ = bus.subscribe(|| {});
        });
        let _panic = changes.subscribe(|| panic!("bad observer"));
        let count = Arc::new(AtomicUsize::new(0));
        let observed = count.clone();
        let _last = changes.subscribe(move || {
            observed.fetch_add(1, Ordering::SeqCst);
        });
        changes.notify();
        assert_eq!(count.load(Ordering::SeqCst), 1);
    }
}
