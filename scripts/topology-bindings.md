# Topology bindings

Foundation owns Camera and Matter Node source DTOs, empty requests, targets and
validation in `crates/core-api/src/node/contracts/camera_topology.rs` and
`crates/core-api/src/node/contracts/matter_device_sources.rs`.
Camera sources use `/camera/device/sources`; Matter sources use `/matter/device/sources`.
These are Node contracts. They are not the retired App Facade Device Topology graph.

Run from Foundation, with the Baycat worktree you intend to update:

```sh
node scripts/generate-topology-bindings.mjs --baycat ../baycat
node scripts/generate-topology-bindings.mjs --baycat ../baycat --check
node --test scripts/generate-topology-bindings.test.mjs
```

Outputs are committed generated source. Never edit generated outputs by hand.
