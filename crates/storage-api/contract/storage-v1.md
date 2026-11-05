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
path IDs, system/deleting flags, timestamps, direct usage and subtree quota
snapshots. Paths have no display-name alias or rename endpoint. Storage does not
automatically evict data or infer consumer-folder mappings.

App/CLI/Agent users may create, delete and acquire data sessions only under
`home/`. Space-authorized diagnostics may list Plugin/system Folder usage.
Bound Plugins may create multiple Folders, read and acquire sessions, or delete
only inside `plugin/<name>/<instance>/`. Core derives the name from verified
binding metadata and the instance from authenticated context; `nodeType`, a
request path or a caller-supplied name is not ownership proof. Core internal
modules use `core/<module>/...`. The protected `artifact` Folder is created and
used only through Artifact APIs; it has no ordinary sessions or Storage settings.

## Folder quota settings

`folders/set-limit` takes exactly `{ "folderId": "<path>", "maxBytes": "<bytes>" }`.
Both fields are required. `maxBytes` is a canonical positive decimal string in
1..9223372036854775807, or explicit `null` to remove this Folder's own limit.
The result is the updated Folder. App Facade uses
`/app/object-storage/folders/set-limit`; authenticated Node requests use the same
target with Plugin context derived by Core. Space members can change any ordinary Folder
setting, including Plugin Folders, without acquiring content mutation permission.
Plugins can change only their own authenticated namespace. System `artifact` and
deleting Folders reject settings. There is no version check: last committed write wins.

Limits cover a Folder and all descendants across every physical repository.
Every limited ancestor applies even if a child has no own limit. `quota` contains
`maxBytes` (own setting), `usedBytes` (subtree committed bytes), `reservedBytes`
(subtree admitted/uncertain/cleanup writes), `remainingBytes` (minimum remaining
allowance across this Folder and limited ancestors, or null if none), and
`exceeded` (any applicable limit is below its charged usage). All byte fields use
decimal strings. Existing top-level `usedBytes`, `reservedBytes`, and `objectCount`
continue to describe only the Folder's direct objects.

Core checks and reserves the declared object length atomically against all
applicable limits before physical write preparation. Concurrent descendants cannot
spend the same allowance. Publication transfers reserved to used; confirmed
physical deletion/retirement releases the corresponding charge. Lost responses,
offline repositories, restarts and failed cleanup do not free unconfirmed bytes.
Immutable-object retries keep their original reservation and do not charge twice.
`QUOTA_EXCEEDED` is a typed refusal (HTTP 507), distinct from physical `VOLUME_FULL`.

Lowering below used plus reserved saves immediately. Existing objects and admitted
uploads remain; new admissions fail until enough charged bytes are removed or the
limit increases. Storage never deletes content in response to a setting change.
Null removes only this Folder's limit; ancestor limits, physical reserves and
staging requirements still apply. New Folders start with no own limit.

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

For SDK writes, Core prefers a healthy external repository on the authenticated
source Host, then another external repository, and finally its internal disk.
Within each external group, writable staging bytes descending and stable repository/
binding identities decide the order. An unknown source Host has no local preference.
Existing objects keep their recorded repository UUID; enabling a Plugin never moves
them. Discovery excludes unavailable or insufficient candidates. Once selected,
preparation or transfer failure fails this write: there is no Core byte relay,
cross-repository replay, persistent sender spool or automatic media retransmission.
Uncertain results remain pinned to that repository until publication or cleanup.
Core-internal Artifact consumers retain their existing engine write/recovery path.

All Folders share available capacity. Host physical reserves and staging space
still apply. A write validates full size and SHA-256; no staging space means
failure without deleting old data. Storage never infers business references.
Camera owns retention, pinned clips and shared segments, using the recording
Folder quota with no separate default byte budget. Temporary Artifact content
retains its transient store and idle TTL. Logically unlimited Artifact storage still needs real space.

Object keys are immutable: the same key, size and SHA-256 is an idempotent retry;
new content requires a new key. Management byte counts and timestamps are decimal
strings; object/physical receipts use JSON u64.

## Folder SDK and internal transport

Business callers use `ensureFolder` (explicit, idempotent creation), `openFolder`
(existing only), Folder listing/deletion and overview. A Folder handle provides
put/get/stat/list/delete. The existing Node SDK owns the Plugin client, and the
App SDK owns the `home/` client. Core internal consumers call the engine directly.
Camera retains recording retention and shared-segment references.

Object HEAD/stat verifies the selected physical repository, not just its Catalog
receipt. Confirmed physical absence returns 404 (`NOT_FOUND` for GET); a missing
repository, transport failure or timeout remains unavailable, never absence.
Length/hash declaration mismatches remain integrity errors. Existence probes do
not remove receipts or release quota charges: only confirmed deletion does that.
SDK HEAD maps 404 to no object, and reads preserve the typed `NOT_FOUND` failure.

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

Object data uses SDK-owned direct transfers. POST to the session's
`objects/<key>` URL carries a Core API `StorageObjectControl`: `prepare`
(`sizeBytes`, `sha256`, `contentType`, `localImport`), `complete`, `status`, or
`abort` (each with `uploadId`), or `read`. Prepare reserves quota once and returns
an already-published receipt or the exact repository transfer. A transfer contains
its URL, token, Host TLS identity, expiry and an optional local capability.
The SDK sends bytes to that repository and completes through Core. Only Core's
published receipt is success. Complete verifies physical metadata rather than
trusting the client. Aborting a published upload leaves its object intact.
Unfinished/cancelled writes remain charged until confirmed cleanup; existing
recovery may publish verified bytes after a lost response. Terminal journals have
a 24-hour cleanup window; the immutable object receipt remains accessible by key.

Read obtains the selected repository's grant and streams directly, including HTTP
Range. Session HEAD/list/delete remain Core metadata/management operations. Session
PUT only accepts a prepared internal-disk upload; external data cannot be relayed
through the Core Folder gateway. Old SDK byte routes are not served in parallel.

The Node SDK writes byte buffers without a temporary file and reuses pinned HTTPS
connections without sharing request authorization. A local capability is projected
only for a verified same-Host Plugin. Its private Unix socket accepts one authorized
object stream, or a read-only source file descriptor for import. Source pathnames
and repository directory access are never transferred. Import clones on supported
filesystems or allocates/copies locally, verifies the result, and preserves the
source. Inaccessible local namespaces may use HTTPS to the same selected repository
before a local request starts. App WebViews use the native pinned file bridge;
exact-object upload tickets permit PUT only. No offline quota delegation is added.

These logical sessions do not replace physical backend grants. Physical read,
stat, delete and prepared-write grants remain separate, including Artifact read
and removal-only cleanup authority.

## Physical backend and reads

`storage.backend.v1` remains separate from the public `storage.v1` capability.
Storage is in development: both protocols evolve in place as v1, with no legacy
readers, version negotiation or compatibility branches.
Describe reports capacity; grant authorizes stat/read/delete; prepare-write reserves
space for an immutable Blob before issuing a write grant. A lost reply can have
reserved bytes. Receipts are resolved before changing repositories. Issued grants survive temporary Core control disconnection until normal expiry;
explicit binding removal still revokes unused grants. TLS is pinned,
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
