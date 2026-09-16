use app_facade_api::system::*;
use serde_json::{json, Value};
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
    let mut info = Observation::default();
    info.success(serde_json::from_value::<HostInfo>(json!({"hostId":"h","hostName":"Home","hostType":"host","os":"linux","osVersion":null,"cpu":{"arch":"arm64","logicalCores":4,"model":"CPU"},"memory":{"totalBytes":1024},"disk":null,"gpus":[],"private":"stripped"})).unwrap(),10,9);
    let inventory = Inventory {
        contract: INVENTORY_CONTRACT.into(),
        scope_id: "s".into(),
        observed_at_ms: 10,
        discovery_error: None,
        hosts: vec![Host {
            host_id: "h".into(),
            host_name: "Home".into(),
            host_type: "host".into(),
            is_hub_host: true,
            source: "local".into(),
            reachable: true,
            info,
            services: Observation::default(),
        }],
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
        include_str!("../schema/system-inventory.v1.schema.json"),
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
    let mut sample = Observation::default();
    sample.success(
        HostMetrics {
            host_id: "h".into(),
            sampled_at_ms: 10,
            cpu_usage_percent: Some(1.5),
            memory_available_bytes: None,
            disk_available_bytes: None,
            network_received_bytes_per_second: None,
            network_sent_bytes_per_second: None,
            services: vec![],
        },
        10,
        9,
    );
    sample.failed(ObservationStatus::Unavailable, "timeout", 20);
    conforms(
        serde_json::to_value(Metrics {
            host_id: "h".into(),
            sample,
        })
        .unwrap(),
        include_str!("../schema/system-metrics.v1.schema.json"),
    );
}
