# mhome-core-api

Canonical internal wire protocol for MeowCore, host services, Nodes, and
provider runtimes. Public `/app/*` contracts belong to
`mhome-app-facade-api`; this crate owns internal MWS, authentication, LLM,
Messaging normalization, Node runtime, and Storage contracts.

`interaction_flow` defines the serializable Core-to-Node flow definition and
handler protocol. Definitions name logical operations only; MeowCore chooses
and pins the Node instance and fixed transport routes for each session.

Storage separates backing-filesystem capacity from Storage-owned logical
usage. Namespace is an internal protocol term; user-facing clients present it
as a Folder.
