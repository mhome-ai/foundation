use anyhow::{bail, Result};
use async_trait::async_trait;
use mhome_storage::*;
use sha2::{Digest, Sha256};
use std::{
    collections::HashMap,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
};
use tokio::sync::RwLock;

struct Controlled {
    disk: Arc<Disk>,
    online: AtomicBool,
    full: AtomicBool,
    lose_put: AtomicBool,
    lose_delete: AtomicBool,
    hold: AtomicBool,
    entered: tokio::sync::Notify,
    release: tokio::sync::Notify,
    deny: std::sync::Mutex<Option<String>>,
}
impl Controlled {
    async fn new(path: std::path::PathBuf) -> Arc<Self> {
        Arc::new(Self {
            disk: Disk::open_with_reserve(path, Some(0)).await.unwrap(),
            online: AtomicBool::new(true),
            full: AtomicBool::new(false),
            lose_put: AtomicBool::new(false),
            lose_delete: AtomicBool::new(false),
            hold: AtomicBool::new(false),
            entered: tokio::sync::Notify::new(),
            release: tokio::sync::Notify::new(),
            deny: std::sync::Mutex::new(None),
        })
    }
    fn available(&self) -> Result<()> {
        if !self.online.load(Ordering::SeqCst) {
            bail!("offline")
        }
        Ok(())
    }
}
#[async_trait]
impl Backend for Controlled {
    fn repository_id(&self) -> &str {
        self.disk.repository_id()
    }
    async fn capacity(&self) -> Result<Capacity> {
        self.available()?;
        self.disk.capacity().await
    }
    async fn prepare(&self, owner: &str, blob: &Blob) -> Result<()> {
        self.available()?;
        if self.full.load(Ordering::SeqCst) {
            bail!("VOLUME_FULL")
        }
        self.disk.prepare(owner, blob).await
    }
    async fn put(&self, owner: &str, blob: &Blob, reader: Reader) -> Result<()> {
        self.available()?;
        if self.hold.swap(false, Ordering::SeqCst) {
            self.entered.notify_one();
            self.release.notified().await;
        }
        self.disk.put(owner, blob, reader).await?;
        if self.lose_put.swap(false, Ordering::SeqCst) {
            bail!("lost reply")
        }
        Ok(())
    }
    async fn stat(&self, owner: &str, id: &str) -> Result<Option<Blob>> {
        self.available()?;
        self.disk.stat(owner, id).await
    }
    async fn source(&self, owner: &str, id: &str) -> Result<ReadSource> {
        self.available()?;
        self.disk.source(owner, id).await
    }
    async fn delete(&self, owner: &str, id: &str) -> Result<()> {
        self.available()?;
        if self.deny.lock().unwrap().as_deref() == Some(id) {
            bail!("physical delete denied")
        }
        self.disk.delete(owner, id).await?;
        if self.lose_delete.swap(false, Ordering::SeqCst) {
            bail!("lost reply")
        }
        Ok(())
    }
}
#[derive(Default)]
struct Routes {
    preferred: RwLock<Option<Arc<Controlled>>>,
    known: RwLock<HashMap<String, Arc<Controlled>>>,
}
impl Routes {
    async fn attach(&self, backend: Arc<Controlled>) {
        self.known
            .write()
            .await
            .insert(backend.repository_id().into(), backend.clone());
        *self.preferred.write().await = Some(backend);
    }
}
#[async_trait]
impl Resolver for Routes {
    async fn preferred(&self, _: &str, _: &str) -> Result<Option<(Arc<dyn Backend>, String)>> {
        Ok(self
            .preferred
            .read()
            .await
            .clone()
            .filter(|b| b.online.load(Ordering::SeqCst))
            .map(|b| (b as Arc<dyn Backend>, "binding".into())))
    }
    async fn resolve(&self, _: &str, _: &str, id: &str, _: &str) -> Result<Arc<dyn Backend>> {
        self.known
            .read()
            .await
            .get(id)
            .cloned()
            .map(|b| b as Arc<dyn Backend>)
            .ok_or_else(|| anyhow::anyhow!("repository unavailable"))
    }
}
async fn fixture() -> (
    tempfile::TempDir,
    Arc<Catalog>,
    Arc<Controlled>,
    Arc<Routes>,
) {
    let root = tempfile::tempdir().unwrap();
    let disk = Controlled::new(root.path().join("internal")).await;
    let routes = Arc::new(Routes::default());
    let catalog = Catalog::open(
        root.path().join("catalog.sqlite"),
        disk.clone(),
        routes.clone(),
    )
    .await
    .unwrap();
    (root, catalog, disk, routes)
}
async fn put(c: &Arc<Catalog>, f: &Folder, key: &str, bytes: &[u8]) -> Result<Object> {
    c.put(
        &f.tenant_id,
        &f.scope_id,
        &f.id,
        key,
        bytes.len() as u64,
        &hex::encode(Sha256::digest(bytes)),
        None,
        Input::Bytes(Arc::new(bytes.to_vec())),
    )
    .await
}

#[tokio::test]
async fn internal_external_and_offline_placement_survive_catalog_restart() {
    let (root, c, internal, routes) = fixture().await;
    let f = c
        .create_folder("t", "s", "Files", 100, Overflow::Reject)
        .await
        .unwrap();
    let a = put(&c, &f, "a", b"internal").await.unwrap();
    let external = Controlled::new(root.path().join("external")).await;
    routes.attach(external.clone()).await;
    let b = put(&c, &f, "b", b"external").await.unwrap();
    assert_eq!(a.repository_id, internal.repository_id());
    assert_eq!(b.repository_id, external.repository_id());
    external.online.store(false, Ordering::SeqCst);
    let d = put(&c, &f, "d", b"fallback").await.unwrap();
    assert_eq!(d.repository_id, a.repository_id);
    assert!(c.source("t", "s", &b).await.is_err());
    assert!(c.source("t", "s", &a).await.is_ok());
    drop(c);
    let c = Catalog::open(root.path().join("catalog.sqlite"), internal, routes)
        .await
        .unwrap();
    external.online.store(true, Ordering::SeqCst);
    assert_eq!(
        c.object("t", "s", &f.id, "a")
            .await
            .unwrap()
            .unwrap()
            .object_id,
        a.object_id
    );
    assert!(c.source("t", "s", &b).await.is_ok());
    assert!(c.object("t", "other", &f.id, "a").await.is_err());
}
#[tokio::test]
async fn a_lost_upload_reply_is_resolved_on_the_original_repository() {
    let (root, c, _, routes) = fixture().await;
    let external = Controlled::new(root.path().join("external")).await;
    routes.attach(external.clone()).await;
    let f = c
        .create_folder("t", "s", "Files", 100, Overflow::Reject)
        .await
        .unwrap();
    external.lose_put.store(true, Ordering::SeqCst);
    assert!(put(&c, &f, "a", b"data").await.is_err());
    *routes.preferred.write().await = None;
    assert!(put(&c, &f, "other", b"different").await.is_ok());
    c.recover().await.unwrap();
    let a = put(&c, &f, "a", b"data").await.unwrap();
    assert_eq!(a.repository_id, external.repository_id());
    assert_eq!(c.folder("t", "s", &f.id).await.unwrap().object_count, 2);
}
#[tokio::test]
async fn deletion_failure_keeps_catalog_and_quota_until_physical_confirmation() {
    let (_root, c, disk, _) = fixture().await;
    let f = c
        .create_folder("t", "s", "Files", 4, Overflow::Reject)
        .await
        .unwrap();
    let a = put(&c, &f, "a", b"1234").await.unwrap();
    disk.online.store(false, Ordering::SeqCst);
    assert!(c.delete("t", "s", &f.id, "a").await.is_err());
    assert_eq!(c.folder("t", "s", &f.id).await.unwrap().used_bytes, 4);
    disk.online.store(true, Ordering::SeqCst);
    c.recover().await.unwrap();
    assert!(c.object("t", "s", &f.id, "a").await.unwrap().is_some());
    disk.lose_delete.store(true, Ordering::SeqCst);
    assert!(c.delete("t", "s", &f.id, "a").await.is_err());
    c.recover().await.unwrap();
    assert!(c.object("t", "s", &f.id, "a").await.unwrap().is_none());
    let replacement = put(&c, &f, "a", b"5678").await.unwrap();
    assert_ne!(a.object_id, replacement.object_id);
    disk.delete(&a.owner, &a.object_id).await.unwrap();
    assert!(c.source("t", "s", &replacement).await.is_ok());
}
#[tokio::test]
async fn eviction_is_per_object_and_can_fail_after_an_earlier_delete() {
    let (_root, c, disk, _) = fixture().await;
    let f = c
        .create_folder("t", "s", "Files", 8, Overflow::EvictOldest)
        .await
        .unwrap();
    let a = put(&c, &f, "a", b"1234").await.unwrap();
    tokio::time::sleep(std::time::Duration::from_millis(2)).await;
    let b = put(&c, &f, "b", b"5678").await.unwrap();
    *disk.deny.lock().unwrap() = Some(b.object_id.clone());
    assert!(put(&c, &f, "incoming", b"abcdefgh").await.is_err());
    assert!(c.object("t", "s", &f.id, "a").await.unwrap().is_none());
    assert!(c.object("t", "s", &f.id, "b").await.unwrap().is_some());
    assert!(c
        .object("t", "s", &f.id, "incoming")
        .await
        .unwrap()
        .is_none());
    let usage = c.folder("t", "s", &f.id).await.unwrap();
    assert_eq!(usage.evicted_object_count, 1);
    assert_eq!(usage.used_bytes, 4);
    assert!(disk.stat(&a.owner, &a.object_id).await.unwrap().is_none());
}
#[tokio::test]
async fn lack_of_staging_space_never_pre_evicts_and_external_preallocation_can_fallback() {
    let (root, c, internal, routes) = fixture().await;
    let f = c
        .create_folder("t", "s", "Files", 4, Overflow::EvictOldest)
        .await
        .unwrap();
    let old = put(&c, &f, "a", b"1234").await.unwrap();
    internal.full.store(true, Ordering::SeqCst);
    assert!(put(&c, &f, "b", b"5678").await.is_err());
    assert!(c.object("t", "s", &f.id, "a").await.unwrap().is_some());
    assert!(internal
        .stat(&old.owner, &old.object_id)
        .await
        .unwrap()
        .is_some());
    internal.full.store(false, Ordering::SeqCst);
    let ext = Controlled::new(root.path().join("external")).await;
    routes.attach(ext.clone()).await;
    ext.full.store(true, Ordering::SeqCst);
    let other = c
        .create_folder("t", "s", "Other", 100, Overflow::Reject)
        .await
        .unwrap();
    let new = put(&c, &other, "c", b"fallback").await.unwrap();
    assert_eq!(new.repository_id, internal.repository_id());
}
#[tokio::test]
async fn system_folder_has_no_quota_and_cannot_be_managed_as_an_ordinary_folder() {
    let (_root, c, _, _) = fixture().await;
    for name in ["artifact", "Artifact", "ＡＲＴＩＦＡＣＴ"] {
        assert!(c
            .create_folder("t", "s", name, 1, Overflow::Reject)
            .await
            .is_err())
    }
    let f = c.artifact_folder("t", "s").await.unwrap();
    assert!(f.system);
    assert_eq!(f.max_bytes, None);
    assert!(c
        .update_folder("t", "s", &f.id, 1, "Other", 1, Overflow::Reject)
        .await
        .is_err());
    put(&c, &f, "a", b"unlimited").await.unwrap();
    let ordinary = c
        .create_folder("t", "s", "Other", 1, Overflow::Reject)
        .await
        .unwrap();
    put(&c, &ordinary, "b", b"x").await.unwrap();
    c.clear_artifacts("t", "s").await.unwrap();
    assert!(c
        .object("t", "s", &ordinary.id, "b")
        .await
        .unwrap()
        .is_some());
}
#[tokio::test]
async fn scope_removal_waits_for_an_offline_repository_and_can_resume_after_restart() {
    let (root, c, internal, routes) = fixture().await;
    let ext = Controlled::new(root.path().join("external")).await;
    routes.attach(ext.clone()).await;
    let f = c
        .create_folder("t", "s", "Files", 100, Overflow::Reject)
        .await
        .unwrap();
    let object = put(&c, &f, "a", b"external").await.unwrap();
    ext.online.store(false, Ordering::SeqCst);
    assert!(c.purge_scope("t", "s").await.is_err());
    assert!(put(&c, &f, "new", b"x").await.is_err());
    assert!(c.activate_scope("t", "s").await.is_err());
    drop(c);
    let c = Catalog::open(root.path().join("catalog.sqlite"), internal, routes)
        .await
        .unwrap();
    ext.online.store(true, Ordering::SeqCst);
    c.purge_scope("t", "s").await.unwrap();
    assert!(ext
        .stat(&object.owner, &object.object_id)
        .await
        .unwrap()
        .is_none());
    c.activate_scope("t", "s").await.unwrap();
    assert!(c.list_folders("t", "s").await.unwrap().is_empty());
}
#[tokio::test]
async fn concurrent_writes_cannot_overrun_a_folder_quota() {
    let (_root, c, _, _) = fixture().await;
    let f = c
        .create_folder("t", "s", "Files", 4, Overflow::Reject)
        .await
        .unwrap();
    let (a, b) = tokio::join!(put(&c, &f, "a", b"1234"), put(&c, &f, "b", b"5678"));
    assert_eq!(a.is_ok() as u8 + b.is_ok() as u8, 1);
    assert_eq!(c.folder("t", "s", &f.id).await.unwrap().used_bytes, 4);
}

#[tokio::test]
async fn a_slow_upload_does_not_serialize_other_writes_in_the_same_folder() {
    let (_root, c, disk, _) = fixture().await;
    let f = c
        .create_folder("t", "s", "Files", 100, Overflow::Reject)
        .await
        .unwrap();
    disk.hold.store(true, Ordering::SeqCst);
    let first = {
        let c = c.clone();
        let f = f.clone();
        tokio::spawn(async move { put(&c, &f, "slow", b"1234").await })
    };
    disk.entered.notified().await;
    let second = tokio::time::timeout(
        std::time::Duration::from_secs(1),
        put(&c, &f, "fast", b"5678"),
    )
    .await
    .expect("independent upload waited for the slow body")
    .unwrap();
    assert_eq!(second.key, "fast");
    assert_eq!(c.folder("t", "s", &f.id).await.unwrap().reserved_bytes, 4);
    disk.release.notify_one();
    first.await.unwrap().unwrap();
    assert_eq!(c.folder("t", "s", &f.id).await.unwrap().used_bytes, 8);
}
#[tokio::test]
async fn concurrent_evictions_claim_distinct_victims() {
    let (_root, c, disk, _) = fixture().await;
    let f = c
        .create_folder("t", "s", "Files", 8, Overflow::EvictOldest)
        .await
        .unwrap();
    put(&c, &f, "a", b"1234").await.unwrap();
    put(&c, &f, "b", b"5678").await.unwrap();
    disk.hold.store(true, Ordering::SeqCst);
    let first = {
        let c = c.clone();
        let f = f.clone();
        tokio::spawn(async move { put(&c, &f, "slow", b"abcd").await })
    };
    disk.entered.notified().await;
    put(&c, &f, "fast", b"efgh").await.unwrap();
    disk.release.notify_one();
    first.await.unwrap().unwrap();
    let usage = c.folder("t", "s", &f.id).await.unwrap();
    assert_eq!(usage.used_bytes, 8);
    assert_eq!(usage.evicted_object_count, 2);
}
#[tokio::test]
async fn a_cross_scope_write_cannot_use_an_existing_object_as_an_authorization_bypass() {
    let (_root, c, _, _) = fixture().await;
    let f = c
        .create_folder("t", "s", "Files", 8, Overflow::Reject)
        .await
        .unwrap();
    put(&c, &f, "a", b"same").await.unwrap();
    let mut foreign = f.clone();
    foreign.scope_id = "other".into();
    assert!(put(&c, &foreign, "a", b"same").await.is_err());
}
#[tokio::test]
async fn consumer_folder_identity_is_idempotent_and_preserves_user_settings() {
    let (_root, c, _, _) = fixture().await;
    let f = c
        .consumer_folder("t", "s", "application", "Recordings", 100)
        .await
        .unwrap();
    let edited = c
        .update_folder(
            "t",
            "s",
            &f.id,
            f.revision,
            "My recordings",
            200,
            Overflow::Reject,
        )
        .await
        .unwrap();
    let again = c
        .consumer_folder("t", "s", "application", "Recordings", 100)
        .await
        .unwrap();
    assert_eq!(again.id, edited.id);
    assert_eq!(again.max_bytes, Some(200));
    assert_eq!(again.name, "My recordings");
}

#[tokio::test]
async fn an_oversized_eviction_claim_cannot_lend_quota_before_physical_deletion() {
    let (_root, c, disk, _) = fixture().await;
    let f = c
        .create_folder("t", "s", "Files", 8, Overflow::EvictOldest)
        .await
        .unwrap();
    put(&c, &f, "old", b"12345678").await.unwrap();
    disk.hold.store(true, Ordering::SeqCst);
    let slow = {
        let c = c.clone();
        let f = f.clone();
        tokio::spawn(async move { put(&c, &f, "slow", b"12").await })
    };
    disk.entered.notified().await;
    assert!(put(&c, &f, "fast", b"123456").await.is_err());
    assert_eq!(c.folder("t", "s", &f.id).await.unwrap().used_bytes, 8);
    disk.release.notify_one();
    slow.await.unwrap().unwrap();
    put(&c, &f, "fast", b"123456").await.unwrap();
    assert_eq!(c.folder("t", "s", &f.id).await.unwrap().used_bytes, 8);
}

#[tokio::test]
async fn disk_recovers_a_rename_before_its_ready_receipt() {
    let root = tempfile::tempdir().unwrap();
    let disk = Disk::open_with_reserve(root.path(), Some(0)).await.unwrap();
    let blob = Blob {
        id: uuid::Uuid::new_v4().to_string(),
        size_bytes: 4,
        sha256: hex::encode(Sha256::digest(b"data")),
    };
    disk.prepare("owner", &blob).await.unwrap();
    tokio::fs::write(root.path().join("blobs").join(&blob.id), b"data")
        .await
        .unwrap();
    assert_eq!(
        disk.stat("owner", &blob.id).await.unwrap(),
        Some(blob.clone())
    );
    assert!(disk.stat("other", &blob.id).await.is_err());
}

#[tokio::test]
async fn disconnected_writer_cannot_race_scope_physical_cleanup() {
    let (_root, c, disk, _) = fixture().await;
    let f = c
        .create_folder("t", "s", "Files", 8, Overflow::Reject)
        .await
        .unwrap();
    disk.hold.store(true, Ordering::SeqCst);
    let caller = {
        let c = c.clone();
        let f = f.clone();
        tokio::spawn(async move { put(&c, &f, "slow", b"data").await })
    };
    disk.entered.notified().await;
    caller.abort();
    let purge = {
        let c = c.clone();
        tokio::spawn(async move { c.purge_scope("t", "s").await })
    };
    tokio::task::yield_now().await;
    assert!(!purge.is_finished());
    disk.release.notify_one();
    purge.await.unwrap().unwrap();
    assert_eq!(c.usage().await.unwrap().used_bytes, 0);
    assert_eq!(c.usage().await.unwrap().reserved_bytes, 0);
}

#[tokio::test]
async fn a_forged_read_placement_cannot_escape_the_catalog() {
    let (_root, c, _, _) = fixture().await;
    let owned = c
        .create_folder("tenant", "owned", "Files", 8, Overflow::Reject)
        .await
        .unwrap();
    let foreign = c
        .create_folder("tenant", "foreign", "Files", 8, Overflow::Reject)
        .await
        .unwrap();
    let mut secret = put(&c, &foreign, "secret", b"data").await.unwrap();
    secret.folder_id = owned.id.clone();
    assert!(c.source("tenant", "owned", &secret).await.is_err());
}

#[tokio::test]
async fn independent_consumers_get_distinct_folders_when_display_names_collide() {
    let (_root, c, _, _) = fixture().await;
    c.create_folder("t", "s", "Camera Recordings", 8, Overflow::Reject)
        .await
        .unwrap();
    let a = c
        .consumer_folder("t", "s", "camera-a", "Camera Recordings", 8)
        .await
        .unwrap();
    let b = c
        .consumer_folder("t", "s", "camera-b", "Camera Recordings", 8)
        .await
        .unwrap();
    assert_eq!(a.name, "Camera Recordings (2)");
    assert_eq!(b.name, "Camera Recordings (3)");
    assert_ne!(a.id, b.id);
}
