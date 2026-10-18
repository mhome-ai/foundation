use crate::secure_file::SecureFile;
use anyhow::{Context, Result};
use core_api::host::auth::{SigningKeySet, KEY_REFRESH_INTERVAL_SECONDS};
use serde::{Deserialize, Serialize};
use std::{
    path::PathBuf,
    sync::{Arc, RwLock},
    time::{Duration, SystemTime, UNIX_EPOCH},
};
use tokio::sync::Mutex;
use tokio_util::sync::CancellationToken;

#[derive(Default, Clone, Serialize, Deserialize)]
pub struct KeySnapshot {
    pub key_set: Option<SigningKeySet>,
    pub checked_at: u64,
}
struct RefreshState {
    retry_at: tokio::time::Instant,
    failures: u32,
}
/// One cloud key cache. Both the scheduler and unknown-kid path share this object.
pub struct KeyCache {
    endpoint: reqwest::Url,
    store: SecureFile<KeySnapshot>,
    client: reqwest::Client,
    refresh: Mutex<RefreshState>,
    snapshot: Arc<RwLock<KeySnapshot>>,
}
impl KeyCache {
    pub fn new(api_base: &str, path: PathBuf) -> Result<Self> {
        let api_base = super::cloud_api_base(api_base)?;
        let endpoint = reqwest::Url::parse(&format!("{api_base}/host/auth/keys"))?;
        let client = reqwest::Client::builder()
            .redirect(reqwest::redirect::Policy::none())
            .timeout(Duration::from_secs(10))
            .build()?;
        let store = SecureFile::new(path)?;
        let snapshot = Arc::new(RwLock::new(store.read()?));
        Ok(Self {
            endpoint,
            store,
            snapshot,
            client,
            refresh: Mutex::new(RefreshState {
                retry_at: tokio::time::Instant::now(),
                failures: 0,
            }),
        })
    }
    pub async fn snapshot(&self) -> Result<KeySnapshot> {
        self.snapshot
            .read()
            .map(|snapshot| snapshot.clone())
            .map_err(|_| anyhow::anyhow!("Host key cache unavailable"))
    }
    /// Unknown kid is a hint, not a new URL or a reason to erase current trust.
    pub async fn refresh(&self, unknown_kid: Option<&str>) -> Result<KeySnapshot> {
        // Known keys never wait for a cloud refresh or perform filesystem I/O.
        let current = self.snapshot().await?;
        if unknown_kid.is_some_and(|kid| {
            current
                .key_set
                .as_ref()
                .is_some_and(|set| set.keys.iter().any(|k| k.kid == kid))
        }) {
            return Ok(current);
        }
        let mut gate = self.refresh.lock().await;
        let current = self.snapshot().await?;
        let known = unknown_kid.is_some_and(|kid| {
            current
                .key_set
                .as_ref()
                .is_some_and(|set| set.keys.iter().any(|k| k.kid == kid))
        });
        let now = unix_seconds();
        let fresh = current.key_set.is_some()
            && now >= current.checked_at
            && now - current.checked_at < KEY_REFRESH_INTERVAL_SECONDS;
        if known || (unknown_kid.is_none() && fresh) || tokio::time::Instant::now() < gate.retry_at
        {
            return Ok(current);
        }
        let fetched = self.fetch().await;
        let fetched = match fetched {
            Ok(set) => set,
            Err(error) => {
                gate.failures = gate.failures.saturating_add(1);
                let backoff = (30_u64.saturating_mul(1 << gate.failures.min(6))).min(1800);
                gate.retry_at = tokio::time::Instant::now() + Duration::from_secs(backoff);
                return Err(error);
            }
        };
        let store = self.store.clone();
        let cached = self.snapshot.clone();
        // Re-read under the cross-process lock: another Host may have published a newer set.
        let updated = tokio::task::spawn_blocking(move || {
            let updated = store.update(|snapshot| {
                fetched
                    .validate_update(snapshot.key_set.as_ref())
                    .map_err(anyhow::Error::msg)?;
                snapshot.key_set = Some(fetched);
                snapshot.checked_at = unix_seconds();
                Ok(snapshot.clone())
            })?;
            *cached
                .write()
                .map_err(|_| anyhow::anyhow!("Host key cache unavailable"))? = updated.clone();
            Ok::<_, anyhow::Error>(updated)
        })
        .await?;
        // Malformed or rolled-back responses get the same cooldown as valid unknown IDs.
        gate.retry_at = tokio::time::Instant::now() + Duration::from_secs(60);
        if updated.is_ok() {
            gate.failures = 0;
        }
        updated
    }
    async fn fetch(&self) -> Result<SigningKeySet> {
        let mut response = self
            .client
            .get(self.endpoint.clone())
            .send()
            .await?
            .error_for_status()?;
        anyhow::ensure!(
            response.status().is_success(),
            "unexpected key endpoint response"
        );
        const LIMIT: usize = 256 * 1024;
        anyhow::ensure!(
            response.content_length().unwrap_or(0) <= LIMIT as u64,
            "key set exceeds size limit"
        );
        let mut body = Vec::new();
        while let Some(chunk) = response.chunk().await? {
            anyhow::ensure!(
                body.len().saturating_add(chunk.len()) <= LIMIT,
                "key set exceeds size limit"
            );
            body.extend_from_slice(&chunk);
        }
        let set: SigningKeySet =
            serde_json::from_slice(&body).context("invalid Host signing key response")?;
        set.validate_update(None).map_err(anyhow::Error::msg)?;
        Ok(set)
    }
    pub fn spawn(self: Arc<Self>, stop: CancellationToken) -> tokio::task::JoinHandle<()> {
        tokio::spawn(async move {
            loop {
                tokio::select! {
                    _=stop.cancelled()=>break,
                    result=self.refresh(None)=>if let Err(error)=result { tracing::warn!(error=%error,"Host signing key refresh failed; retaining established offline trust"); }
                }
                // Only a local due-time check each minute. Successful cloud checks are 24h apart.
                tokio::select! { _=stop.cancelled()=>break, _=tokio::time::sleep(Duration::from_secs(60))=>{} }
            }
        })
    }
}
pub fn unix_seconds() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}
