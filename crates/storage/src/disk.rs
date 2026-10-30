use crate::{Backend, Blob, Capacity, ReadSource, Reader};
use anyhow::{ensure, Context, Result};
use async_trait::async_trait;
use fs2::FileExt;
use sha2::{Digest, Sha256};
use sqlx::{
    sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions, SqliteSynchronous},
    Row, SqlitePool,
};
use std::{collections::HashMap, path::PathBuf, sync::Arc};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    sync::Mutex,
};

pub struct Disk {
    root: PathBuf,
    id: String,
    reserve_bytes: Option<u64>,
    db: SqlitePool,
    _lock: std::fs::File,
    allocation: Mutex<()>,
    locks: Mutex<HashMap<String, std::sync::Weak<Mutex<()>>>>,
}
impl std::fmt::Debug for Disk {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Disk")
            .field("root", &self.root)
            .field("id", &self.id)
            .finish()
    }
}
impl Disk {
    pub async fn open(root: impl Into<PathBuf>) -> Result<Arc<Self>> {
        Self::open_with_reserve(root, None).await
    }
    /// An explicit physical headroom policy, used by hosts with a different volume budget and by fault-injection tests.
    pub async fn open_with_reserve(
        root: impl Into<PathBuf>,
        reserve_bytes: Option<u64>,
    ) -> Result<Arc<Self>> {
        let root = root.into();
        tokio::fs::create_dir_all(root.join("blobs")).await?;
        tokio::fs::create_dir_all(root.join("staging")).await?;
        let lock = std::fs::OpenOptions::new()
            .create(true)
            .truncate(false)
            .read(true)
            .write(true)
            .open(root.join("repository.lock"))?;
        lock.try_lock_exclusive()
            .context("storage repository is already open")?;
        let options = SqliteConnectOptions::new()
            .filename(root.join("disk.sqlite"))
            .create_if_missing(true)
            .journal_mode(SqliteJournalMode::Wal)
            .synchronous(SqliteSynchronous::Full);
        let db = SqlitePoolOptions::new()
            .max_connections(1)
            .connect_with(options)
            .await?;
        sqlx::migrate!("./migrations/disk").run(&db).await?;
        let id = match sqlx::query_scalar::<_, String>("SELECT id FROM repository")
            .fetch_optional(&db)
            .await?
        {
            Some(id) => id,
            None => {
                let id = uuid::Uuid::new_v4().to_string();
                sqlx::query("INSERT INTO repository(id) VALUES(?)")
                    .bind(&id)
                    .execute(&db)
                    .await?;
                id
            }
        };
        Ok(Arc::new(Self {
            root,
            id,
            reserve_bytes,
            db,
            _lock: lock,
            allocation: Mutex::new(()),
            locks: Mutex::new(HashMap::new()),
        }))
    }
    async fn object_lock(&self, id: &str) -> Arc<Mutex<()>> {
        let mut locks = self.locks.lock().await;
        locks.retain(|_, value| value.strong_count() > 0);
        if let Some(lock) = locks.get(id).and_then(|lock| lock.upgrade()) {
            return lock;
        }
        let lock = Arc::new(Mutex::new(()));
        locks.insert(id.to_owned(), Arc::downgrade(&lock));
        lock
    }
    pub fn path(&self, id: &str) -> Result<PathBuf> {
        uuid::Uuid::parse_str(id).context("invalid physical object id")?;
        Ok(self.root.join("blobs").join(id))
    }
    async fn row(&self, owner: &str, id: &str) -> Result<Option<(Blob, String)>> {
        let row = sqlx::query("SELECT owner,size_bytes,sha256,state FROM blobs WHERE id=?")
            .bind(id)
            .fetch_optional(&self.db)
            .await?;
        row.map(|r| {
            ensure!(
                r.get::<String, _>("owner") == owner,
                "OBJECT_OWNER_MISMATCH"
            );
            Ok((
                Blob {
                    id: id.to_owned(),
                    size_bytes: r.get::<i64, _>("size_bytes") as u64,
                    sha256: r.get("sha256"),
                },
                r.get("state"),
            ))
        })
        .transpose()
    }
    async fn sync_directory(path: PathBuf) -> Result<()> {
        #[cfg(unix)]
        tokio::task::spawn_blocking(move || std::fs::File::open(path)?.sync_all()).await??;
        #[cfg(not(unix))]
        let _ = path;
        Ok(())
    }
}
#[async_trait]
impl Backend for Disk {
    fn repository_id(&self) -> &str {
        &self.id
    }
    async fn capacity(&self) -> Result<Capacity> {
        let root = self.root.clone();
        let (total, available, volume_id) = tokio::task::spawn_blocking(move || -> Result<_> {
            let total = fs2::total_space(&root)?;
            let available = fs2::available_space(&root)?;
            #[cfg(unix)]
            let volume_id = {
                use std::os::unix::fs::MetadataExt;
                format!("dev:{}", std::fs::metadata(&root)?.dev())
            };
            #[cfg(not(unix))]
            let volume_id = root
                .components()
                .next()
                .map(|x| x.as_os_str().to_string_lossy().into_owned())
                .unwrap_or_default();
            Ok((total, available, volume_id))
        })
        .await??;
        let gib = 1024 * 1024 * 1024;
        Ok(Capacity {
            repository_id: self.id.clone(),
            volume_id,
            total_bytes: total,
            available_bytes: available,
            stop_at_available_bytes: self
                .reserve_bytes
                .unwrap_or_else(|| (total / 20).clamp(2 * gib, 50 * gib).min(total / 2)),
            warning_at_available_bytes: (total / 10).clamp(5 * gib, 100 * gib).min(total * 3 / 4),
        })
    }
    async fn put(&self, owner: &str, blob: &Blob, mut reader: Reader) -> Result<()> {
        ensure!(
            blob.size_bytes <= i64::MAX as u64
                && blob.sha256.len() == 64
                && blob.sha256.bytes().all(|b| b.is_ascii_hexdigit()),
            "INVALID_BLOB_DECLARATION"
        );
        let destination = self.path(&blob.id)?;
        let lock = self.object_lock(&blob.id).await;
        let _guard = lock.lock().await;
        if let Some((existing, state)) = self.row(owner, &blob.id).await? {
            ensure!(
                existing == *blob && state != "deleted",
                "IMMUTABLE_OBJECT_CONFLICT"
            );
            if state == "ready" {
                ensure!(
                    tokio::fs::metadata(&destination).await?.len() == blob.size_bytes,
                    "stored object is missing or truncated"
                );
                return Ok(());
            }
            // A rename may have committed before its durable receipt. Revalidate those bytes.
            if let Ok(mut file) = tokio::fs::File::open(&destination).await {
                let mut sha = Sha256::new();
                let mut size = 0;
                let mut buffer = [0u8; 65536];
                loop {
                    let n = file.read(&mut buffer).await?;
                    if n == 0 {
                        break;
                    }
                    size += n as u64;
                    sha.update(&buffer[..n]);
                }
                ensure!(
                    size == blob.size_bytes && hex::encode(sha.finalize()) == blob.sha256,
                    "stored object integrity failed"
                );
                sqlx::query("UPDATE blobs SET state='ready' WHERE id=?")
                    .bind(&blob.id)
                    .execute(&self.db)
                    .await?;
                return Ok(());
            }
        } else {
            sqlx::query(
                "INSERT INTO blobs(id,owner,size_bytes,sha256,state) VALUES(?,?,?,?,'writing')",
            )
            .bind(&blob.id)
            .bind(owner)
            .bind(blob.size_bytes as i64)
            .bind(&blob.sha256)
            .execute(&self.db)
            .await?;
        }
        let staging = self.root.join("staging").join(&blob.id);
        let file = {
            let _allocation = self.allocation.lock().await;
            let stale_size = tokio::fs::metadata(&staging)
                .await
                .map(|m| m.len())
                .unwrap_or(0);
            ensure!(
                self.capacity()
                    .await?
                    .writable_bytes()
                    .saturating_add(stale_size)
                    >= blob.size_bytes,
                "VOLUME_FULL"
            );
            let file = tokio::fs::OpenOptions::new()
                .create(true)
                .truncate(true)
                .read(true)
                .write(true)
                .open(&staging)
                .await?
                .into_std()
                .await;
            let size = blob.size_bytes;
            tokio::task::spawn_blocking(move || -> Result<_> {
                file.allocate(size)
                    .context("VOLUME_FULL: preallocation failed")?;
                Ok(file)
            })
            .await??
        };
        let mut file = tokio::fs::File::from_std(file);
        let mut size = 0u64;
        let mut sha = Sha256::new();
        let mut buffer = [0u8; 65536];
        loop {
            let n = reader.read(&mut buffer).await?;
            if n == 0 {
                break;
            }
            size = size.checked_add(n as u64).context("SIZE_MISMATCH")?;
            ensure!(size <= blob.size_bytes, "SIZE_MISMATCH");
            sha.update(&buffer[..n]);
            file.write_all(&buffer[..n]).await?;
        }
        ensure!(size == blob.size_bytes, "SIZE_MISMATCH");
        ensure!(hex::encode(sha.finalize()) == blob.sha256, "SHA_MISMATCH");
        file.sync_all().await?;
        drop(file);
        tokio::fs::rename(&staging, &destination).await?;
        Self::sync_directory(self.root.join("blobs")).await?;
        sqlx::query("UPDATE blobs SET state='ready' WHERE id=?")
            .bind(&blob.id)
            .execute(&self.db)
            .await?;
        Ok(())
    }
    async fn prepare(&self, owner: &str, blob: &Blob) -> Result<()> {
        ensure!(
            blob.size_bytes <= i64::MAX as u64
                && blob.sha256.len() == 64
                && blob
                    .sha256
                    .bytes()
                    .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b)),
            "INVALID_BLOB_DECLARATION"
        );
        let destination = self.path(&blob.id)?;
        let lock = self.object_lock(&blob.id).await;
        let _guard = lock.lock().await;
        if let Some((existing, state)) = self.row(owner, &blob.id).await? {
            ensure!(
                existing == *blob && state != "deleted",
                "IMMUTABLE_OBJECT_CONFLICT"
            );
            if state == "ready" || tokio::fs::try_exists(destination).await? {
                return Ok(());
            }
        } else {
            sqlx::query(
                "INSERT INTO blobs(id,owner,size_bytes,sha256,state) VALUES(?,?,?,?,'writing')",
            )
            .bind(&blob.id)
            .bind(owner)
            .bind(blob.size_bytes as i64)
            .bind(&blob.sha256)
            .execute(&self.db)
            .await?;
        }
        let staging = self.root.join("staging").join(&blob.id);
        let _allocation = self.allocation.lock().await;
        let existing = tokio::fs::metadata(&staging)
            .await
            .map(|m| m.len())
            .unwrap_or(0);
        ensure!(
            self.capacity()
                .await?
                .writable_bytes()
                .saturating_add(existing)
                >= blob.size_bytes,
            "VOLUME_FULL"
        );
        let file = tokio::fs::OpenOptions::new()
            .create(true)
            .truncate(false)
            .read(true)
            .write(true)
            .open(staging)
            .await?
            .into_std()
            .await;
        let size = blob.size_bytes;
        tokio::task::spawn_blocking(move || {
            file.allocate(size)
                .context("VOLUME_FULL: preallocation failed")
        })
        .await??;
        Ok(())
    }
    async fn stat(&self, owner: &str, id: &str) -> Result<Option<Blob>> {
        let lock = self.object_lock(id).await;
        let _guard = lock.lock().await;
        let Some((blob, state)) = self.row(owner, id).await? else {
            return Ok(None);
        };
        if state == "deleted" {
            return Ok(None);
        }
        let path = self.path(id)?;
        if state == "writing" {
            // Recover a rename that reached disk before its SQLite receipt. Staging bytes
            // alone never count as a committed object.
            let mut file = match tokio::fs::File::open(&path).await {
                Ok(file) => file,
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
                Err(e) => return Err(e.into()),
            };
            let mut sha = Sha256::new();
            let mut size = 0u64;
            let mut buffer = [0u8; 65536];
            loop {
                let n = file.read(&mut buffer).await?;
                if n == 0 {
                    break;
                }
                size += n as u64;
                sha.update(&buffer[..n]);
            }
            ensure!(
                size == blob.size_bytes && hex::encode(sha.finalize()) == blob.sha256,
                "DATA_MISSING: incomplete immutable blob"
            );
            file.sync_all().await?;
            Self::sync_directory(self.root.join("blobs")).await?;
            sqlx::query("UPDATE blobs SET state='ready' WHERE id=?")
                .bind(id)
                .execute(&self.db)
                .await?;
        }
        ensure!(
            tokio::fs::metadata(path)
                .await
                .context("DATA_MISSING")?
                .len()
                == blob.size_bytes,
            "DATA_MISSING"
        );
        Ok(Some(blob))
    }
    async fn source(&self, owner: &str, id: &str) -> Result<ReadSource> {
        ensure!(self.stat(owner, id).await?.is_some(), "DATA_MISSING");
        Ok(ReadSource::Local {
            path: self.path(id)?,
        })
    }
    async fn delete(&self, owner: &str, id: &str) -> Result<()> {
        let path = self.path(id)?;
        let lock = self.object_lock(id).await;
        let _guard = lock.lock().await;
        if let Some((_, state)) = self.row(owner, id).await? {
            if state == "deleted" {
                return Ok(());
            }
            for target in [path, self.root.join("staging").join(id)] {
                match tokio::fs::remove_file(target).await {
                    Ok(()) => (),
                    Err(e) if e.kind() == std::io::ErrorKind::NotFound => (),
                    Err(e) => return Err(e.into()),
                }
            }
            Self::sync_directory(self.root.join("blobs")).await?;
            Self::sync_directory(self.root.join("staging")).await?;
            sqlx::query("UPDATE blobs SET state='deleted' WHERE id=?")
                .bind(id)
                .execute(&self.db)
                .await?;
        } else {
            // Tombstones make object IDs permanently non-reusable, including delete-before-put retries.
            sqlx::query(
                "INSERT INTO blobs(id,owner,size_bytes,sha256,state) VALUES(?,?,0,'','deleted')",
            )
            .bind(id)
            .bind(owner)
            .execute(&self.db)
            .await?;
        }
        Ok(())
    }
}
