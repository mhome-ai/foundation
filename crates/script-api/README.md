# Script API

Shared leaf contracts for TypeScript-authored Integration functions and Automation
scripts. This crate defines immutable resource references, build-generated
Automation contracts, artifact metadata and internal outcomes. It does not load
code, grant permissions, implement host IO or execute scripts.

The [JavaScript script contract](contract/javascript-v1.md) is authoritative for
the new execution boundary. The fixture corpus under `fixtures/` is shared with
consumer implementations. Models must be validated after deserialization and
before use. Validation is not authorization or digest verification.

```sh
cargo test --locked -p mhome-script-api
```

The new protocol is under migration. Existing Core and Lion consumers still use
the old Wasm paths until their separate cutover; this crate does not make JavaScript
execution available by itself.
