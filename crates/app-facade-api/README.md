# mhome-app-facade-api

Canonical public protocol shared by MeowLink clients, MeowCore, the CLI,
Agents, and cloud adapters.

Every `/app/*` request uses the strict `FacadeCall` envelope. Invocation
metadata lives in `control`; the domain request lives in `input`. The crate
owns public targets, request/response/event schemas, and stable operation
identity. Core-to-Node routes and provider runtime models belong to
`mhome-core-api` instead.

`interaction_flow` exposes only the materialized current-step view and session
commands. Handler operations, the complete step graph, and Node routing
identity remain internal to MeowCore.

Core owns defaults, resolver results, and read-only values. Clients may write
only editable inputs. Password plaintext is accepted as input but is never
projected back; `hasValue` lets a client render back navigation without
recovering the secret. Each current-step resolver projection reports whether
its complete referenced value is available and which visible inputs can make
it stale, so clients can resolve it reactively without learning handlers or
the hidden graph.

## Routing contract

`manifest/routing.v1.json` is the canonical client-side routing policy for the
new `/app/*` facade. Ordinary operations follow the active Space mode. Only
exceptional domains declare a fixed Cloud, Hub, or request-placement executor.
Relay permission is separate from execution authority so a client
cannot silently turn every Hub request into a cloud-proxied request.

The checked-in Rust table is generated from that manifest, and non-Rust clients
consume the same manifest from the npm protocol package. A native Host may
supervise Hub and Node processes, but it is not a public App Facade executor;
Host process management remains an implementation detail behind the selected
Hub. Lion's legacy controller annotations and non-`/app/*` protocols are
outside this contract.
