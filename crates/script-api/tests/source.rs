use mhome_script_api::{ArtifactFormat, ScriptArtifact};
use sha2::{Digest, Sha256};

fn artifact(bytes: &[u8]) -> ScriptArtifact {
    ScriptArtifact {
        format: ArtifactFormat::JavascriptEsm,
        sha256: format!("{:x}", Sha256::digest(bytes)),
        size: bytes.len(),
        sdk_version: "0.2.0".into(),
        contract: None,
    }
}

#[test]
fn source_requires_exact_manifest_bytes_and_executable_text() {
    let bytes = b"export default () => 1;";
    let declared = artifact(bytes);
    assert_eq!(
        declared.verify_source(bytes).unwrap(),
        "export default () => 1;"
    );
    assert!(declared.verify_source(b"export default () => 2;").is_err());
    assert!(declared.verify_source(&bytes[..bytes.len() - 1]).is_err());
    for invalid in [&b""[..], &b"\xff"[..], &b"a\0b"[..]] {
        assert!(artifact(invalid).verify_source(invalid).is_err());
    }
}
