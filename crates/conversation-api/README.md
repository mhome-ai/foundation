# mhome-conversation-api

Typed, transport-neutral DTOs and canonical surface identities for the mHome conversation API.

This crate owns wire targets, request and response bodies, user-visible message content, and
conversation events. `ConversationSurface` is a canonical value object whose string form is the
wire and persistence identity of one isolated conversation endpoint. The crate deliberately
contains no Agent runtime, persistence, transport implementation, or messaging-provider SDK.

`/chat/turn/submit` is the source-neutral application operation for callers that do not manage
threads themselves. The Conversation implementation owns active-thread creation, idle rotation,
pending-interaction policy, idempotency, and enqueueing for that operation.
