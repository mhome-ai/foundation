# mhome-artifact-api

Storage-independent artifact references shared by mHome runtimes.

The crate is the source of truth for the `meow-artifact://v1/` URI format and its
immutable image, audio, video, and file metadata. The published schema and conformance
fixtures are intended for non-Rust runtimes.

The crate also defines the transport-neutral `/artifact/resolve`, `/artifact/put`, and
`/artifact/upload/prepare` request and response contracts. The upload preparation
contract keeps large artifact bytes on an authenticated HTTP data plane instead of
embedding them in a control-plane message.
Delivery is an explicit `DATA_URL` or `SIGNED_URL` union; a signed URL may point to a
cloud object store or an authenticated local streaming endpoint.

TTL, storage placement, filesystem paths, signed URLs, and cloud object keys are
runtime concerns and are deliberately not encoded into an artifact reference.

Capability media values (`Image`, `Audio`) use `MediaReference`: exactly `{ "uri": "meow-artifact://v1/..." }`. The reference has no URL, inline bytes, MIME or creation timestamp fields. Parse and check kind and tenant/scope at capability boundaries; do not implicitly import media. Upload bytes with `PutArtifactRequest` or explicitly download finite HTTP(S) content with `ImportArtifactRequest` via `/app/artifact/put` and `/app/artifact/import`. Storage metadata and upload transports retain their MIME and byte fields. Resolve to temporary delivery URLs only at consumers. Conversation media sources retain their existing tagged artifact protocol. Video storage remains unsupported by the runtimes.

Upload/import preserves bytes. Callers perform conversion before upload. Public ingress is limited to 1 MiB; internal producers retain their own limits. The shared reference and storage contracts do not impose the public ingress limit.

### Temporary artifacts (MeowCore)

`put`, explicit HTTP `import`, and `upload/prepare` accept `temporary: true`.
Omitting the flag preserves durable storage. The flag is a storage request, not
part of the content-addressed URI: putting the same bytes in the same tenant and
scope reuses that content. Existing durable content is never downgraded. Put and
resolve responses report `temporary: true` and `expiresAtUnixMs` for temporary
content; durable responses omit both fields.

MeowCore keeps temporary content only in memory, without a total capacity or
object-count quota and without disk spill. Each successful content read resets
one hour of idle retention. Upload and repeated put / import revalidation also
reset that deadline; resolve and metadata queries do not. Objects never read
expire one hour after upload. Reads reject expired objects immediately, while
periodic maintenance reclaims idle entries. An already opened snapshot may
finish after deletion or expiry. Existing per-upload size and transport limits
still apply. The caller is responsible for peak memory use.

`/artifact/persist` takes `{ "uri": "meow-artifact://…" }` and returns a put
response after durable storage succeeds. Promotion is idempotent, preserves the
URI, and leaves the temporary object readable if writing fails. MeowCore's
existing thirty-day durable retention policy still applies. Expired or missing
objects cannot be promoted. Persist before saving a reference in long-lived
history or scheduling work beyond its expiration.

`/artifact/delete` takes the same request and returns `{}`. Deleting missing
content succeeds. Deletion affects all metadata aliases of that content in the
same scope; an opened snapshot may finish, while subsequent opens fail. Do not
delete after each LLM call when another caller may reuse the URI. There is no
separate eviction API. A signed URL is not a revocable object identity: recreating
the identical content may make an unexpired old URL usable again.

App Facade equivalents are `/app/artifact/persist` and `/app/artifact/delete`;
Wasm hosts expose `artifact_persist_async` and `artifact_delete_async`. LLM inputs
continue to use canonical Artifact references. Temporary HTTP uploads and native
reads transfer content from memory without staging files. Large native transfers use bounded attachments on the existing
IPC connection; the JSON control frame limit is unchanged. Temporary delivery uses
`no-store`, and resolve grants never outlive the reported object expiration.

Temporary state is process-owned and is lost on restart. Startup removes
abandoned spill directories left by older runtimes. This contract adds no database
or migration. Cloud forwarding preserves the local owner’s lifetime metadata and
routes persist/delete to that owner. Cloud-owned storage rejects temporary writes;
it supports idempotent persist for existing durable content and exact-key delete.
A multi-instance cloud service must not emulate temporary storage with an
uncoordinated in-process map.
