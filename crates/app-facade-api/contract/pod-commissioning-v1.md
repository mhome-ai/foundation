# Pod commissioning v1

A pod is a MeowLink end device that acts as a User Client or a Space Client. This
contract is implementation-neutral: ESP-IDF on BLE-capable ESP32 chips is the
first implementation, not a requirement. Commissioners are the native Clients:
desktop/CLI Clientd, Android and iOS. The shared UI and CLI never talk BLE; they
call the `/local/pod/*` targets of their native Client.

Network provisioning and activation are separate transactions. BLE provisions
Wi-Fi; Ethernet and already-connected Wi-Fi skip it. Every initialization then
uses a direct native Client ↔ Pod LAN security-2 session. No Core relay is used.
The commissioner selects `clientMode: "user" | "space"` before submitting
initial BLE Wi-Fi credentials or in the LAN session for an already-networked Pod.
User mode uses the personal Pod credential.
Space mode uses a local Core credential signed by Lion; it carries no user
identity and has no cloud access/refresh token. Changing mode requires reset.

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
credential invalidation, or inside a re-provision window. A commissioned pod does not advertise.

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

Wi-Fi provisioning uses ESP protocomm over BLE GATT. Endpoints are GATT characteristics
named by their Characteristic User Description descriptor (`0x2901`). A
commissioner that cannot read descriptors falls back to fixed UUIDs: the service
UUID with bytes 2–3 replaced by `prov-ctrl` `0xFF4F`, `prov-scan` `0xFF50`,
`prov-session` `0xFF51`, `prov-config` `0xFF52`, `proto-ver` `0xFF53`, then the
custom endpoints in table order from `0xFF54` (`pod-info` `0xFF54`,
`pod-credential` `0xFF55`).

- Security: protocomm security 2 (SRP6a 3072-bit with SHA-512, then AES-256-GCM).
  Username `meow`. Password: a 6-digit decimal code.
- Pods with a display generate a uniformly random code, precompute salt and
  verifier at boot, and show it while connected over BLE or available for LAN initialization. A new
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
  (security 2 patch version 1; patch 0 keeps it fixed). Commissioners require
  patch version 1; fixed-nonce patch 0 is rejected. A lost or failed exchange leaves the two counters
  unknown, so the commissioner abandons the secure session after any transport or
  decryption failure. A new `SessionCmd0` restarts the handshake on the same link.
- While a device refuses handshakes after repeated wrong codes it clears the
  commissionable advertising flag and reports the lockout in its `proto-ver`
  app info, as `"lockedForMs":<remaining ms>` (Hosts) or `"locked":true`. A
  handshake while locked is refused the same way as a wrong code (an ATT error
  such as Insufficient Authorization), so after every rejected handshake the
  commissioner reads `proto-ver` again and fails the session with `code_locked`
  when either field is present. A pod session also fails with `code_locked` after
  the fifth rejected code. Pods generate a fresh code after five rejected handshakes. Once network
  configuration is saved, code rotation preserves that network.
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
| `pod-credential` | yes | LAN credential delivery/status/finish; BLE accepts only reprovision status/finish |

Custom endpoint bodies are UTF-8 JSON, at most 480 bytes over BLE. LAN requests
and responses are bounded to 64 KiB including encryption overhead.

`pod-info` request `{}`, response:

```json
{
  "protocol": 1,
  "env": "prod",
  "deviceId": "a0b1c2d3e4f5",
  "model": "S1",
  "platform": "esp32c5",
  "firmwareVersion": "0.3.0",
  "identity": { "keyId": "p256:…", "alg": "ES256", "x": "…", "y": "…", "fingerprint": "…" },
  "wifiConfigured": false,
  "state": "secured"
}
```

`env` must match the native Client runtime (`prod`, `dev`, `e2e`).
`podId` and `clientMode` (`user` or `space`) are included together when activation
is in flight or durably completed. A new session reads both from the secure
device response before resuming; a missing or unknown mode fails the attempt.
This observes completion without issuing another credential. Native completed
enrollment sessions return both fields. HITL passes this receipt to its handler,
which verifies exactly the corresponding personal or Space enrollment source;
it never probes the other source or treats the mode itself as proof of success.
`platform` is diagnostic only. `wifiConfigured` is true when the pod kept a
working network across logout; the commissioner then skips the Wi-Fi steps after
confirming connectivity with `prov-config` status. Inside a Wi-Fi change window
the response also carries `"mode":"reprovision"` and `wifiConfigured` is false
(section 4).

BLE `pod-info` is the network-phase DTO, not the LAN identity/receipt DTO above.
It carries `protocol`, `env`, `deviceId`, `model`, `firmwareVersion`,
`publicKey: [x, y]`, `wifiConfigured`, `state`, `dependentHost`, and optional
`mode`/`error: {code}`. It never includes `deviceBinding`, `podId`, `clientMode`
or a credential. `publicKey` is the same ES256 P-256 key as LAN: derive
`fingerprint = hex(SHA256(0x04 || x || y))` and `keyId = "p256:" + fingerprint`.
Coordinates are unpadded base64url. The complete JSON must fit 480 bytes,
leaving room for the authenticated record in one 512-byte ATT value.
`dependentHost` identifies a combined product eligible for explicit first-Hub
network setup; it is not an ownership grant. After the LAN handoff, the Client
requires the exact same key, device ID, environment and dependent-Host role,
then verifies the complete product binding and Host proof before authorization.
No fallback accepts the former full LAN DTO on BLE.

`pod-credential` requests:

- `{"op":"deliver","clientMode":"user","podId","loginGrant","spaceId","cloudApi","cloudWs"}` →
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
`local_hub_unavailable`, `refresh_rejected`, `auth_rejected`, `storage_failed` (the final commit, or in a
Wi-Fi change window saving the new network, failed).

The Pod joins using temporary Wi-Fi settings. For a Space with an existing Hub,
it locates and proves the selected Hub's identity through the new Wi-Fi interface
within 30 seconds of receiving an IP address, then commits the network. Personal
Pods commit on IP readiness. A failed join or proof clears the trial and keeps
any previously stored network. Activation happens afterward and commits only
identity and binding data. Activation failure, disconnection, code expiry and
reboot preserve the committed network.

### LAN discovery and channel

- Service: `_meow-pod-init._tcp.local.` while network-ready and uninitialized.
  TXT: full `deviceId`, `v=1`, `env`, optional `model`; SRV/A/AAAA locate the
  endpoint. A failed or expired pairing attempt does not remove this service.
- The same service works on Ethernet and Wi-Fi. BLE supplies the full device ID
  and identity over its secure session; the short display ID is never the match
  key. Discovery data is untrusted and is checked against encrypted `pod-info`.
- HTTP POST `/{endpoint}` with `Content-Type: application/octet-stream` and a
  fresh UUID `X-Meow-Session`. Allowed endpoints: `proto-ver`, `prov-session`,
  `pod-info`, `pod-credential`. Wi-Fi endpoints are unavailable over LAN.
  Clients connect to the resolved socket directly, with proxies/redirects off.
- Run a fresh Security 2 patch-1 handshake on LAN. After BLE, reuse only the
  temporarily held code, never the BLE session key or nonce. Ethernet setup
  requests the code in the App UI. Business messages are AES-GCM protected.
- Each IP session has separate crypto state. Devices cap simultaneous sessions
  at four; pending handshakes expire after two minutes, sessions after ten.
  An encrypted `pod-credential {"op":"close"}` releases that temporary session.
- Stop init advertisement on durable completion. The existing session has a
  short finish window for confirmation; normal business disconnection never
  returns a completed device to initialization.

## 4. Pod state machine

```text
no network -> BLE secure -> join -> verify selected Hub if required -> save Wi-Fi
Ethernet / saved Wi-Fi ------------------------------------------> network ready
network ready -> advertise init -> LAN secure -> authorize -> activate -> save identity
join failure -> end attempt; start Wi-Fi setup again
activation failure -> end attempt; keep network and init advertisement
pairing expiry -> discard temporary sessions/code; keep network; accept a new code
```

The App reports `discovering_lan` and `connecting_lan` during BLE handoff.
Discovery snapshots distinguish `scanning` (BLE), `lanBrowsing`, optional
`lanError`, and candidate `transport` (`ble` or `lan`). LAN candidates carry the
full `deviceId`; Bluetooth denial never blocks them. The App retries initialization
by selecting the LAN candidate, possibly re-entering the current code. It first
reads actual device state: `commissioned` completes immediately, `activating`
waits for the existing attempt, and a failed attempt may issue afresh.

Runtime states after commit:

- `commissioned`: normal operation.
- `reprovision_window`: Wi-Fi replacement only, opened from the pod's own
  settings; the pod restarts and advertises for 10 minutes from boot with a new
  code. The credential is kept and the previous network stays persisted until
  the new one connects. `pod-info` reports `"mode":"reprovision"` and
  `wifiConfigured: false`, and only the Wi-Fi endpoints and `pod-credential`
  `status` and `finish` are accepted (`deliver` answers `wrong_state`). After
  `prov-config` apply the Pod joins the new network, validates its stored Hub for
  Space mode, saves the network and reports `commissioned`. It reports `failed`
  with `wifi_hub_unverified` when the Hub proof fails or `storage_failed` if saving fails. It then
  restarts on `finish`, 30 seconds later, or at the end of the window. A failed
  join or an expired window restarts with the previous network.
- `needs_recommission`: logout, or refresh rejected because the credential was
  revoked or the user no longer exists. Credential and local JWTs are erased,
  Wi-Fi is kept, advertising resumes with the Wi-Fi-configured flag.
- `factory_resetting`: purely offline, erase all persisted data including the
  identity key and cached Hub tokens, reboot to `unprovisioned`. No cloud
  request is needed. Logout also clears local credentials without cloud revocation.

Linux uses its OS time service and ESP uses background SNTP. User login proofs
require usable UTC and cloud TLS retains normal CA and date validation. Space
Client local authentication pins the enrolled Core identity and signs a fresh
nonce; Core checks credential expiry with its own clock. HTTP `Date` is diagnostic
and does not override the system clock or relax TLS validation.

## 5. User Pod login approval

`POST /api/v1/pod/login/approve` uses the assisting App's ordinary User
Authorization. Its body contains `deviceId`, `model`, `platform`,
`firmwareVersion`, `name`, `identity` (P-256 keyId, alg, x, y, fingerprint), and
optional `clientId`. It has no Space authorization header or issued-Space scope.
Lion returns `{podId, loginGrant, activateBefore}`. The random approval expires
in ten minutes and is bound to this User and the Pod identity key. It is not a
login refresh token. The helper must never deliver its own login credentials.

The App delivers the approval through the paired LAN channel. `spaceId` in the
device delivery selects the initial UI Space; it does not constrain the User
identity or grant Space authority. The device independently redeems the approval
at `POST /api/v1/pod/login/redeem` with `{podId, loginGrant, proof}`. The proof is
signed by the paired device's key with audience `user-device-login` and binds the
approval. Lion coordinates Cognito CUSTOM_AUTH to create the device's own
ordinary Cognito session. Approval redemption is single-use. Only the device
stores the resulting refresh token, and refresh uses the standard User login
endpoint. A failed attempt is retried with a new approval; no old protocol is
accepted.

`podId` identifies a single approval attempt, not the durable device or authenticated
User. The durable `registrationId` derives from the business User and the validated
persistent device-key fingerprint. `/app/pod/list` returns that ID;
`/app/pod/rename` accepts `{registrationId, name}` with authenticated User authority.
Successful authentication atomically creates the registration if absent and records
completion on the consumed approval. Retries preserve the initial registration and
current display name. Unauthenticated attempts create no device-list entry.
The attempt receipt remains readable for one day after the approval's expiry;
`expiresAt` still controls its ten-minute, single-use authentication window.
There is no cloud credential-status polling or server-side Pod revocation flow.
Factory reset clears device credentials and rotates the pairing identity.

## 6. Pod runtime authentication

User Pods use ordinary Cognito User authentication for cloud business traffic.
Local Core connections use the same Lion-issued local credentials and Core
verification as other User Clients. Each credential retains the actual User
principal; initial Space selection is not embedded in a special Pod login token.
Space Pods use only their enrolled local Core and their Space credential; they
receive no User login grant or Cognito session. See section 8 for Space enrollment.

Setup succeeds only after the paired device reports `commissioned`, meaning its
business connection has authenticated. Cloud approval issuance alone is not
activation. Reset clears the local credentials; an unreachable device is not
considered reset.

## 7. Native Client targets

All targets are handled by the native Client and never forwarded. Payloads are
domain input JSON. Clients without a usable Bluetooth adapter report
`adapter.state` and reject BLE commissioning with `bluetooth_unavailable`; LAN candidates remain usable.

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
| `space_not_offered` | the Space is not among those offered, or the account has no Space |
| `cloud_unreachable_for_device` | the cloud address is loopback |
| `unknown_lease` | the discovery lease expired or was stopped |
| `busy` | the native Client cannot take the request now; retry |

| Target | Request | Response |
| --- | --- | --- |
| `/local/pod/discovery/start` | `{"transport":"lan"|"ble"}` | `{"leaseId","expiresAtMs"}`, 30 s lease |
| `/local/pod/discovery/renew` | `{"leaseId"}` | `{"leaseId","expiresAtMs"}` |
| `/local/pod/discovery/stop` | `{"leaseId"}` | `{}` |
| `/local/pod/discovery/list` | `{}` | discovery snapshot |
| `/local/pod/commission/start` | `{"candidateId"}` | session snapshot |
| `/local/pod/commission/code` | `{"sessionId","code"}` | session snapshot |
| `/local/pod/commission/wifi` | `{"sessionId","ssid","password"?,"enrollment"?}` | session snapshot |
| `/local/pod/commission/wifi/scan` | `{"sessionId"}` | session snapshot |
| `/local/pod/commission/authorize` | `{"sessionId","spaceId","clientMode","firstHub"?}` | session snapshot |
| `/local/pod/commission/status` | `{}` | `{"session": snapshot or null}` |
| `/local/pod/bluetooth/settings` | `{}` | `{"opened"}` |

`/local/pod/bluetooth/settings` opens the system settings that fix the
current adapter state (enable Bluetooth or Location, or the app's permission
page) and answers whether it did; a platform without such a page answers
`{"opened":false}`.

The lease activates only its selected transport. Renew keeps that transport and
returns `unknown_lease` if the lease was stopped or expired. A LAN lease does
not request Bluetooth permission. BLE discovery uses the current Client's radio.
Snapshots may contain both transports when several callers hold leases. A running
Pod setup may continue LAN browsing to complete a handoff.

Initial BLE Wi-Fi setup requires `enrollment: {spaceId, clientMode, firstHub?}`. An
already-bound Pod changing Wi-Fi uses the stored authorization and omits it.
An already-networked Pod selects its authorization in the LAN session.

Before submitting the Wi-Fi settings, the Client sends `pod-info` the secured
`{"op":"prepareWifi","mode":"user"|"space"|"bootstrap"}` record. A `space`
record also carries `hub`, an ordered array of seven strings: Hub ID, tenant ID,
Space ID, Hub P-256 X/Y and Host TLS P-256 X/Y. The Client obtains this identity
from the published Space Hub and the Hub identity service. No application
credential is sent in the BLE network phase. The device accepts Wi-Fi apply
only after receiving a setup target or when in its authorized reprovision
window. It verifies a selected Hub from the new Wi-Fi before committing the
network. The Client later authorizes the exact enrollment selected before the
trial and rejects a credential from a different Hub.
`bootstrap` requires explicit `firstHub: true` and a verified product with an
embedded dependent Host. The selected Space must have no existing Hub or
pending Hub transition. For BLE, include the flag in `enrollment` before sending
Wi-Fi and repeat the same intent during authorization. An already-networked
product supplies its intent during authorization. A retry may resume an existing
setup or published Hub only when its Host ID matches the securely paired
product. Removal, revocation, or another Host's operation blocks that retry.
The existing Hub setup engine owns continuation; the Client adopts the newly
published Hub only after checking its Host key, account, explicit Space and
unchanged local network. UI focus changes do not change this destination.
A standalone Pod rejects first Hub setup.

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
`space_not_offered`) and again when the authorization prompt is built.

A pod inside a Wi-Fi change window (`pod-info` `"mode":"reprovision"`) gets a
Wi-Fi-only session. From `reading_info` on, its snapshots carry
`"mode":"reprovision"`; the commissioner always offers Wi-Fi, and after the pod
joins it polls `pod-credential` `status` (still `joining_wifi`) until the pod
reports `commissioned`, then records `completed` and sends `finish`. No
authorization prompt is shown, no credential is issued and `completed` carries
no `podId`; the pod keeps its existing credential and Space. After obtaining an
IP, a Space Pod has 30 seconds to verify its pinned Hub through the trial Wi-Fi
interface. On failure it discards the attempted network. A pod that reports
`failed` ends the session with `wifi_failed` carrying the pod's code (for
example `wifi_hub_unverified` or `storage_failed`) in `detail`. UIs show such a session as changing the
pod's Wi-Fi rather than adding a pod.

For a combined Linux product (`dependentHost: true`), the Native Client is the
single Security 2 pairing authority on both BLE and LAN. The Host peripheral
forwards encrypted records over the OS-private Pod socket; the Host owns the
Wi-Fi trial, verification and rollback. This is not a BLE discovery relay.
A secured `pod-info` `{"op":"networkTarget"}` read before a Wi-Fi trial returns
`{"mode":"space","spaceId":"...","tenantId":"..."}` for an ordinary Space
product, `{"mode":"unclaimed"}` before Host binding, or `{"mode":"ownHub"}` when its installed Core durable identity matches its
local Host claim. The ordinary product requires the signed-in owner to obtain the
current Hub identity online and send the `space` preparation record above.
An own-Hub product accepts `{"op":"prepareWifi","mode":"ownHub"}` only after
checking the installed Core identity against its local Host claim. It
changes network without trying to prove LAN membership against itself. Both
paths retain authorization and issue no new Pod credential.
The `commissioned` response in this Wi-Fi-only mode confirms the saved network;
it does not restore a revoked credential. A Linux product that already needs
reauthorization keeps that state and its existing binding after a network change.

The Linux trial binds discovery and the pinned connection to the new Wi-Fi
interface. It saves the NetworkManager profile only after Hub proof succeeds;
a failure or cancellation discards the trial and restores the previous profile.
Its total session polling budget also covers bounded network activation and
rollback; the Hub proof budget remains 30 seconds after obtaining an IP.

Session states, in order: `connecting`, `awaiting_code`, `securing`,
`reading_info`, `awaiting_wifi`, `joining_wifi`, `discovering_lan`,
`connecting_lan`, `awaiting_authorization`,
`issuing`, `delivering`, `activating`, `completed`; terminal `failed`,
`timed_out`. LAN candidates skip all Wi-Fi states. Wi-Fi states are also skipped when the pod reports a
configured network and `prov-config` status reports it connected; while that
status is `connecting` the commissioner keeps polling, and only after a failure
does it reset Wi-Fi (`prov-ctrl`) and offer new credentials. The commissioner
scans for networks once on entering `awaiting_wifi`; a scan request while that
scan runs waits for it. `revision` increases on every observable change.

`expiresAtMs` is 10 minutes after `start` until delivery starts; from
`delivering` on, User approval uses `activateBefore` plus 30 seconds; Space delivery
gets a separate two-minute confirmation deadline. An active Space credential has no
first-use deadline and never causes immediate completion or an expired attempt.

Only the device's durable `commissioned` response confirms successful setup.
Lost LAN confirmation triggers at most three reconnections to this attempt's
endpoint. Each connection reestablishes channel security and checks device,
environment and enrollment identity before reading status. Never issue or
deliver a second credential during recovery. Failure to confirm, or expiry,
ends the attempt; the user may then select the network-ready device again.
Cloud state alone never confirms successful device setup.

A refused `deliver` fails with `delivery_failed` carrying the pod's `reason`
(`invalid_payload` or `wrong_state`) in `detail`. When the pod
itself reports `failed`, the commissioner fails with
`activation_failed` carrying the pod's code. `activation_failed` without a pod
code carries `not_activated`. The commissioner records `completed` before
sending `finish`. Native Clients keep a non-terminal session running while the
app is in the background (Android foreground service of type
`connectedDevice`, iOS background task).

Session error codes: `bluetooth_unavailable`, `device_busy`, `disconnected`,
`code_rejected` (retry allowed), `code_locked`, `wifi_auth_failed`,
`wifi_not_found`, `wifi_failed`, `lan_unavailable`, `device_mismatch`, `issue_failed`, `delivery_failed`,
`activation_failed` (with the pod's activation code, or `not_activated`, in
`detail`), `session_timeout`. `code_locked` is terminal: the device shows a new
code once it accepts handshakes again. `failed` and `timed_out` always carry `error`. A rejected
code returns to `awaiting_code` with `code_rejected`; a failed Pod join ends in
`failed` with the `wifi_*` code. No other state carries `error`. Terminal states release the link and radio
without revoking credentials. Retry over LAN after an uncertain activation outcome; unused pending cloud credentials expire automatically.

The schemas are `schema/pod-discovery.v1.schema.json` and
`schema/pod-commission.v1.schema.json`.


## 8. Space Client local credential

`POST /api/v1/pod/space/credential/issue` uses the assisting App's user
`Authorization` and `SpaceId`, with the same device identity request as personal
issuance. Only the current Space owner may assist. The Space must be local and have one
published Hub with business and Host TLS identities. Cloud Spaces must complete
Hub setup first. The issuer derives `appClientId` from tenant, Space and the validated
persistent device-key fingerprint. The same device in the same Space retains this
ID across retries; a factory reset rotates its key. Candidate local JWTs use the
existing Space signing key and local-client audience.

Claims are `token_type: local_space_client_access`, `sub: <spaceId>`,
`tenant_id`, `space_id`, `hub_id`, `app_client_id`, `jti`, `iat`, `exp`, `is_test`.
The expiry follows existing long-lived User local credentials. No `userId`,
initializer identity, `podId` or `issuedSpaceId` grants authority. User local
credentials retain `local_client_access` and must not contain `app_client_id`.
Personal Pod issuance no longer stores `issuedSpaceId`.

Lion sends `{jwtToken, clientSource:"pod"}` to the authenticated Hub Bridge
`/hub/client/space/register`. Core verifies signature, audience, issuer, current
Hub, environment, Space and canonical installation ID. It reuses a valid current
pending or active credential, or atomically selects a replacement after fresh owner
authorization. Concurrent issuers receive the selected credential; a stale replacement
cannot overwrite activation or revocation. Pending reuse retains its original
first-authentication deadline, ten minutes after JWT issuance. Invalid, revoked,
expired or differently bound credentials are not reused. Signature/key infrastructure
failures propagate instead of being interpreted as absence.
The Bridge returns `{credentialId, appClientId, jwtToken, status, activateBefore}`.
Lion returns `{credentialId, appClientId, jwtToken, credentialState, activateBefore,
hubIdentity}`. State is `pending` with its epoch-millisecond first-use deadline, or
`active` with `activateBefore:null`. Issuance alone does not publish a device-list entry. The Hub identity includes its business P-256 key and
Host TLS public key, bound to `tenantId`, `spaceId`, `hubId` and `hostId`.

The secured LAN `pod-credential` delivery is:

```json
{"op":"deliver","clientMode":"space","credentialId":"…","appClientId":"a:…","jwtToken":"…","tenantId":"…","spaceId":"…","hubIdentity":{}}
```

The actual `hubIdentity` is the complete issuer response. User credential fields
(`refreshToken`, cloud endpoints, `podId`) are rejected in this variant. Space
credential fields are rejected in the User variant. LAN credential plaintext is
bounded to 8192 bytes; BLE credential delivery remains disabled.

The Pod discovers the matching runtime environment and Space's Hub, pins TLS,
verifies the signed Hub proof, and authenticates with the local JWT and signed
`appClientId`. After authentication checks succeed, Core atomically activates the
credential and publishes the durable AppClient registration. Repeated authentication
preserves the first activation; later offline or revoked credentials do not erase the
registration. The Pod requires a Space principal for the same Space, installation and
credential in the authenticated response, then durably saves the profile. Only
then does LAN setup report completion. The LAN status field named `podId` is an
enrollment correlation ID: for Space mode it carries `credentialId`, never the
installation ID or acting principal.

After activation, reconnects do not require Lion or renew a cloud session. The
same Core verifies the JWT and local registration offline, with its cached Scope
verification key. Failed activation preserves working Wi-Fi and leaves LAN
commissioning available. Retry may require the pairing code again. Unused pending
credentials cannot activate after the deadline. Factory reset deletes the local
credential and identity key; no server-side revoke workflow is introduced.

Core carries `AuthenticatedClient` through WS, dispatch, plugin admission,
prepared actions, dialogs and presence. A Space Client can READ/WRITE its Space;
MANAGE, personal resources and Host user administration are rejected. It joins
the canonical Space-client conversation group, never an initializer's threads.
The billing owner fallback is not an acting identity. Host management tokens are
independent and are never issued by this flow.

## Registered Pod list

The public `/app/pod/list` Facade combines all current-User personal registrations
and current-Space Space-owned Pod registrations. UI and tools share this result.
`registrationId` identifies the durable registration; it is not the enrollment
`podId` used by native commissioning. Connection observations are separately
scoped and may be unknown. See [Pod directory](pod-directory-v1.md) for fields,
aggregation, failure handling and personal display-name updates.

`/app/pod/enrollment/get` takes `{podId}` and returns `{podId, clientMode: "user",
registered}` from the authenticated user's successful receipt for that exact attempt.
An older registration for the same device cannot complete a different attempt.
`/app/pod/space/enrollment/get` takes the same input and checks the current Space's
active local credential, returning `clientMode: "space"`; this read requires Space
MANAGE. A cloud Space has no such local enrollment and returns `registered:false`.
Both checks are read-only completion hooks. They do not issue or renew credentials.

### Interrupted combined first Hub

A combined product uses the same durable first-Hub network intent and cloud
recovery permit as [Host provisioning](host-provisioning-v1.md#interrupted-first-hub-installation).
The secured `pod-info {op:"hostIdentity"}` returns the local Host's
`{hostId,publicKey:[x,y]}` through OS-private IPC while a network window is open.
The commissioner retains this identity across BLE-to-LAN handoff and checks it
against the product's signed binding before any claim or credential delivery.
The BLE record contains only the public identity and compact intent/permit;
full Host claim messages and Pod credentials remain on the LAN Security 2 channel.

If the Host is already bound and its first-Hub setup is pending, `networkTarget`
returns `firstHub` with the original intent and Space. Native obtains an online
Owner permit and submits `bootstrapRecovery`. If no Pod profile has been saved,
Wi-Fi success continues the defined LAN binding/Hub setup/Pod credential protocol.
An existing Pod profile keeps the Wi-Fi-only reprovision flow. Neither case
changes Space or clears the current Host authorization to bypass a failed check.
