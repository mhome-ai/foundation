# mhome-artifact-api

Storage-independent artifact references shared by mHome runtimes.

The crate is the source of truth for the `meow-artifact://v1/` URI format and its
immutable image, audio, video, and file metadata. The published schema and conformance
fixtures are intended for non-Rust runtimes.

The crate also defines the transport-neutral `/artifact/resolve` request and response.
Delivery is an explicit `DATA_URL` or `SIGNED_URL` union; a signed URL may point to a
cloud object store or an authenticated local streaming endpoint.

TTL, storage placement, filesystem paths, signed URLs, and cloud object keys are
runtime concerns and are deliberately not encoded into an artifact reference.
