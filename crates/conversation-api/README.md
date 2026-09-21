# mhome-conversation-api

Typed, transport-neutral DTOs and canonical surface identities for the mHome conversation API.

This crate owns wire targets, request and response bodies, user-visible message content, and
conversation events. `ConversationSurface` is a canonical value object whose string form is the
wire and persistence identity of one isolated conversation endpoint. The crate deliberately
contains no Agent runtime, persistence, transport implementation, or messaging-provider SDK.

`/chat/turn/submit` is the source-neutral application operation for callers that do not manage
threads themselves. The Conversation implementation owns active-thread creation, idle rotation,
pending-interaction policy, idempotency, and enqueueing for that operation.

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
