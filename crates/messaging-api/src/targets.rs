pub const PROVIDER_LIST_TARGET: &str = "/messaging/provider/list";
pub const PROVIDER_ACCOUNT_LIST_TARGET: &str = "/messaging/provider-account/list";
pub const PROVIDER_ACCOUNT_UPDATE_TARGET: &str = "/messaging/provider-account/update";
pub const PROVIDER_ACCOUNT_DELETE_TARGET: &str = "/messaging/provider-account/delete";
pub const PROVIDER_ACCOUNT_TEST_TARGET: &str = "/messaging/provider-account/test";
pub const PROVIDER_ACCOUNT_STATUS_TARGET: &str = "/messaging/provider-account/status";
pub const SHARED_ACCOUNT_GRANT_CREATE_TARGET: &str = "/messaging/shared-account-grant/create";
pub const SHARED_ACCOUNT_GRANT_DELETE_TARGET: &str = "/messaging/shared-account-grant/delete";
pub const SHARED_ACCOUNT_GRANT_LIST_TARGET: &str = "/messaging/shared-account-grant/list";
pub const ROUTE_LIST_TARGET: &str = "/messaging/route/list";
pub const ROUTE_UPDATE_TARGET: &str = "/messaging/route/update";
pub const ROUTE_DELETE_TARGET: &str = "/messaging/route/delete";
pub const SETUP_OPTIONS_TARGET: &str = "/messaging/setup/options";
pub const SETUP_START_TARGET: &str = "/messaging/setup/start";
pub const SETUP_STATUS_TARGET: &str = "/messaging/setup/status";
pub const SURFACE_LIST_TARGET: &str = "/messaging/surface/list";
pub const SURFACE_DISMISS_TARGET: &str = "/messaging/surface/dismiss";
pub const SURFACE_BIND_CODE_CREATE_TARGET: &str = "/messaging/surface/bind-code/create";
pub const ACTOR_LINK_CODE_CREATE_TARGET: &str = "/messaging/actor/link-code/create";
pub const ACTOR_LINK_CLAIM_STATUS_TARGET: &str = "/messaging/actor/link-claim/status";
pub const ACTOR_LINK_CLAIM_CONFIRM_TARGET: &str = "/messaging/actor/link-claim/confirm";
pub const ACTOR_LINK_CLAIM_EVENT_TARGET: &str = "/messaging/actor/link-claim/event";
pub const ACTOR_LINK_LIST_TARGET: &str = "/messaging/actor/link/list";
pub const ACTOR_LINK_DELETE_TARGET: &str = "/messaging/actor/link/delete";

pub const MANAGEMENT_TARGETS: &[&str] = &[
    PROVIDER_LIST_TARGET,
    PROVIDER_ACCOUNT_LIST_TARGET,
    PROVIDER_ACCOUNT_UPDATE_TARGET,
    PROVIDER_ACCOUNT_DELETE_TARGET,
    PROVIDER_ACCOUNT_TEST_TARGET,
    PROVIDER_ACCOUNT_STATUS_TARGET,
    SHARED_ACCOUNT_GRANT_CREATE_TARGET,
    SHARED_ACCOUNT_GRANT_DELETE_TARGET,
    SHARED_ACCOUNT_GRANT_LIST_TARGET,
    ROUTE_LIST_TARGET,
    ROUTE_UPDATE_TARGET,
    ROUTE_DELETE_TARGET,
    SETUP_OPTIONS_TARGET,
    SETUP_START_TARGET,
    SETUP_STATUS_TARGET,
    SURFACE_LIST_TARGET,
    SURFACE_DISMISS_TARGET,
    SURFACE_BIND_CODE_CREATE_TARGET,
    ACTOR_LINK_CODE_CREATE_TARGET,
    ACTOR_LINK_CLAIM_STATUS_TARGET,
    ACTOR_LINK_CLAIM_CONFIRM_TARGET,
    ACTOR_LINK_LIST_TARGET,
    ACTOR_LINK_DELETE_TARGET,
];

pub const EVENT_TARGETS: &[&str] = &[ACTOR_LINK_CLAIM_EVENT_TARGET];

pub fn is_management_target(target: &str) -> bool {
    MANAGEMENT_TARGETS.contains(&target)
}
