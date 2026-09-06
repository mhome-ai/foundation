//! Transport-neutral routing policy for public `/app/*` operations.
//!
//! This module describes where an App Facade operation is authoritative. It
//! deliberately does not describe sockets, connection fallback, authentication,
//! or Lion's legacy controller routing.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExecutionSelector {
    /// Execute against the cloud control plane.
    Cloud,
    /// Follow the active Space mode: cloud Space or its selected Hub.
    ScopeMode,
    /// Execute on the selected Space Hub, independent of Space mode.
    Hub,
    /// Execute on the local application host/runtime.
    Host,
    /// Read `input.placement` from the canonical [`crate::FacadeCall`].
    RequestPlacement,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RelayPolicy {
    /// The client must reach the selected executor directly.
    DirectOnly,
    /// A Hub-owned request may use the cloud connection as a transport relay.
    CloudRelayAllowed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RoutePolicy {
    pub execution: ExecutionSelector,
    pub relay: RelayPolicy,
}

const DIRECT_CLOUD: RoutePolicy = RoutePolicy {
    execution: ExecutionSelector::Cloud,
    relay: RelayPolicy::DirectOnly,
};
const DIRECT_SCOPE: RoutePolicy = RoutePolicy {
    execution: ExecutionSelector::ScopeMode,
    relay: RelayPolicy::DirectOnly,
};
const DIRECT_HUB: RoutePolicy = RoutePolicy {
    execution: ExecutionSelector::Hub,
    relay: RelayPolicy::DirectOnly,
};
const RELAYABLE_HUB: RoutePolicy = RoutePolicy {
    execution: ExecutionSelector::Hub,
    relay: RelayPolicy::CloudRelayAllowed,
};
const DIRECT_HOST: RoutePolicy = RoutePolicy {
    execution: ExecutionSelector::Host,
    relay: RelayPolicy::DirectOnly,
};
const REQUEST_PLACEMENT: RoutePolicy = RoutePolicy {
    execution: ExecutionSelector::RequestPlacement,
    relay: RelayPolicy::DirectOnly,
};

/// Exact Cloud-owned operations outside the Cloud-owned `/app/scope/*` and
/// `/app/inspire/*` domains.
pub const CLOUD_TARGETS: &[&str] = &[
    "/app/agent/context/get",
    "/app/credit/record",
    "/app/system/feedback/submit",
    "/app/timeline/entity/latest/list",
];

/// Hub management operations for which Lion provides a transport relay.
///
/// Data-plane operations, setup mutations, and plugin extensions are
/// intentionally absent: their credentials and endpoints must remain on the
/// direct Hub path.
pub const CLOUD_RELAY_TARGETS: &[&str] = &[
    crate::plugin::CANDIDATE_LIST_TARGET,
    crate::plugin::DETAIL_GET_TARGET,
    crate::plugin::INSTALLED_LIST_TARGET,
    "/app/object-storage/overview",
    "/app/object-storage/folders/create",
    "/app/object-storage/folders/get",
    "/app/object-storage/folders/update",
    "/app/object-storage/folders/list",
    "/app/object-storage/settings/update",
];

/// Resolve routing metadata for a canonical App Facade target.
///
/// Exact exceptions are evaluated before domain prefixes. Unknown `/app/*`
/// targets intentionally receive the normal Space-mode policy; protocol
/// support remains the receiving facade's responsibility.
pub fn route_policy_for_target(target: &str) -> Option<RoutePolicy> {
    if !target.starts_with("/app/") {
        return None;
    }

    // Every exact exception is resolved before any domain prefix.
    if target == "/app/scope/context/get" {
        return Some(DIRECT_SCOPE);
    }
    if CLOUD_TARGETS.contains(&target) {
        return Some(DIRECT_CLOUD);
    }
    if target == crate::runtime::STATUS_LIST_TARGET
        || target == crate::runtime::STATUS_CHANGED_TARGET
    {
        return Some(DIRECT_HOST);
    }
    if CLOUD_RELAY_TARGETS.contains(&target) {
        return Some(RELAYABLE_HUB);
    }

    if target.starts_with("/app/messaging/") {
        return Some(REQUEST_PLACEMENT);
    }
    if target.starts_with("/app/plugin/")
        || target.starts_with("/app/interaction-flow/")
        || target.starts_with("/app/object-storage/")
    {
        return Some(DIRECT_HUB);
    }
    if target.starts_with("/app/scope/") || target.starts_with("/app/inspire/") {
        return Some(DIRECT_CLOUD);
    }

    Some(DIRECT_SCOPE)
}

/// Read request-controlled placement from the strict App Facade envelope.
/// Domain payloads without the canonical `input` boundary are rejected.
pub fn request_placement(payload: &str) -> Option<crate::messaging::Placement> {
    let call: crate::FacadeCall = serde_json::from_str(payload).ok()?;
    serde_json::from_value(call.input.get("placement")?.clone()).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exact_exceptions_win_over_domain_defaults() {
        assert_eq!(
            route_policy_for_target("/app/scope/context/get"),
            Some(DIRECT_SCOPE)
        );
        assert_eq!(
            route_policy_for_target(crate::plugin::INSTALLED_LIST_TARGET),
            Some(RELAYABLE_HUB)
        );
        assert_eq!(
            route_policy_for_target(crate::plugin::ADD_START_TARGET),
            Some(DIRECT_HUB)
        );
    }

    #[test]
    fn domains_select_their_authoritative_executor() {
        assert_eq!(
            route_policy_for_target("/app/scope/member/list"),
            Some(DIRECT_CLOUD)
        );
        assert_eq!(
            route_policy_for_target("/app/messaging/provider/list"),
            Some(REQUEST_PLACEMENT)
        );
        assert_eq!(
            route_policy_for_target("/app/object-storage/session/acquire"),
            Some(DIRECT_HUB)
        );
        assert_eq!(
            route_policy_for_target("/app/runtime/status/list"),
            Some(DIRECT_HOST)
        );
        assert_eq!(
            route_policy_for_target("/app/device/list"),
            Some(DIRECT_SCOPE)
        );
    }

    #[test]
    fn legacy_targets_are_outside_the_contract() {
        assert_eq!(route_policy_for_target("/scope/get"), None);
        assert_eq!(route_policy_for_target("/chat"), None);
    }

    #[test]
    fn placement_is_read_only_from_the_canonical_input() {
        assert_eq!(
            request_placement(r#"{"control":{"mode":"direct"},"input":{"placement":"local"}}"#),
            Some(crate::messaging::Placement::Local)
        );
        assert_eq!(request_placement(r#"{"placement":"cloud"}"#), None);
        assert_eq!(
            request_placement(r#"{"control":{"mode":"direct"},"input":{}}"#),
            None
        );
    }
}
