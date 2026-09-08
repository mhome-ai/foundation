# Topology bindings

Foundation owns the Camera provenance DTOs, empty request, target, and validation
in `crates/core-api/src/node/contracts/camera_topology.rs`. The Device Topology
JSON schema owns the graph DTOs; `device-topology-relations.v1.json` owns relation
endpoint kinds, basis and Integration identity constraints.

Run from Foundation, with the consumer worktrees you intend to update:

```sh
node scripts/generate-topology-bindings.mjs --meowcore ../meowcore-rust --baycat ../baycat --lion-service ../lion/module-service
node scripts/generate-topology-bindings.mjs --meowcore ../meowcore-rust --baycat ../baycat --lion-service ../lion/module-service --check
node --test scripts/generate-topology-bindings.test.mjs
```

Outputs are committed generated source, not independently maintained DTOs or
runtime path dependencies. Camera bindings include their canonical tests. Java
records support all six entity kinds, including Hub forwarding; their constructors
validate fields and graph integrity. Rust and Java run the same generated graph
mutation corpus. This allows reproducible consumer builds without requiring a
package release for each binding update. Never edit generated outputs by hand.

Run `cargo test -p mhome-app-facade-api -p mhome-core-api --locked` and the consumer
topology tests after regeneration. `--check` rejects missing or stale bindings.
