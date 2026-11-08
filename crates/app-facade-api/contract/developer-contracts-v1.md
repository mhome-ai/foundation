# Published standard capability contracts

`/app/developer/contracts/list` and `/app/developer/contracts/export` are read-only
Hub operations. The authenticated caller needs Space read access to reach its
online Hub. The Hub requests the matching `/bridge/schema/contracts/*` operation
from Lion using its existing authenticated connection. Space identity authorizes
and routes the request; it never chooses contract content. There is no direct
Client-to-cloud implementation of these operations.

Lion selects the latest complete, unarchived official production packages and
their necessary dependencies, including `base`. Space Beta settings, installed
versions, personal staging and local Preview do not participate. A missing or
invalid requested dependency fails the operation. Unrelated catalog packages are
not loaded. Sources are checked again before returning a result; a publication
change causes a bounded retry, then an explicit temporary-unavailable error.

List input is exactly `{ "integrationId": "basic.device" }`. The result contains
`integrationId`, `sourceSnapshot` and `capabilities`, with each capability's `id`,
`method`, `abstract` flag and `description` from the resolved compiler graph.

Export input contains `selected` (1..64 unique fully-qualified command/state/event/
data IDs) and optional lowercase SHA-256 `ifNoneMatch`. Unknown fields are rejected.
The result is either `{ "kind": "ready", "document": <export> }` or
`{ "kind": "unchanged", "sha256": <digest> }`. These are business results inside
the ordinary Facade response. The compiler-owned document keeps the
`meow-capability-contracts` version 1 format: compiler identity, exact source
inventory/digests, input snapshot, explicit selections, resolved contracts and
referenced types, plus its canonical result digest. It preserves inheritance,
main entity routing fields, request/result semantics, constraints and data
discovery dependencies. It executes no provider code or live business queries.

`ifNoneMatch` is only a transfer optimization: Lion first checks current official
sources, and may reuse immutable content and compiled/exported results. Cached
data is never a freshness fallback when the cloud cannot be reached. A digest is
a content identity, not a publication signature or operation grant.

Each business result is limited to 512 KiB so the ordinary envelopes remain below
the Client's 2 MiB WebSocket message budget. Over-limit queries fail explicitly;
request fewer capabilities. Export does not install a package, activate a source,
change provider configuration, or return runtime credentials/household data.
