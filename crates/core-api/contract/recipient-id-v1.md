# Stable recipient identity

These identities address recipients, not WebSocket sessions or Agent conversations.
Tenant and Space are required routing context outside the identity. Cloud/local
placement is a route property, never a prefix or an identity component.

| Recipient | Canonical representation |
| --- | --- |
| Personal Messaging conversation | `m:p:<provider>:<account>:<conversation>[:<lane>]` |
| Shared Messaging conversation | `m:g:<provider>:<account>:<conversation>[:<lane>]` |
| Node | `n:<nodeType>:<nodeId>` |

Account, conversation, lane and Node ID are strict UTF-8 encoded as canonical
unpadded base64url. Empty values and surrounding whitespace are invalid. Provider
and Node type match `[a-z][a-z0-9_]*`; wire parsing must reject noncanonical input.

Messaging uses exactly the same address components as ConversationSurface. The
conversion between `m:p:` / `m:g:` and `cs1:mp:` / `cs1:mg:` preserves every
component, including lane. Generate both from one structured address; do not store
two independently editable addresses. A base authorization lookup may omit lane,
but the actual recipient must retain it. Personal and shared identities never
collapse into one another.

Node identity resolves against the existing scoped Node registry. It does not
create an App Client, a second registry, or a WebSocket. Node capability/endpoint
selection remains a separate field. Connection close still addresses a specific
connection, not a stable Node identity.

App installation selectors (`a:`), user selectors, connection IDs (`L:` / `C:`),
and Phone Provider credentials (`p:`) are outside this codec. In particular, `p:`
is not a personal Messaging recipient; only `m:p:` is. The codec does not accept
legacy `M:` IDs or ConversationSurface IDs as recipient IDs.

Cross-language implementations must pass `fixtures/recipient-id.conformance.json`.
This additive Foundation release prepares consumers for a coordinated identity
cutover; it does not itself migrate Core/Lion storage or delivery adapters.
