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

## Stable recipients (core-api 1.8.0)

`RecipientId` provides canonical Messaging (`m:p:` / `m:g:`) and Node (`n:`)
addresses. See [the contract](contract/recipient-id-v1.md) and the packaged
cross-language [conformance vectors](fixtures/recipient-id.conformance.json).
This is additive; the external Host transport protocol remains version 14.
Consumers must explicitly migrate their identities after this release is available.

## Host delivery (core-api 1.7.1)

Core resolves logical recipients; the Host executes a concrete App connection,
Node connection or Messaging destination. Delivery responses distinguish queue or
provider acceptance, intentional skips, definite failure and unknown completion. Acceptance is not
a user-read receipt. Connection close has a separate typed control request.

External protocol 14 replaces effect notifications with request/completion events.
ServiceCoreOutput contains only its response and rejects the old effects field.
Messaging returns the same typed outcome, not a delivered boolean. Skipped means
handled without a send and must not be reported as provider acceptance.
There is no old-protocol compatibility path. Consumers must upgrade together;
version 1.7.1 must be published before updating registry-backed consumer locks.
