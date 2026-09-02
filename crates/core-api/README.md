# mhome-core-api

Shared serializable wire types and service contracts used by MeowCore and its Rust clients and
hosts. This crate contains no transport, persistence, or runtime implementation. Storage Node
control-plane requests, object metadata, namespace state, session grants, repository statistics,
route names, and the Storage protocol version are defined here so Core and the Node compile against
one contract.
