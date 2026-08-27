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

`fixtures/normalized-inbound.conformance.json` is the executable cross-language corpus. Every
runtime implementation must accept every `valid` case and reject every `invalid` case; adding a
content variant or validation rule requires updating this corpus in the same change.
