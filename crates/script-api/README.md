# Script API

Shared leaf contracts for TypeScript-authored Integration functions and Automation
scripts. This crate defines immutable resource references, build-generated
Automation contracts, artifact metadata and internal outcomes. It does not load
code, grant permissions, implement host IO or execute scripts.

The [JavaScript script contract](contract/javascript-v1.md) is authoritative for
the script execution boundary. The fixture corpus under `fixtures/` is shared with
consumer implementations. Models must be validated after deserialization and
before use. Validation is not authorization or digest verification.

```sh
cargo test --locked -p mhome-script-api
```

The [Script repository](https://github.com/mhome-ai/script) owns the TypeScript SDK,
builder and shared native QuickJS-NG runtime. Core uses that runtime for Integration
functions and Automation scripts; Lion validates contracts and generates execution
plans. Cloud Automation builds run through Tiger's managed TypeScript build service.
Cloud JS execution and script-bearing Space mode switches remain disabled until
the trusted Space host adapter is implemented. There is no Wasm compatibility path.
