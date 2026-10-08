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
The native login owner clears capabilities and cancels transfers on logout.
Storage sessions explicitly grant a same-origin `/storage/v1` prefix; ordinary file
grants allow only GET/HEAD of their exact URL. Cloud file URLs keep normal CA trust.

Run `cargo test -p mhome-host-auth --features file-gateway` for transport tests.
