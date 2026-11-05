# Space-owned Integration source reads

`extension_sources` defines the compiler's source-read callback, not publication,
installation authority, or an Integration business lifecycle. The caller supplies
tenant/Space through the existing authenticated Bridge envelope. Neither payload
accepts caller-controlled tenant/Space fields.

| Route | Request | Response |
| --- | --- | --- |
| `/dispatch/extension/sources/snapshot` | `{}` | `SpaceSources` |
| `/dispatch/extension/sources/read` | `SourceReadRequest` | `SourceReadResponse` |
| `/dispatch/extension/acquire` | `{integrations: string[]}` | `true` after preparation/activation |
| `/dispatch/extension/resources/plan` | `{capabilityId, role, source}` | `{invokePlan}` from the exact local cache |
| `/dispatch/extension/resources/guides` | `{keys: [{capabilityId, version?, snapshot?, type}]}` | `{guides: [{key, guide}]}` from the exact local cache |

Local Spaces own these records on their bound Hub. Cloud Spaces own published
selections in a separate Space-partitioned cloud table and cannot select a local
preview. Published package bodies remain in the release registry/object store.
The developer's desktop Host may differ from that Hub. Preview material is
transferred as content, not as a desktop-local pathname. Native Plugin installation
and its Space binding remain separate from this source-read protocol.

A source contains `integrationId`, `type` (`standard` or `composite`), and exactly
one content identity: a positive signed-32-bit `version` for a registry release,
or a lowercase SHA-256 `snapshot` for Preview. Preview has no numeric version and
cannot replace `base` or a composite package. Neither identity may substitute for
the other.

`SpaceSources` contains a safe-integer `revision` and up to 1,000 `integrations`,
with at most 256 KiB of encoded selection metadata on both storage platforms.
There is at most one published source and one Preview override per
`(integrationId, type)`. The shared resolver selects Preview first regardless of
list order. Updating a published source does not displace its override; exiting
Preview removes only that override and resolves the ordinary source again. Without
an ordinary source, the ID is unavailable. The complete selection is
replaced using its expected revision; a stale writer fails without partial edits.
Revision zero and an empty list represent a Space with no selections. Acquiring a
package and creating its business Connection remain separate operations.

The read request contains one exact `source`. The response echoes that exact
source, `schemas` (normalized relative YAML paths to UTF-8 text), and
`scriptDigests` (function name to lowercase SHA-256), and required `i18n`
(language tag to intro-translation YAML; an empty map is valid). English remains
in the schema. Language tags follow the existing intro-i18n grammar; path-like,
empty, or malformed tags and empty content are rejected. Relative paths may include
directories but reject empty/dot/parent segments, backslashes, colons and NULs.
Each inventory has at most 1,000 members; the encoded material is at most
16 MiB, below the existing general Bridge payload ceiling. Its normal bulk
transport handles large responses. No JavaScript body or credentials are sent
to the compiler by this callback.

Preparation stores immutable candidate material without changing availability.
Core stores compiler material as bounded, digest-checked files under its workdir's
`packages/extensions/preview/sources/`; database rows retain identities and hashes,
not duplicate Schema bodies. Files must be complete before recording references.
The compiler can read that exact candidate while preparation waits for validation.
Source and resource read handlers do not compile, call the cloud, or acquire Automation/lifecycle/cache
locks. They only read the selected Space's stored material. Missing or mismatched
material fails; there is no owner-catalog or latest-version fallback.

Removing a selection does not delete immutable material required by cleanup or
saved plans. Preview and published Integrations use the same disconnect/uninstall
business lifecycle; these read routes do not delete Connections or rewrite user
Automations. Resource retention and invocation admission belong to the resource
and lifecycle consumers, not this read protocol.

Preview compilation uses the exact candidate plus explicitly acquired dependency
materials. Its result must retain every source identity; numeric versions and
staging/prod channel names do not identify Preview bytes. Channels choose release
heads/configuration, and promotion does not change the immutable release identity.
The complete material set for one compilation is bounded to 16 MiB of encoded
material. Translation contents contribute to the material hash and byte bounds.

Provider configuration is not compiler material. Preview provider vars are supplied
explicitly to the owning Hub through a separate configuration operation, remain
Space-scoped, and are never part of this source callback or a publication artifact.
Omitted configuration on repeated Preview preserves existing values; explicit
updates/deletions are validated against the candidate schema. That configuration
consumer, compiled-result transport and runtime activation are subsequent work.

## Runtime preparation

`/bridge/schema/prepare` carries the Hub's current `SpaceSources`, requested package
IDs, and known model content hashes. Lion acquires latest published dependencies,
keeps Preview overrides, and prepares one exact graph without saving the selection.
The response includes the candidate selection, manifest, complete model identities,
changed models, published provider configuration and affected derived plans/guides.
A source or publication head changing during preparation fails the response.
Core stores immutable resources first, then commits selection and matching runtime
models/provider data under one local transaction. A stale selection revision rolls
back that transaction. Projection refresh remains the existing retryable operation.

Runtime `PlanSource` contains exactly one positive `version` or lowercase SHA-256
`snapshot`, plus `type` and `archived`. Plan cache keys use `v<version>` or
`p<snapshot>`; guide keys and frozen invocation references carry the same source.
Derived snapshots cover exact dependency material, environment and compiler/protocol
identity. Unaffected published packages retain their released keys. A missing Preview
plan/guide/script is an error, never a registry lookup or published fallback.

Acquisition is the mutating route above; it is distinct from source/resource read
callbacks and shares the caller's deadline. It finishes before installation or
connection reads its first runtime model. Catalog browsing does not acquire packages.
Catalog guide reads may supply `integrationIds` to `/bridge/schema/manifest`;
Lion returns the prospective package/dependency selection without activating it.
An empty list reads the current Space selection. Installed guide reads continue to
use committed local sources, including while offline. Public Preview commands and
configuration activation remain a later slice.
