# Product operation authority v1

This is the normative authority profile for [product onboarding](onboarding-v1.md).
The exported claim types and schemas validate structure and target relationships;
they do not verify signatures, issue grants or register HTTP routes. Production
issuers, device verification and role admission must implement this entire profile
before a product entrypoint is exposed.

## Approval and proof boundaries

An authenticated Space Owner approves one exact Product and Plan. With an existing
Hub, the native commissioner first verifies its authenticated direct-LAN connection
to that selected Hub. A first Hub instead uses the existing cloud-exclusive setup
intent. A caller's `firstHub`, `verified`, role list or Space ID is never authority.
The issuer rechecks the actual Owner, environment and current Hub/setup intent.

Cloud durably saves an approval under environment + root identity/key + installation
generation + operation ID, including the exact descriptor, Plan, approving User
and the admitted Hub/setup intent. This record is not a role credential or a new
User session. Initial approval is durable before returning a usable operation
grant. The same operation cannot acquire another target or different roles.
Revoked, superseded, reset or completed admissions cannot be repurposed by retry.
Retries of a completed admission may retrieve its original outcome without starting
new effects. Completion does not itself terminate the ordinary role credentials.

Use compact JWS with unpadded base64url and a three-member protected header:
`alg`, `typ`, `kid`. Cloud signs RS256 using the existing configured Cloud signing
key infrastructure; this does not create a second signing-key lifecycle.
Unknown protected-header members and duplicate JSON member names are rejected.
The accepted key set is environment-specific, provisioned or refreshed through the
configured trusted Cloud. Honor disabled keys and key-set rollback checks. Never
trust token-provided URLs, keys or an App-provided key set. `kid` identifies that
signing key; it is not the product's P-256 point fingerprint.

The operation grant has `typ:meow-onboard-operation+jws`. Its payload is exactly
`OperationGrant`: `protocolVersion`, `iss:meow-onboard-authority`,
`aud:meow-product-onboard`, `sub` (approving User ID), fresh 128-bit hex `jti`,
`environment`, `product` (root identity), the complete `plan`, `issuedAtMs` and
`expiresAtMs`. Times are UTC epoch milliseconds, **not JWT NumericDate seconds**.
There are no generic permissions, package IDs, User refresh tokens or operating
credentials. Maximum lifetime is five minutes. Issuance accepts at most five
seconds of future clock skew; expiry has no extension or grace period. All times
must remain positive JavaScript-safe integers. The entire proof is bounded by
Start's 8192-byte limit. Verify signature/type/key, issuer/audience, lifetime and
every target field; signature verification alone is insufficient.

The product's trusted installation authenticates its own root key and component
declarations. Receiving a matching public descriptor is not proof of possession.
The operation verifier must be wired to that real root identity. Cloud and Hub
component admissions also authenticate a fresh root-signed exchange, rather than
redeeming a copied operation token as an unbound bearer credential. The signed
plan authorizes only its declared components in the same Space.

## User retry and root possession

The approving User is independent of the commissioning phone. Once inputs and
valid authority reach the product, the device owns execution. An error, restart
or expired authority pauses unfinished onboarding; no unattended authority-renewal
loop is required or exposed. A user explicitly retries through an authenticated
App. The App reads the saved Plan and operation ID and requests fresh authority
for that exact operation. It does not generate a replacement operation, execute
role steps, clear healthy credentials or treat a timeout as confirmed absence.
The issuer rechecks the actual Owner, original approval and current Hub/setup
intent before approving the retry. A different authenticated Client can carry
this exchange; possession of the original phone is not authority.

The following challenge/possession exchange remains required for initial and
user-retried approval and applicable role admission. Removing unattended renewal
does not turn an operation grant into an unbound bearer credential. The App may
carry the bounded exchange, but only the product holds its root private key.
Root keys and role credential candidates stay in their protected owners' stores;
User login tokens are not saved on the product, and raw operation proofs never
enter the public operation journal or status.

1. The device creates a fresh random 128-bit `deviceNonce` for this live exchange.
2. Cloud returns a signed `AuthorityChallenge`, with type
   `meow-onboard-challenge+jws`, exact environment/root/Plan, reflected device nonce,
   a fresh server nonce, and a lifetime of at most 60 seconds. Cloud persists the
   challenge against the approved operation before accepting its redemption.
3. The device verifies the configured Cloud signature and reflected nonce. It signs
   `PossessionProof` with its root P-256 key, type
   `meow-onboard-possession+jws`, ES256 and the existing RFC 7638 thumbprint header
   convention. `challengeDigest` is SHA-256 of the exact ASCII compact challenge
   bytes, not a reserialized JSON payload. ES256 signatures use 64-byte R||S.
4. Cloud verifies the root public point saved with the approval, the signature,
   digest and challenge expiry, then consumes its server nonce once. It rechecks
   the approving User's current Owner authority and the admitted Hub/setup intent.
   Only then may it issue another short-lived grant for the exact same operation.

A lost approval reply does not change the approval, candidates or component grants;
a later explicit user retry uses a fresh challenge for the same Plan. Challenge
consumption and the approved target are serialized by the issuer. Reset/revocation
invalidates outstanding challenges. Knowing an operation ID or a challenge cannot
approve, continue or transfer a product.

Native adapters use OS UTC. MCU uptime is never epoch time. An MCU obtains a
trusted time anchor from a pinned Hub's fresh authenticated exchange or the
configured Cloud's signed response reflecting its new nonce, with bounded measured
round-trip time and monotonic holdover. Do not disable general TLS verification,
trust an unsigned Date header or retain a pre-reboot monotonic deadline. Until a
usable time/authority anchor is available, report the blocked state and start no
new effects. Ordinary role reconnect remains separate from this onboarding gate.

Expired authority cannot initiate writes. Reading/reconciling already committed
facts does not imply permission to issue another grant. On restart, load the
pending operation and expose the paused result. On an explicit user retry, verify
fresh authority for the same Plan, reconcile its original candidate, and only
then continue. Reconciliation does not replay business actions. Never create a
replacement candidate merely because the old grant or transport session expired.

Clock unavailability pauses new effects and is reported as `clock_unavailable`.
Do not persist a fabricated zero or uptime `updatedAtMs`; retain the last valid
saved timestamp while reporting the current volatile blocked reason. Restoring
time alone does not restart a paused operation: explicit retry with valid
authority is still required. Ordinary role reconnect is unaffected.

## Component admission, including Wave

The device owner submits the approved operation and fresh root possession to each
actual issuer. Cloud retains Host authorization authority; the pinned Core retains
Pod registration and Wave admission authority, using the established Cloud trust
where required. Each issuer verifies the relevant declared component, exact Space,
root/generation/operation and current policy. It stores a stable result keyed by
installation + operation + role before reporting commitment. Role private keys,
claim receipts, registration candidates and grants retain their existing owners.
No adapter impersonates the User by forwarding a cached login token to an old
User-only endpoint.

Physical pairing approves the product operation. It must not require a second
physical authorization for each declared role. In particular, a first-Hub cloud
sync can outlast Wave's ordinary 120-second addition window. A product adapter
therefore needs an explicit, verified operation-bound Wave admission path. It
must not silently extend/reopen the public Wave window or skip its security
checks by calling an internal `confirmed` hook. Standalone Wave physical pairing
keeps its bounded window and still requires the approved product operation;
physical pairing alone is never a Cloud-approval bypass.

The operation-bound path authenticates both the product approval/root and the
selected Core, then uses Wave's existing candidate persistence, grant commitment
and authenticated access checks. It is not a local grant import or a public
bypass API. After a reboot, an explicit user retry obtains fresh operation
authority; it does not require another physical pairing while the original
approved operation is recoverable. Test this with first-Hub synchronization
exceeding 120 seconds, restart between
components, expired/revoked approvals and lost Wave commit replies.

Wave admission completes with confirmed credentials and authenticated protocol
access. Integration metadata, inventory validation, package selection/acquisition
and OTA capability discovery occur afterwards and confer no onboarding authority.

## Required issuer and adapter verification

Production acceptance includes wrong issuer/key/type/algorithm, disabled keys,
wrong root/environment/generation/descriptor/Space/Hub, changed roles or Plan,
expired/future/overlong authority, invalid signatures, replayed challenge, copied
grants without root possession, Owner removal, superseded first-Hub intent and
user retry after restart/expiry, unavailable time, and uncertain credential
commitment. Schema/fixture tests do not satisfy these cryptographic or
installed-product acceptance checks.
