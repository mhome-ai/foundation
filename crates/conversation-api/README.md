# mhome-conversation-api

Typed, transport-neutral DTOs and canonical surface identities for the MeowLink conversation API.

This crate owns wire targets, request and response bodies, user-visible message content, and
conversation events. `ConversationSurface` is a canonical value object whose string form is the
wire and persistence identity of one isolated conversation endpoint. The crate deliberately
contains no Agent runtime, persistence, transport implementation, or messaging-provider SDK.

`/chat/turn/submit` is the source-neutral application operation for callers that do not manage
threads themselves. The Conversation implementation owns active-thread creation, idle rotation,
pending-interaction policy, idempotency, and enqueueing for that operation.

Each newly created `ThreadSummary` carries a durable `origin`: either
`{ "type": "turn", "requestId": "…" }` for creation/idle rotation caused by
`turn/submit`, or `{ "type": "operation", "operationId": "…" }` for an explicit
thread/session operation. The owner assigns it when committing creation; callers
cannot supply it as request metadata. It is immutable and appears in control,
catalog and thread-load projections, including queries after reconnection.
It identifies creation, not turn admission or execution. Origin can be absent
for records written before tracking was introduced; clients must not infer a
cause from operation-ID spelling, timing or the presence of an unbound send.

The canonical `cs1` surface families are client personal (`cp`), client group (`cg`), messaging
personal (`mp`), and messaging group (`mg`). Messaging surfaces include provider, provider account,
external conversation, and an optional lane. A lane isolates a provider sub-conversation such as a
Telegram forum topic without changing the provider account's base authorization route.

Message content is a provider-independent ordered list of text, image, audio, video, and file parts.
Messaging runtimes must materialize provider media handles into this content before invoking the
Conversation application port; the Agent and this contract never receive provider SDK objects.

## Execution contracts

`execution` owns Agent command/event DTOs and deployment ports. `execution::wire` owns the cloud
transport envelopes and `schema/execution` / `fixtures/execution` conformance assets. These moved
from the retired Agent contract/protocol crates. Client-facing root modules remain unchanged.

Model data is re-exported from `llm-api`; there is one model-message representation. User-facing
messages intentionally use separate content types and never contain private model continuation.
Runtime state machines, checkpoint formats, prompts and recovery policy remain in Agent Runtime.

Connection HITL previews resume one exact lifecycle session:
`{ "type": "connect_integration", "integrationId": "provider.example", "sessionId": "session" }`.
The execution and conversation contracts use the same closed client-task shape.
The UI reads and resumes that session; it never starts a replacement on reopening.
The Agent verifies its authoritative completion result and Connection ID. Expired
or lost sessions fail explicitly. Direct installation confirmations have no client task.

`RunErrorCode` is the closed catalog for `runOutcome.errorCode` and Agent `Failed.code`. Host
terminalize and Agent failures emit the same strings. `status` and `failure.source` are properties
of the code; `failure.code` equals `errorCode`. User-visible copy stays in `runOutcome.message`,
not in this crate. Command `/chat/*` errors and adapter `ExternalErrorKind` are separate layers.

## Cloud admission model snapshot

Every cloud `LlmRoute` carries a required `model_snapshot` containing confirmed capabilities
and catalog `generation_support` (the camelCase `metadata.generationSupport` shape). Lion
persists it with desired parameters at request admission. Deployments adapt preferences against
these immutable facts; tool rounds and approval resumption never rediscover current metadata.
The portable `CompletionRequest` and its caller-owned constraints remain separate from this
snapshot. Version 3 stores `capabilities.input` as `image|video|audio|file` instead of
`vision: bool`. An old plan without the snapshot, or with `vision`, fails decoding instead
of silently changing behavior. This development contract change must be published and
adopted by Lion and Cloud together before deployment.

### Strict execution envelope decoding

Execution commands, events, facade responses and their typed nested objects reject undeclared fields, including null-valued unknown fields. Opaque JSON inputs/results and model continuation payloads remain extensible; known optional null fields remain accepted. Enforcement lives on the typed DTOs as well as the wire entry point because Serde's internally tagged enum buffering bypasses an outer `serde_ignored` observer. Tagged unit variants explicitly require an empty remaining map. This restores the existing wire-v1 contract without changing its message shape or major version.

## Disposable execution presentation

`assistant.preview`, `run.progress`, and `run.system_failed` are best-effort live
presentation. They are not replayed by `thread/load`, which has no `live` field.
Each event replaces its display slot completely; preview text never appends and
is capped by the host at 8,192 Unicode characters. Clients retain at most one
event per slot, scoped to the active request and `baseSnapshotVersion`.

`sequence` is assigned by the execution owner, may contain gaps, and only rejects
older or duplicate events in the same slot. A missing presentation event must not
trigger history resynchronization. A new snapshot generation resets ordering;
terminal or pending-interaction control clears presentation. Joining or reconnecting
starts from durable history/control and future live events, with a generic running
indicator until a new presentation arrives.

Final replies, failures, cancellations, queue state, and actionable user
confirmations remain durable. Snapshot deltas keep their existing contiguous
version/recovery rules. Agent checkpoints and billing are independent of live
presentation delivery.

## Waiting queue operations

`/chat/queue/update` replaces the complete content of one waiting request. Its
request ID, original admission fingerprint, author, and frozen execution settings
remain unchanged. Concurrent edits use the last successful server commit; client
clocks are irrelevant. Claiming the request for execution and editing it share the
same transactional boundary, so an edit cannot succeed after execution begins.

`/chat/queue/reorder` moves `requestId` before `beforeRequestId`; a null anchor
moves it to the tail. The server applies the move to its current queue, preserving
new arrivals and unrelated moves. A missing request or anchor is a conflict; a
client must never replace the whole queue from its stale local list.

Both operations require an `operationId`, scoped to the surface and request ID.
An exact retry returns `already_applied` and the original operation's queue version,
without replaying its changes. Reusing it with another payload is a conflict.
Command responses still carry a fresh authoritative control projection; clients
apply control versions monotonically rather than treating a mutation receipt as
a queue snapshot. Events and command responses use the same control versions.

Cancellation, including archive cleanup, retains a durable terminal admission
receipt independently of transcript retention. `/chat/request/status` reads that
receipt by request ID, returning null admission for an unknown request. It recovers
missed `admission.changed` events without reenqueuing or guessing from an empty
queue. Retry of the original submission still checks its original content, even
when the queued execution content was later edited.

## Pod Talk

`talk` is a separate bounded presentation facade for small voice devices. Its
targets appear under `podTalk` in the target manifest, with independent schema
`schema/pod-talk.v1.schema.json` and cross-language examples in `fixtures/talk/`.
The canonical `/chat/*` targets and Agent execution wire remain unchanged.

Each view carries the authoritative owner, authenticated conversation surface,
active thread, control version and exact decimal history version. Personal
Clients retain their own `cp`; Space Clients share only `cg:space-clients` in the
same tenant and scope. Admission receipts never establish a run outcome. Results
are read by original request ID, including after active-thread rotation; unknown
results remain unknown. Request cancellation may omit the thread and resolve it
through the original durable admission. No automatic write replay is implied.

When the active thread has no running, queued or pending request, a view's
`settlement` carries the latest run's `requestId` and bounded `outcome`, even
without assistant text. It is null in other phases. `interaction_required` is
not a terminal outcome, and reply text alone does not establish success. This
lets a restarted device or another Pod recover the same terminal presentation
without persisting a local request. The context's thread identifies the
settlement; archived-thread results never replace a different active view.

Reply offsets count UTF-8 bytes in the immutable plain text identified by
`contentRevision`. A single bounded window includes explicit next/previous
offsets. Pending previews mark omissions and prohibit approving incomplete
previews or unsupported client tasks. Spoken responses target the original
thread, request and batch, with a second validation before canonical submission.

The complete escaped MWS frame is limited to 64 KiB, changed notifications to
2 KiB, transcripts to 4 KiB UTF-8 and reply windows to 16 KiB. Implementations
must enforce bytes and parsing depth/node budgets in addition to schema string
lengths. All egress paths to Pod, including bridged pushes and errors, must honor
these limits before entering the physical WebSocket queue.

Audio uses short-lived binary HTTP grants, not base64 in MWS. Input is mono
16 kHz PCM16 WAV, 300–30,000 ms and at most 960,044 bytes; metadata, actual bytes
and SHA-256 must agree. TTS references a formal assistant message, permits only
finite PCM16 mono WAV at 16/24/48 kHz, at most 32 seconds and 1,536,044 bytes, and
returns an Artifact reference or text-only reason. Media steps retain the same
owner. Upload grants and download descriptors reuse the Artifact contract,
including pinned host identity when the owner is a local Core. Resolve responses
include the content digest and require signed URLs, never data URLs.
