use app_facade_api::{device_topology::DeviceTopologySnapshot, topology::TopologySnapshot};
use serde::de::DeserializeOwned;
use serde_json::{json, Value};

fn assert_required_fields<T: DeserializeOwned>(schema: &str, payload: Value) {
    let schema: Value = serde_json::from_str(schema).unwrap();
    let validator = jsonschema::validator_for(&schema).unwrap();
    assert!(validator.is_valid(&payload));
    assert!(serde_json::from_value::<T>(payload.clone()).is_ok());
    for field in schema["required"].as_array().unwrap() {
        let field = field.as_str().unwrap();
        let mut missing = payload.clone();
        missing.as_object_mut().unwrap().remove(field);
        assert!(!validator.is_valid(&missing), "Schema must require {field}");
        assert!(
            serde_json::from_value::<T>(missing).is_err(),
            "Rust must also require {field}"
        );
    }
}

#[test]
fn system_snapshot_requires_every_schema_required_field() {
    assert_required_fields::<TopologySnapshot>(
        include_str!("../schema/topology.v1.schema.json"),
        json!({
            "contract": "mhome.space.topology.v1", "scopeId": "space-1",
            "generation": "generation-1", "revision": 1, "observedAtMs": 1,
            "entities": [], "edges": []
        }),
    );
}

#[test]
fn device_snapshot_requires_every_schema_required_field() {
    assert_required_fields::<DeviceTopologySnapshot>(
        include_str!("../schema/device-topology.v1.schema.json"),
        serde_json::from_str(include_str!("../fixtures/device-topology.snapshot.json")).unwrap(),
    );
}
