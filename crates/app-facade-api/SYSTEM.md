# Space runtime observations

- `/app/system/instances/get` returns the selected Space's persisted Hub and
  active Node bindings, including disconnected instances.
- `/app/system/clients/get` returns the selected Space's Client sessions.
- `/app/system/hosts/get` discovers Hosts from the current Core's LAN. Its
  optional `authorizations` map carries cloud-issued User Host tokens and pins
  for authenticated information. Public signed challenges expose only claim
  status. Other-Space Hosts are hidden.
- `/app/system/hosts/runtime` forwards one Host action with its required
  `authorization`. Core checks the original User, target Host and Space routing
  binding. Host validates the cloud signature and its current policy. Core does
  not hold a Host identity, private key, refresh token or independent authority.

The Core proxy requires current Space read membership. Host tokens independently
control management; Space Clients are denied both Host observation and operations.
Read membership cannot create management authority. Native Clients prefer direct
LAN; Web and off-LAN User Clients may use the current Space's Core via cloud relay.

First claim is exclusively native Client to Host on LAN. There is no proxy claim
route. Minimal discovery remains available before claim. Unauthorized information
remains a visible denied observation, never permission to reclaim. Identity changes
require explicit confirmation after device reset.

`/app/host/access/issue` is a cloud-only User issuer. It validates the Host's signed
challenge and current administrator/Space-owner authorization, returning a
five-minute access token. Native direct management uses the corresponding REST
issuer. Neither issuer represents Core or a Space Client as a user.

`/app/host/authorization` is a cloud-only personal-Client coordinator for existing
administrator requests. `prepare` issues a ten-minute User applicant credential;
`commit` confirms a Host-signed approval receipt. The Host owns request status.
Applicant actions use `/v1/auth/administrators` and cannot perform management.
Listing requests/members, deciding requests and removing administrators require
`manageAdministrators` and a current administrator. Neither the cloud coordinator
nor the Core proxy can bypass Host consent or become an administrator.
