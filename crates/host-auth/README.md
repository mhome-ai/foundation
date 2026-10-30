# mhome-host-auth

Shared Host management policy, cloud-token validation rules and Host TLS identity.
The canonical wire types live in `core_api::host::access`. The optional `authority`
feature owns server-side JWT verification, persistent Host identity and administrator
ACLs. Platform adapters own runtime lifecycle and transport wiring.

- `policy`: fixed `admin`, `spaceSelf` or `spaceDep` mode, current admin ACL or one Space,
  binding generation, permissions and effective service runtime policy.
- `access`: short-token issuer/audience/target/lifetime/permission validation,
  signed challenge validation and first-claim exchange-secret binding.
- `signatures`: Host P-256 identity, key thumbprints and purpose-bound compact JWS.
- `authority` (optional): shared cloud key cache and validation, claim completion,
  administrator application/approval/removal and secure identity/ACL persistence.
  `authority::http` supplies the common bootstrap admission and HTTPS token gate.
  Adapters must not implement a second platform-specific authorization engine.
- `local-state` (optional): private atomic binding and reset intent files. The
  colocated Client is the only binding writer; a dependent Host reads it for
  management and service authorization. No User credentials are shared in files.

User Clients hold their ordinary cloud login plus cached five-minute Host access
tokens. Native Clients persist Host pins and pending claim receipts; they do not
create Host-specific Client keys. Core forwards a User's token and preserves the
original caller. Space Clients have no Host management authority. There are no
Host refresh tokens, enrollment endpoints, offline leases or delegated Core grants.

First claim requires a direct LAN native Client, a fresh Host challenge and a
locally held random exchange secret. Cloud confirmation never substitutes for
that direct exchange. Daily management prefers native LAN and may use the current
Space Core as a proxy. Both paths end at the same Host checks. Runtime mutations
are never automatically replayed after an ambiguous failure.

See `../core-api/contract/host-management-auth.md` for wire endpoints and reset
semantics. Run `cargo test -p mhome-host-auth --features authority`.

The Rust signature API uses `p256` 0.14 types. Persisted PKCS#8 keys, public
coordinates, RFC 7638 thumbprints and 64-byte ES256 signatures remain compatible
with 0.13; no identity rotation is required. The `elliptic-curve` minimum version
also enforces the patched `crypto-bigint` dependency floor for downstream builds.
`tests/crypto_compatibility.rs` uses 0.13 only as a test oracle for old keys and
signatures; it is excluded from normal runtime dependencies.

## Host TLS and native files

`tls` exports a self-signed server certificate from the existing Host P-256 identity.
The Host prepares `identity.json`, `host-key.pem` and `host-cert.pem` under
`<workdir>/security/host/` before starting independently listening services.
Production uses `~/.meow/security/host/`; development and E2E retain their isolated
workdirs. Core/Node business identity keys remain independent of Host management.

`PeerPin::trusted` verifies the authenticated Host public key and TLS handshake
signature. It does not depend on a LAN DNS name, public CA or offline device clock.
Only explicit first setup/public challenge operations use a temporary bootstrap pin.
A signed challenge or authenticated cloud response must establish the key before
sending credentials. Never substitute a new key after a mismatch or fall back to HTTP.
`tls-server` adds the bounded, concurrent Axum TLS listener. Certificates are prepared
on startup; changing identity requires restarting the owning services.

`file-gateway` provides a loopback-only, random capability URL for native WebViews.
It pins the upstream HTTPS key from the authenticated artifact/Storage response,
streams data with Range support, forbids redirects and bounds active transfers.
Each native login holds a `FileOwner`, with child owners for WebView pages.
Retire the login synchronously before publishing its replacement; retire a page
on unload. Retirement cancels that owner and its descendants, including in-flight
transfers. Delayed tasks must retain their captured owner, never clear global state.
`register(&owner, grant)` rejects retired owners. Call `release(&owner, url)`
when a download finishes or its consumer retires; it releases only that owner’s
exact ticket and is idempotent. Keep media tickets for Range rereads until playback
consumers retire, and Storage tickets until session release or expiry.
Storage sessions explicitly grant a same-origin `/storage/v1` prefix; ordinary file
grants allow only GET/HEAD of their exact URL. Cloud file URLs keep normal CA trust.

Run `cargo test -p mhome-host-auth --features file-gateway` for transport tests.


## Managed service control (`local-state`)

`service_control` stores Host-owned per-service intent in private, atomically
replaced JSON. A separate, stable OS lock file coordinates every process:
shared permits cover Space activation, binding writes and restoration; the Host
uses a nonblocking exclusive permit to inspect usage and publish intent.
Only Host initializes records and writes `enabled`. Readers reject missing or
corrupt state. Platform defaults are supplied explicitly on first installation.

Managed launchers supply both `MEOW_HOST_SERVICE_CONTROL_DIR` (an absolute
shared directory) and `MEOW_HOST_SERVICE_ID`. Both absent denotes an explicitly
standalone runtime; a partial context is an error. Embedded Android services use
the Host security root, not their separate business workdirs.

A usage query returns all durable Space bindings, including offline/incomplete
ones, and unfinished local cleanup. Queries and teardown must not acquire the
admission lock. Unknown usage refuses disable. After committing `enabled=false`,
Host releases admission and stops the process under its per-service operation
lock. Stop failure retains intent, allowing retry even if the process is already
gone. Process death releases OS locks without erasing intent. Restart and upgrade
use ordinary stop/start, preserve intent, and never bypass a disabled setting.
