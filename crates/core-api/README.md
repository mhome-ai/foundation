# mhome-core-api

Canonical internal wire protocol for MeowCore, host services, Nodes, and
provider runtimes. Public `/app/*` contracts belong to
`mhome-app-facade-api`; this crate owns internal MWS, authentication, LLM,
Messaging normalization, Node runtime, and Storage contracts.

`interaction_flow` defines the serializable Core-to-Node flow definition and
handler protocol. Definitions name logical operations only; MeowCore chooses
and pins the Node instance and fixed transport routes for each session.

The Node owns the source session returned with a definition. Core closes that
session through the fixed close route after completion, cancellation, expiry,
or a failed start. Close is idempotent, and Nodes must also expire abandoned
source sessions so a Core restart cannot leak them. Execute operations are
idempotent within a source session: replaying the same `operationId` and
request returns the original result, while reusing the ID for a different
request is rejected.

Storage separates backing-filesystem capacity from Storage-owned logical
usage. Namespace is an internal protocol term; user-facing clients present it
as a Folder.
