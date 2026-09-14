# Node commissioning HTTP contract

Scope: health, challenge, prepare, status, accept. Cancel is unchanged and excluded.
Matter implementations adopt this contract. Other service preparation protocols are not migrated here.

- Health uses ServiceStatusPayload, not the internal SDK state enum.
- Challenge uses NodeChallengePayload camelCase JSON before signing. ES256 and Ed25519 are valid. The payload is distinct from snake_case JWT claims.
- Repeating an identical live challenge must reuse its original signature evidence; it must not extend the original deadline. Different bindings sharing a nonce conflict. A reserved challenge cannot be replaced.
- Prepare/status/accept authenticate token, Hub and local challenge binding. hubUrl is an address hint, never a trust root.
- Prepare requires a nonblank idempotencyKey (at most 256 UTF-8 bytes) and optional nonnegative expectedRevision. Keys are scoped to the authenticated onboarding transaction. Replay of the current operation is resolved before revision comparison and returns started=false plus current readiness. A distinct key starts or joins a new preparation attempt after revision comparison. Replacing the key supersedes the old attempt: retry guarantees apply to the current attempt within the transaction window.
- started means this request initiated shared backend preparation, not that a per-client task was allocated. A ready runtime or an already running preparation returns false. Preparation never reboots a machine or chip.
- Prepare returns {ok, started, readiness}; status returns {ok, readiness}. readiness conforms to node-preflight-v1.json. revision changes only when observable preflight state changes, not on reads or Hub socket changes. It is scoped to one process lifetime; pre-accept transactions do not survive restart.
- preparing is pending, ready means runtime checks passed, failed includes a typed error. commissionable requires runtimeUsable and ready. Admission capacity is checked separately for the requesting identity; resource exhaustion returns an HTTP error, never successful readiness for a rejected admission.
- Private diagnostics belong under details. No caller may infer commissioning readiness from private fields.
- HTTP errors use {ok:false, reason:<stable code>, error:<message>}, with optional detail. 400 invalid input, 409 revision/identity conflict, 429 admission throttling, 503 temporary unavailability, 500 persistence/internal failure. No normal response is sent before a durable accept commit.
- Accept returns {ok:true}; additive fields are permitted. Identical durable credentials may be acknowledged on replay without a live challenge. Different credentials conflict. Failed persistence releases the reservation. Success schedules Hub connection; it does not promise that the WebSocket is already connected.
- Implementations may bound body sizes, session counts, cache entries and worker counts. They must reject excess work explicitly, never silently evict active transactions. The ESP profile admits 12 KiB bodies, 8 KiB JWTs and four pending transactions.
- Hub identity caches use the verified Hub/tenant/scope tuple, have bounded capacity and at most the five-minute transaction TTL, and do not replace JWT signature, expiry or challenge verification.
