pub mod agent_gateway;
pub mod audiobridge;
pub mod camera;
pub mod llm;

/// Revisions are per Node type. Bump only the affected entry when its Core-facing
/// protocol breaks. Private UI/CLI APIs do not change these revisions.
/// Source ownership (first/third party) is separate from this protocol boundary.
pub fn first_party_core_protocol_version(node_type: &str) -> Option<u32> {
    match node_type {
        "camera" => Some(1),
        "matter" => Some(1),
        "llm" => Some(1),
        "storage" => Some(1),
        "audiobridge" => Some(1),
        "mac" => Some(1),
        _ => None,
    }
}

#[cfg(test)]
mod admission_tests {
    #[test]
    fn first_party_contracts_include_mac_but_do_not_assign_extension_revisions() {
        for node_type in ["camera", "matter", "llm", "storage", "audiobridge", "mac"] {
            assert_eq!(super::first_party_core_protocol_version(node_type), Some(1));
        }
        assert_eq!(
            super::first_party_core_protocol_version("third-party"),
            None
        );
    }
}
