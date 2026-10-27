use app_facade_api::{
    hub::{HubState, LocalHubState},
    system::{Clients, Hosts, HostsRuntime, Instances},
};
use serde::de::DeserializeOwned;
use serde_json::{json, Value};

fn public_identity<T: DeserializeOwned>(value: Value) {
    assert!(serde_json::from_value::<T>(value.clone()).is_ok());
    let mut legacy = value;
    let id = legacy.as_object_mut().unwrap().remove("spaceId").unwrap();
    legacy["scopeId"] = id;
    assert!(serde_json::from_value::<T>(legacy).is_err());
}

#[test]
fn app_directories_require_public_space_identity() {
    public_identity::<HubState>(json!({"spaceId":"s","configured":false}));
    public_identity::<Clients>(
        json!({"contract":"mhome.system.clients.v1","spaceId":"s","observedAtMs":1,"sessions":[]}),
    );
    public_identity::<Instances>(
        json!({"contract":"mhome.system.instances.v1","spaceId":"s","observedAtMs":1,"instances":[]}),
    );
    public_identity::<Hosts>(
        json!({"contract":"mhome.system.hosts.v1","spaceId":"s","observedAtMs":1,"hosts":[]}),
    );
    public_identity::<HostsRuntime>(
        json!({"contract":"mhome.system.hosts.runtime.v1","spaceId":"s","hostId":"h","result":{}}),
    );
    // Native local contracts are a separate boundary.
    assert!(
        serde_json::from_value::<LocalHubState>(json!({"scopeId":"s","configured":false})).is_ok()
    );
    assert!(
        serde_json::from_value::<LocalHubState>(json!({"spaceId":"s","configured":false})).is_err()
    );
}
