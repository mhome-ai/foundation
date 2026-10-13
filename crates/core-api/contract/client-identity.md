# Authenticated Client identity

`client_identity::AuthenticatedClient` is the common identity value for an
authenticated interactive Client. Pod is a device form, not a principal kind.
Personal Pods use `ClientPrincipal::User`; Space Pods use
`ClientPrincipal::Space` and never inherit the initializing user's identity.

The stable application identifier is `appClientId` (the existing `a:` namespace).
The connection identifier remains a separate, ephemeral routing value.
`credentialId` identifies an enrollment, formerly called `podId` in Pod-specific
code. Re-enrollment may keep the installation identifier while replacing the
credential and key. Neither identifier proves possession of a credential.

A Space Client is bound to one `(tenantId, scopeId)` and has a required enrollment
identifier. Its principal is not a Space owner, Core service, or user. User
Clients may select different Spaces subject to current membership and operation
authorization. Enrollment identity does not freeze a personal Pod to its initial
Space. The initializing user's ID belongs in enrollment audit data, never as a
fallback acting user for a Space Client.

Only trusted ingress may attach this value after verifying credentials.
Deserializing the wire value or calling `validate_for` is not authentication.
`validate_for` checks canonical identifiers and target binding; operation-specific
authorization must follow it. Never accept the identity from an ordinary request
body, source/device-type string, clientInfo supplied by the peer, or UI mode.
Keep the original identity across cloud relay, Core dispatch and queued work.

## Implementation boundary

This additive module is an identity foundation, not an ingress cutover. Existing
`AuthenticatedSession`, MWS bridge metadata and `ServiceCoreInput::ClientRequest`
still use the previous user-based contracts. No runtime entrypoint accepts Space
Client credentials merely because these types exist. Replace the contracts and
all their producers/consumers together with the Space Client operation policy.
There is no intended legacy-user fallback for a Space principal.

Host policy is already decided: Space Clients cannot manage Admin Hosts; they
have read-only Host-management access to a Host bound to the same Space. User
writes to Space Hosts require current Space OWNER through Core. This does not
define general Space business permissions. The agreed business hierarchy is
[READ < WRITE < MANAGE](space-access.md), with Space Clients capped at WRITE.
Ordinary business configuration such as dashboards and automations is writable;
identity, credentials and infrastructure require management. The existing
`SPACE_EXECUTE` classification also contains Host claim/runtime operations, and
`SPACE_WRITE` contains both ordinary data edits and runtime configuration. Neither
classification can be granted wholesale to a Space Client. API adapters must
reclassify these operations by their actual effects before enabling the policy.

Device factory reset remains the supported way to clear device enrollment. It
must stop old sessions, persist reset intent, erase device credentials and keys,
and resume cleanup before authentication after interruption. Remote reset is an
authorized command executed by the device, not a server-only revocation. It must
bind the current enrollment/reset generation and must not replay after a later
enrollment. Pod-only reset and whole-appliance reset have different data scope.
This increment adds no remote reset endpoint or server-side revocation system.

## Verification

`tests/client_identity.rs` covers exact Space binding, absence of an acting user
for Space Clients, personal Pod identity, rejection of mixed/unknown principal
fields, missing enrollment, connection-ID confusion and credential replacement.
These are structural contract tests; they do not establish signature validation,
operation authorization, lifecycle safety or end-to-end runtime behavior.
