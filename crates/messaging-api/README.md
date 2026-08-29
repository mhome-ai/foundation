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

Every normalized message carries provider-neutral attention evidence: `addressed` means the
provider determined that the bot was explicitly targeted, `unaddressed` means it determined the
opposite, and `unknown` means the provider has no equivalent signal. Provider adapters extract this
evidence; common runtimes ignore only `unaddressed` messages on shared surfaces. Personal messages,
actions, and providers with unknown attention continue through the normal pipeline.

Interactive delivery is modeled as an `ActionSet` of user-facing labels and opaque tokens. An
inbound selection contains only `ActionSelected { token }`; approval decisions, dialog values, and
provider callback payloads stay in runtime-owned route stores. Setup claims and account lifecycle
are framework application ports rather than chat content, so they are intentionally not part of
this public normalized data plane.

Shared-conversation discovery, group binding challenges, and self-service actor links are common
management operations. Actor-link challenges explicitly target either a personal conversation or
one already-bound shared surface. A shared `/link` command records an immutable external-actor
candidate and requires confirmation by the authenticated challenge creator; a personal command may
complete immediately. Status is the recovery source of truth and the matching event target is only
a realtime hint. Challenge requests never carry a target user: each runtime derives the user from
its authenticated request context, authorizes scope ownership or membership, and stores only a
short-lived one-time code hash. Provider adapters remain responsible only for translating provider
events and sending provider-native replies.

A personal route belongs to one linked mHome user. Its `scopeId` is only the conversation's current
scope: authorization comes from live scope membership, and `/switch` may move it to any scope the
user still belongs to. Personal routes never depend on an account grant.

A shared account grant authorizes one provider account to serve one scope. It has no beneficiary
user and no default flag; `grantedByUserId` is audit metadata. Every shared route is fixed to one
scope, depends on the matching shared account grant, and cannot switch scopes. Current scope owners
manage grants and shared routes. Any linked actor who is still a member of the bound scope may answer
a shared interaction, while personal interactions remain restricted to the personal route owner.
Routes deliberately address the provider's base conversation only; lane identity remains on each
normalized address and canonical Conversation surface so topic-level threads do not multiply
authorization records.

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
