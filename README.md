# foundation

Shared Rust leaf libraries and TypeScript script tooling for MeowLink and MeowCore.

## Crates

- `mhome-script-api`: JavaScript resource identities, artifacts and Automation contracts.
- `mhome-script-runtime`: native QuickJS-NG execution, bounded host calls and shared JS workers. Core uses this runtime; cloud execution remains deferred.

- `mhome-person-api`: Space-owned Person management, embedding contracts and authoritative observations.
- `mhome-face-matching`: shared pure matching over model-compatible galleries.

- `mhome-artifact-api`: storage-independent, scoped artifact references.
- `mhome-storage-api`: canonical Folder, object and storage failure contracts.
- `mhome-app-facade-api`: the canonical public `/app/*` request, response, and event protocol.
- `mhome-conversation-api`: client conversation protocol plus Agent execution contracts/ports in `execution`.
- `mhome-llm-api`: canonical model messages, tool schemas, completions and private continuation.
- `mhome-machine-identity`: persistent local machine identity derivation and host naming.
- `mhome-host-auth`: transport-independent Host signatures, verification and Client session rules.
- `mhome-os-permissions`: in-process permission status, requests, and System Settings links.
- `mhome-core-api`: shared Core/Node contracts and the canonical Host machine, metrics and permission types in `core_api::host`.
- `mhome-playground-models`: deterministic playground device models and projections.
- `mhome-runtime-paths`: process-safe runtime paths, endpoint names, and daemon ownership checks.

Foundation crates may depend on third-party crates or lower-level crates in this workspace. They
must not depend on Baycat, MeowCore, Agent, or cloud implementations, databases, transports, or
product workflows. Shared wire contracts and deterministic domain models are allowed when they
remain leaf libraries. A crate belongs here only when it has multiple repository consumers or
defines a genuinely shared leaf contract.

The script contract is [JavaScript script v1](crates/script-api/contract/javascript-v1.md).
TypeScript SDK and builder sources live in `packages/script-sdk` and `packages/script-build`.
Scripts use ESM JavaScript; there is no Rust/Wasm execution fallback.

## Development

The repository toolchain and CI use Rust 1.95.0.

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace --locked
```

Crates are versioned independently. A tag named `<package>-v<version>` publishes exactly one crate,
for example `mhome-runtime-paths-v0.1.0`. The tag version must equal that package's manifest version.
Publication is immutable and restricted to the allowlist in `scripts/publish-tag.sh`.

The JavaScript contract and native runtime publish as `mhome-script-api-v<version>`
and `mhome-script-runtime-v<version>`. Publish the API's exact pinned version
before the runtime. `@meowlink/script` and `@meowlink/script-build` are independently
installable npm packages. Source workspaces use private local links;
`scripts/stage-script-package.mjs` creates distributable manifests with exact registry
dependencies. Their offline packaging/install tests run in CI. Crate tags do not
publish these npm packages. Dedicated `meowlink-script-v<version>` and
`meowlink-script-build-v<version>` tags use the same hosted publication workflow.
Publish the SDK first; the builder release verifies its exact published SDK bytes.
`bash scripts/publish-tag.sh <tag>` builds, tests and packs locally without upload;
only the tagged workflow uses `--publish`. Configure npm trusted publishing for
both packages before their first release. The workspace release coordinator
publishes these packages in SDK → builder order after the Foundation crates,
waiting for exact-tag CI and registry availability before advancing consumers.
Reused tags are verified against their original commit, and
`bash scripts/publish-tag.sh <tag> --verify-published` checks the current packed
contents against the immutable registry version without uploading. Changed package
contents require a new version. Dry-run defers this build/content verification.

Registry upload acceptance does not imply Cargo index availability. Tagged
publication waits for exact internal dependency versions in the crates.io sparse
index before packaging, and for the uploaded version before completing CI.
`scripts/wait-for-crates.mjs` shares this check with the workspace coordinator:
it waits up to ten minutes, bounds individual requests, rejects yanked or invalid
entries, and never retries uploads. Run its offline regression tests with
`node --test scripts/tests/crate-readiness.test.mjs`.

The artifact, conversation, App Facade, and Core protocol crates also publish matching, data-only npm
packages for JavaScript consumers' build-time conformance checks. Each npm package contains the
crate's manifest, JSON Schemas, and fixtures without an executable or browser entry point. Its
version is identical to the Cargo crate version and is published publicly with provenance from the
same release tag:

- `@mhome/artifact-protocol`
- `@mhome/conversation-protocol`
- `@mhome/app-facade-protocol`
- `@mhome/core-protocol`

Run `npm ci` before protocol checks or tagged crate verification. Then run
`npm run test:protocol-packages` to verify package staging, manifest file references
in actual npm tarballs, strict AJV schema compilation and Artifact conformance fixtures.
Tagged protocol releases run these checks before
uploading either package. To inspect a tarball locally:

```bash
staging="$(mktemp -d)/package"
node scripts/stage-protocol-package.mjs appFacade "${staging}"
npm pack "${staging}"
```

Publication process tests execute the real inline Python parser and simulate
only the Python registry CLI boundary. The ordinary CI also runs the protocol
and publication tool tests on Linux with the release Node/npm versions, alongside
the existing macOS CI.

The published protocol packages trust GitHub Actions for organization `mhome-ai`, repository
`foundation`, and workflow `publish-crate.yml`. That workflow runs on `ubuntu-latest`, grants
`id-token: write`, and publishes with provenance. No npm token is required.
The release workflow pins Node 24.22.1 and npm 12.2.1. Before uploading either
package, `scripts/protocol-provenance.mjs` signs the final npm tarball's SLSA
statement and verifies its certificate identity, signature and transparency-log
proof. Its Rekor client fetches an equivalent existing entry after a duplicate
registration response, preserving provenance when a successful request loses its
response. npm receives that verified bundle through `--provenance-file`; it does
not generate another attestation. Signature or proof verification failures stop
publication. Run the offline recovery tests with
`node --test scripts/tests/protocol-provenance.test.mjs`.

`mhome-artifact-api` 0.1.0 was originally published from Baycat. The contract moved here and this
repository is authoritative beginning with version 0.2.0.

The proposed Host permission extension is documented in
[`crates/core-api/contract/host-permissions.md`](crates/core-api/contract/host-permissions.md). It defines
passive Host observations and separately authenticated local macOS actions.
Protocol definitions alone do not implement the Host/Client/Core/UI behavior.

## Self-hosted CI source layout

CI uses `[self-hosted, macOS, ARM64, release-macos-primary]`. Provision `~/.mhome/{releases,foundation}` with GitHub read access. The CI workflow fetches the shared Releases bootstrap and creates an isolated worktree for the exact workflow event SHA under `~/.mhome/work/`. Crate publishing uses GitHub-hosted `ubuntu-latest` and checks out the tagged commit directly, so npm trusted publishing and provenance work.
