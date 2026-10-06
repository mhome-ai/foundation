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
  | 5 | 1 | Device kind: `1` pod, `2` Host (reserved) |
  | 6 | 1 | Flags: bit 0 commissionable, bit 1 Wi-Fi configured |
  | 7 | 4 | `shortId` bytes |

  Receivers ignore records whose company ID, magic or version they do not know.
- Local name: `MeowPod <model>`. `<model>` comes from product configuration and
  is at most 8 ASCII characters. Commissioners display `<name> · <shortId>`.
- Commissioners scan actively and filter on the service UUID.

## 3. Secure session

Transport is ESP protocomm over BLE GATT. Endpoints are GATT characteristics
named by their Characteristic User Description descriptor (`0x2901`).

- Security: protocomm security 2 (SRP6a 3072-bit with SHA-512, then AES-256-GCM).
  Username `meow`. Password: a 6-digit decimal code.
- Pods with a display generate a uniformly random code, precompute salt and
  verifier at boot, and show the code only while a central is connected. A new
  code is generated after a successful commissioning, after 5 failed handshakes,
  or 10 minutes after it was first shown. Pods without a display are out of scope
  for v1.
- One BLE connection at a time; a second central receives a disconnect.
- Session deadline: 10 minutes from connect, or 2 minutes waiting for a
  handshake. On expiry the pod discards all unpersisted state.

| Endpoint | Encrypted | Purpose |
| --- | --- | --- |
| `proto-ver` | no | protocomm version JSON; `meow` app info `{"ver":1,"kind":"pod"}` |
| `prov-session` | handshake | security 2 |
| `prov-scan` | yes | standard Wi-Fi scan |
| `prov-config` | yes | standard Wi-Fi set/apply/status |
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
confirming connectivity with `prov-config` status.

`pod-credential` requests:

- `{"op":"deliver","podId","refreshToken","scopeId","cloudApi","cloudWs"}` →
  `{"ok":true,"state":"activating"}`. Accepted only in `wifi_connected`.
- `{"op":"status"}` → `{"state":"activating"|"commissioned"|"failed","error"?:{"code","message"}}`.
- `{"op":"finish"}` → `{"ok":true}`; the pod stops BLE. It also stops BLE
  30 seconds after reaching `commissioned` without `finish`.

Activation error codes: `time_sync_failed`, `cloud_unreachable`,
`refresh_rejected`, `auth_rejected`.

Wi-Fi is applied to RAM only during the session. The pod persists Wi-Fi,
credential, endpoints and Space in one commit after a successful activation.

## 4. Pod state machine

```text
unprovisioned ──connect──▶ session_open ──handshake──▶ secured
secured ──prov-config apply──▶ wifi_joining ──▶ wifi_connected | wifi_failed
wifi_failed ──retry──▶ wifi_joining
wifi_connected ──time sync──▶ (still wifi_connected; deliver refused until synced)
wifi_connected ──deliver──▶ activating ──▶ commissioned | failed
any pre-commit state ──timeout/disconnect/cancel──▶ unprovisioned
```

Runtime states after commit:

- `commissioned`: normal operation.
- `reprovision_window`: Wi-Fi replacement only; the credential is kept and the
  previous network stays persisted until the new one connects.
- `needs_recommission`: logout, or refresh rejected because the credential was
  revoked or the user no longer exists. Credential and local JWTs are erased,
  Wi-Fi is kept, advertising resumes with the Wi-Fi-configured flag.
- `factory_resetting`: best-effort self-revoke, erase all persisted data
  including the identity key, reboot to `unprovisioned`.

Time sync uses SNTP; if NTP is unreachable the pod may use the `Date` header of a
Lion HTTPS response. Activation proofs require a synced clock.

## 5. Lion credential API

All bodies are JSON. Errors use the standard Lion envelope; codes below are the
`message` discriminators.

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

Lion requires Space membership for `ScopeId`, validates the identity, and
creates a `pending` credential that must activate before `activateBefore`
(issue time + 10 minutes). Response `{"podId","refreshToken","activateBefore"}`.

### Refresh and activation

`POST /api/v1/pod/token/refresh`, body `{"podId","refreshToken","proof"}`.

`proof` is a compact ES256 JWS signed by the identity key: header `kid=keyId`;
claims `iss` and `sub` = `podId`, `aud` = `pod-token-refresh`, `token_hash` =
standard base64 SHA-256 of the refresh token, `iat`, `exp` with
`exp - iat ≤ 120 s`, and a unique `jti`. Lion allows 120 s clock skew and rejects
reused `jti` values for 5 minutes.

The first successful refresh moves `pending` to `active` and revokes any other
active credential with the same `deviceId`. A `pending` credential past
`activateBefore` is rejected and deleted. Every refresh checks that the
credential is active and its user still exists. Response
`{"accessToken","expiresIn"}`.

### Revoke

`POST /api/v1/pod/credential/revoke`, body `{"podId"}` with the owning user's
`Authorization`, or `{"podId","refreshToken","proof"}` where the proof uses
`aud` = `pod-credential-revoke` (pod logout and factory reset). Idempotent.

### List and rename

`POST /api/v1/pod/list` (user) returns the user's active and pending pods
without secrets. `POST /api/v1/pod/rename` `{"podId","name"}` (user).

### Tokens

- Access: `pod-` + HS256 JWT. Claims: `iss`, `aud`, `sub` = `podId`, `user_id`,
  `tenant_id`, `device_id`, `token_type` = `pod_access`, `iat`, `exp` (3600 s),
  `jti`. The signing secret is dedicated to pods.
- Refresh: `pod-` + 43-character base64url of 32 random bytes. Lion stores only
  its SHA-256. It never rotates; the identity proof binds it to the device.
- `pod-` is dispatched by prefix before any JWT parsing. Only the cloud WebSocket
  `/auth` and `/api/v1/hub/token/exchange` accept pod access tokens. Every other
  API keeps accepting user tokens only. No target allowlist applies in v1.
- A pod session acts as its user across all of that user's Spaces. Revocation
  takes effect at the next refresh (up to one access lifetime).

## 6. Pod runtime authentication

- Cloud `/auth`: `{"token":"pod-…","devToken":null,"source":"pod","deviceId","activeScope"}`,
  then `/focus`. The pod refreshes before access expiry.
- Space switching: the pod chooses among the Spaces returned by `/auth` and
  re-sends `/focus`; it keeps one local JWT and Hub key per Hub.
- Local Hub: fetch the Hub key with the pod access token, prove the Hub
  identity, then `/auth` with `source=cloud` and the pod access token; store the
  returned local JWT for that Hub and prefer it afterwards.

## 7. Native Client targets

All targets are handled by the native Client and never forwarded. Payloads are
domain input JSON. Clients without a usable Bluetooth adapter report
`adapter.state` and reject commissioning with `bluetooth_unavailable`.

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
| `/local/pod/commission/cancel` | `{"sessionId"}` | session snapshot |
| `/local/pod/commission/status` | `{}` | `{"session": snapshot or null}` |

Events: `/local/pod/discovery/changed` carries a discovery snapshot;
`/local/pod/commission/changed` carries a session snapshot. Scanning runs only
while at least one lease is live and stops while a session is connecting.
Candidates unseen for 10 s are removed. One session per native Client.

Session states, in order: `connecting`, `awaiting_code`, `securing`,
`reading_info`, `awaiting_wifi`, `joining_wifi`, `awaiting_authorization`,
`issuing`, `delivering`, `activating`, `completed`; terminal `failed`,
`cancelled`, `timed_out`. Wi-Fi states are skipped when the pod reports a working
configured network. `revision` increases on every observable change.

Session error codes: `bluetooth_unavailable`, `device_busy`, `disconnected`,
`code_rejected` (retry allowed), `code_locked`, `wifi_auth_failed`,
`wifi_not_found`, `wifi_failed`, `issue_failed`, `delivery_failed`,
`activation_failed` (with the pod's activation code in `detail`),
`session_timeout`. After issue, every terminal state other than `completed`
revokes the credential.

The schemas are `schema/pod-discovery.v1.schema.json` and
`schema/pod-commission.v1.schema.json`.
