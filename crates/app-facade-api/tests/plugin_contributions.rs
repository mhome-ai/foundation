use app_facade_api::{
    authorization::{self, OperationPolicy},
    plugin, routing,
};
use serde_json::json;

#[test]
fn contribution_reads_use_space_read_access_and_the_bound_hub_not_host_management() {
    assert_eq!(
        authorization::required(plugin::CONTRIBUTION_READ_TARGET),
        Some(OperationPolicy::SpaceRead)
    );
    assert_eq!(
        routing::route_policy_for_target(plugin::CONTRIBUTION_READ_TARGET)
            .unwrap()
            .execution,
        routing::ExecutionSelector::Hub
    );
    let input = json!({"nodeId":"garden-node","nodeType":"garden","sourceManifestSha256":"a".repeat(64),"resource":"skills/native.md"});
    let parsed: plugin::ReadPluginContributionInput =
        serde_json::from_value(input.clone()).unwrap();
    assert_eq!(parsed.node_id, "garden-node");
    assert_eq!(parsed.resource.as_deref(), Some("skills/native.md"));
    let mut wrong = input;
    wrong["hostAdmin"] = json!(true);
    assert!(serde_json::from_value::<plugin::ReadPluginContributionInput>(wrong).is_err());
}
