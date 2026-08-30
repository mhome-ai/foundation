use crate::ManagementOperation;

pub const PROVIDER_LIST_TARGET: &str = "/app/messaging/provider/list";
pub const PROVIDER_ACCOUNT_LIST_TARGET: &str = "/app/messaging/provider-account/list";
pub const PROVIDER_ACCOUNT_UPDATE_TARGET: &str = "/app/messaging/provider-account/update";
pub const PROVIDER_ACCOUNT_DELETE_TARGET: &str = "/app/messaging/provider-account/delete";
pub const PROVIDER_ACCOUNT_TEST_TARGET: &str = "/app/messaging/provider-account/test";
pub const PROVIDER_ACCOUNT_STATUS_TARGET: &str = "/app/messaging/provider-account/status";
pub const SHARED_ACCOUNT_GRANT_CREATE_TARGET: &str = "/app/messaging/shared-account-grant/create";
pub const SHARED_ACCOUNT_GRANT_DELETE_TARGET: &str = "/app/messaging/shared-account-grant/delete";
pub const SHARED_ACCOUNT_GRANT_LIST_TARGET: &str = "/app/messaging/shared-account-grant/list";
pub const ROUTE_LIST_TARGET: &str = "/app/messaging/route/list";
pub const ROUTE_UPDATE_TARGET: &str = "/app/messaging/route/update";
pub const ROUTE_DELETE_TARGET: &str = "/app/messaging/route/delete";
pub const SETUP_OPTIONS_TARGET: &str = "/app/messaging/setup/options";
pub const SETUP_START_TARGET: &str = "/app/messaging/setup/start";
pub const SETUP_STATUS_TARGET: &str = "/app/messaging/setup/status";
pub const SURFACE_LIST_TARGET: &str = "/app/messaging/surface/list";
pub const SURFACE_DISMISS_TARGET: &str = "/app/messaging/surface/dismiss";
pub const SURFACE_BIND_CODE_CREATE_TARGET: &str = "/app/messaging/surface/bind-code/create";
pub const ACTOR_LINK_CODE_CREATE_TARGET: &str = "/app/messaging/actor/link-code/create";
pub const ACTOR_LINK_CLAIM_STATUS_TARGET: &str = "/app/messaging/actor/link-claim/status";
pub const ACTOR_LINK_CLAIM_CONFIRM_TARGET: &str = "/app/messaging/actor/link-claim/confirm";
pub const ACTOR_LINK_CLAIM_EVENT_TARGET: &str = "/messaging/actor/link-claim/event";
pub const ACTOR_LINK_LIST_TARGET: &str = "/app/messaging/actor/link/list";
pub const ACTOR_LINK_DELETE_TARGET: &str = "/app/messaging/actor/link/delete";

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

/// Returns the provider capability required by a provider-scoped management target.
/// Provider discovery is intentionally not capability-gated because it selects no provider.
pub fn required_management_operation(target: &str) -> Option<ManagementOperation> {
    use ManagementOperation::*;
    Some(match target {
        PROVIDER_ACCOUNT_LIST_TARGET => ProviderAccountList,
        PROVIDER_ACCOUNT_UPDATE_TARGET => ProviderAccountUpdate,
        PROVIDER_ACCOUNT_DELETE_TARGET => ProviderAccountDelete,
        PROVIDER_ACCOUNT_TEST_TARGET => ProviderAccountTest,
        PROVIDER_ACCOUNT_STATUS_TARGET => ProviderAccountStatus,
        SHARED_ACCOUNT_GRANT_CREATE_TARGET => SharedAccountGrantCreate,
        SHARED_ACCOUNT_GRANT_DELETE_TARGET => SharedAccountGrantDelete,
        SHARED_ACCOUNT_GRANT_LIST_TARGET => SharedAccountGrantList,
        ROUTE_LIST_TARGET => RouteList,
        ROUTE_UPDATE_TARGET => RouteUpdate,
        ROUTE_DELETE_TARGET => RouteDelete,
        SETUP_OPTIONS_TARGET => SetupOptions,
        SETUP_START_TARGET => SetupStart,
        SETUP_STATUS_TARGET => SetupStatus,
        SURFACE_LIST_TARGET => SurfaceList,
        SURFACE_DISMISS_TARGET => SurfaceDismiss,
        SURFACE_BIND_CODE_CREATE_TARGET => SurfaceBindCodeCreate,
        ACTOR_LINK_CODE_CREATE_TARGET => ActorLinkCodeCreate,
        ACTOR_LINK_CLAIM_STATUS_TARGET => ActorLinkClaimStatus,
        ACTOR_LINK_CLAIM_CONFIRM_TARGET => ActorLinkClaimConfirm,
        ACTOR_LINK_LIST_TARGET => ActorLinkList,
        ACTOR_LINK_DELETE_TARGET => ActorLinkDelete,
        PROVIDER_LIST_TARGET | ACTOR_LINK_CLAIM_EVENT_TARGET => return None,
        _ => return None,
    })
}
