# Schemas

`envelope.v1.schema.json` is the normative JSON Schema 2020-12 contract. Protocol tests compile the
schema, validate every committed fixture, deserialize each fixture into Rust DTOs, and require an
exact normalized round trip.

The Java adapter should generate or hand-maintain DTOs against this schema and run the same fixture
directory in CI. Transport-specific handshakes wrap this envelope; they do not fork it.
