# mhome-app-facade-api

Canonical public protocol shared by MeowLink clients, MeowCore, the CLI,
Agents, and cloud adapters.

Every `/app/*` request uses the strict `FacadeCall` envelope. Invocation
metadata lives in `control`; the domain request lives in `input`. The crate
owns public targets, request/response/event schemas, and stable operation
identity. Core-to-Node routes and provider runtime models belong to
`mhome-core-api` instead.

