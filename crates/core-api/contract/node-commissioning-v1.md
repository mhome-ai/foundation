# Node commissioning HTTP contract

Scope: health, challenge, prepare, status, accept. All Node implementations adopt this contract. There is no Node commissioning cancel endpoint; cancelling a Core add session only stops that session and revokes its provisional credentials.

- Health uses ServiceStatusPayload, not the internal SDK state enum.
- Challenge uses NodeChallengePayload camelCase JSON before signing. ES256 and Ed25519 are valid. The payload is distinct from snake_case JWT claims.
- Repeating an identical live challenge must reuse its original signature evidence; it must not extend the original deadline. Different bindings sharing a nonce conflict. A reserved challenge cannot be replaced.
- Prepare/status/accept authenticate token, Hub and local challenge binding. hubUrl is an address hint, never a trust root.
- Prepare authenticates and checks admission/readiness without starting, stopping or repairing a runtime. It takes the same identity fields as status; no idempotencyKey or expectedRevision. Runtime lifecycle belongs to the Node owner.
- Prepare returns {ok, readiness}; status returns {ok, readiness}. readiness conforms to node-preflight-v1.json. revision changes only when observable preflight state changes, not on reads or Hub socket changes. It is scoped to one process lifetime; pre-accept transactions do not survive restart.
- preparing is pending, ready means runtime checks passed, failed includes a typed error. commissionable requires runtimeUsable and ready. Admission capacity is checked separately for the requesting identity; resource exhaustion returns an HTTP error, never successful readiness for a rejected admission.
- Private diagnostics belong under details. No caller may infer commissioning readiness from private fields.
- HTTP errors use {ok:false, reason:<stable code>, error:<message>}, with optional detail. 400 invalid input, 409 revision/identity conflict, 429 admission throttling, 503 temporary unavailability, 500 persistence/internal failure. No normal response is sent before a durable accept commit.
- Accept returns {ok:true}; additive fields are permitted. Identical durable credentials may be acknowledged on replay without a live challenge. Different credentials conflict. Failed persistence releases the reservation. Success schedules Hub connection; it does not promise that the WebSocket is already connected.
- Implementations may bound body sizes, session counts, cache entries and worker counts. They must reject excess work explicitly, never silently evict active transactions. The ESP profile admits 12 KiB bodies, 8 KiB JWTs and four pending transactions.
- Hub identity caches use the verified Hub/tenant/scope tuple, have bounded capacity and at most the five-minute transaction TTL, and do not replace JWT signature, expiry or challenge verification.

Core does not automatically retry commissioning requests. Pending readiness may be polled within a fixed deadline. A failed/cancelled session and revocation of its provisional credentials commit together in Core local storage. Node clears stale identities only following authenticated Hub rejection or expiry, and checks persistence failures before declaring cleanup complete. Removal compares the exact old credentials. A new add may be refused while an old binding cannot yet be verified/cleared.
