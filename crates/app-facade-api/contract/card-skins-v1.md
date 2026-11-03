# Card skins v1

These direct App Facade operations manage MeowLink Dashboard skins. They do not
change device state or expose the application SDK to a skin. `control` remains
the standard Facade envelope; `input` must match the named definition in
`schema/card-skins.v1.schema.json`. Responses are JSON. Tenant, Space and caller
identity come exclusively from authenticated ingress, never from domain input.

## Catalog and publication

`/app/ui/customize/cards/catalog/*` always executes in Cloud. Catalog reads require a
User principal. Package creation reserves `packageId` in that tenant. Only the
stored author may edit its display metadata or publish versions. Authorship is
immutable. Creation requires `baseUi` (for example `default` or `default-pod`).
The package keeps this base across all versions; prepare and complete reject a
manifest with another base. Package lookup is by known package ID; this is not a public marketplace
or a searchable cross-tenant catalog.

| Target suffix | Input definition | Output |
| --- | --- | --- |
| `catalog/package/get` | `packageGet` | `{revision, value: {packageId, baseUi, authorUserId, name, description, createdAt}}` |
| `catalog/package/create` | `packageCreate` | `{}` |
| `catalog/package/update` | `packageUpdate` | `{}` |
| `catalog/release/get` | `releaseGet` | `{id, baseUrl, manifest}` |
| `catalog/release/list` | `packageGet` | `{releases: [...]}`; ordering is unspecified |
| `catalog/publish/prepare` | `publishPrepare` | `{uploadId, expiresAt, files: [{path, url, headers}]}` |
| `catalog/publish/complete` | `publishComplete` | The immutable release |

Times are Unix milliseconds. Prepare validates the complete manifest against
`schema/card-skin-package.v1.schema.json`, its config schemas, paths, file types,
unique renderer IDs/patterns and inventory. A package has one nonempty bundled
`.mjs` entry, at most 128 files and 128 renderers, at most 8 MiB per file and 32 MiB
in total. Metadata documents are limited to 256 KiB UTF-8 JSON. The manifest lists
only supported asset extensions with their exact media type, size and SHA-256.
`manifest.json` is reserved. Config schemas are synchronous, self-contained JSON
Schema draft-07, with only fragment `$ref`s, no `$async`, and at most 64 levels.
Each config root must have `type: object`.

Each prepared session binds a manifest, authenticated author and immutable delivery directory.
It expires after 15 minutes. Each file's returned URL and exact headers authorize a direct
S3/OSS PUT to a private staging key, binding its size and forbidding overwrite. Upload grants
are temporary secrets and are not persisted in the publishing session. Staging is outside
CloudFront's readable origin prefix. No App HTTP endpoint relays upload or download bytes.

Complete checks ownership and expiry, then reads the actual staged bytes with the declared
size plus one byte as a hard bound. Size, SHA-256 and completeness must match the manifest;
uploader-supplied object metadata is not an integrity receipt. The exact verified bytes are
written with create-only semantics to a separate published prefix. Only after all files
succeed can a create-only release become visible. Concurrent sessions for one version have
one winner. Retrying that winning completion returns its exact release even after expiry.
A failed completion may leave unreferenced objects, never a partial installable release.

Published `baseUrl` is a stable HTTPS directory under the configured static origin, for
example `https://static.mhome.ai/ui/cards/<namespace>/<package>/<version>/<deliveryId>/`.
CloudFront reads only the `published/` storage prefix and supplies CORS, correct media types,
`nosniff` and immutable caching. Temporary uploads use `staging/`, which CDN readers cannot
access. There is no expiring download signature in a release. Possession of a delivery URL
permits reading its public executable bytes; catalog and upload authority remain separate.
Config values stay in the Space selection and secrets must not enter card packages.

Web loads compiled modules from the static origin. Native clients may resolve a release to
verified local package files through a host-only resource port. Such runtime resource URLs
are never persisted in Catalog or Space selection records and do not alter release identity.
Package caching does not install, select, upgrade, or grant additional device authority.
All platforms retain the renderer isolation requirement, including for cached files.

The publication service validates metadata and bytes, not JavaScript behavior.
The renderer contract remains `apiVersion: 1` during development and changes in
place. Hosts and skins use that current contract without a compatibility layer.
The authoring build verifies the bundled ESM structure; the browser host verifies
renderer exports against the manifest before using them. Each base owns its
presentation adapter and renderer host; common card state and actions do not
require a common rendering technology. Hosts must enforce their execution
boundary before enabling external code. A manifest or publication grants no
general App privileges, SDK access or native bridge access. Publication is not a
claim of runtime isolation, a malicious-code audit or browser SRI enforcement.

## Space selection

These operations execute on the authoritative owner of the Space: Cloud for
cloud Spaces, Core for local Spaces. Read requires Space read access; mutations
require Space write access. Agent/MCP surface exposure is separate and is not
introduced by this contract.

| Target suffix | Input definition | Output |
| --- | --- | --- |
| `selection/get` | `empty` | `{revision: string|null, selection: {installations, overrides, releases}}` |
| `install` | `install` | `{}` |
| `uninstall` | `uninstall` | `{}` |
| `upgrade` | `upgrade` | `{}` |
| `override/set` | `overrideSet` | `{}` |
| `override/reset` | `overrideReset` | `{}` |

Every mutation **requires** `expectedRevision`: null means the document has never
been created; otherwise it must equal the latest read token. A stale token returns
`PRECONDITION_FAIL` without retry or overwrite. An empty selection can have a
non-null revision. Mutation responses acknowledge the write; read the selection
again for its latest state/revision, which may include another writer's edit.

Install requires the target `baseUi`, a known package ID and exact version; the service resolves the
published release from the tenant catalog. Core uses its authenticated cloud
bridge for that read. Neither accepts client-provided release IDs, manifests or
URLs. The release base must equal the requested base. Installation records include
that base and must match their pinned release. Installation does not
automatically enable pattern overrides. Each package
has one installation per Space; installing another version uses `upgrade`.

Set override requires `baseUi` and selects an installation of that base with a renderer declaring that pattern and validates
the supplied config against its schema. Reset requires `baseUi` and removes only that base and pattern's override.
The unique selection key is `(Space, baseUi, patternId)`; the same pattern can
have independent overrides in different bases. Uninstall removes the installation, its pinned descriptor and every
override referencing it in one commit. Reset/uninstall/config edits use local
pinned metadata; they do not need a live catalog connection.

Upgrade keeps the installation ID, UI base and all existing overrides. A release
with another base is rejected before any mutation. Every affected
renderer must still exist, declare exactly the same pattern/contract ID/version,
and accept the existing config. A different catalog identity is never silently
substituted after Space transfer. Any incompatibility rejects the entire write;
there is no partial upgrade or implicit config conversion. The user can explicitly
reset incompatible overrides before switching versions. Successful upgrades
atomically replace the pinned release while preserving valid overrides.

The cloud/SQLite storage contract and `ui/cardSkins` backup category retain
complete pinned descriptors. They do not cache executable bytes for offline use.
An installed host still performs its own current-pattern compatibility check;
publication does not assume every MeowLink client supports the same contracts.

## Deployment and boundaries

The cloud service needs a private `ui.customize.cards.storage.bucket` on its configured
AWS/OSS provider and the platform-wide HTTPS origin `resources.static.base-url` (normally
`https://static.mhome.ai`, without a path). CloudFront maps its origin to `published/`;
its S3 access policy must exclude `staging/`. End users receive only bounded, expiring
create-only staging writes. They receive no published-object overwrite/delete authority.
This development cutover removes the old binary routes without a compatibility alias.
Existing development releases/selections containing old delivery URLs require explicit
recreation; startup and ordinary reads never rewrite or erase them.


Unfinished publishing sessions expire but their draft records and unreferenced
objects are currently retained. This version provides no automatic garbage
collector, package deletion or release deletion. Cleanup must never remove a
published directory. The App dashboard and Space management page consume these
endpoints through the existing SDK transport port. The page supports installation,
pattern selection, configuration, explicit version changes and uninstall. Visible
clients refresh selection on reconnect, foregrounding and every 30 seconds;
realtime notifications remain future work. The standalone versioned Card DevKit
provides project creation, metadata, typed builds, verification, real App preview
and publication. The MeowLink CLI and Agent/MCP tools expose catalog queries and
Space installation/override management; the external coding Agent Skill guides
authoring with the DevKit. Built-in Agent project editing and compilation require
a future local execution node. These source capabilities require their matching
published artifacts and deployed services. See the frontend's
[card guide](../../../../pallas-cat/docs/cards/README.md) for the authoring tools
and management flow.

The UI-base fields are mandatory. There is no legacy-base inference or automatic
startup conversion. Existing incompatible development documents require explicit
recreation; ordinary reads and startup must not rewrite them.
