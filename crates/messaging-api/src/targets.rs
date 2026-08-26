pub const PROVIDER_LIST_TARGET: &str = "/messaging/provider/list";
pub const CONNECTION_LIST_TARGET: &str = "/messaging/connection/list";
pub const CONNECTION_UPDATE_TARGET: &str = "/messaging/connection/update";
pub const CONNECTION_DELETE_TARGET: &str = "/messaging/connection/delete";
pub const CONNECTION_TEST_TARGET: &str = "/messaging/connection/test";
pub const CONNECTION_STATUS_TARGET: &str = "/messaging/connection/status";
pub const ACCOUNT_BIND_TARGET: &str = "/messaging/account/bind";
pub const ACCOUNT_UNBIND_TARGET: &str = "/messaging/account/unbind";
pub const ACCOUNT_BINDING_LIST_TARGET: &str = "/messaging/account/binding/list";
pub const SETUP_OPTIONS_TARGET: &str = "/messaging/setup/options";
pub const SETUP_START_TARGET: &str = "/messaging/setup/start";
pub const SETUP_STATUS_TARGET: &str = "/messaging/setup/status";

pub const MANAGEMENT_TARGETS: &[&str] = &[
    PROVIDER_LIST_TARGET,
    CONNECTION_LIST_TARGET,
    CONNECTION_UPDATE_TARGET,
    CONNECTION_DELETE_TARGET,
    CONNECTION_TEST_TARGET,
    CONNECTION_STATUS_TARGET,
    ACCOUNT_BIND_TARGET,
    ACCOUNT_UNBIND_TARGET,
    ACCOUNT_BINDING_LIST_TARGET,
    SETUP_OPTIONS_TARGET,
    SETUP_START_TARGET,
    SETUP_STATUS_TARGET,
];

pub fn is_management_target(target: &str) -> bool {
    MANAGEMENT_TARGETS.contains(&target)
}
