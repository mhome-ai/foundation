# mhome-artifact-api

Storage-independent artifact references shared by mHome runtimes.

The crate is the source of truth for the `meow-artifact://v2/` URI format and its
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

Capability media values (`Image`, `Audio`) use `MediaReference`: exactly `{ "uri": "meow-artifact://v2/..." }`. The reference has no URL, inline bytes, MIME or creation timestamp fields. Parse and check kind and tenant/scope at capability boundaries; do not implicitly import media. Upload bytes with `PutArtifactRequest` or explicitly download finite HTTP(S) content with `ImportArtifactRequest` via `/app/artifact/put` and `/app/artifact/import`. Storage metadata and upload transports retain their MIME and byte fields. Resolve to temporary delivery URLs only at consumers. Conversation media sources retain their existing tagged artifact protocol. Video storage remains unsupported by the runtimes.

Upload/import preserves bytes. Callers perform conversion before upload. Public ingress is limited to 1 MiB; internal producers retain their own limits. The shared reference and storage contracts do not impose the public ingress limit.

`put` and `upload/prepare` require canonical lowercase MIME types matching the URI
metadata grammar: a nonempty type and subtype, one slash, and at most 255 ASCII
characters. Parameters, whitespace and uppercase letters are rejected before
decoding content or issuing an upload grant, for every lifecycle. Callers must
send `audio/ogg`, for example, rather than `audio/OGG` or `audio/ogg;codecs=opus`.
Image content is still inspected by storage to determine its actual format.

### Lifecycle and resource identity

`put`, explicit HTTP `import`, and `upload/prepare` accept `lifecycle`: `std`, `tmp`,
or `man`. Omission means `std`. Put/resolve responses report the selected lifecycle.
The former `temporary` request/response field is no longer accepted. Optional fields
must be omitted when absent; explicit `null` values are rejected.

The canonical URI is
`meow-artifact://v2/<tenant>/<scope>/<lifecycle>/<resourceId>/<sha256>/<metadata>`.
SHA-256 always hashes the original bytes. Standard and temporary resources use their
digest as resource ID in independent namespaces; the same bytes never share an object
across policies. Metadata remains canonical base64url JSON. Version 1 references are
not accepted; this is a coordinated protocol cutover, without automatic migration.

- `std`: durable content retained until explicit deletion or a Space/store purge.
  Reads, metadata queries and repeated ingestion do not maintain an expiry timestamp.
- `tmp`: short-lived content whose retention policy belongs to its storage runtime.
  Local MeowCore keeps bytes only in memory, with one-hour idle retention, lost on
  process restart. Content reads and repeated ingestion renew it; metadata and resolve
  do not. There is no disk spill, total capacity or object-count quota. Cloud runtimes
  keep bytes in shared object storage, with a fixed one-hour access lifetime from the
  storage server's last-modified timestamp. Re-upload renews cloud content; reads,
  metadata and resolve do not. Both runtimes reject expired content immediately and
  report the current deadline in `expiresAtUnixMs`; callers must not assume that a
  read extends it. Existing per-upload limits still apply. Cloud object reclamation
  uses an asynchronous, tmp-only bucket lifecycle rule (one day), so physical removal
  can follow logical expiry. Explicit deletion remains available in both runtimes.
- `man`: caller-managed durable content. Automatic retention never deletes it.
  Callers must supply and retain `creationId`; the service never generates one.
  IDs contain 1-128 ASCII letters, digits, hyphens or underscores, and are preserved
  verbatim, including case. Conventional UUID strings are accepted. The resource
  identity is `(creationId, sha256)` within its tenant, Space and lifecycle.
  Repeating the same ID and bytes is idempotent; different IDs or different bytes
  address separate resources. Metadata aliases share the same resource. Callers
  save the returned URI and explicitly delete resources when no longer needed.
  Local paths and cloud object keys derive from both the ID and digest, so old URIs
  and delivery URLs cannot be rebound to different content after deletion.

`/artifact/delete` takes `{ "uri": "meow-artifact://…" }` and returns `{}`. Missing content succeeds.
It deletes only the selected resource in its tenant, Space and lifecycle namespace;
metadata aliases of that resource share deletion. Independently created manual resources
do not share deletion. An already opened snapshot may finish after deletion or expiry.
Standard/temporary digest identities can be recreated: do not treat an unexpired signed
URL as a permanently revoked object ID.

The App Facade exposes `/app/artifact/delete`; Wasm hosts expose
`artifact_delete_async`. LLM inputs continue to use
canonical references. Local temporary HTTP uploads and native reads transfer bytes from memory;
large native transfers use bounded attachments on the existing IPC connection. Temporary
delivery uses `no-store`; resolve grants never outlive the reported object expiration.
Only `tmp` responses contain `expiresAtUnixMs`. Native transport descriptors retain their
`temporary` boolean to distinguish memory from disk; it is not an ingress lifecycle policy.

Forwarding services must preserve the owner's policy and URI. Each storage implementation
must adopt the lifecycle contract before consuming this protocol release; a shared cloud
service must not emulate temporary storage with an uncoordinated process-local map.
