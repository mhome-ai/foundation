# Host provisioning v1

Embedded Linux Hosts without a working network get Wi-Fi over BLE from a native
Client (desktop/CLI Clientd, Android, iOS). Desktop, Docker and Android Hosts are
already online and never advertise. Provisioning only connects the Host to the
LAN; ownership is then established by the existing Host claim over the LAN.

The BLE layer is the one pods use (`pod-commissioning-v1.md` sections 2 and 3):
same service UUID, manufacturer data with device kind `2`, protocomm security 2
with username `meow` and a 6-digit code, the same standard endpoints, and the
same characteristic UUID fallback.

## 1. Advertising

- The Host advertises only in first provisioning, inside a re-provision window,
  or after a factory reset. A Host on a working network does not advertise.
- Flags: bit 0 commissionable (always set while advertising), bit 1 set when the
  Host still has a saved network (re-provision window).
- `shortId`: the first 4 bytes of the SHA-256 of the Host bootstrap public key,
  which survives factory reset.
- Local name: the Host name, at most 20 bytes.

## 2. Pairing code

Hosts that support BLE provisioning have a display or a local console.

- The Host generates a uniformly random 6-digit code and precomputes the SRP salt
  and verifier when BLE starts.
- The code is published to the local display interface (and the Host log) only
  while a central is connected.
- A new code is generated after a successful provisioning, after 5 failed
  handshakes, or 10 minutes after it was first shown.
- One BLE connection at a time. A session ends 10 minutes after connect, or
  2 minutes after connect without a handshake.

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
- `{"op":"finish"}` → `{"ok":true}`. The Host closes BLE and continues its
  lifecycle (starts managed services, announces itself over mDNS). Without
  `finish` the Host closes BLE 30 seconds after a successful join, or when the
  central disconnects.

`prov-config` status reports `connected` with the IPv4 address once the Host is
on the LAN; `connection_failed` with `auth_error` or `network_not_found` when
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
| `/local/host/provision/status` | `{}` | `{"session": snapshot or null}` |

Event `/local/host/provision/changed` carries a session snapshot. One session
per native Client, and none while a pod session is active; discovery scanning
pauses during either.

Session states, in order: `connecting`, `awaiting_code`, `securing`,
`reading_info`, `awaiting_wifi`, `joining_wifi`, `completed`; terminal `failed`,
`cancelled`, `timed_out`. `completed` carries `host` and the reported
`addresses`; the commissioner sends `finish` before reporting it.

Error codes are the pod session codes without the credential ones:
`bluetooth_unavailable`, `device_busy`, `disconnected`, `code_rejected` (retry
allowed), `code_locked`, `wifi_auth_failed`, `wifi_not_found`, `wifi_failed`,
`session_timeout`. Retry rules are the pod ones: a rejected code returns to
`awaiting_code`, a failed join to `awaiting_wifi`, each with its error; `failed`
and `timed_out` always carry `error`.

The schema is `schema/host-provision.v1.schema.json`.

## 5. Host-side entry points

- First provisioning starts BLE at boot when the Host has no known network.
- Re-provision window: opened from the Host's local interface (display settings
  or `meowhostd provision open`). Wi-Fi only; Host identity, owners and services
  are kept. Closes after a successful join, on `meowhostd provision close`, or
  after 10 minutes.
- Factory reset: from the local interface (`meowhostd factory-reset`). Stops
  services, forgets saved Wi-Fi, erases owners and the Host security store, keeps
  the bootstrap key, and returns to first provisioning.
