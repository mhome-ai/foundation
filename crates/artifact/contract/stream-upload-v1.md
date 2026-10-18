# Finite streaming Artifact uploads

This contract ingests a finite, ordered byte stream and publishes an immutable
Artifact after explicit completion. It is independent of clients, media codecs,
storage engines and execution locations. It supports generated files and media
whose final length and SHA-256 are unknown when production begins. The existing
`/artifact/upload/prepare` operation remains the single-request upload contract
for already completed content with a known length and digest.

The schemas and Rust types define messages, not a working storage service.
Implementations must enforce the lifecycle below. Runtime capability and ingress
limits still apply: declaring a kind in the shared schema does not enable that
kind in a runtime which does not support it.

## Start and authority

`/artifact/upload/start` accepts `StartArtifactUploadRequest`: `kind`, `mimeType`,
`maxSizeBytes`, and the existing independent `identity`, `retention`, `creationId`
fields. It does not accept bytes, a conversation identifier, a device model,
codec parameters, a sample rate or an ASR request.

The owner derives tenant, scope and caller from authenticated context. It admits
the operation only within its supported kinds, per-object and aggregate resource
limits. A requested limit above policy is rejected, not silently raised or
clamped. A successful response repeats the accepted `maxSizeBytes`, returns an
opaque `uploadId`, an authorized HTTP `url` with required `headers`, and an absolute
`expiresAtUnixMs`. A local TLS endpoint supplies `hostIdentity` under the existing
trusted identity contract. Native endpoints use HTTPS. Secrets in URLs and headers
must not enter ordinary logs.

The lease covers receiving, pending completion and replay of the completion
result. Expiration never means completion. Implementations bound outstanding
leases, concurrency and temporary bytes; abandoning an upload must not allow
unbounded memory, files or object-store parts to accumulate.

An upload is bound to one owner and authenticated authorization context. Other
callers or scopes cannot complete or cancel it by knowing the upload ID. A
runtime permission revocation or owner change prevents further operations.
Client leases bind the verified installation ID, principal and enrollment credential,
not a transient WebSocket connection ID. Reconnecting the same authenticated
installation within the lease preserves its ownership; a new installation or
enrollment cannot take over the old lease. Control requests do not silently switch
owners during an upload.

## HTTP byte transfer

Send one sequential HTTP `PUT` to the supplied URL. The body contains the exact
artifact bytes. The sender can use a known Content-Length or an HTTP mechanism
that represents an unknown final length. The response headers must not require
the final Content-Length or final digest before transmission. Neither a JSON
array of byte chunks nor Base64 is part of this public data plane.

The receiver counts bytes and hashes the body incrementally, enforces the accepted
limit while reading, and stages content without publishing an Artifact. It rejects
an incomplete HTTP body, a concurrent second writer and writes after the transfer
has finished. A failed transfer cannot be resumed or replaced under this lease;
the sender starts a new upload. The implementation reclaims failed transfers.

On a successfully terminated, nonempty HTTP body, return status 200 with
`ArtifactUploadReceipt`: `uploadId`, actual `sizeBytes`, and actual `sha256`.
This is a staging receipt, not an Artifact URI or evidence of a successful commit.
The sender computes its own length and SHA-256 while producing bytes and compares
the receipt. It must not copy the receiver's digest as its only integrity check:
doing so could accept a truncated source that happened to form a valid HTTP body.

If the HTTP response is lost after staging, completion with the sender's own final
values can resolve the outcome. No byte retransmission is required once the owner
has staged the full body. Completion while the body is still being received returns a retryable
application error and does not discard that in-flight body.

## Complete

`/artifact/upload/complete` accepts `CompleteArtifactUploadRequest`: `uploadId`,
the sender's final `sizeBytes` and lowercase SHA-256, and optional `durationMillis`
for audio/video. Final metadata must match the immutable start declaration and
actual staged content. Kind, MIME, identity, retention and creation ID cannot be
changed by completion. Empty content, excess length, digest mismatch and invalid
kind-dependent metadata are rejected. Failed integrity validation invalidates
the upload and reclaims its staged data.

The owner validates before publication. Successful completion returns
`PutArtifactResponse` for a normal canonical Artifact. Consumers continue
to use the existing URI, resolve and read APIs. Upload IDs are never substituted
for Artifact URIs, and there is no public read API for in-progress content.

After the owner records successful completion, repeating the identical completion
within the lease returns that result without saving again or extending retention.
Reusing the upload with different completion metadata is a conflict. Once a final
save attempt fails or times out, that upload is terminal: it cannot be completed
or cancelled again. The caller retries by starting a new upload, without waiting
for the failed lease to expire. No background publication recovery is required.
The stored digest is always the digest of the final stored bytes. A transport
receipt is not a trusted substitute for the owner's validation at publication.

## Cancel, expiration and races

`/artifact/upload/cancel` accepts `{ "uploadId": "..." }` and returns `{}`.
Cancellation of an open, receiving or staged upload makes further writes and
completion fail and reclaims its staged resources. Repeating cancellation of
the same cancelled upload succeeds while its lease is valid. Cancellation after
a failed final save reports a conflict, not successful cancellation. Unknown or
expired uploads return not-found/expired without disclosing another caller's upload.

Completion and cancellation serialize at the owner. If cancellation wins, no
Artifact is published. If completion wins, cancellation reports a conflict and
does not delete the committed Artifact. An in-flight final save also excludes
cancellation. Deletion remains the existing explicit Artifact delete operation.
Expiration reclaims incomplete content; it does not delete already committed
Artifacts, whose retention remains authoritative.

An owner restart may invalidate pending uploads. Durable restart recovery and
resumable upload are not guarantees of this contract. Cloud owners with multiple
instances must nevertheless use shared state or equivalent routing that preserves
one upload's authority and lifecycle across normal requests.

## Failure contract

Control operations use the existing application error envelope and codes. HTTP
transfer failures use the status in this table (without disclosing grant details).
Successful control responses continue to use the schemas in the manifest.

| Condition | Application code | HTTP status | Client action |
| --- | --- | --- | --- |
| Malformed declaration, final metadata, empty or truncated body, digest mismatch | `BAD_REQUEST` | 400 | Correct input; a failed body or integrity check requires a new upload |
| Missing authentication or invalid transfer grant | `UNAUTHENTICATED` | 401 | Reauthenticate; do not blindly replay bytes |
| Authenticated caller has lost permission | `UNAUTHORIZED` | 403 | Stop the upload |
| Unknown, expired or another caller's upload | `NOT_FOUND` | 404 | Start a new upload if still needed |
| Complete before a body is fully staged | `TEMPORARY_UNAVAILABLE` | — (control only) | Retry completion with the same final metadata within the lease |
| Concurrent writer; complete after cancellation/failure; changed completion replay; cancel during or after completion, or after failure | `PRECONDITION_FAIL` | 409 | Stop; never overwrite or delete committed content |
| Byte cap exceeded | `BAD_REQUEST` | 413 | Start a new upload within the supported cap |
| Admission quota exhausted, or a completion is already in flight | `TEMPORARY_UNAVAILABLE` | 503 | Back off; bounded control retries are allowed within the lease |
| HTTP transfer timed out | — | 408 | Start a new upload if still needed |
| Final save failed or timed out | `INTERNAL` | 500 | This upload is terminal; start a new upload if still needed |

Authorization is checked before exposing upload state. An authorized caller uses
`NOT_FOUND` for both unknown and expired IDs; no separate expiry probe is needed.
A failed transfer is terminal even if its HTTP failure was transient. A failed
final save also terminates its upload and releases staged resources and active
reservations; it never returns to a state that permits another save or successful
cancellation. Cleanup is best-effort: if the state store or storage is unavailable,
expiration reclaims abandoned staging and outstanding reservations. Starting a
new upload remains subject to service availability and ordinary admission limits.
Admission/authentication checks and a request made before staging is finished do
not constitute a final save attempt.

A timeout or lost response does not prove that no canonical bytes were written.
A failed upload may leave an unreferenced canonical Artifact, governed by its
existing retention policy (`std` may remain; `tmp` expires). This is an accepted
boundary, not a durable transaction/recovery guarantee. Never delete canonical
content merely because a save response was lost: other references may use it.

After a lost control response, a bounded identical completion retry may retrieve
an already recorded success; it must never repeat a failed save. If the caller
cannot confirm the outcome, it may report failure and start a new upload. There
is no requirement to poll until the lease expires. No error renews a lease.

## Format and storage boundaries

Artifact ingestion preserves bytes. Producers or media adapters perform WAV
framing, transcoding, compression and other transformations before those bytes
are committed as an Artifact. A byte stream is not automatically a streamable
encoding. This API provides no header-patching or arbitrary seek operation.

The existing `auto/man` and `std/tmp` meanings remain unchanged. In particular,
local `tmp` memory-only retention does not become disk-backed because an upload
is streamed; implementations must enforce finite quotas. Client bounded buffering
does not by itself establish constant server memory usage.

The API has no live broadcast, partial-object consumption, multipart resume or
parallel part assembly guarantee. Runtime streaming support must be tested with
generated non-media bytes as well as media, including size limits, truncation,
digest mismatch, cancellation, completion replay and authorization isolation.
