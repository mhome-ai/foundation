# Node release metadata

`POST /node/metadata` with `{}` is available over commissioning HTTPS and the
existing authenticated Node request transport. It returns `NodeMetadata` under
`schema/node-metadata.v1.schema.json`. It does not start a business runtime.

Delivery describes ownership, independently of `kind` (`core` or `extension`):

- `host`: the SDK asks its local Host for the actual installed release manifest.
  The Host returns persisted installation evidence, never the latest catalog.
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

The response carries exact signed manifest bytes and the detached Minisign
signature. Host manifests use `meow.plugin.manifest[-bundle]`; embedded releases
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
signature, stable identity, runtime version and private protocol compatibility.
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
