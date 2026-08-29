# foundation

Small, stable Rust leaf libraries shared across mHome repositories.

## Crates

- `mhome-artifact-api`: storage-independent, scoped artifact references.
- `mhome-conversation-api`: transport-neutral DTOs for the client-facing conversation protocol.
- `mhome-messaging-api`: transport-neutral DTOs for messaging provider management.
- `mhome-machine-identity`: persistent local machine identity derivation and host naming.
- `mhome-core-api`: shared core wire types and service contracts.
- `mhome-playground-models`: deterministic playground device models and projections.
- `mhome-plugin-api`: transport-neutral envelopes, settings primitives, and typed built-in Plugin contracts.
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

The conversation, messaging, and Plugin protocol crates also publish matching, data-only npm
packages for JavaScript consumers' build-time conformance checks. Each npm package contains the
crate's manifest, JSON Schemas, and fixtures without an executable or browser entry point. Its
version is identical to the Cargo crate version and is published publicly with provenance from the
same release tag:

- `@mhome/conversation-protocol`
- `@mhome/messaging-protocol`
- `@mhome/plugin-protocol`

Run `npm run test:protocol-packages` to verify package staging. To inspect a tarball locally:

```bash
staging="$(mktemp -d)/package"
node scripts/stage-protocol-package.mjs messaging "${staging}"
npm pack "${staging}"
```

New npm packages are bootstrapped once with the repository `NPM_TOKEN` secret. After the first
publish, configure each package's npm Trusted Publisher for GitHub Actions with organization
`mhome-ai`, repository `foundation`, and workflow `publish-crate.yml`. The workflow grants
`id-token: write` and uses a compatible npm CLI, so subsequent publishes authenticate with
short-lived OIDC credentials and generate provenance without a long-lived npm publish token.
After all protocol packages have a Trusted Publisher, remove the `NPM_TOKEN` repository secret and
revoke the bootstrap token on npm.

`mhome-artifact-api` 0.1.0 was originally published from Baycat. This repository contains the same
source and becomes authoritative beginning with its next release.
