//! Process-local native request ownership. A transport timeout never releases it.
use crate::PermissionKey;
use std::{
    collections::{BTreeMap, BTreeSet},
    sync::Mutex,
};

#[derive(Default)]
pub(crate) struct Requests(Mutex<State>);
#[derive(Default)]
struct State {
    active: BTreeSet<PermissionKey>,
    errors: BTreeMap<PermissionKey, String>,
}
impl Requests {
    pub(crate) fn begin(&self, key: &PermissionKey) -> bool {
        let mut state = self.0.lock().unwrap_or_else(|e| e.into_inner());
        if !state.active.insert(key.clone()) {
            return false;
        }
        state.errors.remove(key);
        true
    }
    pub(crate) fn finish(&self, key: &PermissionKey, error: Option<String>) {
        let mut state = self.0.lock().unwrap_or_else(|e| e.into_inner());
        state.active.remove(key);
        match error {
            Some(error) => {
                state.errors.insert(key.clone(), error);
            }
            None => {
                state.errors.remove(key);
            }
        }
    }
    pub(crate) fn error(&self, key: &PermissionKey) -> Option<String> {
        self.0
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .errors
            .get(key)
            .cloned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Barrier};

    #[test]
    fn concurrent_clicks_start_one_native_request_until_it_really_finishes() {
        let requests = Arc::new(Requests::default());
        let barrier = Arc::new(Barrier::new(16));
        let workers: Vec<_> = (0..16)
            .map(|_| {
                let requests = requests.clone();
                let barrier = barrier.clone();
                std::thread::spawn(move || {
                    barrier.wait();
                    requests.begin(&PermissionKey::Microphone {})
                })
            })
            .collect();
        assert_eq!(
            workers
                .into_iter()
                .map(|worker| usize::from(worker.join().unwrap()))
                .sum::<usize>(),
            1
        );
        // Losing all HTTP/UI waiters changes nothing in native request ownership.
        assert!(!requests.begin(&PermissionKey::Microphone {}));
        assert!(requests.begin(&PermissionKey::Bluetooth {}));
        requests.finish(&PermissionKey::Microphone {}, Some("Native error".into()));
        assert_eq!(
            requests.error(&PermissionKey::Microphone {}).as_deref(),
            Some("Native error")
        );
        assert!(requests.begin(&PermissionKey::Microphone {}));
        assert!(requests.error(&PermissionKey::Microphone {}).is_none());
        assert!(!requests.begin(&PermissionKey::Bluetooth {}));
    }

    #[test]
    fn automation_targets_are_independent() {
        let requests = Requests::default();
        let first = PermissionKey::Automation {
            target_bundle_id: "com.apple.Music".into(),
        };
        let second = PermissionKey::Automation {
            target_bundle_id: "com.apple.Finder".into(),
        };
        assert!(requests.begin(&first));
        assert!(requests.begin(&second));
        assert!(!requests.begin(&first));
        requests.finish(&first, None);
        assert!(requests.begin(&first));
        assert!(!requests.begin(&second));
    }
}
