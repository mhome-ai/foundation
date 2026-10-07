# mhome-protocomm

ESP-IDF protocomm security 2 and the Wi-Fi provisioning endpoints, for both
sides of a MeowLink BLE commissioning link:

- `ProtocommClient` (commissioner): `version`, `establish_sec2`, encrypted
  `call` / `call_json`, and `wifi_scan`, `wifi_set_config`, `wifi_apply`,
  `wifi_status`, `wifi_reset`, over any `Transport`.
- `SessionResponder` (device): the `prov-session` handshake and the record
  layer, as ESP-IDF `protocomm_security2` implements them.
- `srp`: SRP6a over the 3072-bit RFC 5054 group with SHA-512.

## Wire rules

- `A` is always 384 bytes; the client picks a new ephemeral rather than send a
  padded value, and the responder rejects any other length.
- The salt is hashed as an integer in `x` and as sent in `M`; generated salts
  never start with a zero byte.
- Records use AES-256-GCM with the first 32 bytes of the session key. From
  patch version 1 the 12-byte nonce is 8 session bytes and a big-endian counter
  starting at 1 that both sides advance after every record.
- A new `SessionCommand0` restarts the handshake in any state; a rejected proof
  returns the responder to waiting for one.
- A failed encrypted exchange ends the client's secure session, since the device
  may or may not have consumed the record. The caller runs a new handshake.

`Transport::exchange` returns `TransportError::Rejected` when the device answers
with an error (ESP-IDF reports a failed handler as an ATT error) and
`Disconnected` when the link is gone. On `SessionCommand1` a rejection is a
wrong code.

## Vectors

`tests/vectors/esp-idf-sec2.json` is generated from ESP-IDF v5.5.4 `esp_srp.c`
by [`tools/esp-idf-vectors`](tools/esp-idf-vectors/README.md) and covers a
leading-zero salt, `A` and `B`, and the record nonce counter including a lost
record.
