# Card skins v1

These direct App Facade operations manage MeowLink Dashboard skins. They do not
change device state or expose the application SDK to a skin. `control` remains
the standard Facade envelope; `input` must match the named definition in
`schema/card-skins.v1.schema.json`. Responses are JSON. Tenant, Space and caller
identity come exclusively from authenticated ingress, never from domain input.

## Catalog and publication

`/app/card-skins/catalog/*` always executes in Cloud. Catalog reads require a
User principal. Package creation reserves `packageId` in that tenant. Only the
stored author may edit its display metadata or publish versions. Authorship is
immutable. Package lookup is by known package ID; this is not a public marketplace
or a searchable cross-tenant catalog.

| Target suffix | Input definition | Output |
| --- | --- | --- |
| `catalog/package/get` | `packageGet` | `{revision, value: {packageId, authorUserId, name, description, createdAt}}` |
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

Each prepared session binds one manifest, author and immutable delivery directory.
It expires after 15 minutes. Upload each file with HTTP PUT to its returned URL
and the exact returned headers. Upload grants are bearer secrets; only their
SHA-256 digests are persisted. The server verifies the actual body before writing
it to a private bucket. S3 conditional create / OSS forbid-overwrite makes each
verified object immutable; clients never receive an object-store write URL.
A repeated identical PUT is safe. Different bytes, sizes or media types fail.

Complete checks authorship, session expiry and all server-verified immutable
object receipts before a create-only release write. Missing files never create a
release. Two sessions racing for the same version have one winner. Retrying the
winning completion returns that exact release, including after session expiry;
a different session cannot replace it. Unknown storage outcomes are errors, not
automatically replayed business operations. Reload the version to resolve them.
A newer version requires another prepare/upload/complete cycle.

Published `baseUrl` is a stable HTTPS directory. GET of a declared file checks the
published release and its random delivery-directory ID, verifies content integrity,
and serves the correct media type with CORS, `nosniff`, and immutable caching.
There is no expiring signed URL in a release. Unpublished files cannot be fetched
through delivery. A delivery URL is shareable: knowledge of it permits reading
that release's executable bytes, without granting catalog or upload permissions.
Do not put secrets in skins. Config values stay in the Space selection.

The publication service validates metadata and bytes, not JavaScript behavior.
The authoring build verifies the bundled ESM structure; the browser host verifies
renderer exports against the manifest before using them. Skins run in the host's
JavaScript realm and are not sandboxed by the UI interface. Publication is not a
claim of a malicious-code audit or cryptographic browser SRI enforcement.

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

Install takes a known package ID and exact version; the service resolves the
published release from the tenant catalog. Core uses its authenticated cloud
bridge for that read. Neither accepts client-provided release IDs, manifests or
URLs. Installation does not automatically enable pattern overrides. Each package
has one installation per Space; installing another version uses `upgrade`.

Set override selects an installed renderer declaring that pattern and validates
the supplied config against its schema. Reset removes just that pattern's
override. Uninstall removes the installation, its pinned descriptor and every
override referencing it in one commit. Reset/uninstall/config edits use local
pinned metadata; they do not need a live catalog connection.

Upgrade keeps the installation ID and all existing overrides. Every affected
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

The cloud service needs a private `card-skins.storage.bucket` on its configured
AWS/OSS provider and an HTTPS `card-skins.delivery.base-url` reaching the binary
controller. Object-store credentials must not grant end users overwrite/delete
access to skin objects. Existing release directories and URLs must be preserved
across deployment. A trusted reverse proxy/CDN may cache successful delivery;
upload and error responses are `no-store`.

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
