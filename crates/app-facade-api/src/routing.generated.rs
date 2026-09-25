// Generated from manifest/routing.v1.json. DO NOT EDIT.

const TARGET_PREFIX: &str = "/app/";

const CLOUD_PREFIXES: &[&str] = &[
    "/app/scope/",
    "/app/inspire/",
];

/// Exact Cloud-owned operations outside Cloud-owned domains.
pub const CLOUD_TARGETS: &[&str] = &[
    "/app/agent/context/get",
    "/app/credit/record",
    "/app/hub/cache/clear",
    "/app/hub/get",
    "/app/hub/remove",
    "/app/plugin/catalog/list",
    "/app/system/feedback/submit",
    "/app/timeline/entity/latest/list",
];

const SCOPE_MODE_TARGETS: &[&str] = &[
    "/app/artifact/resolve",
    "/app/scope/context/get",
];

const HUB_PREFIXES: &[&str] = &[
    "/app/plugin/",
    "/app/interaction-flow/",
    "/app/object-storage/",
    "/app/runtime/",
    "/app/system/instances/",
    "/app/system/clients/",
    "/app/system/hosts/",
    "/app/person/",
];

const REQUEST_PLACEMENT_PREFIXES: &[&str] = &[
    "/app/messaging/",
];

/// Hub management operations for which Lion provides a transport relay.
pub const CLOUD_RELAY_TARGETS: &[&str] = &[
    "/app/plugin/candidate/list",
    "/app/plugin/detail/get",
    "/app/plugin/enabled/list",
    "/app/object-storage/overview",
    "/app/object-storage/folders/create",
    "/app/object-storage/folders/get",
    "/app/object-storage/folders/update",
    "/app/object-storage/folders/list",
    "/app/object-storage/settings/update",
    "/app/system/instances/get",
    "/app/system/clients/get",
    "/app/system/hosts/get",
    "/app/system/hosts/runtime",
    "/app/system/hosts/claim",
    "/app/person/status",
    "/app/person/list",
    "/app/person/get",
    "/app/person/reset",
    "/app/person/clusters/list",
    "/app/person/clusters/image",
    "/app/person/backup/export",
    "/app/person/backup/read",
    "/app/person/backup/close",
    "/app/person/backup/import/start",
    "/app/person/backup/import/write",
    "/app/person/backup/import/commit",
    "/app/person/clusters/name",
    "/app/person/clusters/delete",
];
