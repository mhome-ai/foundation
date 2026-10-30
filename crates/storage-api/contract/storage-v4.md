# Storage capability, version 4

`mhome-storage-api` is the canonical Rust/JSON contract for logical Folders,
object receipts, permissions, usage and structured errors. It contains no
Catalog, database migrations, filesystem IO, network client or Core bindings.
`mhome-core-api` re-exports these types and adds Core/Plugin control messages and
Host TLS grants. `mhome-app-facade-api::storage` re-exports the same types for
`/app/object-storage/`; it does not define another Folder model.

## Logical capability

Every durable object belongs to a Folder in one tenant and Space. Object keys
are immutable: replaying the same key, size and SHA-256 is idempotent; changing
content requires another key. Callers receive logical object IDs and Folder
sessions, never a backend selector or a fake repository ID. Session acquisition
requires explicit permissions and returns a short-lived HTTPS endpoint, token,
expiry and Host identity. Capacity diagnostics may expose real repository UUIDs.
Management counts, byte sizes and timestamps are canonical unsigned decimal
strings. Object data-plane receipts and private backend counters use JSON u64.

Core owns Folder quotas, eviction, placement, Catalog recovery and authorization.
It prefers a healthy authorized external repository for new writes and otherwise
uses its internal repository. Existing objects retain their recorded repository
UUID. Enabling a plugin never moves existing data. A read or delete never falls
back to a different repository. New writes may fall back before a physical
mutation starts. After an uncertain prepare or upload, the operation stays pinned
until an idempotent retry or confirmed physical retirement resolves it.

Only per-Folder quotas apply. The reserved `artifact` Folder is system-owned,
logically unlimited, cannot acquire an ordinary Folder session, cannot be renamed
or configured through Storage settings and never participates in eviction. Real
physical capacity and host safety headroom still apply. Camera owns its recording
retention and shared references; Storage does not infer those relationships.
Every Folder declares an immutable `cleanupOwner`: `storage` permits quota
policy selection; `consumer` requires `overflowStrategy: reject` and prohibits
Storage eviction through all management paths. Application-provisioned Folders
and the Artifact system Folder use `consumer`. Name and quota remain editable
for ordinary consumer Folders; the consumer chooses complete business resources
to delete. Camera refreshes Folder quotas and targets the smaller of its own
retention limit and 90% of the Folder quota, while preserving pinned/shared media.
Temporary Artifact content uses the Core transient store and a one-hour idle TTL.

A write reserves staging space and validates its full size and SHA-256 before
quota eviction. Lack of staging space fails without deleting existing objects.
Eviction deletes victims one at a time; earlier successful deletions remain if a
later deletion fails. Publication happens only after all required evictions.
Deletion succeeds only after physical deletion is confirmed; unavailable bytes
remain indexed and counted. Space/Core removal fences new business access and
waits for physical cleanup, using cleanup-only authorization when needed.

## Backend binding

`storage.backend.v1` is an independent Core/Plugin protocol, not the public
Folder protocol version. `/storage/backend/describe` returns its version and
capacity. `/storage/backend/grant` issues read/stat/delete grants and never
reserves bytes. `/storage/backend/prepare-write` declares the immutable Blob,
reserves physical space and returns a write grant. A lost reply may have reserved
space: the caller retries the same physical ID or confirms deletion before
changing placement. Control replies are tagged `status: ok | error` with either
`value` or a structured `StorageError`. HTTP backend errors use that same error
body (HEAD has only its HTTP status). Host identity and grant expiry must be
forwarded unchanged. TLS is pinned; redirects and proxy inheritance are disabled.

`mhome-storage-disk`, an internal source crate (`publish = false`) in Core, owns only
physical byte durability, repository identity, reservation receipts and idempotent
unlink. It does not depend on Core and cannot see Folder policies or Artifact
identities. Core's internal disk and the Storage Plugin use this implementation.
Core uses a local path; the Plugin pins a full Core Git commit. The disk crate
has no independent public package release or versioned compatibility contract.

## Errors and reads

Errors carry an enum `code`, human-readable `detail`, and `outcome`:
`notApplied`, `partial`, or `unknown`. Callers branch on codes, never detail text.
`partial` eviction errors include the count of deletions confirmed in that
attempt. Unknown transport results do not prove that a mutation failed.
Pending physical reservations and cleanup may outlive a failed logical write.

Range reads validate range/length without a full-file SHA-256 pre-read. Full
Artifact streams may verify incrementally; the final bytes must be withheld
until integrity succeeds so HTTP Content-Length cannot hide a late error.
Backend protocol mismatches make a candidate unavailable for new placement;
existing placements are retained, not silently rebound to another disk.

This version supports new databases only. Baselines are not startup converters.
Old databases/files require a separate explicit import and remain preserved.
