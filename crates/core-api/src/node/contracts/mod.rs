pub mod agent_gateway;
pub mod audiobridge;
pub mod camera;
pub mod llm;

/// Revisions are per Node type. Bump only the affected entry when its Core-facing
/// protocol breaks. Private UI/CLI APIs do not change these revisions.
/// Revision 2 requires declared App access levels and Core enforcement.
/// Source ownership (first/third party) is separate from this protocol boundary.
pub fn first_party_core_protocol_version(node_type: &str) -> Option<u32> {
    match node_type {
        "camera" => Some(2),
        "matter" => Some(2),
        "llm" => Some(2),
        "storage" => Some(2),
        "audiobridge" => Some(2),
        "mac" => Some(2),
        _ => None,
    }
}

#[cfg(test)]
mod admission_tests {
    #[test]
    fn first_party_contracts_include_mac_but_do_not_assign_extension_revisions() {
        for node_type in ["camera", "matter", "llm", "storage", "audiobridge", "mac"] {
            assert_eq!(super::first_party_core_protocol_version(node_type), Some(2));
        }
        assert_eq!(
            super::first_party_core_protocol_version("third-party"),
            None
        );
    }
}
