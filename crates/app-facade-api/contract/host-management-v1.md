# Host management

All operations require a personal User principal. They accept business input only;
Host authentication, identity pinning and transport selection remain internal.
The request schemas are in `schema/host-management.v1.schema.json` and the public
targets in the `hostManagement` manifest section.

`/app/host/list` returns registrations together with current observations and an
individual `detail` observation. Offline records remain. Each observation has
`status`, `data`, `stale` and an error on failure. Host metadata never grants
management access. Native list implementations use `/app/host/directory/list` as
the historical input, avoiding a request back into the public list composition.

Business execution follows Space mode, independently of LLM placement:

- Local Space: Core composes the list and sends Host commands through its existing
  authenticated LAN transport. Remote App/MCP requests reach that same Core via
  cloud forwarding. Core obtains cloud-owned directory metadata and user grants
  through the private `/bridge/management/control` endpoint; this endpoint neither
  dispatches Host commands nor impersonates a Client.
- Cloud Space: cloud executes business through the initiating personal desktop
  Client. No other online App is selected. MCP, web and mobile have no such
  transport and management calls fail if no suitable route exists. List still
  returns registrations with unavailable observations.

Host mode, ACL and Space ownership remain authoritative. The private bridge is
bound to its authenticated tenant/Space and rechecks personal User read access;
existing grant issuers check Host authorization. The fixed operations are Host
and personal Pod directory reads, Host access issuance, administrator application
preparation and signed receipt confirmation. It accepts no arbitrary target.
Public signing endpoints remain Client-only and are not Agent/MCP tools.

The private `/app/host/execute` Client request is a fixed management transport,
not a capability or public Facade. It accepts no service switch or admin removal.
A missing route fails; a mutation is never replayed through another transport.
If a cloud entrypoint cannot reach a local Space's Core, list alone may return
cloud registrations with unavailable observations and `sourceErrors.hub`.
Authorization errors and registration read failures remain errors.

Administrator request/approval credentials and signed receipts remain internal.
A successful approval receipt must be confirmed before `complete: true`; pending
requests do not grant access. Results exclude credentials, including nested ones.

Service install prepares exactly the selected service and submits its existing
Host operation. Restart uses a stable invocation-derived operation identity.
The returned Host operation state is `queued`, `running`, `succeeded` or `failed`;
acceptance is not completion. List details include current operations. There is
no public plan, start or operation-polling tool. The local CLI waits internally.

Permission reads never prompt. Permission requests belong to the target's local
UI/CLI. Native Agent UserAction opens the existing desktop permission UI, then
reads actual Host state on continuation. MCP cannot invoke that local interaction.
Service enable/disable is reserved for human UI/CLI. These management entrypoints
do not alter commissioning, claim proofs, Admin, SpaceSelf or SpaceDep semantics.
