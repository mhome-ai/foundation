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
    "/app/topology/",
];

const REQUEST_PLACEMENT_PREFIXES: &[&str] = &[
    "/app/messaging/",
];

/// Hub management operations for which Lion provides a transport relay.
pub const CLOUD_RELAY_TARGETS: &[&str] = &[
    "/app/plugin/candidate/list",
    "/app/plugin/detail/get",
    "/app/plugin/installed/list",
    "/app/object-storage/overview",
    "/app/object-storage/folders/create",
    "/app/object-storage/folders/get",
    "/app/object-storage/folders/update",
    "/app/object-storage/folders/list",
    "/app/object-storage/settings/update",
    "/app/topology/get",
];
