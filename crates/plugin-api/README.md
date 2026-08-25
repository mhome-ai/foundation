# mhome-plugin-api

Typed, transport-neutral wire contracts for controlling mHome Plugins.

The crate owns the common control/runtime envelopes, the reusable settings protocol, and the
public request contracts for built-in Plugin types. It deliberately contains no transport,
dispatch, authorization, persistence, runtime lifecycle, or Plugin implementation logic.

Cargo package versions and wire contract versions are independent. Each built-in Plugin contract
declares its own wire version so that one Plugin can evolve without forcing unrelated Plugins to
change version.
