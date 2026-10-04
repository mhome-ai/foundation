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
