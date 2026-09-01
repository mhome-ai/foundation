# mhome-plugin-api

Typed, transport-neutral wire contracts for the mHome Plugin App Facade.

The crate owns the common Node-facing/runtime envelopes, the reusable settings protocol, and the
public request contracts for built-in Plugin types. It deliberately contains no transport,
dispatch, authorization, persistence, runtime lifecycle, or Plugin implementation logic.

The LLM v1 contract also owns the runtime target prefix, resumable custom-model import control
routes, snapshots, upload grant, chunk acknowledgement, and LAN HTTP upload path prefix.
File-system paths are intentionally absent from the Node-facing contract: a client-side service
reads the local artifact and transfers only its bytes and public metadata.

Cargo package versions and wire contract versions are independent. Each built-in Plugin contract
declares its own wire version so that one Plugin can evolve without forcing unrelated Plugins to
change version.

`contract/node-protocol-v1.json` is the language-neutral v1 manifest used by Rust and browser
adapters. Consumers vendor the exact manifest as a pinned build dependency and verify their
generated constants or serialized wire shapes in tests. During pre-release development, v1 is
replaced in lockstep across all consumers; after the protocol is released, incompatible changes
require a new manifest version. The backend-status entries describe the common transport envelope
consumed alongside the Plugin App Facade, but ownership of its Rust runtime model remains in Baycat
`service-api`.
