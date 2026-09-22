# mhome-host-auth

Shared Rust implementation of the existing `core_api::host::auth` protocol. This
crate has no HTTP client, database, filesystem, login flow, or Space dependency.
It does not add an API endpoint or merge Client identities.

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
