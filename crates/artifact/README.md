# mhome-artifact-api

Storage-independent artifact references shared by mHome runtimes.

The crate is the source of truth for the `meow-artifact://v3/` URI format and its
immutable image, audio, video, and file metadata. The published schema and conformance
fixtures are intended for non-Rust runtimes.

The crate also defines the transport-neutral `/artifact/resolve`, `/artifact/put`, and
`/artifact/upload/prepare` request and response contracts. The upload preparation
contract keeps large artifact bytes on an authenticated HTTP data plane instead of
embedding them in a control-plane message.
Delivery is an explicit `DATA_URL` or `SIGNED_URL` union; a signed URL may point to a
cloud object store or an authenticated local streaming endpoint.

Retention mode and resource identity are encoded in the reference. Exact TTL deadlines,
filesystem paths, signed URLs, and cloud object keys remain runtime concerns.

Capability media values (`Image`, `Audio`) use `MediaReference`: exactly `{ "uri": "meow-artifact://v3/..." }`. The reference has no URL, inline bytes, MIME or creation timestamp fields. Parse and check kind and tenant/scope at capability boundaries; do not implicitly import media. Upload bytes with `PutArtifactRequest` or explicitly download finite HTTP(S) content with `ImportArtifactRequest` via `/app/artifact/put` and `/app/artifact/import`. Storage metadata and upload transports retain their MIME and byte fields. Resolve to temporary delivery URLs only at consumers. Conversation media sources retain their existing tagged artifact protocol. Video storage remains unsupported by the runtimes.

Upload/import preserves bytes. Callers perform conversion before upload. Public ingress is limited to 1 MiB; internal producers retain their own limits. The shared reference and storage contracts do not impose the public ingress limit.

`put` and `upload/prepare` require canonical lowercase MIME types matching the URI
metadata grammar: a nonempty type and subtype, one slash, and at most 255 ASCII
characters. Parameters, whitespace and uppercase letters are rejected before
decoding content or issuing an upload grant, for every policy combination. Callers must
send `audio/ogg`, for example, rather than `audio/OGG` or `audio/ogg;codecs=opus`.
Image content is still inspected by storage to determine its actual format.
Put and upload schemas also enforce kind-dependent MIME and duration rules:
image/audio/video require their respective MIME prefix; files cannot use a video
MIME type, and only audio/video may carry `durationMillis`. Shared negative fixtures
cover these combinations so schema-based callers and Rust reject the same inputs.
Put content uses canonical padded standard base64; missing padding, whitespace and
nonzero unused trailing bits are rejected across runtimes.

### Independent identity and retention

`put`, explicit HTTP `import`, and `upload/prepare` accept `identity` (`auto` or
`man`, default `auto`) and `retention` (`std` or `tmp`, default `std`). Responses
report both dimensions. All four combinations are supported. Removed `lifecycle`
and `temporary` fields are rejected. Optional fields must be omitted when absent;
explicit `null` is rejected.

The canonical URI is
`meow-artifact://v3/<tenant>/<scope>/<retention>/<identity>/<resourceId>/<sha256>/<metadata>`.
SHA-256 always hashes the original bytes. Both policy dimensions participate in
resource identity and deletion. The same bytes stored as std and tmp are independent.
Metadata remains canonical base64url JSON. Versions 1 and 2 are rejected; callers
and runtimes must cut over together, without automatic legacy data migration.

- `auto`: resource ID equals the content digest. Repeated identical content shares
  a resource within the same tenant, Space and retention namespace.
- `man`: callers supply and retain `creationId`; the service never generates it.
  IDs contain 1-128 ASCII letters, digits, hyphens or underscores, preserving case.
  UUID strings are accepted. Identity is `(creationId, sha256)` within the policy
  namespace. Identical pairs retry idempotently; different IDs or bytes are independent.
  Manual identity does not disable temporary expiration or add creator authorization.
- `std`: durable content retained until explicit deletion or Space/store purge.
  Reads, metadata queries and repeated ingestion do not maintain expiry timestamps.
- `tmp`: local MeowCore uses memory only, with one-hour idle retention and loss on
  process restart. Content reads and repeated ingestion renew it; metadata and
  resolve do not. No disk spill or aggregate memory quota is applied. Cloud uses
  shared object storage with one-hour access lifetime from server Last-Modified;
  re-upload renews it, reads do not. Both reject expired content immediately and
  report `expiresAtUnixMs`. Existing per-upload limits apply. Cloud physical
  reclamation uses an asynchronous tmp-only bucket rule (one day). Callers may
  explicitly delete sooner. These rules apply equally to auto/tmp and man/tmp.

`/artifact/delete` takes `{ "uri": "meow-artifact://…" }` and returns `{}`. Missing content succeeds.
It deletes only the selected resource in its tenant, Space and identity/retention namespace;
metadata aliases of that resource share deletion. Independently created manual resources
do not share deletion. An already opened snapshot may finish after deletion or expiry.
Content-addressed identities can be recreated: do not treat an unexpired signed
URL as a permanently revoked object ID.

The App Facade exposes `/app/artifact/delete`; Wasm hosts expose
`artifact_delete_async`. LLM inputs continue to use
canonical references. Local temporary HTTP uploads and native reads transfer bytes from memory;
large native transfers use bounded attachments on the existing IPC connection. Temporary
delivery uses `no-store`; resolve grants never outlive the reported object expiration.
Only `tmp` responses contain `expiresAtUnixMs`. Native transport descriptors retain their
`temporary` boolean to distinguish memory from disk; it is not an ingress policy.

Forwarding services must preserve the owner's policy and URI. Each storage implementation
must adopt the identity/retention contract before consuming this protocol release; a shared cloud
service must not emulate temporary storage with an uncoordinated process-local map.

Explicit import `mimeType` overrides must use the same canonical lowercase MIME and kind rules as uploads. Importers normalize the source HTTP Content-Type only when no override is supplied.
