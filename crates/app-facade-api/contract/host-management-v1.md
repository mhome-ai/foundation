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

Metrics, administrator operations, permission reads, service installation and
service restart use the initiating personal desktop Client when it has the route,
or the current Space's Hub. No other online App is selected. MCP has no native
Client route. Mobile/web clients use the Hub route. A missing route fails; a mutation is not replayed through another
transport after dispatch. Host mode, ACL and Space ownership remain authoritative.
The private `/app/host/execute` Client request is a fixed management transport,
not a capability or public Facade. It accepts no service switch or admin removal.

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
