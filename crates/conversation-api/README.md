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
