# mhome-messaging-api

Transport-neutral management contract for messaging providers. The crate owns the typed Rust DTOs,
the target inventory, strict request/response JSON Schema, and cross-language conformance fixtures.

The common contract deliberately treats `data`, `providerData`, `options`, and `result` as provider
extension objects. Their contents are decoded and validated by the selected provider; all fields
outside those explicit extension points are closed and versioned here.

Conformance frames contain `target`, `direction`, and `body`. `direction` is part of the contract
artifact used by tests and generators; the surrounding MWS transport continues to carry request and
response correlation.

It intentionally contains no provider SDK, transport, runtime lifecycle, persistence, or Agent
integration.
