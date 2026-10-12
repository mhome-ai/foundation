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

MeowCore retains temporary objects for ten minutes from commit or repeated put /
import revalidation. Ordinary reads do not renew expiration. Small objects (up to
1 MiB) use memory, subject to 64 MiB per runtime and 16 MiB per scope. New objects
over these limits use temporary disk, capped at 512 MiB per runtime and 128 MiB
per scope. Object limits are 4096 per runtime / 1024 per scope. Open snapshots
continue to count against capacity after deletion or expiry. Exhausted capacity
returns an availability error; unrelated live objects are not silently deleted.
Ingress is bounded separately. These are implementation defaults, not URI fields.

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
continue to use canonical Artifact references. Small temporary HTTP uploads and
native read descriptors carry bounded inline bytes without a staging file;
larger objects retain file-based streaming. Temporary delivery uses `no-store`,
and host resolve grants never outlive the reported object expiration.

Temporary state is process-owned and is lost on restart, including spilled
objects. The runtime removes abandoned spill directories on startup and expired
entries during maintenance. This contract adds no database or migration. A
cloud-owned artifact store needs its own implementation; a multi-instance cloud
service must not emulate this with an uncoordinated in-process map.
