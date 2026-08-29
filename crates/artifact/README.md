# mhome-artifact-api

Storage-independent artifact references shared by mHome runtimes.

The crate is the source of truth for the `meow-artifact://v1/` URI format and its
immutable image, audio, and file metadata. The published schema and conformance
fixtures are intended for non-Rust runtimes.

TTL, storage placement, filesystem paths, signed URLs, and cloud object keys are
runtime concerns and are deliberately not encoded into an artifact reference.
