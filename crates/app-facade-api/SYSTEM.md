# Space runtime observations

- `/app/system/instances/get` returns `mhome.system.instances.v1`: the selected
  Space's persisted Hub and active Node bindings, including disconnected instances.
- `/app/system/clients/get` returns the selected Space's client sessions.
- Both take empty input, require Space membership, and are executed by the Space
  Hub with cloud relay permitted. Host discovery does not filter Space instances.

Machine inventory, installation, updates, restart, metrics and OS permissions are
native Client responsibilities. They have no App Facade route and require no Space.
See core-api `contract/host-management-v1.json`. A UI may join Client Host observations
with Space instances for a topology view; this does not make Host management scoped.
