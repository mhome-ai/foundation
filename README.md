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
version is identical to the Cargo crate version and is published publicly with provenance from the
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

The published protocol packages trust GitHub Actions for organization `mhome-ai`, repository
`foundation`, and workflow `publish-crate.yml`. The workflow grants `id-token: write` and uses a
compatible npm CLI, so releases authenticate with short-lived OIDC credentials and generate
provenance without an npm publish token. A future package that does not yet have an npm settings
page must be bootstrapped once with a temporary `NPM_TOKEN` wired to `NODE_AUTH_TOKEN`; immediately
after that first publish, configure its Trusted Publisher, remove the repository secret and workflow
fallback, and revoke the bootstrap token on npm.

`mhome-artifact-api` 0.1.0 was originally published from Baycat. The contract moved here and this
repository is authoritative beginning with version 0.2.0.

The proposed Host permission extension is documented in
[`crates/core-api/contract/host-permissions.md`](crates/core-api/contract/host-permissions.md). It defines
passive Host observations and separately authenticated local macOS actions.
Protocol definitions alone do not implement the Host/Client/Core/UI behavior.

## Self-hosted CI source layout

CI uses `[self-hosted, Linux, AMD64, release-linux-amd64]`. Crate publishing uses `[self-hosted, macOS, ARM64, release-macos-primary]`, the runner that already has `~/.mhome/foundation`. Provision `~/.mhome/{releases,foundation}` with GitHub read access. Workflows fetch the shared Releases bootstrap and create an isolated worktree for the exact workflow event SHA under `~/.mhome/work/`; they never clone product sources or move the canonical checkout branch. Cleanup runs after success or failure. Deploy the Releases bootstrap before enabling these workflows.

Self-hosted npm publication needs an explicit authentication decision: npm trusted publishing and `--provenance` currently require cloud-hosted runners (https://docs.npmjs.com/trusted-publishers/). Until token-based or manual npm publishing is selected, protocol-tag publishing fails before publishing either registry. Rust-only crate tags are unaffected. No npm token has been created or configured by this change.
