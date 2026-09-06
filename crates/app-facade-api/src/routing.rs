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
const REQUEST_PLACEMENT: RoutePolicy = RoutePolicy {
    execution: ExecutionSelector::RequestPlacement,
    relay: RelayPolicy::DirectOnly,
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
        return Some(DIRECT_SCOPE);
    }
    if CLOUD_TARGETS.contains(&target) {
        return Some(DIRECT_CLOUD);
    }
    if CLOUD_RELAY_TARGETS.contains(&target) {
        return Some(RELAYABLE_HUB);
    }

    if REQUEST_PLACEMENT_PREFIXES
        .iter()
        .any(|prefix| target.starts_with(prefix))
    {
        return Some(REQUEST_PLACEMENT);
    }
    if HUB_PREFIXES.iter().any(|prefix| target.starts_with(prefix)) {
        return Some(DIRECT_HUB);
    }
    if CLOUD_PREFIXES
        .iter()
        .any(|prefix| target.starts_with(prefix))
    {
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
            Some(DIRECT_HUB)
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
