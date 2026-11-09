# Installed Plugin contribution reads

`/node/contributions/read` is a shared SDK control request on the already
authenticated Space-bound Node connection. It is not a Host-management endpoint
or an arbitrary filesystem API. `/app/plugin/contribution/read` requires current
Space read access and an enabled, connected exact `nodeId`/`nodeType`.

The signed Plugin definition may contain `contributions: {size, sha256}`. It pins
the exact bytes of `contributions.json` in that process's installed package. For
Preview the existing Host proof covers this definition; published definitions use
the existing release signature. Absence means no contribution reader is available.
An advertised Node contract digest alone is not source evidence.

The index is `{formatVersion: 1, pluginId, version, files}` where `files` maps exact
public contribution paths to `{size, sha256}`. Only declared contracts, UI builds,
Skills, assets and `extension-package.json` belong here. Native executables,
service configuration, databases and credentials never do. The index is limited
to 128 KiB/512 files and each resource to 1 MiB. Base64 plus its envelope fits the
existing 2 MiB Node message limit. Packagers reject oversize contributions.

Request: `{sourceManifestSha256, resource?}`. Omit `resource` to read the index;
otherwise name an exact indexed resource. The response is
`{sourceManifestSha256, resource, contentBase64}`. The App Facade adds `nodeId` and
`nodeType` to its input and wraps the verified response in `content`.
App callers use the ordinary Facade call envelope with `responseDetail: "full"`;
the successful Full result's `data` is `ReadPluginContributionOutput`.

The current reader serves Host-managed published packages and native Preview.
Firmware packages currently omit this index. Their contribution hosting requires
an embedded-resource producer/reader through the same Node transport before a
consumer can depend on it; no Host-file fallback is supplied for firmware.

Core fixes one current authenticated session for index and body reads, verifies
the admitted source digest, size and content digests, and rejects a changed
connection before returning. Contract reads also check Node type, contract
identity/version/digest, supported operation subset and its actual routes/access.
Index bytes are cached only for that session; the SDK caches its immutable index
at the actual executable's package root, never through the mutable current pointer.
Body reads verify the file every time. No persistent content cache, offline Host
fallback or new signing/permission system is introduced.

Reading a declaration does not authorize executing it or register a public tool.
Invocation must recheck current Space access, binding, source and operation policy.
These reads alone do not implement dynamic Agent/CLI/MCP or custom UI hosting.
