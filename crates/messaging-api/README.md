# mhome-messaging-api

Transport-neutral control and normalized data-plane contracts for messaging providers. The crate
owns typed Rust DTOs, target and capability inventories, strict JSON Schemas, and cross-language
conformance fixtures.

The common contract deliberately treats `data`, `providerData`, `options`, and `result` as provider
extension objects. Their contents are decoded and validated by the selected provider; all fields
outside those explicit extension points are closed and versioned here.

Conformance frames contain `target`, `direction`, and `body`. `direction` is part of the contract
artifact used by tests and generators; the surrounding MWS transport continues to carry request and
response correlation.

`control` models the MWS management API. `model` defines the canonical provider-independent address,
external actor, and inbound content semantics implemented independently by local Rust and cloud
Java runtimes. Neither module contains provider SDKs, transport callbacks, runtime lifecycle,
persistence, or Agent integration.

Interactive delivery is modeled as an `ActionSet` of user-facing labels and opaque tokens. An
inbound selection contains only `ActionSelected { token }`; approval decisions, dialog values, and
provider callback payloads stay in runtime-owned route stores. Setup claims and account lifecycle
are framework application ports rather than chat content, so they are intentionally not part of
this public normalized data plane.

Shared-conversation discovery, group binding challenges, and self-service actor links are common
management operations. Challenge requests never carry a target user: each runtime derives the user
from its authenticated request context, authorizes scope ownership or membership, and stores only a
short-lived one-time code hash. Provider adapters remain responsible only for translating provider
events and sending provider-native replies.

An account grant authorizes one mHome principal and scope to use one provider account. A route binds
one provider conversation to that principal/scope (personal) or scope (shared). Routes deliberately
address the provider's base conversation only; lane identity remains on each normalized address and
canonical Conversation surface so topic-level threads do not multiply authorization records.

Setup is provider-extensible but has a common lifecycle. A completed setup may return both the
created provider `accountId` and the initial personal `routeId`; providers that only configure an
account omit `routeId` until the first inbound conversation creates the route.

Actor links are provider-account-scoped identities. One exact external identity maps to at most one
mHome principal, while a principal may own any number of external identities across or within
provider accounts. Actor-link management therefore targets an account, never a conversation
surface; shared-surface binding remains a separate scope relationship.

`fixtures/normalized-inbound.conformance.json` is the executable cross-language corpus. Every
runtime implementation must accept every `valid` case and reject every `invalid` case; adding a
content variant or validation rule requires updating this corpus in the same change.
