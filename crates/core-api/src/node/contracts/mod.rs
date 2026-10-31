pub mod agent_gateway;
pub mod audiobridge;
pub mod llm;

/// Revisions are per Node type. Bump only the affected entry when its Core-facing
/// protocol breaks. Private UI/CLI APIs do not change these revisions.
/// Revision 3 requires explicit Full operation results on App and Integration calls.
/// Source ownership (first/third party) is separate from this protocol boundary.
pub fn first_party_core_protocol_version(node_type: &str) -> Option<u32> {
    match node_type {
        "camera" => Some(3),
        "matter" => Some(3),
        "llm" => Some(3),
        "storage" => Some(2),
        "audiobridge" => Some(3),
        "mac" => Some(3),
        _ => None,
    }
}

#[cfg(test)]
mod admission_tests {
    #[test]
    fn first_party_contracts_include_mac_but_do_not_assign_extension_revisions() {
        for node_type in ["camera", "matter", "llm", "audiobridge", "mac"] {
            assert_eq!(super::first_party_core_protocol_version(node_type), Some(3));
        }
        // Storage has no public App/Integration business operations; its backend is unchanged.
        assert_eq!(super::first_party_core_protocol_version("storage"), Some(2));
        assert_eq!(
            super::first_party_core_protocol_version("third-party"),
            None
        );
    }
}
