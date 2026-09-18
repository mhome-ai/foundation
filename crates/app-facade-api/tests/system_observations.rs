use app_facade_api::system::*;
use core_api::host::{Capacity, Cpu, HostInfo};
use serde_json::Value;
fn conforms(value: Value, schema: &str) {
    let schema: Value = serde_json::from_str(schema).unwrap();
    let validator = jsonschema::validator_for(&schema).unwrap();
    assert!(
        validator.is_valid(&value),
        "{value}: {:?}",
        validator
            .iter_errors(&value)
            .map(|e| e.to_string())
            .collect::<Vec<_>>()
    );
}
#[test]
fn typed_system_responses_match_the_public_schemas() {
    let inventory = Instances {
        contract: INSTANCES_CONTRACT.into(),
        scope_id: "s".into(),
        observed_at_ms: 10,
        instances: vec![Instance {
            id: "n".into(),
            kind: "node".into(),
            host_id: "h".into(),
            service_id: "camera".into(),
            display_name: "Camera".into(),
            node_type: Some("camera".into()),
            hub_id: "hub".into(),
            commissioned: true,
            lifecycle: None,
            hub_connection: Some("disconnected".into()),
            cloud_connection: None,
            health: Observation::default(),
        }],
    };
    conforms(
        serde_json::to_value(inventory).unwrap(),
        include_str!("../schema/system-instances.v1.schema.json"),
    );
    conforms(
        serde_json::to_value(Clients {
            contract: CLIENTS_CONTRACT.into(),
            scope_id: "s".into(),
            observed_at_ms: 10,
            sessions: vec![ClientSession {
                app_client_id: "a".into(),
                session_id: "c".into(),
                source: "app".into(),
                device_type: "phone".into(),
                is_current: true,
                route_kind: None,
            }],
        })
        .unwrap(),
        include_str!("../schema/system-clients.v1.schema.json"),
    );
    let mut info = Observation::default();
    info.success(
        HostInfo {
            host_id: "h".into(),
            host_name: "Home".into(),
            host_type: "host".into(),
            os: "macos".into(),
            os_version: Some("15".into()),
            cpu: Cpu {
                arch: "arm64".into(),
                logical_cores: 8,
                model: "M".into(),
            },
            memory: Capacity { total_bytes: 8 },
            disk: None,
            gpus: vec![],
        },
        10,
        10,
    );
    conforms(
        serde_json::to_value(Hosts {
            contract: HOSTS_CONTRACT.into(),
            scope_id: "s".into(),
            observed_at_ms: 10,
            hosts: vec![Host {
                host_id: "h".into(),
                host_name: "Home".into(),
                host_type: "host".into(),
                source: "local".into(),
                reachable: true,
                info,
            }],
        })
        .unwrap(),
        include_str!("../schema/system-hosts.v1.schema.json"),
    );
    conforms(
        serde_json::to_value(HostsRuntime {
            contract: HOSTS_RUNTIME_CONTRACT.into(),
            scope_id: "s".into(),
            host_id: "h".into(),
            result: serde_json::json!({"hostId":"h"}),
        })
        .unwrap(),
        include_str!("../schema/system-hosts-runtime.v1.schema.json"),
    );
    conforms(
        serde_json::to_value(HostsClaim {
            contract: HOSTS_CLAIM_CONTRACT.into(),
            scope_id: "s".into(),
            host_id: "h".into(),
            claimed: true,
        })
        .unwrap(),
        include_str!("../schema/system-hosts-claim.v1.schema.json"),
    );
}
#[test]
fn host_management_uses_the_hosts_runtime_facade() {
    let routing = include_str!("../manifest/routing.v1.json");
    assert!(!routing.contains("/app/system/host/"));
    assert!(!routing.contains("/app/system/inventory/"));
    assert!(routing.contains(INSTANCES_TARGET));
    assert!(routing.contains(HOSTS_TARGET));
    assert!(routing.contains(HOSTS_RUNTIME_TARGET));
    assert!(routing.contains(HOSTS_CLAIM_TARGET));
}
