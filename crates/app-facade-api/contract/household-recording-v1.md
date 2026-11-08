# Household behavior records

This is input for background household learning, not an audit log. Plugin Facades,
queries, UI layout, account settings, infrastructure and Agent memory updates skip.
`manifest/recording.v1.json` owns the explicit Facade projections and canonical
Device command allowlist (including declared provider overrides). Mixed-purpose
hooks may return Skip. New Facades default to Skip until explicitly registered.
Rust `recording::project` and Lion `BehaviorProjection` implement the same pure
hooks and share `fixtures/recording.conformance.json`. The Lion runtime definition
is a packaged mirror verified against this canonical manifest in joint-source tests.

A call may carry optional `source: {kind, page?, cardId?}` beside `control` and
`input`. Kinds are `ui`, `agent` (built-in Agent), `messaging`, `cli`, `mcp`.
Source hints never select authorization or actor. Trusted Agent/MCP ingress overrides
hints; a canonical Messaging Agent surface keeps `messaging`, including through
Cloud-to-Hub forwarding. Direct Messaging Dialog replies use the verified external sender
and a Messaging-only forwarding binding; they create no Agent run and can only invoke
`/app/dialog/reply` in direct mode. CLI calls, including external Skills using the CLI,
are always attributed as `cli`; no caller-supplied CLI source option is needed.
UI sets `ui`. `appClientId` comes only from the verified
Client identity, not connection routing IDs or an input hint.

The runtime derives actor from verified initiation: User -> `{kind: user,userId}`;
shared Space -> `{kind: space}`. The record schema also reserves `{kind: agent}`
for autonomous initiation. Current user-directed Agent operations retain their
verified principal. Source `agent` never makes an operation autonomous. Wiring a
future proactive scheduler requires trusted initiation metadata; an LLM cannot
self-declare an actor in the Facade call.

Only actual execution records. Prepare, reject, validation before execution and
cached commit replay do not. Internal automation side effects are separate event
sources, not additional manual behavior. The record uses one `id`: trusted delegated
operation identity, prepared operation identity when applicable, or a generated UUID.
It contains `occurredAtMs` (server UTC at execution dispatch), `finishedAtMs` (server
UTC when that dispatch settles), actor, source and domain `data`. Status comes from
the canonical Full outcome before Essential presentation. An accepted async run is
not a claim that all its physical effects completed. Failures keep intent and error
code, never raw exception messages, internal details, complete request/response or
source scripts. Runtime completion/Twin observations belong to their own sources.

Facades may contribute frozen business facts through an execution-scoped hook. The
hook runs in the background writer and cannot replace canonical event/status/error
fields. Automation save and Overview save contribute the resolved YAML document
already obtained by the business flow. Their projection keeps name, description,
root `$` parameter values and input-variable references; task/action bodies, compiler
models, attached scripts and diagnostics are excluded. Hook failure leaves the
business response unchanged.

Space mode determines the single storage owner: Local -> Hub SQLite, Cloud ->
Lion's configured cloud store. Routing/forwarding creates no second record.
The existing Space cleanup removes this partition. Hub queued deliveries retain
the existing runtime fence so revoked/replaced generations cannot recreate records
after cleanup. Cloud writers register with Space admission before reading the
authoritative Space identity; only ACTIVE Cloud Spaces accept queued records.
Terminal deletion marks the identity first, drains admitted work, then deletes the
partition, so other instances and delayed queue entries cannot recreate it.
The physical row has `pk`, `sk`, `schemaVersion=1`, `payloadYaml` only:

- `pk = tenant|space`.
- `sk = zero-padded 13-digit UTC milliseconds|id`.
- Fixed string order supports `[start,end)` queries in one partition. Pagination
  uses the last returned SK exclusively. No secondary index is required.
- Inserts never replace an existing key. Retrying persistence reuses the same row.
- `household/behavior` participates in ordinary Cloud/Hub DataSync. Mode switches
  merge immutable rows, preserve original keys/schema/YAML bytes, reject foreign
  partitions and conflicting existing rows, and rely on the existing successful
  import acknowledgement before source cleanup. A retry does not clear history.
- YAML retains null, false, zero and typed values; no custom tags or raw JSON copy.

Recording is best effort. Request code only captures immutable facts and offers
work to a bounded queue (256). Projection, YAML and storage run in the background.
Failures, overload and the 32 KiB projected-payload limit drop/count the record,
without changing the business result or replaying the operation. No durable outbox,
start-record requirement, recovery workflow, or logging warning in Facade responses.

Core adds a table to its complete SQLite baseline. An existing incompatible Dev
DB requires the explicit standard rebuild workflow; startup never rewrites migration
checksums, deletes data or silently migrates the old baseline. Publishing Foundation
and reconciling normal consumer protocol receipts remain release prerequisites.
