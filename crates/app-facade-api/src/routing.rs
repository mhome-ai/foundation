//! Transport-neutral routing policy for public `/app/*` operations.
//!
//! This module describes where an App Facade operation is authoritative. It
//! deliberately does not describe sockets, connection fallback, authentication,
//! or Lion's legacy controller routing. Route data is generated from
//! `manifest/routing.v1.json`, the language-neutral source of truth.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExecutionSelector {
    /// Execute against the cloud control plane.
    Cloud,
    /// Follow the active Space mode: cloud Space or its selected Hub.
    ScopeMode,
    /// Execute on the selected Space Hub, independent of Space mode.
    Hub,
    /// Read `input.placement` from the canonical [`crate::FacadeCall`].
    RequestPlacement,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RoutePolicy {
    pub execution: ExecutionSelector,
}

const CLOUD: RoutePolicy = RoutePolicy {
    execution: ExecutionSelector::Cloud,
};
const SCOPE: RoutePolicy = RoutePolicy {
    execution: ExecutionSelector::ScopeMode,
};
const HUB: RoutePolicy = RoutePolicy {
    execution: ExecutionSelector::Hub,
};
const REQUEST_PLACEMENT: RoutePolicy = RoutePolicy {
    execution: ExecutionSelector::RequestPlacement,
};

include!("routing.generated.rs");

/// Resolve routing metadata for a canonical App Facade target.
///
/// Exact exceptions are evaluated before domain prefixes. Unknown `/app/*`
/// targets intentionally receive the normal Space-mode policy; protocol
/// support remains the receiving facade's responsibility.
pub fn route_policy_for_target(target: &str) -> Option<RoutePolicy> {
    if !target.starts_with(TARGET_PREFIX) {
        return None;
    }

    // Every exact exception is resolved before any domain prefix.
    if SCOPE_MODE_TARGETS.contains(&target) {
        return Some(SCOPE);
    }
    if CLOUD_TARGETS.contains(&target) {
        return Some(CLOUD);
    }
    if HUB_TARGETS.contains(&target) {
        return Some(HUB);
    }

    if REQUEST_PLACEMENT_PREFIXES
        .iter()
        .any(|prefix| target.starts_with(prefix))
    {
        return Some(REQUEST_PLACEMENT);
    }
    if HUB_PREFIXES.iter().any(|prefix| target.starts_with(prefix)) {
        return Some(HUB);
    }
    if CLOUD_PREFIXES
        .iter()
        .any(|prefix| target.starts_with(prefix))
    {
        return Some(CLOUD);
    }

    Some(SCOPE)
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
            Some(SCOPE)
        );
        assert_eq!(
            route_policy_for_target(crate::plugin::ENABLED_LIST_TARGET),
            Some(HUB)
        );
        assert_eq!(
            route_policy_for_target(crate::system::INSTANCES_TARGET),
            Some(SCOPE)
        );
        assert_eq!(
            route_policy_for_target(crate::plugin::ENABLE_START_TARGET),
            Some(HUB)
        );
    }

    #[test]
    fn domains_select_their_authoritative_executor() {
        assert_eq!(
            route_policy_for_target("/app/scope/member/list"),
            Some(CLOUD)
        );
        assert_eq!(
            route_policy_for_target("/app/messaging/provider/list"),
            Some(REQUEST_PLACEMENT)
        );
        assert_eq!(
            route_policy_for_target("/app/object-storage/session/acquire"),
            Some(HUB)
        );
        assert_eq!(
            route_policy_for_target("/app/runtime/status/list"),
            Some(HUB)
        );
        assert_eq!(
            route_policy_for_target(crate::system::INSTANCES_TARGET),
            Some(SCOPE)
        );
        assert_eq!(
            route_policy_for_target(crate::system::HOSTS_TARGET),
            Some(HUB)
        );
        assert_eq!(
            route_policy_for_target(crate::system::HOSTS_RUNTIME_TARGET),
            Some(HUB)
        );
        assert_eq!(
            route_policy_for_target("/app/host/access/issue"),
            Some(CLOUD)
        );
        assert_eq!(route_policy_for_target("/app/device/list"), Some(SCOPE));
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
