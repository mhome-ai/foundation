CREATE TABLE folders (
 id TEXT PRIMARY KEY NOT NULL, tenant_id TEXT NOT NULL, scope_id TEXT NOT NULL,
 name TEXT NOT NULL, normalized_name TEXT NOT NULL, system INTEGER NOT NULL DEFAULT 0,
 max_bytes INTEGER CHECK(max_bytes > 0), overflow_strategy TEXT NOT NULL,
 revision INTEGER NOT NULL DEFAULT 1, created_at_ms INTEGER NOT NULL, updated_at_ms INTEGER NOT NULL,
 evicted_object_count INTEGER NOT NULL DEFAULT 0, evicted_bytes INTEGER NOT NULL DEFAULT 0,
 rejected_write_count INTEGER NOT NULL DEFAULT 0,
 UNIQUE(tenant_id,scope_id,normalized_name),
 CHECK((system=1 AND max_bytes IS NULL AND normalized_name='artifact') OR (system=0 AND max_bytes IS NOT NULL AND normalized_name!='artifact'))
);
CREATE TABLE backends (
 tenant_id TEXT NOT NULL, scope_id TEXT NOT NULL, repository_id TEXT NOT NULL, handle TEXT NOT NULL,
 PRIMARY KEY(tenant_id,scope_id,repository_id)
);
CREATE TABLE consumers (
 tenant_id TEXT NOT NULL,scope_id TEXT NOT NULL,owner TEXT NOT NULL,
 folder_id TEXT NOT NULL UNIQUE REFERENCES folders(id),PRIMARY KEY(tenant_id,scope_id,owner)
);
CREATE TABLE objects (
 object_id TEXT PRIMARY KEY NOT NULL, folder_id TEXT NOT NULL REFERENCES folders(id),
 key TEXT NOT NULL, repository_id TEXT NOT NULL, owner TEXT NOT NULL,
 size_bytes INTEGER NOT NULL CHECK(size_bytes>=0), sha256 TEXT NOT NULL,
 content_type TEXT, committed_at_ms INTEGER NOT NULL, UNIQUE(folder_id,key)
);
CREATE TABLE operations (
 id TEXT PRIMARY KEY NOT NULL, folder_id TEXT NOT NULL REFERENCES folders(id), key TEXT NOT NULL,
 object_json TEXT NOT NULL, victims_json TEXT NOT NULL,
 state TEXT NOT NULL CHECK(state IN ('prepared','staged','published','cleanup','failed')),
 error TEXT, created_at_ms INTEGER NOT NULL
);
CREATE UNIQUE INDEX one_pending_write ON operations(folder_id,key) WHERE state IN ('prepared','staged');
CREATE TABLE eviction_claims (
 object_id TEXT PRIMARY KEY NOT NULL REFERENCES objects(object_id) ON DELETE CASCADE,
 operation_id TEXT NOT NULL REFERENCES operations(id) ON DELETE CASCADE
);
CREATE TABLE deletions (
 object_id TEXT PRIMARY KEY NOT NULL, folder_id TEXT NOT NULL, eviction INTEGER NOT NULL DEFAULT 0
);
CREATE TABLE fences (tenant_id TEXT NOT NULL, scope_id TEXT NOT NULL, PRIMARY KEY(tenant_id,scope_id));
