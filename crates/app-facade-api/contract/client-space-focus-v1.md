# Client Space focus

`/app/client/space/focus` is a Client-only, connection-owned App Facade. It replaces
`/focus`; no legacy alias exists. It is not a user preference or an Agent/MCP tool.
The receiving transport must prove the live authenticated Client connection and
must never forward this call to a Space owner or accept a client ID in input.

Request: `{"control":{"mode":"direct"},"input":{"targetSpaceId":"space-b"}}`.
`targetSpaceId` is required: non-empty unpadded text selects a Space, explicit
`null` clears focus. User membership is checked authoritatively by Cloud; a local
Host accepts only its authenticated session's Space (or clear). A bound Space
Client can only affect its enrolled Space. No read of the old Space is required.

Both request-response (`arq`) and one-way state synchronization (`ad`) use this
same envelope. Native switch orchestration retains its existing ACK and session
fences. Background synchronization remains per physical connection and is not
an additional user switch. It never replays an obsolete clear after reconnect.
For Cloud clear, the existing envelope `scopeId`, when non-empty, is a condition:
a delayed clear for A cannot erase a newer focus on B. Omitted/blank scope clears
unconditionally. Local connections can only focus/clear their bound Space.

Full success: `{"status":"success","data":{"focusedSpaceId":"space-b"}}`;
clear returns `focusedSpaceId:null`. `essential` uses normal Facade projection.
One-way synchronization emits no response. Persistence failure retires the
connection; success never means that every other connection has synchronized.
Internal Host/Core presence events and `/bridge/app/focus` retain their own roles.
