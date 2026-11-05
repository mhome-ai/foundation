# Integration Preview apply

This source contract implements standard Integration Preview on a Local Space Hub.
Native Plugin Preview and long-lived subscription refresh have separate owners.

App calls use the ordinary Facade envelope and Space authorization:

| Target | Input | Business output | Owner |
| --- | --- | --- | --- |
| `/app/integration/preview/apply` | `ApplyPreviewRequest` | `spaceId`, exact `source`, `activated: true` | Selected Local Hub; Space write |
| `/app/integration/preview/status` | `{}` | `spaceId`, active Preview `sources`, `refreshPending` | Selected Local Hub; Space read |
| `/app/resource/transfer/prepare` | `sizeBytes`, `sha256` | `reference`, presigned PUT `url`, required `headers` | Cloud; Space write |

`ApplyPreviewRequest.material` contains a `PreviewManifest` as UTF-8 JSON.
`scripts` maps function names to the separately transferred UTF-8 ESM bodies.
Each resource is `{"kind":"inline","text":"..."}` or
`{"kind":"object","reference":{...}}`. No client-selected download URL, filesystem
path, tenant or Hub identifier is accepted. The CLI sends bytes to the explicitly
selected Space; the desktop and Hub may be on different machines.

The manifest has `formatVersion: 1`, `integrationId`, `schemas` (relative filename
to YAML), `i18n` (language to YAML), and `scripts` (function to ScriptArtifact).
The Hub derives its content identity from canonical serialization, using sorted
maps. A Preview has no release version. Its exact script inventory and every size,
digest and runtime SDK version are checked before activation. Limits: 16 MiB
manifest, 100 scripts, 4 MiB per script, 1 MiB serialized apply input. Script bodies
do not share the manifest's 16 MiB budget.

Optional `configuration` is a separate JSON object, limited to 64 KiB/1000 keys.
Omission preserves Hub values; explicit null removes a key. Values are checked
against the candidate's declarations, and removed declarations are projected out.
Configuration is absent from source hashes, publication objects, compiler callbacks
and status responses. It is stored by Space/Integration only on the Hub and committed
with the source/models. Published configuration sync cannot replace it. Runtime-only
cloud evaluation can read the exact active Preview configuration through the
authenticated `/dispatch/integration/preview/env/read` callback (`integrationId`,
`snapshot` -> `values`); a changed source rejects the read. Cloud JS remains deferred.

Temporary transfers reuse the private transport bucket, not a release bucket.
References contain a random 32-hex `id`, `expiresAt` in Unix milliseconds,
`sizeBytes` and a SHA-256 digest. They expire after five minutes. Hub resolution uses
`/bridge/resource/transfer/read` in its authenticated tenant/Space; the cloud derives
the object key and returns a GET grant. Receivers reject changed references,
redirects, oversized/truncated bodies, invalid UTF-8 and digest mismatches.
Objects are transient; no upload-session database or Preview operation journal is added.

The ordinary 60-second request deadline covers Hub downloads, Lion preparation,
activation and request-awaited projection. Child calls inherit remaining time;
resource HTTP has an additional 30-second cap. Build/capture is a prior local step.
Preparation failure leaves active source and configuration unchanged. Immutable
candidate resources may remain unreferenced. Activation commits locally before
projection; a subsequent failure reports `details.activated: true` and can be retried
with the same content. Timeout may prevent that response, so query status if needed.
There is no automatic rollback, exit operation, history selector or published fallback.
