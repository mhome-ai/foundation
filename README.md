# foundation

Small, stable Rust leaf libraries shared across mHome repositories.

## Crates

- `mhome-artifact-api`: storage-independent, scoped artifact references.
- `mhome-app-facade-api`: the canonical public `/app/*` request, response, and event protocol.
- `mhome-conversation-api`: client conversation protocol plus Agent execution contracts/ports in `execution`.
- `mhome-llm-api`: canonical model messages, tool schemas, completions and private continuation.
- `mhome-machine-identity`: persistent local machine identity derivation and host naming.
- `mhome-host-auth`: transport-independent Host signatures, verification and Client session rules.
- `mhome-core-api`: shared Core/Node contracts and the canonical Host machine, metrics and permission types in `core_api::host`.
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

The artifact, conversation, App Facade, and Core protocol crates also publish matching, data-only npm
packages for JavaScript consumers' build-time conformance checks. Each npm package contains the
crate's manifest, JSON Schemas, and fixtures without an executable or browser entry point. Its
version is identical to the Cargo crate version and is published publicly from the
same release tag:

- `@mhome/artifact-protocol`
- `@mhome/conversation-protocol`
- `@mhome/app-facade-protocol`
- `@mhome/core-protocol`

Run `npm run test:protocol-packages` to verify package staging. To inspect a tarball locally:

```bash
staging="$(mktemp -d)/package"
node scripts/stage-protocol-package.mjs appFacade "${staging}"
npm pack "${staging}"
```

Protocol npm packages publish from the self-hosted macOS runner with an npm automation token.
The workflow passes repository secret `NPM_TOKEN` as `NODE_AUTH_TOKEN`. It does not request
`id-token: write` and does not generate npm provenance. The token should be limited to
`@mhome/artifact-protocol`, `@mhome/conversation-protocol`, `@mhome/app-facade-protocol`, and
`@mhome/core-protocol`. Those packages must still allow token publishing.

`mhome-artifact-api` 0.1.0 was originally published from Baycat. The contract moved here and this
repository is authoritative beginning with version 0.2.0.

The proposed Host permission extension is documented in
[`crates/core-api/contract/host-permissions.md`](crates/core-api/contract/host-permissions.md). It defines
passive Host observations and separately authenticated local macOS actions.
Protocol definitions alone do not implement the Host/Client/Core/UI behavior.

## Self-hosted CI source layout

CI and crate publishing use `[self-hosted, macOS, ARM64, release-macos-primary]`, the runner that already has `~/.mhome/foundation`. Provision `~/.mhome/{releases,foundation}` with GitHub read access. Workflows fetch the shared Releases bootstrap and create an isolated worktree for the exact workflow event SHA under `~/.mhome/work/`; they never clone product sources or move the canonical checkout branch. Cleanup runs after success or failure. Deploy the Releases bootstrap before enabling these workflows.

Protocol tags publish the crate and the npm package from this runner. Rust-only tags do not read `NPM_TOKEN`.
