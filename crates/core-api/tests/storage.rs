use core_api::{
    STORAGE_BACKEND_PROTOCOL_VERSION, STORAGE_GATEWAY_PREFIX, STORAGE_PROTOCOL_VERSION,
};

#[test]
fn storage_development_contracts_stay_on_v1() {
    assert_eq!(STORAGE_PROTOCOL_VERSION, "storage.v1");
    assert_eq!(STORAGE_BACKEND_PROTOCOL_VERSION, "storage.backend.v1");
    assert_eq!(STORAGE_GATEWAY_PREFIX, "/storage/v1/");
}
