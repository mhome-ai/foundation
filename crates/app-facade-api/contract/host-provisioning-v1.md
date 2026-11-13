# Host provisioning v1

BLE-capable embedded Linux Hosts get Wi-Fi or recover an unfinished first claim
over BLE from a native Client (desktop/CLI Clientd, Android, iOS). Desktop, Docker
and Android Hosts use LAN discovery rather than advertising BLE setup.
Provisioning connects the Host to the LAN
and hands the provisioner a one-time claim token; ownership is then established
by the existing Host claim over the LAN, which requires that token (section 4).

The BLE layer is the one pods use (`pod-commissioning-v1.md` sections 2 and 3):
same service UUID, manufacturer data with device kind `2`, protocomm security 2
with username `meow` and a 6-digit code, the same standard endpoints, and the
same characteristic UUID fallback.

## 1. Advertising

- A BLE-capable Host advertises during first network provisioning, while awaiting
  its first claim (including with a working network), or inside an explicit
  re-provision window. Restart never reopens first-claim authority on an already
  claimed Host.
- Flags: bit 0 commissionable (set while advertising, except during a code
  lockout), bit 1 set when the Host has a saved network, including unclaimed
  recovery and re-provision windows.
- `shortId`: the first 4 bytes of the SHA-256 of the Host bootstrap public key,
  which is replaced after factory reset. The bootstrap key only names the Host
  (`shortId`, `fingerprint`); it signs nothing in provisioning or claim.
- Local name: the Host name, at most 20 bytes.

## 2. Pairing code

Hosts that support BLE provisioning have a display or a local console.

- The Host generates a uniformly random 6-digit code and precomputes the SRP salt
  and verifier when BLE starts.
- The code is published to the local display interface (and the Host log) only
  while a central is connected.
- A new code is generated after a successful provisioning, after 5 failed
  handshakes, or 10 minutes after it was first shown.
- Lockout: each rotation caused by 5 failed handshakes is followed by 30 seconds
  in which the Host refuses every handshake, doubling for each consecutive such
  rotation up to 10 minutes; a successful handshake resets the backoff. While
  locked the Host advertises with the commissionable flag cleared, its
  `proto-ver` app info is `"meow":{"ver":"1","cap":["host"],"lockedForMs":N}`
  with the remaining lockout, and every handshake is refused with ATT
  Insufficient Authorization, like a wrong code. The commissioner reads
  `proto-ver` after each refused handshake and fails the session with
  `code_locked` when `lockedForMs` is present. The Host's local status reports
  the lockout end as `lockedUntilMs`.
- One BLE connection at a time: a second central is disconnected, and only the
  central that owns the setup session may write. Request and response buffers
  are kept per central.
- A session ends 10 minutes after connect, or 2 minutes after connect without
  a handshake.

## 3. Endpoints

| Endpoint | Encrypted | Purpose |
| --- | --- | --- |
| `proto-ver` | no | app info `"meow":{"ver":"1","cap":["host"]}` |
| `prov-session` | handshake | security 2 |
| `prov-scan` | yes | standard Wi-Fi scan |
| `prov-config` | yes | standard Wi-Fi set/apply/status |
| `prov-ctrl` | yes | standard reset after a failed join, before a retry |
| `host-info` | yes | Host information and finish |

Fallback characteristic UUIDs: the standard endpoints as for pods, `host-info`
`0xFF54`.

`host-info` bodies are UTF-8 JSON, at most 480 bytes:

- `{"op":"info"}` →

  ```json
  {
    "protocol": 1,
    "hostId": "…",
    "name": "Kitchen Host",
    "productId": "camera",
    "firmwareVersion": "0.9.0",
    "fingerprint": "<hex SHA-256 of the bootstrap public key>",
    "wifiConfigured": false,
    "onboardingRequired": true,
    "networkOnline": false
  }
  ```

  `productId` is optional. `onboardingRequired` reports whether this Host still
  awaits its first claim and is required. `networkOnline` reports current LAN
  readiness. A securely paired Host with `networkOnline: true`
  and `onboardingRequired: true` can finish without resubmitting Wi-Fi. `finish`
  issues a fresh first-claim token after checking authoritative Host ownership.
  Already claimed Hosts never issue a new first-claim token.
- `{"op":"finish"}` → `{"ok":true,"claimToken":"…"}`. After a successful
  first-provisioning join, or secure recovery with an already working network,
  the response carries a claim token: 32 random bytes as 43 unpadded base64url
  characters. A new join persists its token hash before applying Wi-Fi and
  exposes the token only after success. Recovery issues a fresh token after
  checking that the Host is still unclaimed. Repeated `finish` requests in the
  same session return the same token. A claimed Host's re-provision window
  carries no first-claim token.
- `finish` returns `{"ok":false}` when the requested network has not been
  established and the Host is not recovering an unfinished claim on a working
  network. Refusal keeps the current setup session open. The Client reports
  completion only after a positive receipt, with a valid token when the secured
  Host information indicated that first claim was pending. A receipt that
  unexpectedly carries a token for an already claimed Host also fails.
  `finish` ends the central's setup session. Without it, the Host disconnects the
  central 30 seconds after a successful join; the commissioner retries `finish`
  within that window. Network readiness and mDNS announcement do not establish
  ownership. An unclaimed Host keeps its BLE recovery surface available;
  a claimed Host closes a re-provision window
  after the join.

Every request and every encrypted response is at most 512 bytes; the response
is read with offset reads. `prov-scan` start blocks while the Host scans and
answers within 12 seconds, so commissioners give that exchange a longer timeout
(the shared core uses 30 seconds). Scan result requests return as many of the
requested entries as fit one encrypted 512-byte value, possibly fewer than
`count`; commissioners advance `start_index` by the number of entries returned
and stop at `result_count` or on an empty page. This also works with ESP-IDF
devices, which return exactly the requested entries (commissioners ask for at
most 4 at a time).

`prov-config` status reports `connected` with the IPv4 address after the Host
has saved the new profile. The Linux backend first activates a temporary
profile. On failure or local cancellation it removes that profile and attempts
to reactivate the previous one. `connection_failed` may distinguish
`auth_error` or `network_not_found`. After a failed join the commissioner sends
`prov-ctrl` reset before the next `set_config`.

## 4. Native Client targets

Handled by the native Client, never forwarded. Payloads are domain input JSON.
Candidates come from the shared discovery (`/local/pod/discovery/*`) with
`kind` = `host`.

| Target | Request | Response |
| --- | --- | --- |
| `/local/host/provision/start` | `{"candidateId"}` | session snapshot |
| `/local/host/provision/code` | `{"sessionId","code"}` | session snapshot |
| `/local/host/provision/wifi` | `{"sessionId","ssid","password"?}` | session snapshot |
| `/local/host/provision/wifi/scan` | `{"sessionId"}` | session snapshot |
| `/local/host/provision/status` | `{}` | `{"session": snapshot or null}` |
| `/local/host/provision/cancel` | `{"sessionId"}` | session snapshot |

Event `/local/host/provision/changed` carries a session snapshot. One session
per native Client, and none while a pod session is active; discovery scanning
pauses during either. Rejected requests use the error envelope and the stable
`details.reason` values of `pod-commissioning-v1.md` section 7. The session is
owned by the native core until success, failure or the session deadline. Host
setup is bound to the signed-in native account context, including before a Space
exists. An account change retires the session and its claim token; status and
queued events cannot transfer an old handoff to the new login.
Cancellation is explicit. It transitions through `cancelling` and reaches
`cancelled` only after the link and radio are released. Repeated cancellation
returns the same session; a different session ID is rejected. Cancellation during
`joining_wifi` is rejected as busy: applying Wi-Fi and handing off the first-claim
token form a bounded critical section. Leaving the UI does not cancel a session.
There is no renewal endpoint.

Session states, in order: `connecting`, `awaiting_code`, `securing`,
`reading_info`, `awaiting_wifi`, `joining_wifi`, `completed`; terminal `failed`,
`timed_out`, `cancelled`; transitional `cancelling`. `completed` carries `host`, the reported `addresses`
(IPv4 only, possibly empty) and `claimToken` from the `finish` response; the
commissioner sends `finish` (up to 3 times while the secure session lasts)
before reporting it. A missing or refused `finish` is a failed session. The
first-claim handoff includes `claimToken`. A completed Wi-Fi update on an
already claimed Host omits it. An account change can redact the token from a
terminal event, so confirm a tokenless completion through the native status
target before displaying the Wi-Fi update as successful. No other state
carries `claimToken`.

The first claim (TOFU) after BLE provisioning must include the returned token.
Native `host.claim` passes it to `POST /v1/auth/claim`, alongside the environment,
cloud proof and direct Client exchange secret (`core-api::host::access::ClaimRequest`).
There is no Core-proxied first-claim route. The Host rejects a missing or incorrect
provisioning token with 403 `HOST_CLAIM_TOKEN_REQUIRED`. The token remains valid
until claim succeeds, another setup issues a replacement token, or factory reset.
BLE `completed` confirms the network/handoff step, not Host ownership. The App's
Host onboarding operation waits for explicit ownership confirmation, then claims
over the LAN. It completes only after the existing claim API confirms cloud
registration. It does not install Core or Plugins. Hosts that were not provisioned
over BLE use the ordinary direct LAN claim flow. See the Foundation Host
management authorization contract.

Error codes are the pod session codes without the credential ones:
`bluetooth_unavailable`, `device_busy`, `disconnected`, `code_rejected` (retry
allowed), `code_locked`, `wifi_auth_failed`, `wifi_not_found`, `wifi_failed`,
`session_timeout`. Retry rules are the pod ones: a rejected code returns to
`awaiting_code`, a failed join to `awaiting_wifi`, each with its error; `failed`
and `timed_out` always carry `error`. `code_locked` is terminal.

The schema is `schema/host-provision.v1.schema.json`.

## 5. Host-side entry points

- First provisioning starts BLE at boot when the Host has no known network.
  BLE-capable Hosts whose authoritative local claim is still absent also restore
  setup after restart, keeping saved Wi-Fi. Network readiness is not ownership.
  An existing local claim, including one awaiting cloud confirmation, must never
  reopen first-claim authority. An unreadable ownership store refuses recovery.
- Re-provision window: opened on the target Host with `meow host provision open`.
  The CLI talks directly to the Host's local control socket, without a Client
  daemon. Mutating local provisioning commands (`open`,
  `close`, `factory-reset`) require root or membership in the `meow-admin`
  group (checked with the peer credentials of the local control socket); status
  is available with `meow host provision status`.
  Host identity, owners and services are kept. The window closes after a
  successful join, on `meow host provision close`, or after 10 minutes.
- Factory reset: from the local interface (`meowhostd factory-reset`). Stops
  services, forgets saved Wi-Fi on BLE provisioning appliances, erases owners,
  the Host security store and bootstrap key, and returns to first provisioning.
  Machine identity and installed software are retained.
