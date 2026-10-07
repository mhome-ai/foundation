//! GATT endpoint names and the characteristic UUIDs a device without `0x2901`
//! user descriptions uses.
use app_facade_api::host_provision::HOST_ENDPOINT_UUIDS;
use app_facade_api::pod::{
    DeviceKind, BLE_SERVICE_UUID, POD_ENDPOINT_UUIDS, STANDARD_ENDPOINT_UUIDS,
};

/// The service UUID with bytes 2..4 replaced by `short`, lowercase.
pub fn endpoint_uuid(short: u16) -> String {
    let mut uuid = BLE_SERVICE_UUID.to_ascii_lowercase();
    uuid.replace_range(4..8, &format!("{short:04x}"));
    uuid
}

/// Endpoint name and fallback characteristic UUID for every endpoint of `kind`.
pub fn fallback_endpoint_uuids(kind: DeviceKind) -> Vec<(&'static str, String)> {
    let custom = match kind {
        DeviceKind::Pod => POD_ENDPOINT_UUIDS,
        DeviceKind::Host => HOST_ENDPOINT_UUIDS,
    };
    STANDARD_ENDPOINT_UUIDS
        .iter()
        .chain(custom)
        .map(|(name, short)| (*name, endpoint_uuid(*short)))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fallback_uuids_follow_the_esp_layout() {
        assert_eq!(
            endpoint_uuid(0xFF51),
            "a17aff51-9b2f-4010-9cf0-aa15ac54c40e"
        );
        let pod = fallback_endpoint_uuids(DeviceKind::Pod);
        assert!(pod
            .iter()
            .any(|(name, uuid)| *name == "pod-credential" && uuid.starts_with("a17aff55")));
        let host = fallback_endpoint_uuids(DeviceKind::Host);
        assert!(host.iter().any(|(name, _)| *name == "host-info"));
        assert!(!host.iter().any(|(name, _)| *name == "pod-credential"));
        assert!(host.iter().any(|(name, _)| *name == "prov-session"));
    }
}
