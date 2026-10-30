# Storage capability, version 1

`mhome-storage-api` owns logical Folder and object contracts, usage,
path validation and structured errors. It has no Catalog, filesystem or network
implementation. Core API re-exports it and adds binding and Host TLS messages;
App Facade API re-exports the same contracts for `/app/object-storage/`.

The wire generation is `storage.v1`; package versions independently identify
published build artifacts. During development, providers and consumers are
updated together. Earlier development contracts are not negotiated or served.

## Names and ownership

A Folder's immutable `id` is its exact case-sensitive UTF-8 path within one
(tenant, Space). Names are never normalized or silently trimmed. Use `home/<path>`,
`plugin/<unique-plugin-name>/<bound-instance>/<path>`, or `core/<module>/<path>`.
Segments contain 1–80 Unicode characters, with no edge whitespace, control
characters, backslash, empty segments, `.` or `..`. Paths contain at most 16
segments and 1024 UTF-8 bytes; no leading or trailing slash. These logical paths
are never joined to disk paths. Unicode spellings remain distinct exact IDs.

`folders/create` takes `folderId`, is idempotent, and creates missing parents
below the namespace root in one transaction. Namespace roots themselves are
virtual and cannot be created or deleted. `folders/get` and `folders/list` return
path IDs, system/deleting flags, timestamps and usage. There is no display-name
alias, rename/update endpoint, Folder quota, aggregate logical quota, overflow
policy, automatic eviction or consumer-folder mapping.

App/CLI/Agent users may create, delete and acquire data sessions only under
`home/`. Space-authorized diagnostics may list Plugin/system Folder usage.
Bound Plugins may create multiple Folders, read and acquire sessions, or delete
only inside `plugin/<name>/<instance>/`. Core derives the name from verified
binding metadata and the instance from authenticated context; `nodeType`, a
request path or a caller-supplied name is not ownership proof. Core internal
modules use `core/<module>/...`. The protected `artifact` Folder is created and
used only through Artifact APIs; it has no ordinary sessions or Storage settings.

## Deletion and lifetime

`folders/delete` takes `folderId` and `recursive` (false by default). False
requires an empty Folder with no descendants or active/cleanup writes. True
explicitly removes all its contents and descendants. Physical cleanup must
succeed before the Folder tree is removed; failure can leave earlier physical
deletions applied. Remaining objects and staged bytes remain indexed and counted.
A persistent deletion fence blocks new writes and descendant creation until
cleanup is retried and finishes. The response is `{ "deleted": true }`; an
already absent path returns false. `deleting` exposes incomplete cleanup.

The catalog retains a private creation incarnation for object ownership and
sessions, separate from the public path. Reusing a deleted path creates a new
incarnation; old sessions cannot read, delete or write into it. A fresh management
delete always addresses the current path, so callers inspect state after an
uncertain response before retrying. Scope/Core removal stops ordinary access and
waits for all physical repositories, including offline repositories, to be cleaned.

## Placement and capacity

Core prefers a healthy, authorized external repository for new writes; otherwise
it uses its internal disk. Existing objects keep their recorded repository UUID;
enabling a Plugin does not migrate them. Reads and deletes resolve that recorded
placement. New writes may fall back before physical mutation; an uncertain
prepare/upload remains pinned until retry or confirmed retirement resolves it.

All Folders share available capacity. Host physical reserves and staging space
still apply. A write validates full size and SHA-256; no staging space means
failure without deleting old data. Storage never infers business references.
Camera owns retention, pinned clips and shared segments, using its own retention
budget independently of Storage. Temporary Artifact content retains its transient
store and idle TTL. Logically unlimited Artifact storage still needs real space.

Object keys are immutable: the same key, size and SHA-256 is an idempotent retry;
new content requires a new key. Management byte counts and timestamps are decimal
strings; object/physical receipts use JSON u64.

## Folder SDK and internal transport

Business callers use `ensureFolder` (explicit, idempotent creation), `openFolder`
(existing only), Folder listing/deletion and overview. A Folder handle provides
put/get/stat/list/delete. The existing Node SDK owns the Plugin client, and the
App SDK owns the `home/` client. Core internal consumers call the engine directly.
Camera retains recording retention and shared-segment references.

`/app/object-storage/session/acquire` is an internal transport operation, with
`folderId` and optional `expectedIncarnation`. It accepts no `permissions` or
`ttlSeconds`. An authorized namespace owner receives all object operations for
that Folder; the fixed 15-minute expiry is not a business setting. Its Core API
response includes `folderIncarnation`, HTTPS endpoint, token, Host identity and
expiry. SDKs keep these private and renew shortly before expiry. The incarnation
is an opaque transport fence, not another public Folder ID.

Each handle binds to its originating Space and owner. Renewals send the original
creation identity and never call Folder creation. A deleted/recreated path cannot
replace an existing handle. App login/Space changes retire handles and active
transfers; Plugin context changes reject the old binding. Reopen explicitly after
an authorization refusal. HTTP 401 `SESSION_EXPIRED` is raised before object
mutation and permits one retry after refresh. HTTP 403 remains terminal. Other
failures, including uncertain writes, are returned without automatic replay.
Retried uploads retain their original key, length and SHA-256. Native file tickets
are owned per transfer: renewal leaves an admitted read alive, and completion,
cancellation or SDK disposal releases its ticket.

These logical sessions do not replace physical backend grants. Physical read,
stat, delete and prepared-write grants remain separate, including Artifact read
and removal-only cleanup authority.

## Physical backend and reads

`storage.backend.v1` remains separate from the public `storage.v1` capability.
Describe reports capacity; grant authorizes stat/read/delete; prepare-write reserves
space for an immutable Blob before issuing a write grant. A lost reply can have
reserved bytes. Receipts are resolved before changing repositories. TLS is pinned,
with redirects and proxy inheritance disabled. Core's internal `mhome-storage-disk`
source crate (`publish = false`) owns disk durability, repository identity,
reservations and idempotent unlink. Core and the Storage Plugin reuse this code;
it has no independent public package release.

Errors carry `code`, `detail` and `outcome` (`notApplied`, `partial`, `unknown`),
with optional `deletedObjects` for confirmed per-attempt deletions. Unknown
transport results are not proof of failure. Range reads do not hash the whole
file first. Full Artifact streams can check integrity incrementally, withholding
the last bytes until verification succeeds.

This version supports new databases only. Edit complete baselines; never convert
old data at startup. Preserve old databases/files for a separately authorized import.
