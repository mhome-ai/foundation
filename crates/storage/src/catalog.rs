use crate::*;
use anyhow::{bail, ensure, Context, Result};
use sqlx::{
    sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions, SqliteSynchronous},
    Row, SqlitePool,
};
use std::{
    collections::HashMap,
    path::PathBuf,
    sync::{Arc, Weak},
};
use tokio::sync::Mutex;
use unicode_normalization::UnicodeNormalization;

pub struct Catalog {
    db: SqlitePool,
    internal: Arc<dyn Backend>,
    resolver: Arc<dyn Resolver>,
    locks: Mutex<HashMap<String, Weak<Mutex<()>>>>,
}
impl std::fmt::Debug for Catalog {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Catalog").finish_non_exhaustive()
    }
}
impl Catalog {
    pub async fn open(
        path: impl Into<PathBuf>,
        internal: Arc<dyn Backend>,
        resolver: Arc<dyn Resolver>,
    ) -> Result<Arc<Self>> {
        let path = path.into();
        tokio::fs::create_dir_all(path.parent().context("catalog path requires parent")?).await?;
        let db = SqlitePoolOptions::new()
            .max_connections(1)
            .connect_with(
                SqliteConnectOptions::new()
                    .filename(path)
                    .create_if_missing(true)
                    .foreign_keys(true)
                    .journal_mode(SqliteJournalMode::Wal)
                    .synchronous(SqliteSynchronous::Full),
            )
            .await?;
        sqlx::migrate!("./migrations/catalog").run(&db).await?;
        Ok(Arc::new(Self {
            db,
            internal,
            resolver,
            locks: Mutex::new(HashMap::new()),
        }))
    }
    async fn lock(&self, key: &str) -> Arc<Mutex<()>> {
        let mut locks = self.locks.lock().await;
        locks.retain(|_, v| v.strong_count() > 0);
        if let Some(lock) = locks.get(key).and_then(Weak::upgrade) {
            return lock;
        }
        let lock = Arc::new(Mutex::new(()));
        locks.insert(key.to_owned(), Arc::downgrade(&lock));
        lock
    }
    async fn admit(&self, tenant: &str, scope: &str) -> Result<()> {
        ensure!(
            sqlx::query_scalar::<_, i64>(
                "SELECT COUNT(*) FROM fences WHERE tenant_id=? AND scope_id=?"
            )
            .bind(tenant)
            .bind(scope)
            .fetch_one(&self.db)
            .await?
                == 0,
            "SCOPE_REMOVING"
        );
        Ok(())
    }
    pub async fn fence(&self, tenant: &str, scope: &str) -> Result<()> {
        sqlx::query("INSERT OR IGNORE INTO fences VALUES(?,?)")
            .bind(tenant)
            .bind(scope)
            .execute(&self.db)
            .await?;
        Ok(())
    }
    pub async fn activate_scope(&self, tenant: &str, scope: &str) -> Result<()> {
        let fenced = sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM fences WHERE tenant_id=? AND scope_id=?",
        )
        .bind(tenant)
        .bind(scope)
        .fetch_one(&self.db)
        .await?;
        if fenced > 0 {
            ensure!(
                sqlx::query_scalar::<_, i64>(
                    "SELECT COUNT(*) FROM folders WHERE tenant_id=? AND scope_id=?"
                )
                .bind(tenant)
                .bind(scope)
                .fetch_one(&self.db)
                .await?
                    == 0,
                "Previous storage removal is incomplete"
            );
            sqlx::query("DELETE FROM fences WHERE tenant_id=? AND scope_id=?")
                .bind(tenant)
                .bind(scope)
                .execute(&self.db)
                .await?;
        }
        Ok(())
    }
    /// Resolve durable receipts, without assuming that an offline backend lost its bytes.
    pub async fn recover(&self) -> Result<()> {
        let rows=sqlx::query("SELECT o.*,f.tenant_id,f.scope_id FROM operations o JOIN folders f ON f.id=o.folder_id WHERE o.state IN ('prepared','staged','cleanup') ORDER BY o.created_at_ms").fetch_all(&self.db).await?;
        for row in rows {
            let id: String = row.get("id");
            let lock = self.lock(&format!("operation:{id}")).await;
            let Ok(_guard) = lock.try_lock() else {
                continue;
            };
            let Some(current) =
                sqlx::query("SELECT state,object_json,victims_json FROM operations WHERE id=?")
                    .bind(&id)
                    .fetch_optional(&self.db)
                    .await?
            else {
                continue;
            };
            let state: String = current.get("state");
            if !["prepared", "staged", "cleanup"].contains(&state.as_str()) {
                continue;
            }
            let tenant: String = row.get("tenant_id");
            let scope: String = row.get("scope_id");
            let object: Object = serde_json::from_str(current.get("object_json"))?;
            if state == "cleanup" {
                let _ = self.cleanup_operation(&tenant, &scope, &id, &object).await;
                continue;
            }
            if self.admit(&tenant, &scope).await.is_err() {
                continue;
            }
            let Ok(backend) = self.backend(&tenant, &scope, &object.repository_id).await else {
                continue;
            };
            match backend.stat(&object.owner, &object.object_id).await {
                Ok(Some(blob))
                    if blob.size_bytes == object.size_bytes && blob.sha256 == object.sha256 =>
                {
                    let _ = self
                        .finish_operation(
                            &tenant,
                            &scope,
                            &id,
                            object,
                            serde_json::from_str(current.get("victims_json"))?,
                            Input::Bytes(Arc::new(Vec::new())),
                        )
                        .await;
                }
                Ok(None)
                    if now_ms().saturating_sub(row.get::<i64, _>("created_at_ms") as u64)
                        > 30 * 60 * 1000 =>
                {
                    self.mark_cleanup(&id, "upload expired").await?;
                    let _ = self.cleanup_operation(&tenant, &scope, &id, &object).await;
                }
                _ => (),
            }
        }
        // A failed deletion is only completed here when its physical result is already confirmed.
        let rows=sqlx::query("SELECT o.*,f.tenant_id,f.scope_id,d.eviction FROM deletions d JOIN objects o ON o.object_id=d.object_id JOIN folders f ON f.id=o.folder_id").fetch_all(&self.db).await?;
        for row in rows {
            let tenant: String = row.get("tenant_id");
            let scope: String = row.get("scope_id");
            let eviction: bool = row.get("eviction");
            let object = object_row(row);
            if let Ok(backend) = self.backend(&tenant, &scope, &object.repository_id).await {
                if backend
                    .stat(&object.owner, &object.object_id)
                    .await
                    .is_ok_and(|blob| blob.is_none())
                {
                    let _ = self
                        .delete_physical(&tenant, &scope, &object, eviction)
                        .await;
                }
            }
        }
        // The object row is the durable create receipt. Completed journals need not grow with every historic write.
        sqlx::query(
            "DELETE FROM operations WHERE state IN ('published','failed') AND created_at_ms<?",
        )
        .bind(now_ms().saturating_sub(24 * 60 * 60 * 1000) as i64)
        .execute(&self.db)
        .await?;
        Ok(())
    }
    pub async fn create_folder(
        &self,
        tenant: &str,
        scope: &str,
        name: &str,
        max_bytes: u64,
        overflow: Overflow,
    ) -> Result<Folder> {
        self.admit(tenant, scope).await?;
        validate_name(name)?;
        ensure!(normalized(name) != "artifact", "SYSTEM_FOLDER_RESERVED");
        ensure!(
            max_bytes > 0 && max_bytes <= i64::MAX as u64,
            "INVALID_QUOTA"
        );
        self.insert_folder(tenant, scope, name, Some(max_bytes), overflow, false)
            .await
    }
    /// Bound applications may provision one ordinary Folder under their server-derived identity.
    pub async fn consumer_folder(
        &self,
        tenant: &str,
        scope: &str,
        owner: &str,
        name: &str,
        max_bytes: u64,
    ) -> Result<Folder> {
        self.admit(tenant, scope).await?;
        validate_name(name)?;
        ensure!(
            normalized(name) != "artifact" && max_bytes > 0 && max_bytes <= i64::MAX as u64,
            "INVALID_CONSUMER_FOLDER"
        );
        let lock = self
            .lock(&format!("consumer:{tenant}:{scope}:{owner}"))
            .await;
        let _guard = lock.lock().await;
        if let Some(id) = sqlx::query_scalar::<_, String>(
            "SELECT folder_id FROM consumers WHERE tenant_id=? AND scope_id=? AND owner=?",
        )
        .bind(tenant)
        .bind(scope)
        .bind(owner)
        .fetch_optional(&self.db)
        .await?
        {
            return self.folder(tenant, scope, &id).await;
        }
        let id = uuid::Uuid::new_v4().to_string();
        let now = now_ms() as i64;
        let mut tx = self.db.begin().await?;
        ensure!(
            sqlx::query_scalar::<_, i64>(
                "SELECT COUNT(*) FROM fences WHERE tenant_id=? AND scope_id=?"
            )
            .bind(tenant)
            .bind(scope)
            .fetch_one(&mut *tx)
            .await?
                == 0,
            "SCOPE_REMOVING"
        );
        let mut display_name = name.to_owned();
        let mut suffix = 1u64;
        while sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM folders WHERE tenant_id=? AND scope_id=? AND normalized_name=?",
        )
        .bind(tenant)
        .bind(scope)
        .bind(normalized(&display_name))
        .fetch_one(&mut *tx)
        .await?
            > 0
        {
            suffix += 1;
            let ending = format!(" ({suffix})");
            display_name = format!(
                "{}{}",
                name.chars().take(80 - ending.len()).collect::<String>(),
                ending
            );
        }
        sqlx::query("INSERT INTO folders(id,tenant_id,scope_id,name,normalized_name,max_bytes,overflow_strategy,created_at_ms,updated_at_ms) VALUES(?,?,?,?,?,?,'reject',?,?)").bind(&id).bind(tenant).bind(scope).bind(&display_name).bind(normalized(&display_name)).bind(max_bytes as i64).bind(now).bind(now).execute(&mut *tx).await.context("FOLDER_NAME_CONFLICT")?;
        sqlx::query("INSERT INTO consumers(tenant_id,scope_id,owner,folder_id) VALUES(?,?,?,?)")
            .bind(tenant)
            .bind(scope)
            .bind(owner)
            .bind(&id)
            .execute(&mut *tx)
            .await?;
        tx.commit().await?;
        self.folder(tenant, scope, &id).await
    }
    async fn insert_folder(
        &self,
        tenant: &str,
        scope: &str,
        name: &str,
        max_bytes: Option<u64>,
        overflow: Overflow,
        system: bool,
    ) -> Result<Folder> {
        let id = uuid::Uuid::new_v4().to_string();
        let now = now_ms() as i64;
        let mut tx = self.db.begin().await?;
        ensure!(
            sqlx::query_scalar::<_, i64>(
                "SELECT COUNT(*) FROM fences WHERE tenant_id=? AND scope_id=?"
            )
            .bind(tenant)
            .bind(scope)
            .fetch_one(&mut *tx)
            .await?
                == 0,
            "SCOPE_REMOVING"
        );
        sqlx::query("INSERT INTO folders(id,tenant_id,scope_id,name,normalized_name,system,max_bytes,overflow_strategy,created_at_ms,updated_at_ms) VALUES(?,?,?,?,?,?,?,?,?,?)")
            .bind(&id).bind(tenant).bind(scope).bind(name).bind(normalized(name)).bind(system).bind(max_bytes.map(|n| n as i64)).bind(if overflow==Overflow::Reject {"reject"} else {"evictOldest"}).bind(now).bind(now).execute(&mut *tx).await.context("FOLDER_NAME_CONFLICT")?;
        tx.commit().await?;
        self.folder(tenant, scope, &id).await
    }
    pub async fn artifact_folder(&self, tenant: &str, scope: &str) -> Result<Folder> {
        self.admit(tenant, scope).await?;
        let lock = self.lock(&format!("system:{tenant}:{scope}")).await;
        let _guard = lock.lock().await;
        if let Some(id) = sqlx::query_scalar::<_, String>(
            "SELECT id FROM folders WHERE tenant_id=? AND scope_id=? AND system=1",
        )
        .bind(tenant)
        .bind(scope)
        .fetch_optional(&self.db)
        .await?
        {
            return self.folder(tenant, scope, &id).await;
        }
        self.insert_folder(tenant, scope, "artifact", None, Overflow::Reject, true)
            .await
    }
    pub async fn folder(&self, tenant: &str, scope: &str, id: &str) -> Result<Folder> {
        self.admit(tenant, scope).await?;
        let row = sqlx::query("SELECT f.*,COALESCE((SELECT SUM(size_bytes) FROM objects WHERE folder_id=f.id),0) AS used_bytes,(SELECT COUNT(*) FROM objects WHERE folder_id=f.id) AS object_count FROM folders f WHERE id=? AND tenant_id=? AND scope_id=?")
            .bind(id).bind(tenant).bind(scope).fetch_optional(&self.db).await?.context("FOLDER_NOT_FOUND")?;
        let reserved = sqlx::query_scalar::<_,String>("SELECT object_json FROM operations WHERE folder_id=? AND state IN ('prepared','staged')").bind(id).fetch_all(&self.db).await?;
        let reserved_bytes = reserved
            .into_iter()
            .map(|s| serde_json::from_str::<Object>(&s).map(|o| o.size_bytes))
            .collect::<std::result::Result<Vec<_>, _>>()?
            .into_iter()
            .sum();
        let n = |key| row.get::<i64, _>(key) as u64;
        Ok(Folder {
            id: row.get("id"),
            tenant_id: row.get("tenant_id"),
            scope_id: row.get("scope_id"),
            name: row.get("name"),
            system: row.get("system"),
            max_bytes: row.get::<Option<i64>, _>("max_bytes").map(|n| n as u64),
            overflow_strategy: if row.get::<String, _>("overflow_strategy") == "reject" {
                Overflow::Reject
            } else {
                Overflow::EvictOldest
            },
            revision: n("revision"),
            created_at_ms: n("created_at_ms"),
            updated_at_ms: n("updated_at_ms"),
            used_bytes: n("used_bytes"),
            reserved_bytes,
            object_count: n("object_count"),
            evicted_object_count: n("evicted_object_count"),
            evicted_bytes: n("evicted_bytes"),
            rejected_write_count: n("rejected_write_count"),
        })
    }
    pub async fn list_folders(&self, tenant: &str, scope: &str) -> Result<Vec<Folder>> {
        self.admit(tenant, scope).await?;
        let ids = sqlx::query_scalar::<_,String>("SELECT id FROM folders WHERE tenant_id=? AND scope_id=? ORDER BY system DESC,normalized_name,id").bind(tenant).bind(scope).fetch_all(&self.db).await?;
        let mut result = Vec::new();
        for id in ids {
            result.push(self.folder(tenant, scope, &id).await?);
        }
        Ok(result)
    }
    pub async fn update_folder(
        &self,
        tenant: &str,
        scope: &str,
        id: &str,
        revision: u64,
        name: &str,
        max_bytes: u64,
        overflow: Overflow,
    ) -> Result<Folder> {
        self.admit(tenant, scope).await?;
        validate_name(name)?;
        ensure!(normalized(name) != "artifact", "SYSTEM_FOLDER_RESERVED");
        ensure!(
            max_bytes > 0 && max_bytes <= i64::MAX as u64,
            "INVALID_QUOTA"
        );
        let lock = self.lock(id).await;
        let _guard = lock.lock().await;
        let f = self.folder(tenant, scope, id).await?;
        ensure!(!f.system, "SYSTEM_FOLDER_PROTECTED");
        ensure!(f.revision == revision, "FOLDER_REVISION_CONFLICT");
        ensure!(sqlx::query_scalar::<_,i64>("SELECT COUNT(*) FROM operations WHERE folder_id=? AND state IN ('prepared','staged')").bind(id).fetch_one(&self.db).await? == 0, "FOLDER_BUSY");
        ensure!(f.used_bytes <= max_bytes, "FOLDER_QUOTA_TOO_SMALL");
        sqlx::query("UPDATE folders SET name=?,normalized_name=?,max_bytes=?,overflow_strategy=?,revision=revision+1,updated_at_ms=? WHERE id=?")
            .bind(name).bind(normalized(name)).bind(max_bytes as i64).bind(if overflow==Overflow::Reject {"reject"} else {"evictOldest"}).bind(now_ms() as i64).bind(id).execute(&self.db).await.context("FOLDER_NAME_CONFLICT")?;
        self.folder(tenant, scope, id).await
    }
    pub async fn object(
        &self,
        tenant: &str,
        scope: &str,
        folder: &str,
        key: &str,
    ) -> Result<Option<Object>> {
        self.admit(tenant, scope).await?;
        self.folder(tenant, scope, folder).await?;
        self.object_unchecked(folder, key).await
    }
    async fn object_unchecked(&self, folder: &str, key: &str) -> Result<Option<Object>> {
        Ok(
            sqlx::query("SELECT * FROM objects WHERE folder_id=? AND key=?")
                .bind(folder)
                .bind(key)
                .fetch_optional(&self.db)
                .await?
                .map(object_row),
        )
    }
    pub async fn list_objects(
        &self,
        tenant: &str,
        scope: &str,
        folder: &str,
        prefix: &str,
        after: Option<&str>,
        limit: usize,
    ) -> Result<Vec<Object>> {
        self.admit(tenant, scope).await?;
        self.folder(tenant, scope, folder).await?;
        Ok(sqlx::query("SELECT * FROM objects WHERE folder_id=? AND substr(key,1,length(?))=? AND (? IS NULL OR key>?) ORDER BY key LIMIT ?")
            .bind(folder).bind(prefix).bind(prefix).bind(after).bind(after).bind(limit.clamp(1,1001) as i64).fetch_all(&self.db).await?.into_iter().map(object_row).collect())
    }
    async fn backend(
        &self,
        tenant: &str,
        scope: &str,
        repository: &str,
    ) -> Result<Arc<dyn Backend>> {
        if repository == self.internal.repository_id() {
            return Ok(self.internal.clone());
        }
        let handle = sqlx::query_scalar::<_, String>(
            "SELECT handle FROM backends WHERE tenant_id=? AND scope_id=? AND repository_id=?",
        )
        .bind(tenant)
        .bind(scope)
        .bind(repository)
        .fetch_optional(&self.db)
        .await?
        .context("BACKEND_UNAVAILABLE")?;
        let backend = self
            .resolver
            .resolve(tenant, scope, repository, &handle)
            .await
            .context("BACKEND_UNAVAILABLE")?;
        ensure!(
            backend.repository_id() == repository,
            "REPOSITORY_IDENTITY_MISMATCH"
        );
        Ok(backend)
    }
    pub async fn capacities(
        &self,
        tenant: &str,
        scope: &str,
    ) -> Result<Vec<(String, Option<Capacity>)>> {
        let mut result = vec![("internal".into(), Some(self.internal.capacity().await?))];
        if let Some((external, handle)) = self.resolver.preferred(tenant, scope).await? {
            self.remember(tenant, scope, external.repository_id(), &handle)
                .await?;
        }
        let ids=sqlx::query_scalar::<_,String>("SELECT repository_id FROM backends WHERE tenant_id=? AND scope_id=? ORDER BY repository_id").bind(tenant).bind(scope).fetch_all(&self.db).await?;
        for id in ids {
            let capacity = match self.backend(tenant, scope, &id).await {
                Ok(b) => b.capacity().await.ok(),
                Err(_) => None,
            };
            result.push((id, capacity));
        }
        Ok(result)
    }
    pub async fn usage(&self) -> Result<Usage> {
        let used = sqlx::query(
            "SELECT COALESCE(SUM(size_bytes),0) AS bytes,COUNT(*) AS count FROM objects",
        )
        .fetch_one(&self.db)
        .await?;
        let pending = sqlx::query_scalar::<_, String>(
            "SELECT object_json FROM operations WHERE state IN ('prepared','staged','cleanup')",
        )
        .fetch_all(&self.db)
        .await?;
        let count = pending.len() as u64;
        let reserved = pending
            .into_iter()
            .map(|s| serde_json::from_str::<Object>(&s).map(|o| o.size_bytes))
            .collect::<std::result::Result<Vec<_>, _>>()?
            .into_iter()
            .sum();
        Ok(Usage {
            used_bytes: used.get::<i64, _>("bytes") as u64,
            reserved_bytes: reserved,
            object_count: used.get::<i64, _>("count") as u64,
            folder_count: sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM folders")
                .fetch_one(&self.db)
                .await? as u64,
            active_upload_count: count,
            pending_deletion_count: sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM deletions")
                .fetch_one(&self.db)
                .await? as u64,
        })
    }
    pub async fn owns_physical(
        &self,
        tenant: &str,
        scope: &str,
        repository: &str,
        owner: &str,
        id: &str,
    ) -> Result<bool> {
        let count=sqlx::query_scalar::<_,i64>("SELECT COUNT(*) FROM objects o JOIN folders f ON f.id=o.folder_id WHERE f.tenant_id=? AND f.scope_id=? AND o.repository_id=? AND o.owner=? AND o.object_id=?").bind(tenant).bind(scope).bind(repository).bind(owner).bind(id).fetch_one(&self.db).await?;
        if count > 0 {
            return Ok(true);
        }
        for row in sqlx::query_scalar::<_,String>("SELECT o.object_json FROM operations o JOIN folders f ON f.id=o.folder_id WHERE f.tenant_id=? AND f.scope_id=? AND o.state IN ('prepared','staged','cleanup')").bind(tenant).bind(scope).fetch_all(&self.db).await?{let o:Object=serde_json::from_str(&row)?;if o.repository_id==repository&&o.owner==owner&&o.object_id==id{return Ok(true)}}
        Ok(false)
    }
    async fn remember(
        &self,
        tenant: &str,
        scope: &str,
        repository: &str,
        handle: &str,
    ) -> Result<()> {
        sqlx::query("INSERT INTO backends(tenant_id,scope_id,repository_id,handle) VALUES(?,?,?,?) ON CONFLICT DO UPDATE SET handle=excluded.handle").bind(tenant).bind(scope).bind(repository).bind(handle).execute(&self.db).await?;
        Ok(())
    }
    /// Immutable create. An unfinished operation for this key is resolved before another repository is considered.
    pub async fn put(
        self: &Arc<Self>,
        tenant: &str,
        scope: &str,
        folder: &str,
        key: &str,
        size: u64,
        sha: &str,
        content_type: Option<&str>,
        input: Input,
    ) -> Result<Object> {
        let catalog = self.clone();
        let tenant = tenant.to_owned();
        let scope = scope.to_owned();
        let folder = folder.to_owned();
        let key = key.to_owned();
        let sha = sha.to_owned();
        let content_type = content_type.map(str::to_owned);
        // Keep file mutation and its receipt in one owned task, even when a request disconnects.
        tokio::spawn(async move {
            catalog
                .put_inner(
                    &tenant,
                    &scope,
                    &folder,
                    &key,
                    size,
                    &sha,
                    content_type.as_deref(),
                    input,
                )
                .await
        })
        .await?
    }
    async fn put_inner(
        &self,
        tenant: &str,
        scope: &str,
        folder: &str,
        key: &str,
        size: u64,
        sha: &str,
        content_type: Option<&str>,
        input: Input,
    ) -> Result<Object> {
        self.admit(tenant, scope).await?;
        ensure!(
            !key.is_empty()
                && key.len() <= 1024
                && size <= i64::MAX as u64
                && sha.len() == 64
                && sha
                    .bytes()
                    .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b)),
            "INVALID_OBJECT"
        );
        self.folder(tenant, scope, folder).await?;
        if let Some(object) = self.object_unchecked(folder, key).await? {
            ensure!(
                object.size_bytes == size && object.sha256 == sha,
                "OBJECT_EXISTS"
            );
            return Ok(object);
        }
        if let Some(id)=sqlx::query_scalar::<_,String>("SELECT id FROM operations WHERE folder_id=? AND key=? AND state IN ('prepared','staged')").bind(folder).bind(key).fetch_optional(&self.db).await?{return self.run_operation(tenant,scope,&id,size,sha,input).await}
        // Resolve the target before reserving Folder quota; no Folder lock spans discovery or transport.
        let mut target = None;
        if let Some((external, handle)) = self.resolver.preferred(tenant, scope).await? {
            if external
                .capacity()
                .await
                .is_ok_and(|c| c.writable_bytes() >= size)
            {
                self.remember(tenant, scope, external.repository_id(), &handle)
                    .await?;
                target = Some(external);
            }
        }
        let target = target.unwrap_or_else(|| self.internal.clone());
        let lock = self.lock(folder).await;
        let guard = lock.lock().await;
        let f = self.folder(tenant, scope, folder).await?;
        if let Some(object) = self.object_unchecked(folder, key).await? {
            ensure!(
                object.size_bytes == size && object.sha256 == sha,
                "OBJECT_EXISTS"
            );
            return Ok(object);
        }
        let existing=sqlx::query_scalar::<_,String>("SELECT id FROM operations WHERE folder_id=? AND key=? AND state IN ('prepared','staged')").bind(folder).bind(key).fetch_optional(&self.db).await?;
        let id = if let Some(id) = existing {
            id
        } else {
            let mut tx = self.db.begin().await?;
            ensure!(
                sqlx::query_scalar::<_, i64>(
                    "SELECT COUNT(*) FROM fences WHERE tenant_id=? AND scope_id=?"
                )
                .bind(tenant)
                .bind(scope)
                .fetch_one(&mut *tx)
                .await?
                    == 0,
                "SCOPE_REMOVING"
            );
            let used = sqlx::query_scalar::<_, i64>(
                "SELECT COALESCE(SUM(size_bytes),0) FROM objects WHERE folder_id=?",
            )
            .bind(folder)
            .fetch_one(&mut *tx)
            .await? as u64;
            // An operation may reserve its own victims, but cannot lend their excess bytes to
            // another writer before physical deletion completes.
            let reservations = sqlx::query("SELECT p.object_json,COALESCE((SELECT SUM(o.size_bytes) FROM eviction_claims c JOIN objects o ON o.object_id=c.object_id WHERE c.operation_id=p.id),0) AS claimed FROM operations p WHERE p.folder_id=? AND p.state IN ('prepared','staged')").bind(folder).fetch_all(&mut *tx).await?;
            let mut reserved = 0u64;
            for row in reservations {
                let incoming: Object = serde_json::from_str(row.get("object_json"))?;
                reserved = reserved.saturating_add(
                    incoming
                        .size_bytes
                        .saturating_sub(row.get::<i64, _>("claimed") as u64),
                );
            }
            let mut needed = used
                .saturating_add(reserved)
                .saturating_add(size)
                .saturating_sub(f.max_bytes.unwrap_or(u64::MAX));
            let mut victims = Vec::new();
            if needed > 0 {
                if f.overflow_strategy == Overflow::Reject || size > f.max_bytes.unwrap_or(u64::MAX)
                {
                    sqlx::query(
                        "UPDATE folders SET rejected_write_count=rejected_write_count+1 WHERE id=?",
                    )
                    .bind(folder)
                    .execute(&mut *tx)
                    .await?;
                    tx.commit().await?;
                    bail!("QUOTA_EXCEEDED")
                }
                for object in sqlx::query("SELECT o.* FROM objects o WHERE o.folder_id=? AND NOT EXISTS(SELECT 1 FROM eviction_claims c WHERE c.object_id=o.object_id) ORDER BY o.committed_at_ms,o.object_id").bind(folder).fetch_all(&mut *tx).await?.into_iter().map(object_row) {
                    needed = needed.saturating_sub(object.size_bytes);
                    victims.push(object);
                    if needed == 0 {break}
                }
                if needed > 0 {
                    sqlx::query(
                        "UPDATE folders SET rejected_write_count=rejected_write_count+1 WHERE id=?",
                    )
                    .bind(folder)
                    .execute(&mut *tx)
                    .await?;
                    tx.commit().await?;
                    bail!("QUOTA_EXCEEDED")
                }
            }
            let object = Object {
                object_id: uuid::Uuid::new_v4().to_string(),
                folder_id: folder.into(),
                key: key.into(),
                repository_id: target.repository_id().into(),
                owner: folder.into(),
                size_bytes: size,
                sha256: sha.into(),
                content_type: content_type.map(str::to_owned),
                committed_at_ms: now_ms(),
            };
            let id = uuid::Uuid::new_v4().to_string();
            sqlx::query("INSERT INTO operations(id,folder_id,key,object_json,victims_json,state,created_at_ms) VALUES(?,?,?,?,?,'prepared',?)").bind(&id).bind(folder).bind(key).bind(serde_json::to_string(&object)?).bind(serde_json::to_string(&victims)?).bind(now_ms() as i64).execute(&mut *tx).await?;
            for victim in victims {
                sqlx::query("INSERT INTO eviction_claims(object_id,operation_id) VALUES(?,?)")
                    .bind(victim.object_id)
                    .bind(&id)
                    .execute(&mut *tx)
                    .await?;
            }
            tx.commit().await?;
            id
        };
        drop(guard);
        self.run_operation(tenant, scope, &id, size, sha, input)
            .await
    }
    async fn run_operation(
        &self,
        tenant: &str,
        scope: &str,
        id: &str,
        size: u64,
        sha: &str,
        input: Input,
    ) -> Result<Object> {
        let operation = self.lock(&format!("operation:{id}")).await;
        let _operation = operation.lock().await;
        let row = sqlx::query("SELECT object_json,victims_json,state FROM operations WHERE id=?")
            .bind(&id)
            .fetch_optional(&self.db)
            .await?
            .context("OPERATION_NOT_FOUND")?;
        let object: Object = serde_json::from_str(row.get("object_json"))?;
        ensure!(
            object.size_bytes == size && object.sha256 == sha,
            "OPERATION_CONFLICT"
        );
        let state: String = row.get("state");
        if state == "published" {
            return Ok(object);
        }
        ensure!(
            ["prepared", "staged"].contains(&state.as_str()),
            "OPERATION_FAILED"
        );
        self.finish_operation(
            tenant,
            scope,
            &id,
            object,
            serde_json::from_str(row.get("victims_json"))?,
            input,
        )
        .await
    }
    async fn mark_cleanup(&self, id: &str, error: &str) -> Result<()> {
        let mut tx = self.db.begin().await?;
        sqlx::query("UPDATE operations SET state='cleanup',error=? WHERE id=?")
            .bind(error)
            .bind(id)
            .execute(&mut *tx)
            .await?;
        sqlx::query("DELETE FROM eviction_claims WHERE operation_id=?")
            .bind(id)
            .execute(&mut *tx)
            .await?;
        tx.commit().await?;
        Ok(())
    }
    async fn finish_operation(
        &self,
        tenant: &str,
        scope: &str,
        id: &str,
        object: Object,
        victims: Vec<Object>,
        input: Input,
    ) -> Result<Object> {
        let result = self
            .execute_operation(tenant, scope, id, object, victims, input)
            .await;
        if let Err(error) = &result {
            if [
                "VOLUME_FULL",
                "SHA_MISMATCH",
                "SIZE_MISMATCH",
                "INVALID_BLOB",
                "BACKEND_REJECTED",
            ]
            .iter()
            .any(|tag| error.to_string().contains(tag))
            {
                let json = sqlx::query_scalar::<_, String>(
                    "SELECT object_json FROM operations WHERE id=?",
                )
                .bind(id)
                .fetch_one(&self.db)
                .await?;
                let object: Object = serde_json::from_str(&json)?;
                self.mark_cleanup(id, &error.to_string()).await?;
                let _ = self.cleanup_operation(tenant, scope, id, &object).await;
            }
        }
        result
    }
    async fn execute_operation(
        &self,
        tenant: &str,
        scope: &str,
        id: &str,
        mut object: Object,
        victims: Vec<Object>,
        input: Input,
    ) -> Result<Object> {
        let mut backend = self.backend(tenant, scope, &object.repository_id).await?;
        let mut blob = Blob {
            id: object.object_id.clone(),
            size_bytes: object.size_bytes,
            sha256: object.sha256.clone(),
        };
        if backend
            .stat(&object.owner, &object.object_id)
            .await?
            .as_ref()
            != Some(&blob)
        {
            if let Err(error) = backend.prepare(&object.owner, &blob).await {
                if error.to_string().contains("VOLUME_FULL")
                    && object.repository_id != self.internal.repository_id()
                {
                    // A confirmed failed preallocation has consumed no input. Retire the old physical ID before fallback.
                    backend.delete(&object.owner, &object.object_id).await?;
                    object.object_id = uuid::Uuid::new_v4().to_string();
                    object.repository_id = self.internal.repository_id().into();
                    blob.id = object.object_id.clone();
                    backend = self.internal.clone();
                    sqlx::query("UPDATE operations SET object_json=? WHERE id=?")
                        .bind(serde_json::to_string(&object)?)
                        .bind(id)
                        .execute(&self.db)
                        .await?;
                    backend.prepare(&object.owner, &blob).await?;
                } else {
                    return Err(error);
                }
            }
            // Transport uncertainty keeps this operation and repository pinned for an idempotent retry.
            backend
                .put(&object.owner, &blob, input.reader().await?)
                .await?;
        }
        sqlx::query("UPDATE operations SET state='staged' WHERE id=?")
            .bind(id)
            .execute(&self.db)
            .await?;
        for victim in victims {
            if self
                .object_unchecked(&victim.folder_id, &victim.key)
                .await?
                .is_none()
            {
                continue;
            }
            if let Err(error) = self.delete_physical(tenant, scope, &victim, true).await {
                self.mark_cleanup(id, &error.to_string()).await?;
                // Successful earlier evictions remain applied; the incoming object is not published.
                let _ = self.cleanup_operation(tenant, scope, id, &object).await;
                return Err(error
                    .context("EVICTION_FAILED: earlier per-object deletions may have completed"));
            }
        }
        object.committed_at_ms = now_ms();
        let folder_lock = self.lock(&object.folder_id).await;
        let _publish = folder_lock.lock().await;
        self.admit(tenant, scope).await?;
        let mut tx = self.db.begin().await?;
        ensure!(
            sqlx::query_scalar::<_, i64>(
                "SELECT COUNT(*) FROM fences WHERE tenant_id=? AND scope_id=?"
            )
            .bind(tenant)
            .bind(scope)
            .fetch_one(&mut *tx)
            .await?
                == 0,
            "SCOPE_REMOVING"
        );
        sqlx::query("INSERT INTO objects(object_id,folder_id,key,repository_id,owner,size_bytes,sha256,content_type,committed_at_ms) VALUES(?,?,?,?,?,?,?,?,?)")
            .bind(&object.object_id).bind(&object.folder_id).bind(&object.key).bind(&object.repository_id).bind(&object.owner).bind(object.size_bytes as i64).bind(&object.sha256).bind(&object.content_type).bind(object.committed_at_ms as i64).execute(&mut *tx).await?;
        sqlx::query("UPDATE operations SET state='published',object_json=? WHERE id=?")
            .bind(serde_json::to_string(&object)?)
            .bind(id)
            .execute(&mut *tx)
            .await?;
        tx.commit().await?;
        Ok(object)
    }
    async fn cleanup_operation(
        &self,
        tenant: &str,
        scope: &str,
        id: &str,
        object: &Object,
    ) -> Result<()> {
        self.backend(tenant, scope, &object.repository_id)
            .await?
            .delete(&object.owner, &object.object_id)
            .await?;
        let mut tx = self.db.begin().await?;
        sqlx::query("UPDATE operations SET state='failed' WHERE id=?")
            .bind(id)
            .execute(&mut *tx)
            .await?;
        sqlx::query("DELETE FROM eviction_claims WHERE operation_id=?")
            .bind(id)
            .execute(&mut *tx)
            .await?;
        tx.commit().await?;
        Ok(())
    }
    pub async fn source(&self, tenant: &str, scope: &str, object: &Object) -> Result<ReadSource> {
        self.admit(tenant, scope).await?;
        self.folder(tenant, scope, &object.folder_id).await?;
        ensure!(
            self.object_unchecked(&object.folder_id, &object.key)
                .await?
                .as_ref()
                == Some(object),
            "OBJECT_NOT_FOUND"
        );
        let backend = self.backend(tenant, scope, &object.repository_id).await?;
        let blob = backend
            .stat(&object.owner, &object.object_id)
            .await?
            .context("DATA_MISSING")?;
        ensure!(
            blob.size_bytes == object.size_bytes && blob.sha256 == object.sha256,
            "DATA_MISSING: immutable blob metadata changed"
        );
        backend.source(&object.owner, &object.object_id).await
    }
    pub async fn delete(
        self: &Arc<Self>,
        tenant: &str,
        scope: &str,
        folder: &str,
        key: &str,
    ) -> Result<()> {
        let catalog = self.clone();
        let tenant = tenant.to_owned();
        let scope = scope.to_owned();
        let folder = folder.to_owned();
        let key = key.to_owned();
        tokio::spawn(async move { catalog.delete_inner(&tenant, &scope, &folder, &key).await })
            .await?
    }
    async fn delete_inner(&self, tenant: &str, scope: &str, folder: &str, key: &str) -> Result<()> {
        self.admit(tenant, scope).await?;
        self.folder(tenant, scope, folder).await?;
        if let Some(object) = self.object_unchecked(folder, key).await? {
            self.delete_physical(tenant, scope, &object, false).await?;
        }
        Ok(())
    }
    async fn delete_physical(
        &self,
        tenant: &str,
        scope: &str,
        object: &Object,
        eviction: bool,
    ) -> Result<()> {
        let lock = self.lock(&format!("delete:{}", object.object_id)).await;
        let _delete = lock.lock().await;
        if sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM objects WHERE object_id=?")
            .bind(&object.object_id)
            .fetch_one(&self.db)
            .await?
            == 0
        {
            return Ok(());
        }
        sqlx::query("INSERT OR IGNORE INTO deletions(object_id,folder_id,eviction) VALUES(?,?,?)")
            .bind(&object.object_id)
            .bind(&object.folder_id)
            .bind(eviction)
            .execute(&self.db)
            .await?;
        self.backend(tenant, scope, &object.repository_id)
            .await?
            .delete(&object.owner, &object.object_id)
            .await?;
        let mut tx = self.db.begin().await?;
        let removed = sqlx::query("DELETE FROM objects WHERE object_id=?")
            .bind(&object.object_id)
            .execute(&mut *tx)
            .await?
            .rows_affected();
        if eviction && removed > 0 {
            sqlx::query("UPDATE folders SET evicted_object_count=evicted_object_count+1,evicted_bytes=evicted_bytes+? WHERE id=?").bind(object.size_bytes as i64).bind(&object.folder_id).execute(&mut *tx).await?;
        }
        sqlx::query("DELETE FROM deletions WHERE object_id=?")
            .bind(&object.object_id)
            .execute(&mut *tx)
            .await?;
        tx.commit().await?;
        Ok(())
    }
    /// Removal is retryable and never forgets an inaccessible repository.
    pub async fn purge_all(&self) -> Result<()> {
        let scopes = sqlx::query("SELECT DISTINCT tenant_id,scope_id FROM folders")
            .fetch_all(&self.db)
            .await?;
        for row in scopes {
            self.purge_scope(row.get("tenant_id"), row.get("scope_id"))
                .await?;
        }
        Ok(())
    }
    pub async fn clear_artifacts(&self, tenant: &str, scope: &str) -> Result<()> {
        let ids = sqlx::query_scalar::<_, String>(
            "SELECT id FROM folders WHERE tenant_id=? AND scope_id=? AND system=1",
        )
        .bind(tenant)
        .bind(scope)
        .fetch_all(&self.db)
        .await?;
        for folder in ids {
            let pending =
                sqlx::query_scalar::<_, String>("SELECT id FROM operations WHERE folder_id=?")
                    .bind(&folder)
                    .fetch_all(&self.db)
                    .await?;
            for id in pending {
                let lock = self.lock(&format!("operation:{id}")).await;
                let _operation = lock.lock().await;
                if let Some(row)=sqlx::query("SELECT object_json FROM operations WHERE id=? AND state IN ('prepared','staged','cleanup')").bind(&id).fetch_optional(&self.db).await?{let object:Object=serde_json::from_str(row.get("object_json"))?;self.mark_cleanup(&id,"Artifact cleanup").await?;self.cleanup_operation(tenant,scope,&id,&object).await?;}
            }
            let lock = self.lock(&folder).await;
            let _guard = lock.lock().await;
            for row in sqlx::query("SELECT * FROM objects WHERE folder_id=?")
                .bind(&folder)
                .fetch_all(&self.db)
                .await?
            {
                self.delete_physical(tenant, scope, &object_row(row), false)
                    .await?;
            }
        }
        Ok(())
    }
    pub async fn clear_all_artifacts(&self) -> Result<()> {
        for row in sqlx::query("SELECT DISTINCT tenant_id,scope_id FROM folders WHERE system=1")
            .fetch_all(&self.db)
            .await?
        {
            self.clear_artifacts(row.get("tenant_id"), row.get("scope_id"))
                .await?;
        }
        Ok(())
    }
    pub async fn purge_scope(&self, tenant: &str, scope: &str) -> Result<()> {
        self.fence(tenant, scope).await?;
        let ids = sqlx::query_scalar::<_, String>(
            "SELECT id FROM folders WHERE tenant_id=? AND scope_id=?",
        )
        .bind(tenant)
        .bind(scope)
        .fetch_all(&self.db)
        .await?;
        for folder in ids {
            let operations =
                sqlx::query_scalar::<_, String>("SELECT id FROM operations WHERE folder_id=?")
                    .bind(&folder)
                    .fetch_all(&self.db)
                    .await?;
            for id in operations {
                let lock = self.lock(&format!("operation:{id}")).await;
                let _operation = lock.lock().await;
                if let Some(row)=sqlx::query("SELECT object_json FROM operations WHERE id=? AND state IN ('prepared','staged','cleanup')").bind(&id).fetch_optional(&self.db).await?{let object:Object=serde_json::from_str(row.get("object_json"))?;self.mark_cleanup(&id,"scope removal").await?;self.cleanup_operation(tenant,scope,&id,&object).await?;}
            }
            let lock = self.lock(&folder).await;
            let _guard = lock.lock().await;
            let objects = sqlx::query("SELECT * FROM objects WHERE folder_id=?")
                .bind(&folder)
                .fetch_all(&self.db)
                .await?;
            for row in objects {
                self.delete_physical(tenant, scope, &object_row(row), false)
                    .await?;
            }
            let mut tx = self.db.begin().await?;
            sqlx::query("DELETE FROM operations WHERE folder_id=?")
                .bind(&folder)
                .execute(&mut *tx)
                .await?;
            sqlx::query("DELETE FROM consumers WHERE folder_id=?")
                .bind(&folder)
                .execute(&mut *tx)
                .await?;
            sqlx::query("DELETE FROM folders WHERE id=?")
                .bind(&folder)
                .execute(&mut *tx)
                .await?;
            tx.commit().await?;
        }
        sqlx::query("DELETE FROM backends WHERE tenant_id=? AND scope_id=?")
            .bind(tenant)
            .bind(scope)
            .execute(&self.db)
            .await?;
        // The fence survives removal. A separately authorized new Core generation uses a new catalog scope.
        Ok(())
    }
}
fn normalized(name: &str) -> String {
    name.nfkc().flat_map(char::to_lowercase).collect()
}
fn validate_name(name: &str) -> Result<()> {
    ensure!(
        !name.is_empty()
            && name == name.trim()
            && name.chars().count() <= 80
            && !name
                .chars()
                .any(|c| c.is_control() || c == '/' || c == '\\'),
        "INVALID_FOLDER_NAME"
    );
    Ok(())
}
fn object_row(row: sqlx::sqlite::SqliteRow) -> Object {
    Object {
        object_id: row.get("object_id"),
        folder_id: row.get("folder_id"),
        key: row.get("key"),
        repository_id: row.get("repository_id"),
        owner: row.get("owner"),
        size_bytes: row.get::<i64, _>("size_bytes") as u64,
        sha256: row.get("sha256"),
        content_type: row.get("content_type"),
        committed_at_ms: row.get::<i64, _>("committed_at_ms") as u64,
    }
}
