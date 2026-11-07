// Generated from manifest/routing.v1.json. DO NOT EDIT.

const TARGET_PREFIX: &str = "/app/";

const CLOUD_PREFIXES: &[&str] = &[
    "/app/space/",
    "/app/user/",
    "/app/ui/customize/cards/catalog/",
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
    "/app/messaging/link/code/create",
    "/app/messaging/link/list",
    "/app/messaging/unlink",
    "/app/messaging/code/status",
    "/app/messaging/binding/list",
    "/app/messaging/bind/code/create",
    "/app/messaging/unbind",
    "/app/pod/enrollment/get",
    "/app/host/directory/list",
    "/app/pod/rename",
    "/app/artifact/external/resolve",
    "/app/presentation/space/list",
    "/app/presentation/space/get",
    "/app/resource/transfer/prepare",
    "/app/nfc/sources/list",
    "/app/billing/summary/get",
    "/app/billing/payment/list",
    "/app/billing/credit/usage",
    "/app/billing/subscription/checkout/create",
    "/app/billing/subscription/cancel",
    "/app/billing/subscription/resume",
    "/app/billing/subscription/change",
];

const SCOPE_MODE_TARGETS: &[&str] = &[
    "/app/artifact/resolve",
    "/app/space/context/get",
    "/app/artifact/put",
    "/app/artifact/import",
    "/app/artifact/delete",
    "/app/artifact/upload/start",
    "/app/artifact/upload/complete",
    "/app/artifact/upload/cancel",
    "/app/system/instances/get",
    "/app/system/clients/get",
    "/app/pod/space/enrollment/get",
    "/app/pod/space/list",
    "/app/host/list",
    "/app/host/metrics",
    "/app/host/administrators/list",
    "/app/host/administrator/request",
    "/app/host/administrator/status",
    "/app/host/administrator/cancel",
    "/app/host/administrator/approve",
    "/app/host/administrator/reject",
    "/app/host/permissions/get",
    "/app/host/service/install",
    "/app/host/service/restart",
    "/app/pod/list",
    "/app/now/attention/list",
    "/app/space/location/list",
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

/// Operations owned by the receiving authenticated connection; never forward.
pub const CONNECTION_TARGETS: &[&str] = &[
    "/app/client/space/focus",
];

/// Exact Hub-owned operations outside Hub-owned domains.
pub const HUB_TARGETS: &[&str] = &[
    "/app/hub/restart",
    "/app/hub/status",
    "/app/integration/preview/apply",
    "/app/integration/preview/status",
];
