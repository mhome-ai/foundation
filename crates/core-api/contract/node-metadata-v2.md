# Node source metadata

`POST /node/metadata` with `{}` is available over commissioning HTTPS and the
existing authenticated Node request transport. It returns `NodeMetadata` under
`schema/node-metadata.v2.schema.json`. It does not start a business runtime.

Delivery describes ownership, independently of `kind` (`core` or `extension`):

- `host`: published services ask their local Host for the actual installed release
  manifest. Preview services read their launch-specific Host proof, described below.
  Published evidence comes from installation state, never the latest catalog.
  The SDK checks node type, service ID and version against its running process.
- `firmware`: a dedicated implementation embeds its publisher-signed definition
  and version in the binary. Updates replace it with the firmware/application.
  A single-service firmware can use its firmware version as `serviceVersion`.
  This requires no Host installation or separately incremented version counter.

An Android Node bundled through an AAR uses `firmware` delivery even on a
general-purpose phone: the AAR is an App build dependency, and the running Node
reports its embedded AAR release version. Publishing a new AAR does not update an
installed App. This classification describes the update owner, not the hardware
category or whether the Plugin is `core` or `extension`.

A `source.kind=published` response carries `source.releaseManifest` with exact
signed manifest bytes and the detached Minisign signature. Host manifests use `meow.plugin.manifest[-bundle]`; embedded releases
use `meow.node.firmware-manifest[-bundle]`, with `platform`, `pluginId`,
`componentId`, `version`, `nodeMetadata.plugin` and `compatibility` at the payload
root. Host release components retain their normal package manifest structure.

Core verifies environment-scoped publisher trust, schema, definition, runtime
identity and version. A Node cannot choose its own trust key. `core` definitions
must match Core's private protocol revision; `extension` has no private protocol.
The immutable identity consists of id, nodeType, serviceId, componentId, kind and
bindingCardinality, consistent across versions, platforms and delivery forms.
Display strings can change. The catalog is not a binding authority.

The signed onboarding challenge includes `serviceVersion` and `manifestSha256`
(SHA-256 of the exact signed manifest UTF-8 bytes). The token-issue request carries
`metadata`; Core compares its verified version/hash to the challenge. Fetching
metadata before a challenge is a browsing hint only. Token-bearing commissioning
requests still require the challenge-proven TLS key.

Core persists `NodeBindingMetadata` with each binding for offline management.
Every `/node/auth` request includes required `metadata` for the running release.
Core authenticates the token and binding first, then verifies the publisher
signature (or explicitly selected Host Preview proof), stable identity, runtime
version and private protocol compatibility.
Only then may it activate the token and return `ok: true`. Release validation
failure rejects the connection without revoking the binding or requesting
`CLEAR_LOCAL_IDENTITY`. No catalog/network lookup is part of authentication.

A successful auth response includes `bindingMetadata` and the accepted optional
`coreProtocolVersion`. Transports check this acknowledgement against the requested
release (including its manifest hash), then carry that exact snapshot into
connection registration. Each connection retains its own immutable snapshot;
a concurrent reconnect changing persisted metadata cannot change an existing
connection's admitted version. Persisted metadata is written only when changed.
There is no fingerprint negotiation, signature cache or second readiness handshake.

After authentication Core loads `/<nodeType>/describe` for that connection's App
access policy. Its schema, node type, service ID and version must agree with the
admitted release. App operations wait for a valid policy; normal Node operations
can start after auth. A failed description closes the connection. Core does not
pull `/node/metadata` again after auth. The metadata endpoint remains available
for commissioning. No legacy static registry or inferred cardinality is permitted.

The complete serialized authentication envelope must fit the peer's transport
limit, including JSON escaping of the nested manifest and MWS payload. Native
MWS allows 2 MiB; firmware may impose a smaller limit (ESP uses 32 KiB). SDK and
firmware reject oversized auth frames before sending them. Publisher metadata
proves release identity and compatibility, not remote binary attestation;
installed package integrity remains the installation chain's responsibility.

Manifest bytes are bounded to 1 MiB, signatures to 4 KiB. No publisher secret or
account credential is included. Existing databases require explicit rebuilding
when the baseline binding snapshot column is introduced; startup does not migrate
or erase data.

## Native Preview

`source.kind=preview` is Host delivery only. It carries exact `manifest` bytes and
`hostProof`, a compact JWS under the existing Host P-256 key and the dedicated
`meow-plugin-preview+jwt` purpose. The proof binds Host ID, runtime environment and
manifest SHA-256. It is provenance, not a Space token; there is no separate lease.
A publisher verification failure never tries this path.

The small manifest contains formatVersion=1, Plugin definition, platform,
serviceVersion, optional coreProtocol and filesSha256. The complete file inventory
stays on Host in `preview-files.json`; its hash is covered by the manifest/proof.
The manifest also carries an explicit `requires` map (possibly empty), keyed by
`host`/`client` with version requirements. The installing Host parses and checks
these against installed platform versions before stopping the current service.
It persists them with installed compatibility for subsequent platform update
planning. Preview does not silently upgrade Host or Client.
The inventory contains safe relative paths and each file's size and SHA-256.
The Host copies and verifies a complete candidate before selection. Proofs never
include private keys, account credentials or native payload bytes.

Initial enable uses the ordinary authenticated Space Manage request with an
explicit Preview expectation (hostId, hostPublicKey, manifestSha256) obtained from
that computer's OS-authenticated Host socket. Node-supplied metadata cannot select
the trusted key. The signed commissioning challenge must match that expectation.
Bindings record their verified Host public key for both published and Preview
sources. Reconnect verifies Preview with that saved key, the token's Host identity,
and current environment; this allows one Host's replacement to reach every
existing bound Space without issuing new credentials. A failed check leaves the
binding intact and disconnected.

The SDK reads Preview evidence from its actual canonical executable package,
never from the current mutable Host selection. Missing/corrupt Preview launch
evidence fails; a running older same-version process cannot claim a newer build.
Auth acknowledgements must match source kind as well as manifest hash/version.
Published firmware retains publisher verification and never accepts Preview.

This v2 envelope replaces v1; Host, Core, native SDK consumers and firmware must
consume the matching protocol together. No legacy reader or database conversion
is provided. Runtime business data and binding identities remain unchanged by a
normal Preview replacement.

## Community published service provenance

A verified Host release may include `publication` with `kind: community`,
`publisherId` and `integrations` (at most 100 distinct qualified package IDs).
The authenticated stable account ID owns `u-<first 32 lowercase hex SHA-256>-<slug>`;
community definitions must use that namespace and `kind: extension`. This field
is interpreted only after validating the enclosing platform signature. It is
retained in `NodeBindingMetadata` for Space source acquisition and native updates.
Absence retains ordinary official release behavior; Preview never claims this
provenance. The signature does not grant Space, Host or operation permissions.

Host validates the release's artifact digest before choosing the community macOS
payload policy: intact ad-hoc signatures are allowed without an Apple Team
requirement. Official payload verification retains its existing Team requirement.
The Dev Kit supplies missing ad-hoc signatures; publishers manage no keys. The
community test fixture is signed with a discarded ephemeral test key and is never
a production trust source.
