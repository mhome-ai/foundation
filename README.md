# foundation

Small, stable Rust leaf libraries shared across mHome repositories.

## Crates

- `mhome-artifact-api`: storage-independent, scoped artifact references.
- `mhome-machine-identity`: persistent local machine identity derivation and host naming.
- `mhome-meowcore-api`: shared MeowCore wire types and service contracts.
- `mhome-playground-models`: deterministic playground device models and projections.
- `mhome-runtime-paths`: process-safe runtime paths, endpoint names, and daemon ownership checks.

Foundation crates may depend on third-party crates or lower-level crates in this workspace. They
must not depend on Baycat, MeowCore, Agent, or cloud implementations, databases, transports, or
product workflows. Shared wire contracts and deterministic domain models are allowed when they
remain leaf libraries. A crate belongs here only when it has multiple repository consumers or
defines a genuinely shared leaf contract.

## Development

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace --locked
```

Crates are versioned independently. A tag named `<package>-v<version>` publishes exactly one crate,
for example `mhome-runtime-paths-v0.1.0`. The tag version must equal that package's manifest version.
Publication is immutable and restricted to the allowlist in `scripts/publish-tag.sh`.

`mhome-artifact-api` 0.1.0 was originally published from Baycat. This repository contains the same
source and becomes authoritative beginning with its next release.
