# Host offline authorization (implementation draft)

This contract is not published. Authentication is independent of Space/Core routing.
`hostId` identifies the machine; Host P-256 identity keys and local user ACL survive updates.
Client keys are separate from OAuth sessions. Expiring a cloud session does not revoke an offline Client;
explicit logout deletes its local private key. Host users are added only via protected local administrator IPC.

The trusted cloud API is a configured HTTPS origin/path, never a URL obtained from LAN discovery or a token.
`GET /api/v1/host/auth/keys` returns `SigningKeySet`: issuer, monotonically increasing version and complete RSA public-key list.
RS256 is fixed. Exactly one active key; verification-only keys allow overlap; disabled keys cannot authorize bindings.
Missing/disabled enrollment kids require re-registration. Disabled tombstones cannot disappear or reactivate.
Same-version conflicting content, key-ID rebound and version rollback must be rejected atomically.

Host checks every 24 hours since the last successful check, including an overdue check after restart.
Unknown enrollment-proof kids trigger a coalesced rate-limited check. Known-kid invalid signatures do not.
Network failure preserves established trust and backs off; 24 hours is not an authorization expiry.
A learned disablement prevents affected bindings from managing Host until a fresh cloud proof is registered.

Bootstrap endpoints (POST):
- `/v1/auth/context`: random probeId → Host-signed context (protocol, hostId, bootId, probeId, serverTime).
- `/v1/auth/enroll/challenge`: issuer/user/Client P-256 key/probe → Host-signed short-lived challenge.
- `/v1/auth/enroll`: cloud enrollment proof + Client signature of its digest → durable registration.
Bootstrap responses cannot grant permissions without cryptographic verification and the local user ACL.

Cloud APIs under `/api/v1/host`:
- POST `/authorizations/init`, `/authorizations/check`, `/authorizations/approve`, `/authorizations/result`, `/authorizations/commit` for the local administrator transaction.
- POST `/identity/get`: authenticated user's previously confirmed Host public key.
- POST `/client-enrollments/issue`: authenticated user plus Host challenge and Client possession proof → five-minute RS256 enrollment proof.

HTTP signatures follow a fixed RFC 9421 profile:
- `Content-Digest`: SHA-256 over exact transmitted body; GET has an empty body.
- `Content-Type`: `application/json`.
- `X-Meow-Auth`: unpadded base64url of UTF-8 JSON identity.
- Request identity: issuer, userId, hostId, clientKeyId, bootId, requestId, created, expires.
- Request covered components, in order: `@method`, `@target-uri`, `content-digest`, `content-type`, `x-meow-auth`.
- Response identity: hostId and requestDigest (SHA-256 binding to the signature base and signature of the original request).
- Response covered components: `@status`, `content-digest`, `content-type`, `x-meow-auth`.
- Signature label `meow`, alg `ecdsa-p256-sha256`, tag `meow-host-auth-v1`; keyid is the RFC 7638 P-256 JWK thumbprint.
- ECDSA signatures are raw 64-byte r||s, not ASN.1 DER. No algorithm negotiation or token key URLs.
- Reject duplicate covered headers; verify before parsing or applying management results.

Request IDs are random 32-byte base64url values. Maximum validity 60 seconds, bounded replay table retains entries until expiry.
A full table rejects new requests; it never evicts unexpired entries. A random bootId invalidates all prior process contexts on restart.
An already accepted request is never automatically replayed because the network failed.

Host rejects invalid authorization before dispatch. `HOST_REAUTH_REQUIRED` and `HOST_ENROLLMENT_REQUIRED` are
signed and bound to the original request. After verifying them, Client may coalesce cloud-assisted enrollment and
retry once, preserving the business operationId while using a fresh signature requestId. `HOST_CONTEXT_CHANGED`
allows one local context refresh. Unsigned errors, timeouts and business errors do not authorize automatic replay.

HTTP signatures provide authentication/integrity/replay detection, not confidentiality. LAN TLS is separate work.
Cross-language schemas, fixed vectors and full native Client integration remain required before release.
