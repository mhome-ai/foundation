# mhome-host-auth

Shared Rust implementation of the existing `core_api::host::auth` protocol. This
crate keeps login, authorization persistence and Space policy in its adapters.
Optional TLS/file features provide shared transport; Client identities remain separate.

- `signatures`: P-256 keys, compact bootstrap JWS, request/response HTTP message
  signatures, and request-bound response verification.
- `session`: verified Host contexts and enrollment challenges, context caching,
  identity generation fencing, and a single recovery decision after an
  authenticated pre-dispatch rejection. A context refresh cannot satisfy a
  waiter requiring re-enrollment.

Native Clients keep their own signing keys and user-login cloud adapter. A Hub
keeps separate keys per Space member and its Hub-credential cloud adapter. Each
identity owns a separate `Sessions`; refresh coordination and persistent pins
belong to the adapter. The same `hostId` must not join caches belonging to two
identities. Native logout and Hub member removal retain their existing owners.

Adapters pass raw response bytes and headers to `Exchange::verify` before
interpreting success or error. Only `Outcome::Refresh` permits a bounded retry.
Timeouts, disconnects, unverified responses and ambiguous mutation results are
never automatically replayed. Disable redirects/decompression for signed LAN
responses and cap response bodies with `MAX_RESPONSE_BYTES`.

`cloud_confirmed_enrollment` accepts only the direct response from an
application-configured authenticated HTTPS cloud (or a loopback development
server). It is not a JWT verifier and must never accept LAN-supplied grants as
trust. It cross-checks the cloud-confirmed key/user/Host/Client against the
signed challenge. A replacement pin requires an explicit cloud-confirmed
recovery path, not a failed mutation's retry.

A warm management request uses its cached context and needs one LAN round trip.
A restarted Client fetches and verifies context using the persisted Host pin;
a restarted Host requests a context refresh. An expired registration requires
a cloud grant through the identity owner's adapter. Cloud token expiry alone
must not retire an already-established offline identity.

Run `cargo test -p mhome-host-auth`. Release with `mhome-host-auth-v<version>`
after publishing its Foundation dependencies. JavaScript/Android/iOS wire
contracts remain in Core API; mobile native transports do not automatically
link this Rust library.

## Host TLS and native files

`tls` exports a self-signed server certificate from the existing Host P-256 identity.
The Host prepares `identity.json`, `host-key.pem` and `host-cert.pem` under
`<workdir>/security/host/` before starting independently listening services.
Production uses `~/.meow/security/host/`; development and E2E retain their isolated
workdirs. Hub business signing keys and Client signing keys remain independent.

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

## D2 bearer-session lifecycle foundation

`token_session` provides lifecycle rules and renewal coordination for the planned
Host bearer protocol. It is separate from the currently deployed signed-request
`session` adapter. Adding this library does **not** change a running Host, expose
new HTTP endpoints, delete Client keys, migrate credentials, or enable bearer
authentication. Native/Core/cloud integration and the protocol cutover are later
work. No legacy fallback is specified for that cutover.

The shared policy is:

| Deadline | Effect | Recovery |
| --- | --- | --- |
| Access: 60 minutes | That access token expires | Refresh locally using the fixed Host refresh token |
| Online review: 24 hours | Schedule an independent background check | Keep valid LAN access during transient cloud failure |
| Offline authority: 7 days from a verified check | Pause new Host management requests | New verified authority can renew a non-revoked, non-idle-expired session |
| Refresh idle: 30 days since a successful refresh | The old session cannot refresh or renew | Establish a new session using an independently valid upstream identity |

Thirty days does not mandate manual login or rebinding. A still-valid user cloud
login, Hub identity or provisioned Space-device authority can authorize a new
session automatically. An expired Host refresh token alone cannot. Revocation
must not be treated as expiry and must not trigger automatic re-enrollment.
These deadlines do not stop running services or reset the Host's binding.

`Session` contains durable metadata only. Its `Binding` fixes the Host, environment,
binding generation, controller and typed User/Space principal. Adapters verify
permissions and authority; an untrusted request cannot choose these fields.
Each controller/Host pair has its own credentials. Refresh never changes its
principal or permissions, and never rotates the refresh token. Renewing online
authority does not touch the idle timer or extend existing access-token expiries.
Every access token has its own `AccessWindow`, capped by the current lease.

### Authority and persistence contract

- `AuthorizationLease::from_verified_authority` consumes **already verified**
  authority, not arbitrary JSON or a parsed-but-unverified JWT. Check the trusted
  issuer, Host/TLS identity, audience, environment, subject, controller, session,
  binding generation, purpose, redemption expiry and one-time use in the adapter.
- The short exchange proof expiry and the seven-day authorization deadline are
  different fields. Never turn a five-minute redemption expiry into the offline
  lease, or use receipt time to invent a new seven days.
- A delegated Hub preserves the upstream online confirmation time and bounds its
  child lease by its own deadline. An offline Hub cannot renew itself or its
  Clients forever. Permissions must likewise never exceed the parent authority.
- Persist `Session` beside token hashes in one transaction; create cryptographically
  random token material in the adapter. Read its state back without resetting
  timestamps. Failed refreshes must leave state unchanged. Host-side token lookup
  must resolve the correct session and check revocation on every protected request.
- Use trusted OS/Host UTC for lease checks. This module does not synchronize time
  or establish that a cold boot's UTC is reliable. Reject authentication when no
  credible time is available; retain provisioning and existing service operation.
- A failed or ambiguous management mutation is never automatically replayed by
  renewal. Authentication recovery is separate from business-operation retry.

### Driving background renewal

Use one `renewal::Coordinator` per owner and Host, guarded by the same owner lock
as logout. Call `Session::maintenance` at startup, foreground, connectivity
restoration, before a management request, and from the owning runtime's timer.
For example, a one-minute timer can check the persisted 24-hour deadline; it does
not perform an online check every minute. Skip timer-driven LAN access refresh
when the owner has no work: background maintenance must not artificially keep an
unused refresh session alive.

`Maintenance::access` distinguishes usable access, LAN refresh, required online
renewal and new session establishment. Missing sessions start with establishment.
`review_in_background` is independent: a failed daily online check must not force
a usable LAN request to wait for the cloud. If the offline lease has expired,
management waits for successful online renewal. If idle expiry has also elapsed,
establish a fresh session through upstream authentication instead.

Use `Coordinator::begin` to coalesce each maintenance operation. LAN refresh and
online review have independent lanes. Transient failures retry after five seconds
with exponential backoff and jitter, capped at five minutes. An actual connection
restoration can retry immediately. Thirty-second attempt deadlines retire stalled
tasks; late responses cannot publish. A trusted login-required response suspends
upstream retries until the same owner's login is restored. Trusted revocation
blocks every lane and must also be persisted in the session store.

Apply results through `Coordinator::succeed` while holding the owner lock. Its
publication closure must atomically compare the persisted owner generation and
session binding, merge into the latest stored state, and commit credentials and
metadata. Never write a captured pre-request snapshot over a newer lease. Retire
the coordinator synchronously on logout/account replacement before clearing
credentials; in-flight old-owner responses cannot publish. Use a fresh owner
generation for the replacement. The coordinator itself is ephemeral; restart
does not reconstruct expired session deadlines from `now`.

Run `cargo test -p mhome-host-auth --test token_lifecycle` for seven/30-day recovery,
restart persistence, delegated limits, revocation, account fences and retry tests.
