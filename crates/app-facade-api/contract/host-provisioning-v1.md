# Host provisioning v1

Embedded Linux Hosts without a working network get Wi-Fi over BLE from a native
Client (desktop/CLI Clientd, Android, iOS). Desktop, Docker and Android Hosts are
already online and never advertise. Provisioning connects the Host to the LAN
and hands the provisioner a one-time claim token; ownership is then established
by the existing Host claim over the LAN, which requires that token (section 4).

The BLE layer is the one pods use (`pod-commissioning-v1.md` sections 2 and 3):
same service UUID, manufacturer data with device kind `2`, protocomm security 2
with username `meow` and a 6-digit code, the same standard endpoints, and the
same characteristic UUID fallback.

## 1. Advertising

- The Host advertises only in first provisioning, inside a re-provision window,
  or after a factory reset. A Host on a working network does not advertise.
- Flags: bit 0 commissionable (set while advertising, except during a code
  lockout), bit 1 set when the Host still has a saved network (re-provision
  window).
- `shortId`: the first 4 bytes of the SHA-256 of the Host bootstrap public key,
  which survives factory reset. The bootstrap key only names the Host (`shortId`,
  `fingerprint`); it signs nothing in provisioning or claim.
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
    "wifiConfigured": false
  }
  ```

  `productId` is optional.
- `{"op":"finish"}` → `{"ok":true,"claimToken":"…"}`. After a successful join
  during first provisioning the response carries the claim token: 32 random
  bytes as 43 unpadded base64url characters. It is created when the join
  succeeds, the same token is returned by every `finish` until the Host closes
  BLE, and it is absent when no join succeeded and in a re-provision window
  (the Host already has an owner). The Host then closes
  BLE and continues its lifecycle (starts managed services, announces itself
  over mDNS). Without `finish` the Host closes BLE 30 seconds after a successful
  join, or when the central disconnects; the commissioner retries `finish`
  within that window.

Every request and every encrypted response is at most 512 bytes; the response
is read with offset reads. `prov-scan` start blocks while the Host scans and
answers within 12 seconds, so commissioners give that exchange a longer timeout
(the shared core uses 30 seconds). Scan result requests return as many of the
requested entries as fit one encrypted 512-byte value, possibly fewer than
`count`; commissioners advance `start_index` by the number of entries returned
and stop at `result_count` or on an empty page. This also works with ESP-IDF
devices, which return exactly the requested entries (commissioners ask for at
most 4 at a time).

`prov-config` status reports `connected` with the IPv4 address once the Host is
on the LAN (the address may be empty when the Host has none yet); `connection_failed` with `auth_error` or `network_not_found` when
the join failed. After a failed join the commissioner sends `prov-ctrl` reset
before the next `set_config`. In a re-provision window the previous network
stays saved until the new one connects.

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
| `/local/host/provision/cancel` | `{"sessionId"}` | session snapshot |
| `/local/host/provision/renew` | `{"sessionId"}` | session snapshot |
| `/local/host/provision/status` | `{}` | `{"session": snapshot or null}` |

Event `/local/host/provision/changed` carries a session snapshot. One session
per native Client, and none while a pod session is active; discovery scanning
pauses during either. Rejected requests use the error envelope and the stable
`details.reason` values of `pod-commissioning-v1.md` section 7. The session is
owned through the same 30-second lease, renewed with
`/local/host/provision/renew` every 10 seconds; an expired lease cancels it.

Session states, in order: `connecting`, `awaiting_code`, `securing`,
`reading_info`, `awaiting_wifi`, `joining_wifi`, `completed`; terminal `failed`,
`cancelled`, `timed_out`. `completed` carries `host`, the reported `addresses`
(IPv4 only, possibly empty) and `claimToken` from the `finish` response; the
commissioner sends `finish` (up to 3 times while the secure session lasts)
before reporting it. `claimToken` is absent when `finish` could not be
delivered, in a re-provision window, and with Hosts that predate claim tokens.
No other state carries `claimToken`.

The first claim (TOFU) of a Host that was provisioned over BLE must present this
token: `/app/system/hosts/claim` takes `{"hostId","claimToken"?}` and the native
`host.claim` passes it through to the Host's offline claim, `POST /v1/auth/claim`
`{"proof","claimToken"?}` (`core-api` `host::auth::ClaimRequest`). Clients send
`claimToken` only when they have one, because older Hosts reject unknown fields.
While a token is outstanding the Host answers a claim without it, or with a
different one, with 403 `{"code":"HOST_CLAIM_TOKEN_REQUIRED"}`. The token stays valid until a claim succeeds or the Host is
factory reset; the UI claims automatically with it after `completed`. Hosts
that were never provisioned over BLE keep the plain first claim.

Error codes are the pod session codes without the credential ones:
`bluetooth_unavailable`, `device_busy`, `disconnected`, `code_rejected` (retry
allowed), `code_locked`, `wifi_auth_failed`, `wifi_not_found`, `wifi_failed`,
`session_timeout`. Retry rules are the pod ones: a rejected code returns to
`awaiting_code`, a failed join to `awaiting_wifi`, each with its error; `failed`
and `timed_out` always carry `error`. `code_locked` is terminal.

The schema is `schema/host-provision.v1.schema.json`.

## 5. Host-side entry points

- First provisioning starts BLE at boot when the Host has no known network.
- Re-provision window: opened from the Host's local interface (display settings
  or `meowhostd provision open`). Mutating local provisioning commands (`open`,
  `close`, `factory-reset`) require root or membership in the `meow-admin`
  group (checked with the peer credentials of the local control socket); status
  stays readable by the same user. The command help says to use `sudo`. Wi-Fi only; Host identity, owners and services
  are kept. Closes after a successful join, on `meowhostd provision close`, or
  after 10 minutes.
- Factory reset: from the local interface (`meowhostd factory-reset`). Stops
  services, forgets saved Wi-Fi, erases owners and the Host security store, keeps
  the bootstrap key, and returns to first provisioning.
