# Pod commissioning v1

A pod is a MeowLink end device that acts as an App client of one user. This
contract is implementation-neutral: ESP-IDF on BLE-capable ESP32 chips is the
first implementation, not a requirement. Commissioners are the native Clients:
desktop/CLI Clientd, Android and iOS. The shared UI and CLI never talk BLE; they
call the `/local/pod/*` targets of their native Client.

Commissioning order is fixed: secure BLE session, Wi-Fi joined, user
authorization, credential issue, delivery, activation. A credential is issued only
after the pod reports a joined network. Any failure after issue revokes it.

## 1. Device identity

- On first boot and after factory reset the pod generates a P-256 identity key.
- `keyId` is `p256:<lowercase hex SHA-256 of the 65-byte uncompressed point>`;
  `fingerprint` is the same hex without prefix. `x`/`y` are unpadded base64url
  32-byte coordinates. This matches the Hub identity format.
- `deviceId` is a stable hardware identifier chosen by the implementation that
  survives factory reset (ESP: lowercase hex MAC without separators).
- `shortId` is the first 4 bytes of the fingerprint, shown as 8 uppercase hex
  characters in UI and advertising. It changes with the identity key.

## 2. BLE advertising

Pods advertise only while unprovisioned, while re-commissioning after logout or
revocation, or inside a re-provision window. A commissioned pod does not advertise.

- Service UUID (128-bit, shared by every MeowLink commissionable device):
  `a17a7ad5-9b2f-4010-9cf0-aa15ac54c40e`.
- Manufacturer data, in the advertisement or scan response:

  | Offset | Size | Value |
  | --- | --- | --- |
  | 0 | 2 | Company ID, little-endian. Development value `0xFFFF` |
  | 2 | 2 | Magic `0x4D 0x57` (`MW`) |
  | 4 | 1 | Advertising format version, `1` |
  | 5 | 1 | Device kind: `1` pod, `2` Host (see `host-provisioning-v1.md`) |
  | 6 | 1 | Flags: bit 0 commissionable, bit 1 Wi-Fi configured |
  | 7 | 4 | `shortId` bytes |

  Receivers ignore records whose company ID, magic or version they do not know.
- Local name: `MeowPod <model>`. `<model>` comes from product configuration and
  is at most 8 ASCII characters. Commissioners display `<name> · <shortId>`.
- Commissioners scan actively and filter on the service UUID.

## 3. Secure session

Transport is ESP protocomm over BLE GATT. Endpoints are GATT characteristics
named by their Characteristic User Description descriptor (`0x2901`). A
commissioner that cannot read descriptors falls back to fixed UUIDs: the service
UUID with bytes 2–3 replaced by `prov-ctrl` `0xFF4F`, `prov-scan` `0xFF50`,
`prov-session` `0xFF51`, `prov-config` `0xFF52`, `proto-ver` `0xFF53`, then the
custom endpoints in table order from `0xFF54` (`pod-info` `0xFF54`,
`pod-credential` `0xFF55`).

- Security: protocomm security 2 (SRP6a 3072-bit with SHA-512, then AES-256-GCM).
  Username `meow`. Password: a 6-digit decimal code.
- Pods with a display generate a uniformly random code, precompute salt and
  verifier at boot, and show the code only while a central is connected. A new
  code is generated after a successful commissioning, after 5 failed handshakes,
  or 10 minutes after it was first shown. Pods without a display are out of scope
  for v1.
- The salt is 16 bytes. `x` hashes the salt as an integer (no leading zero
  bytes) while the proof `M` hashes the salt bytes as sent, so a device generates
  salts whose first byte is non-zero. The commissioner sends `A` as exactly 384
  bytes and picks a new ephemeral when `g^a mod N` would need a leading zero
  byte; devices reject any other length. `B` is sent without leading zeros and
  `M` uses it as sent; `u` pads both to 384 bytes.
- The record nonce is 8 random bytes followed by a 32-bit big-endian counter
  starting at 1, advanced after every encrypt and decrypt on both sides
  (security 2 patch version 1; patch 0 keeps it fixed). Commissioners accept
  patch versions 0 and 1 only. A lost or failed exchange leaves the two counters
  unknown, so the commissioner abandons the secure session after any transport or
  decryption failure. A new `SessionCmd0` restarts the handshake on the same link.
- While a device refuses handshakes after repeated wrong codes it clears the
  commissionable advertising flag and reports the lockout in its `proto-ver`
  app info, as `"lockedForMs":<remaining ms>` (Hosts) or `"locked":true`. A
  handshake while locked is refused the same way as a wrong code (an ATT error
  such as Insufficient Authorization), so after every rejected handshake the
  commissioner reads `proto-ver` again and fails the session with `code_locked`
  when either field is present. A pod session also fails with `code_locked` after
  the fifth rejected code. ESP pods restart with a new code after 5 failed
  handshakes instead of reporting a lockout.
- One BLE connection at a time; a second central receives a disconnect.
- Session deadline: 10 minutes from connect, or 2 minutes waiting for a
  handshake. On expiry the pod discards all unpersisted state.

| Endpoint | Encrypted | Purpose |
| --- | --- | --- |
| `proto-ver` | no | protocomm version JSON with app info `"meow":{"ver":"1","cap":["pod"]}` (label `meow`, capability `pod`) |
| `prov-session` | handshake | security 2 |
| `prov-scan` | yes | standard Wi-Fi scan |
| `prov-config` | yes | standard Wi-Fi set/apply/status |
| `prov-ctrl` | yes | standard reset after a failed join, before a retry |
| `pod-info` | yes | device information |
| `pod-credential` | yes | credential delivery and activation status |

Custom endpoint bodies are UTF-8 JSON, at most 480 bytes in each direction.

`pod-info` request `{}`, response:

```json
{
  "protocol": 1,
  "deviceId": "a0b1c2d3e4f5",
  "model": "S1",
  "platform": "esp32c5",
  "firmwareVersion": "0.3.0",
  "identity": { "keyId": "p256:…", "alg": "ES256", "x": "…", "y": "…", "fingerprint": "…" },
  "wifiConfigured": false,
  "state": "secured"
}
```

`platform` is diagnostic only. `wifiConfigured` is true when the pod kept a
working network across logout; the commissioner then skips the Wi-Fi steps after
confirming connectivity with `prov-config` status. Inside a Wi-Fi change window
the response also carries `"mode":"reprovision"` and `wifiConfigured` is false
(section 4).

`pod-credential` requests:

- `{"op":"deliver","podId","refreshToken","scopeId","cloudApi","cloudWs"}` →
  `{"ok":true,"state":"activating"}`. Accepted only in `wifi_connected` outside a
  Wi-Fi change window; otherwise `{"ok":false,"state":<current pod state>,"reason"}`
  with `reason` `invalid_payload` (a missing or malformed field, including a
  scheme this firmware does not accept) or `wrong_state`. `cloudApi` is
  `https://…/api/v1`, `cloudWs` is a `wss://` URL. Production firmware accepts
  only these schemes; development firmware built with insecure cloud access
  enabled (ESP: `MEOW_POD_ALLOW_INSECURE_CLOUD`, default off) also accepts
  `http://` and `ws://`. Commissioners never check the scheme, but they refuse to
  issue a credential when the cloud host is loopback (`localhost`,
  `127.0.0.0/8`, `::1`), because the pod cannot reach it; the request fails with
  reason `cloud_unreachable_for_device`.
- `{"op":"status"}` → `{"state":<pod state>,"error"?:{"code","message"}}`; `error`
  is present only in `failed`.
- `{"op":"finish"}` → `{"ok":true}` in `commissioned`; the pod stops BLE and
  restarts into normal operation. In any other state it answers
  `{"ok":false,"state":<current pod state>,"reason":"wrong_state"}`. The pod also
  stops BLE 30 seconds after reaching `commissioned` without `finish`. `finish` is
  best-effort: the commissioner records `completed` before sending it.

Activation error codes: `time_sync_failed`, `cloud_unreachable`,
`refresh_rejected`, `auth_rejected`, `storage_failed` (the final commit, or in a
Wi-Fi change window saving the new network, failed).

The pod persists Wi-Fi, credential, endpoints and Space in one commit after a
successful activation. Nothing from an abandoned attempt survives it; an
implementation whose Wi-Fi driver caches the network on its own (ESP-IDF
provisioning does) erases that cache when the attempt ends.

## 4. Pod state machine

```text
unprovisioned ──connect──▶ session_open ──handshake──▶ secured
secured ──prov-config apply──▶ wifi_joining ──▶ wifi_connected | wifi_failed
wifi_failed ──retry──▶ wifi_joining
wifi_connected ──deliver──▶ activating ──time sync, refresh, commit──▶ commissioned | failed
any pre-commit state ──timeout/disconnect──▶ unprovisioned
```

Runtime states after commit:

- `commissioned`: normal operation.
- `reprovision_window`: Wi-Fi replacement only, opened from the pod's own
  settings; the pod restarts and advertises for 10 minutes from boot with a new
  code. The credential is kept and the previous network stays persisted until
  the new one connects. `pod-info` reports `"mode":"reprovision"` and
  `wifiConfigured: false`, and only the Wi-Fi endpoints and `pod-credential`
  `status` and `finish` are accepted (`deliver` answers `wrong_state`). After
  `prov-config` apply the pod joins the new network, saves it, and reports
  `commissioned`, or `failed` with `storage_failed` if saving failed; it then
  restarts on `finish`, 30 seconds later, or at the end of the window. A failed
  join or an expired window restarts with the previous network.
- `needs_recommission`: logout, or refresh rejected because the credential was
  revoked or the user no longer exists. Credential and local JWTs are erased,
  Wi-Fi is kept, advertising resumes with the Wi-Fi-configured flag.
- `factory_resetting`: best-effort self-revoke, erase all persisted data
  including the identity key, reboot to `unprovisioned`.

Time sync uses SNTP; if NTP is unreachable the pod may use the `Date` header of a
Lion HTTPS response. Activation proofs require a synced clock.

## 5. Lion credential API

All bodies are JSON. Errors use the standard Lion envelope
`{"error","message","details"?}`. Credential failures are `UNAUTHORIZED` with one
of these `message` values: `pod_credential_invalid` (unknown pod, wrong or
revoked refresh token, or an owner account that no longer exists),
`pod_proof_invalid`, `pod_proof_replayed`, `pod_activation_expired`. A pod treats
`pod_credential_invalid` and `pod_activation_expired` as revocation. Renaming a
pod the caller does not own, or that does not exist, is `NOT_FOUND` with
`pod_not_found`.

### Issue

`POST /api/v1/pod/credential/issue`, user `Authorization`, `ScopeId` header.

```json
{
  "deviceId": "a0b1c2d3e4f5",
  "model": "S1",
  "platform": "esp32c5",
  "firmwareVersion": "0.3.0",
  "name": "MeowPod S1",
  "identity": { "keyId": "p256:…", "alg": "ES256", "x": "…", "y": "…", "fingerprint": "…" },
  "clientId": "issuing client id"
}
```

`deviceId` matches `[A-Za-z0-9._:-]{1,64}`; `model` and `name` are 1–64
characters, `platform` and `firmwareVersion` at most 64, `clientId` at most 128. Lion requires Space membership for `ScopeId`, validates the identity, and
creates a `pending` credential that must activate before `activateBefore`
(issue time + 10 minutes). Response `{"podId","refreshToken","activateBefore"}`.

### Refresh and activation

`POST /api/v1/pod/token/refresh`, body `{"podId","refreshToken","proof"}`.

`proof` is a compact ES256 JWS signed by the identity key: header `kid=keyId`;
claims `iss` and `sub` = `podId`, a single-valued `aud` = `pod-token-refresh`,
`token_hash` = unpadded base64url SHA-256 of the UTF-8 refresh token (including
`pod-`), `iat`, `exp` with `0 ≤ exp - iat ≤ 120 s`, and a unique `jti` of at most
128 characters. Lion allows 120 s clock skew and remembers each `jti` for
7 minutes (proof lifetime plus twice the skew, plus a minute), longer than any
proof can be accepted; a reused `jti` in that window fails with
`pod_proof_replayed`.

The first successful refresh moves `pending` to `active` and revokes every other
credential with the same `identity.keyId` (whichever user holds it) and the same
user's other credentials with the same `deviceId`. A `pending` credential past
`activateBefore` is rejected and deleted. Revoked credentials are deleted, so a
later refresh fails with `pod_credential_invalid`. Response
`{"accessToken","expiresIn"}`.

### Status

`POST /api/v1/pod/credential/status`, user `Authorization`, body `{"podId"}` →
`{"podId","status","activatedAt"?}` with `status` one of `pending`, `active`,
`absent` (schema `schema/pod-credential-status.v1.schema.json`). `absent`
covers no such credential, revoked, pending past `activateBefore`, and a
credential owned by another user, so the answer never reveals other users'
pods. `activatedAt` (epoch ms) is present when `active`. Commissioners use it to
decide the outcome of an activation they could not observe (section 7).

### Revoke

`POST /api/v1/pod/credential/revoke`, body `{"podId"}` with the owning user's
`Authorization`, or `{"podId","refreshToken","proof"}` where the proof uses
`aud` = `pod-credential-revoke` (pod logout and factory reset). Idempotent:
revoking a pod that does not exist or that the caller does not own is a no-op
that also answers `{"ok":true}`, so the answer never reveals other users' pods.
Response `{"ok":true}`.

### List and rename

`POST /api/v1/pod/list` (user) returns `{"pods":[…]}`, the user's active pods
and unexpired pending pods, each
`{"podId","name","deviceId","model","platform","firmwareVersion","identityFingerprint","status","issuedAt","activatedAt"}`.
`POST /api/v1/pod/rename` `{"podId","name"}` (user) returns the updated pod.

### Tokens

- Access: `pod-` + HS256 JWT. Claims: `iss`, `aud`, `sub` = `podId`, `user_id`,
  `tenant_id`, `device_id`, `token_type` = `pod_access`, `iat`, `exp` (3600 s),
  `jti`. The signing key is derived from the Hub secret with HMAC-SHA256 over the
  label `mhome-pod-access-v1`, so Hub and pod tokens never verify as each other.
- Refresh: `pod-` + 43-character base64url of 32 random bytes. Lion stores only
  its SHA-256. It never rotates; the identity proof binds it to the device.
- Pods send `Authorization: Bearer pod-…` (and `"token":"Bearer pod-…"` on the
  WebSocket). `pod-` is dispatched by prefix before any JWT parsing. Only the cloud WebSocket
  `/auth` and `/api/v1/hub/token/exchange` accept pod access tokens. Every other
  API keeps accepting user tokens only. No target allowlist applies in v1.
- Lion checks an access token against the stored credential on every use, so a
  revoked or superseded credential stops working before its `exp`.
- A pod session acts as its user across all of that user's Spaces. A cloud
  WebSocket session authenticated with a pod access token remembers the `podId`
  and the token `exp`. Lion closes it with a policy-violation close whose reason
  is `POD_TOKEN_EXPIRED` at `exp` and `POD_REVOKED` when the pod is revoked;
  revocations reach sessions on every Lion instance through a shared event
  channel. Either close only ends that connection; whether the pod is revoked
  is decided by its next refresh.
- `/api/v1/hub/token/exchange` with a pod access token returns a Hub JWT with
  `clientKind` = `pod` and `podId` claims and the same lifetime as a Hub JWT
  for a user token.
- Revoking a pod credential, by any path, also revokes the pod on the Hubs of
  its user's local Spaces. Before committing the revocation Lion marks the
  `scope.pods` resource dirty for each such Space; after it commits, Lion
  notifies the Hub through the pull-sync contract. A Hub that is offline pulls
  the marker when its bridge reconnects. `scope.pods` is a snapshot
  `{"podIds":[…]}` of the active pods of the Space's members. The Hub revokes
  the local tokens it issued to any other pod before the pull, disables the
  local auth of that pod's app client once no active token is left, and refuses
  and stops renewing that pod's local connections.

## 6. Pod runtime authentication

- Cloud `/auth`: `{"token":"Bearer pod-…","devToken":null,"source":"pod","deviceId","activeScope"}`,
  then `/focus`. Lion rejects `source=pod` without a pod token, a pod token
  with any other source, and a `deviceId` other than the one the credential was
  issued to.
- The pod refreshes its access token proactively, 2 minutes before `exp`, and
  re-authenticates the cloud session with the new token. A refresh rejected
  with `pod_credential_invalid` or `pod_activation_expired` moves the pod to
  `needs_recommission`.
- Space switching: the pod chooses among the Spaces returned by `/auth` and
  re-sends `/focus`; it keeps one Hub JWT and Hub key per Hub and Space.
- Local Hub: the Hub identity endpoint `POST /api/v1/hub/identity/get`
  (`{"hubId","tenantId","scopeId"}`) is public and needs no token. The pod
  proves the Hub identity, then authenticates to the local Hub the way a Client
  does: the first local `/auth` carries its pod access token with
  `source=cloud` (`clientSource=pod`, `deviceId`, `tenantId`, `scopeId`,
  `hubId`). The Hub exchanges that token with the cloud and returns a
  long-lived local Hub JWT in `jwtToken`, which the pod stores per Hub. Later
  local `/auth` requests use only that JWT with `source=local`. When the stored
  JWT is about to expire or the Hub rejects it, the pod discards it and
  authenticates with `source=cloud` again.

## 7. Native Client targets

All targets are handled by the native Client and never forwarded. Payloads are
domain input JSON. Clients without a usable Bluetooth adapter report
`adapter.state` and reject commissioning with `bluetooth_unavailable`.

A rejected request uses the facade error envelope
`{"error","message","details":{"reason"}}`: `error` is `BAD_REQUEST` for an
unreadable body, `UNSUPPORTED` for an unknown target and `PRECONDITION_FAIL`
otherwise. `PRECONDITION_FAIL` errors carry `details.reason`, one of the stable
values below. User interfaces
map the reason (and the session `error.code` and `detail`) to their own copy and
never show these values or `message` verbatim.

| Reason | Meaning |
| --- | --- |
| `session_active` | another pod or Host session is running |
| `device_gone` | the candidate is no longer nearby |
| `not_commissionable` | the device is not accepting commissioning now (also while it is locked) |
| `wrong_kind` | a Host candidate given to a pod target or the reverse |
| `unknown_session` | no session with this `sessionId` |
| `wrong_state` | the session is not in a state that accepts this request |
| `invalid_code` | the code is not 6 digits |
| `invalid_wifi` | SSID not 1–32 bytes, or password not empty or 8–63 characters (64 hex digits) |
| `bluetooth_unavailable` | no usable Bluetooth adapter |
| `not_signed_in` | no signed-in MeowLink account |
| `scope_not_offered` | the Space is not among those offered, or the account has no Space |
| `cloud_unreachable_for_device` | the cloud address is loopback |
| `busy` | the native Client cannot take the request now; retry |

| Target | Request | Response |
| --- | --- | --- |
| `/local/pod/discovery/start` | `{}` | `{"leaseId","expiresAtMs"}`, 30 s lease |
| `/local/pod/discovery/renew` | `{"leaseId"}` | `{"leaseId","expiresAtMs"}` |
| `/local/pod/discovery/stop` | `{"leaseId"}` | `{}` |
| `/local/pod/discovery/list` | `{}` | discovery snapshot |
| `/local/pod/commission/start` | `{"candidateId"}` | session snapshot |
| `/local/pod/commission/code` | `{"sessionId","code"}` | session snapshot |
| `/local/pod/commission/wifi` | `{"sessionId","ssid","password"?}` | session snapshot |
| `/local/pod/commission/wifi/scan` | `{"sessionId"}` | session snapshot |
| `/local/pod/commission/authorize` | `{"sessionId","scopeId"}` | session snapshot |
| `/local/pod/commission/status` | `{}` | `{"session": snapshot or null}` |
| `/local/pod/bluetooth/settings` | `{}` | `{"opened"}` |

`/local/pod/bluetooth/settings` opens the system settings that fix the
current adapter state (enable Bluetooth or Location, or the app's permission
page) and answers whether it did; a platform without such a page answers
`{"opened":false}`.

Discovery is shared by every MeowLink device kind: each candidate carries
`kind` (`pod` or `host`). `/local/pod/commission/start` accepts only `pod`
candidates; Host candidates go to `/local/host/provision/*`.

Events: `/local/pod/discovery/changed` carries a discovery snapshot, at most
once per second; `/local/pod/commission/changed` carries a session snapshot.
Scanning runs only while at least one lease is live and pauses while a pod or
Host session runs; it resumes after Bluetooth is turned back on. Candidates
unseen for 10 s are removed. One session per native Client. Events are hints:
UIs reconcile with `status` and discovery `list` on start, resume, reconnect and
when they become visible, and a snapshot with a lower `revision` than one
already seen for the same session is stale.

The native core owns the session until success or failure, bounded by
`expiresAtMs`. There is no user cancellation or session renewal. Leaving the
page does not end setup; returning to it reads `status`. Waiting for user input
also counts toward the deadline. Discovery leases still control scanning.

Sign-in and Spaces are checked at `start` (`not_signed_in`,
`scope_not_offered`) and again when the authorization prompt is built.

A pod inside a Wi-Fi change window (`pod-info` `"mode":"reprovision"`) gets a
Wi-Fi-only session. From `reading_info` on, its snapshots carry
`"mode":"reprovision"`; the commissioner always offers Wi-Fi, and after the pod
joins it polls `pod-credential` `status` (still `joining_wifi`) until the pod
reports `commissioned`, then records `completed` and sends `finish`. No
authorization prompt is shown, no credential is issued and `completed` carries
no `podId`; the pod keeps its existing credential and Space. A pod that reports
`failed` ends the session with `wifi_failed` carrying the pod's code (for
example `storage_failed`) in `detail`. UIs show such a session as changing the
pod's Wi-Fi rather than adding a pod.

Session states, in order: `connecting`, `awaiting_code`, `securing`,
`reading_info`, `awaiting_wifi`, `joining_wifi`, `awaiting_authorization`,
`issuing`, `delivering`, `activating`, `completed`; terminal `failed`,
`timed_out`. Wi-Fi states are skipped when the pod reports a
configured network and `prov-config` status reports it connected; while that
status is `connecting` the commissioner keeps polling, and only after a failure
does it reset Wi-Fi (`prov-ctrl`) and offer new credentials. The commissioner
scans for networks once on entering `awaiting_wifi`; a scan request while that
scan runs waits for it. `revision` increases on every observable change.

`expiresAtMs` is 10 minutes after `start` until delivery starts; from
`delivering` on it is `activateBefore` plus 30 seconds.

Only the device's durable `commissioned` response confirms successful setup.
Lost BLE delivery confirmation or activation status fails the attempt, even
when Lion has already activated its credential. Expiry fails or times out the
attempt. The commissioner does not recover interrupted setup through Lion's
credential status. The user resets the device and starts a fresh attempt.

A refused `deliver` fails with `delivery_failed` carrying the pod's `reason`
(`invalid_payload` or `wrong_state`) in `detail`, and revokes. When the pod
itself reports `failed`, the commissioner revokes and fails with
`activation_failed` carrying the pod's code. `activation_failed` without a pod
code carries `not_activated`. The commissioner records `completed` before
sending `finish`. Native Clients keep a non-terminal session running while the
app is in the background (Android foreground service of type
`connectedDevice`, iOS background task).

Session error codes: `bluetooth_unavailable`, `device_busy`, `disconnected`,
`code_rejected` (retry allowed), `code_locked`, `wifi_auth_failed`,
`wifi_not_found`, `wifi_failed`, `issue_failed`, `delivery_failed`,
`activation_failed` (with the pod's activation code, or `not_activated`, in
`detail`), `session_timeout`. `code_locked` is terminal: the device shows a new
code once it accepts handshakes again. `failed` and `timed_out` always carry `error`. A rejected
code returns to `awaiting_code` with `code_rejected`; a failed join returns to
`awaiting_wifi` with the `wifi_*` code. No other state carries `error`. After
issue, every terminal state other than `completed` attempts to revoke the credential; the revoke uses the account that issued the
credential.

The schemas are `schema/pod-discovery.v1.schema.json` and
`schema/pod-commission.v1.schema.json`.
