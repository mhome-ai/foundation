# Schemas

`envelope.v1.schema.json` is the normative JSON Schema 2020-12 contract. Protocol tests compile the
schema, validate every committed fixture, deserialize each fixture into Rust DTOs, and require
fixtures to match canonical Rust serialization.

Emit and accept are one protocol, not a third roundtrip shape:

- **Emit** is Rust `Serialize`: skip-optional `None` is omitted; required `Option` fields stay
  present as `null`.
- **Accept** is the schema plus `serde` + unknown-field rejection + `validate()`. Skip-optional
  keys are optional booleans, not `boolean|null`. An explicit `null` on those keys is still
  deserialized as `None` so older emitters are not a third contract.

The Java adapter must encode through the typed protocol DTOs (same omit rules) rather than
hand-built trees. Transport-specific handshakes wrap this envelope; they do not fork it.
