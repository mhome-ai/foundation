// Generated from manifest/routing.v1.json. DO NOT EDIT.

const TARGET_PREFIX: &str = "/app/";

const CLOUD_PREFIXES: &[&str] = &[
    "/app/scope/",
    "/app/inspire/",
    "/app/user/",
];

/// Exact Cloud-owned operations outside Cloud-owned domains.
pub const CLOUD_TARGETS: &[&str] = &[
    "/app/agent/context/get",
    "/app/agent/credit/usage",
    "/app/hub/cache/clear",
    "/app/hub/get",
    "/app/hub/remove",
    "/app/plugin/catalog/list",
    "/app/system/feedback/submit",
    "/app/timeline/entity/latest/list",
    "/app/host/access/issue",
    "/app/host/authorization",
];

const SCOPE_MODE_TARGETS: &[&str] = &[
    "/app/artifact/resolve",
    "/app/scope/context/get",
    "/app/artifact/put",
    "/app/artifact/import",
    "/app/artifact/delete",
    "/app/artifact/upload/start",
    "/app/artifact/upload/complete",
    "/app/artifact/upload/cancel",
    "/app/system/instances/get",
    "/app/system/clients/get",
];

const HUB_PREFIXES: &[&str] = &[
    "/app/plugin/",
    "/app/interaction-flow/",
    "/app/object-storage/",
    "/app/runtime/",
    "/app/system/hosts/",
    "/app/person/",
];

const REQUEST_PLACEMENT_PREFIXES: &[&str] = &[
    "/app/messaging/",
];

/// Exact Hub-owned operations outside Hub-owned domains.
pub const HUB_TARGETS: &[&str] = &[
    "/app/hub/restart",
    "/app/hub/status",
];
