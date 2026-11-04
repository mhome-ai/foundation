# Space-owned Integration source reads

`extension_sources` defines the compiler's source-read callback, not publication,
installation authority, or an Integration business lifecycle. The caller supplies
tenant/Space through the existing authenticated Bridge envelope. Neither payload
accepts caller-controlled tenant/Space fields.

| Route | Request | Response |
| --- | --- | --- |
| `/dispatch/extension/sources/snapshot` | `{}` | `SpaceSources` |
| `/dispatch/extension/sources/read` | `SourceReadRequest` | `SourceReadResponse` |

Local Spaces own these records on their bound Hub. Cloud Spaces own published
selections in a separate Space-partitioned cloud table and cannot select a local
preview. Published package bodies remain in the release registry/object store.
The developer's desktop Host may differ from that Hub. Preview material is
transferred as content, not as a desktop-local pathname. Native Plugin installation
and its Space binding remain separate from this source-read protocol.

A source contains `integrationId`, a positive signed-32-bit `version`, and `type`
(`standard` or `composite`). A preview additionally contains a lowercase SHA-256
`snapshot`; it cannot replace `base` or a composite package. Registry releases have
no snapshot. Never use a numeric version alone to distinguish these sources.

`SpaceSources` contains a safe-integer `revision` and up to 1,000 `integrations`,
with at most 256 KiB of encoded selection metadata on both storage platforms.
There is at most one source per `(integrationId, type)`. The complete selection is
replaced using its expected revision; a stale writer fails without partial edits.
Revision zero and an empty list represent a Space with no selections. Acquiring a
package and creating its business Connection remain separate operations.

The read request contains one exact `source`. The response echoes that exact
source, `schemas` (normalized relative YAML paths to UTF-8 text), and
`scriptDigests` (function name to lowercase SHA-256). Relative paths may include
directories but reject empty/dot/parent segments, backslashes, colons and NULs.
Both inventories have at most 1,000 members; the encoded material is at most
16 MiB, below the existing general Bridge payload ceiling. Its normal bulk
transport handles large responses. No JavaScript body or credentials are sent
to the compiler by this callback.

Preparation stores immutable candidate material without changing availability.
Core stores compiler material as bounded, digest-checked files under its workdir's
`packages/extensions/preview/sources/`; database rows retain identities and hashes,
not duplicate Schema bodies. Files must be complete before recording references.
The compiler can read that exact candidate while preparation waits for validation.
Read handlers do not compile, call the cloud, or acquire Automation/lifecycle/cache
locks. They only read the selected Space's stored material. Missing or mismatched
material fails; there is no owner-catalog or latest-version fallback.

Removing a selection does not delete immutable material required by cleanup or
saved plans. Preview and published Integrations use the same disconnect/uninstall
business lifecycle; these read routes do not delete Connections or rewrite user
Automations. Resource retention and invocation admission belong to the resource
and lifecycle consumers, not this read protocol.
