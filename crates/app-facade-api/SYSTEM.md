# Space runtime observations

- `/app/system/instances/get` returns `mhome.system.instances.v1`: the selected
  Space's persisted Hub and active Node bindings, including disconnected instances.
- `/app/system/clients/get` returns the selected Space's client sessions.
- `/app/system/hosts/get` returns `mhome.system.hosts.v1`: Hub-vantage Host
  discovery from the current Hub's host, plus signed `/info` for each reachable
  Host. The Hub holds a Client key for the current Space member and enrolls it
  on each Host.
- `/app/system/hosts/runtime` returns `mhome.system.hosts.runtime.v1`: the same
  Host inspect, metrics, plan, start, operation and restart actions as the
  native Client `host.runtime` contract. The Hub uses that member's Hub-held
  Client; the Host still authorizes the user.
- Inventory takes empty input. Runtime takes the native Host management request
  (`hostId` plus `action`). Both require Space membership and run on the Space
  Hub with cloud relay permitted. They are only available for a local Space with
  a connected Hub. Host discovery does not filter Space instances.

Native Client `host.list` / `host.runtime` remain for machines that are not on a
local Space Hub path. Retired `/app/system/host/*` targets stay removed.
