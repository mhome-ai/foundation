//! Temporary content has process lifetime and one hour of idle retention.
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

pub const TEMPORARY_TTL: Duration = Duration::from_secs(3600);
pub type TemporaryContent = Vec<u8>;

#[derive(Debug)]
pub struct Entry {
    pub content: Arc<TemporaryContent>,
    pub expires: Instant,
    pub expires_at_unix_ms: u64,
}
impl Entry {
    pub fn new(content: Arc<TemporaryContent>) -> Self {
        let mut entry = Self {
            content,
            expires: Instant::now(),
            expires_at_unix_ms: 0,
        };
        entry.renew();
        entry
    }
    pub fn renew(&mut self) {
        self.expires = Instant::now() + TEMPORARY_TTL;
        self.expires_at_unix_ms = now_unix_ms() + TEMPORARY_TTL.as_millis() as u64;
    }
}
#[derive(Debug, Default)]
pub struct TemporaryState {
    pub entries: HashMap<PathBuf, Entry>,
}
impl TemporaryState {
    pub fn expire(&mut self) {
        let now = Instant::now();
        self.entries.retain(|_, entry| entry.expires > now);
    }
    pub fn live(&mut self, key: &Path) -> Option<&mut Entry> {
        if self
            .entries
            .get(key)
            .is_some_and(|entry| entry.expires <= Instant::now())
        {
            self.entries.remove(key);
        }
        self.entries.get_mut(key)
    }
    pub fn insert(&mut self, key: PathBuf, content: Arc<TemporaryContent>) -> u64 {
        let entry = Entry::new(content);
        let expiry = entry.expires_at_unix_ms;
        self.entries.insert(key, entry);
        expiry
    }
}
pub fn now_unix_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}
