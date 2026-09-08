use app_facade_api::device_topology::DeviceTopologySnapshot;
use serde_json::Value;

#[test]
fn shared_graph_corpus_matches_rust_validation() {
    let corpus: Value = serde_json::from_str(include_str!(
        "../fixtures/device-topology.graph-conformance.json"
    ))
    .unwrap();
    for case in corpus["cases"].as_array().unwrap() {
        let mut snapshot = corpus["base"].clone();
        for mutation in case["mutations"].as_array().unwrap() {
            let path = mutation["path"].as_str().unwrap();
            if mutation["op"] == "append" {
                snapshot
                    .pointer_mut(path)
                    .unwrap()
                    .as_array_mut()
                    .unwrap()
                    .push(mutation["value"].clone());
            } else {
                let (parent, key) = path.rsplit_once('/').unwrap();
                let object = snapshot
                    .pointer_mut(parent)
                    .unwrap()
                    .as_object_mut()
                    .unwrap();
                if mutation["op"] == "remove" {
                    object.remove(key);
                } else {
                    object.insert(key.into(), mutation["value"].clone());
                }
            }
        }
        let valid = serde_json::from_value::<DeviceTopologySnapshot>(snapshot)
            .is_ok_and(|graph| graph.validate().is_ok());
        assert_eq!(valid, case["valid"].as_bool().unwrap(), "{}", case["name"]);
    }
}
