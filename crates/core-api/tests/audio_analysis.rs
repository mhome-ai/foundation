use artifact_api::{ArtifactMetadata, ArtifactReference, MediaReference};
use core_api::node::contracts::audiobridge::*;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};

fn wav(samples: u32) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(44 + samples as usize * 2);
    bytes.extend_from_slice(b"RIFF");
    bytes.extend_from_slice(&(36 + samples * 2).to_le_bytes());
    bytes.extend_from_slice(b"WAVEfmt ");
    bytes.extend_from_slice(&16_u32.to_le_bytes());
    bytes.extend_from_slice(&1_u16.to_le_bytes());
    bytes.extend_from_slice(&1_u16.to_le_bytes());
    bytes.extend_from_slice(&16_000_u32.to_le_bytes());
    bytes.extend_from_slice(&32_000_u32.to_le_bytes());
    bytes.extend_from_slice(&2_u16.to_le_bytes());
    bytes.extend_from_slice(&16_u16.to_le_bytes());
    bytes.extend_from_slice(b"data");
    bytes.extend_from_slice(&(samples * 2).to_le_bytes());
    for index in 0..samples {
        bytes.extend_from_slice(&(index.wrapping_mul(137) as i16).to_le_bytes());
    }
    bytes
}

fn audio(metadata: ArtifactMetadata, digest: String) -> MediaReference {
    MediaReference {
        uri: ArtifactReference::new("tenant", "scope", digest, metadata)
            .unwrap()
            .uri()
            .unwrap()
            .parse()
            .unwrap(),
    }
}

fn request(bytes: &[u8]) -> AudioAnalysisRequest {
    AudioAnalysisRequest {
        audio: audio(
            ArtifactMetadata::audio(
                "audio/wav",
                bytes.len(),
                Some(((bytes.len() - 44) / 32) as u64),
            )
            .unwrap(),
            format!("{:x}", Sha256::digest(bytes)),
        ),
    }
}

fn model() -> AudioAnalysisModel {
    AudioAnalysisModel {
        id: "fixture model".into(),
        model_sha256: "a".repeat(64),
        labels_sha256: "b".repeat(64),
    }
}

fn analyzed(samples: u32) -> AudioAnalysisResponse {
    AudioAnalysisResponse::Analyzed {
        audio: request(&wav(samples)).audio,
        analysis_version: 1,
        model: model(),
        sample_rate_hz: 16_000,
        sample_count: samples,
        classes: vec![
            AudioAnalysisClass {
                index: 0,
                label: "Speech".into(),
            },
            AudioAnalysisClass {
                index: 1,
                label: "Unmapped label".into(),
            },
        ],
        windows: audio_analysis_coverage(samples)
            .unwrap()
            .into_iter()
            .map(|window| AudioAnalysisWindow {
                offset_samples: window.offset_samples,
                valid_samples: window.valid_samples,
                padding_samples: window.padding_samples,
                scores: vec![0.0, 1.0],
            })
            .collect(),
    }
}

#[test]
fn canonical_wav_verifies_real_bytes_hash_and_metadata_without_copying_pcm() {
    for samples in [4_800, 4_801, 64_000, 64_001, 480_000] {
        let bytes = wav(samples);
        let request = request(&bytes);
        assert_eq!(request.sample_count().unwrap(), samples);
        let pcm = request.validate_wav(&bytes).unwrap();
        assert_eq!(pcm.as_ptr(), bytes[44..].as_ptr());
        assert_eq!(pcm.len(), samples as usize * 2);
    }
    let bytes = wav(4_800);
    let original = request(&bytes);
    for offset in [0, 4, 8, 12, 16, 20, 22, 24, 28, 32, 34, 36, 40] {
        let mut changed = bytes.clone();
        changed[offset] ^= 1;
        // Recompute the reference hash: these fail the real header checks,
        // not just an unrelated content checksum comparison.
        assert!(
            matches!(
                request(&changed).validate_wav(&changed),
                Err(AudioAnalysisValidationError::InvalidAudio)
            ),
            "header offset {offset}"
        );
    }
    let mut changed = bytes.clone();
    changed[44] ^= 1;
    assert_eq!(
        original.validate_wav(&changed),
        Err(AudioAnalysisValidationError::InvalidAudio)
    );
    assert!(original.validate_wav(&bytes[..bytes.len() - 1]).is_err());
    let mut extra = bytes.clone();
    extra.extend_from_slice(b"JUNK\0\0\0\0");
    let riff_len = extra.len() as u32 - 8;
    extra[4..8].copy_from_slice(&riff_len.to_le_bytes());
    assert!(request(&extra).validate_wav(&extra).is_err());
}

#[test]
fn inadmissible_metadata_is_rejected_before_a_body_is_needed() {
    assert_eq!(
        request(&wav(4_799)).validate(),
        Err(AudioAnalysisValidationError::InsufficientAudio)
    );
    assert_eq!(
        request(&wav(480_001)).validate(),
        Err(AudioAnalysisValidationError::InputTooLarge)
    );
    for metadata in [
        ArtifactMetadata::audio("audio/ogg", 9_644, Some(300)).unwrap(),
        ArtifactMetadata::audio("audio/wav", 9_645, Some(300)).unwrap(),
        ArtifactMetadata::audio("audio/wav", 9_644, Some(301)).unwrap(),
        ArtifactMetadata::file("audio/wav", 9_644).unwrap(),
    ] {
        let request = AudioAnalysisRequest {
            audio: audio(metadata, "a".repeat(64)),
        };
        assert!(request.validate().is_err());
    }
    let bytes = wav(4_800);
    let no_hint = AudioAnalysisRequest {
        audio: audio(
            ArtifactMetadata::audio("audio/wav", bytes.len(), None).unwrap(),
            format!("{:x}", Sha256::digest(&bytes)),
        ),
    };
    assert!(no_hint.validate_wav(&bytes).is_ok());
}

#[test]
fn coverage_handles_exact_boundaries_and_never_counts_padding_as_capture() {
    for samples in [
        4_800, 63_999, 64_000, 64_001, 96_000, 96_001, 479_999, 480_000,
    ] {
        let coverage = audio_analysis_coverage(samples).unwrap();
        assert!(!coverage.is_empty() && coverage.len() <= 14);
        assert_eq!(coverage[0].offset_samples, 0);
        for (index, window) in coverage.iter().enumerate() {
            assert_eq!(window.offset_samples, index as u32 * 32_000);
            assert_eq!(window.valid_samples + window.padding_samples, 64_000);
            assert_eq!(
                window.valid_samples,
                (samples - window.offset_samples).min(64_000)
            );
        }
        let last = coverage.last().unwrap();
        assert_eq!(last.offset_samples + last.valid_samples, samples);
        if coverage.len() > 1 {
            let previous = &coverage[coverage.len() - 2];
            assert!(previous.offset_samples + previous.valid_samples < samples);
        }
        assert!(analyzed(samples).validate().is_ok());
    }
    assert_eq!(audio_analysis_coverage(480_000).unwrap().len(), 14);
    assert!(audio_analysis_coverage(u32::MAX).is_err());
    for change in 0..5 {
        let mut response = analyzed(96_001);
        if let AudioAnalysisResponse::Analyzed { windows, .. } = &mut response {
            match change {
                0 => {
                    windows.remove(1);
                }
                1 => windows[1].offset_samples += 1,
                2 => windows[2].valid_samples -= 1,
                3 => windows[2].padding_samples -= 1,
                _ => windows.push(windows[2].clone()),
            }
        }
        assert_eq!(
            response.validate(),
            Err(AudioAnalysisValidationError::InvalidCoverage)
        );
    }
}

#[test]
fn catalogs_and_scores_must_be_complete_ordered_finite_and_bounded() {
    for change in 0..8 {
        let mut response = analyzed(4_800);
        if let AudioAnalysisResponse::Analyzed {
            classes, windows, ..
        } = &mut response
        {
            match change {
                0 => classes[1].index = 0,
                1 => classes.reverse(),
                2 => classes[1].label = "声".repeat(86),
                3 => classes[1].label.clear(),
                4 => windows[0].scores.pop().map(|_| ()).unwrap(),
                5 => windows[0].scores.push(0.0),
                6 => windows[0].scores[0] = f32::NAN,
                _ => windows[0].scores[0] = f32::INFINITY,
            }
        }
        assert!(response.validate().is_err());
    }
    for score in [-0.01, 1.01, f32::NEG_INFINITY] {
        let mut response = analyzed(4_800);
        if let AudioAnalysisResponse::Analyzed { windows, .. } = &mut response {
            windows[0].scores[0] = score;
        }
        assert_eq!(
            response.validate(),
            Err(AudioAnalysisValidationError::IncompleteScores)
        );
    }
    for score in [1.00000001_f64, -1e-100] {
        let mut wire = serde_json::to_value(analyzed(4_800)).unwrap();
        wire["windows"][0]["scores"][0] = json!(score);
        assert!(serde_json::from_value::<AudioAnalysisResponse>(wire).is_err());
    }
}

#[test]
fn exact_audio_and_consumer_selected_model_identity_are_required() {
    let response = analyzed(4_800);
    let original = AudioAnalysisRequest {
        audio: response.audio().clone(),
    };
    assert!(response.validate_for_model(&original, &model()).is_ok());
    let different = request(&wav(4_801));
    assert_eq!(
        response.validate_for(&different),
        Err(AudioAnalysisValidationError::InputMismatch)
    );
    for field in 0..3 {
        let mut expected = model();
        match field {
            0 => expected.id.push('x'),
            1 => expected.model_sha256 = "c".repeat(64),
            _ => expected.labels_sha256 = "d".repeat(64),
        }
        assert!(expected.validate().is_ok()); // No product-specific pin here.
        assert_eq!(
            response.validate_for_model(&original, &expected),
            Err(AudioAnalysisValidationError::InvalidModel)
        );
    }
    for hash in ["A".repeat(64), "g".repeat(64), "a".repeat(63)] {
        let mut invalid = model();
        invalid.model_sha256 = hash;
        assert!(invalid.validate().is_err());
    }
}

#[test]
fn strict_tags_versions_unknown_fields_and_wire_budget_are_enforced() {
    let response = analyzed(4_800);
    let bytes = serde_json::to_vec(&response).unwrap();
    assert_eq!(
        AudioAnalysisResponse::from_json_slice(&bytes).unwrap(),
        response
    );
    let unknown =
        json!({"status":"unknown", "audio":response.audio(), "analysisVersion":1,"reason":"busy"});
    assert!(
        serde_json::from_value::<AudioAnalysisResponse>(unknown.clone())
            .unwrap()
            .validate()
            .is_ok()
    );
    for (key, value) in [
        ("analysisVersion", json!(2)),
        ("reason", json!("weakScores")),
        ("windows", json!([])),
        ("model", json!(model())),
        ("status", json!("success")),
    ] {
        let mut invalid = unknown.clone();
        invalid[key] = value;
        assert!(serde_json::from_value::<AudioAnalysisResponse>(invalid).is_err());
    }
    let duplicate = serde_json::to_string(&unknown).unwrap().replacen(
        "\"analysisVersion\":1",
        "\"analysisVersion\":1,\"analysisVersion\":1",
        1,
    );
    assert!(AudioAnalysisResponse::from_json_slice(duplicate.as_bytes()).is_err());
    let mut oversized = bytes.clone();
    oversized.resize(AUDIO_ANALYSIS_MAX_RESPONSE_BYTES as usize + 1, b' ');
    assert_eq!(
        AudioAnalysisResponse::from_json_slice(&oversized),
        Err(AudioAnalysisValidationError::ResponseTooLarge)
    );
    let mut escaped = analyzed(480_000);
    if let AudioAnalysisResponse::Analyzed {
        classes, windows, ..
    } = &mut escaped
    {
        *classes = (0..1024)
            .map(|index| AudioAnalysisClass {
                index,
                label: "\0".repeat(256),
            })
            .collect();
        for window in windows {
            window.scores = vec![0.5; 1024];
        }
    }
    assert_eq!(
        escaped.validate(),
        Err(AudioAnalysisValidationError::ResponseTooLarge)
    );
}

fn schema(name: &str) -> jsonschema::Validator {
    let mut schema: Value =
        serde_json::from_str(include_str!("../schema/audio-analysis.v1.schema.json")).unwrap();
    schema["$ref"] = json!(format!("#/$defs/{name}"));
    jsonschema::validator_for(&schema).unwrap()
}

#[test]
fn shared_corpus_preserves_wire_and_semantic_boundaries() {
    let corpus: Value =
        serde_json::from_str(include_str!("../fixtures/audio-analysis.conformance.json")).unwrap();
    for case in corpus["cases"].as_array().unwrap() {
        let kind = case["type"].as_str().unwrap();
        let value = case["value"].clone();
        assert_eq!(
            schema(kind).is_valid(&value),
            case["schemaValid"],
            "schema: {}",
            case["name"]
        );
        let valid = match kind {
            "AudioAnalysisCapabilitiesRequest" => {
                serde_json::from_value::<AudioAnalysisCapabilitiesRequest>(value).is_ok()
            }
            "AudioAnalysisRequest" => serde_json::from_value::<AudioAnalysisRequest>(value)
                .is_ok_and(|v| v.validate().is_ok()),
            "AudioAnalysisCapabilities" => {
                serde_json::from_value::<AudioAnalysisCapabilities>(value)
                    .is_ok_and(|v| v.validate().is_ok())
            }
            "AudioAnalysisResponse" => serde_json::from_value::<AudioAnalysisResponse>(value)
                .is_ok_and(|v| v.validate().is_ok()),
            _ => panic!("unrecognized fixture type"),
        };
        assert_eq!(valid, case["valid"], "Rust: {}", case["name"]);
    }
}

#[test]
fn manifest_schema_and_rust_constants_agree_without_a_protocol_bump() {
    let manifest: Value = serde_json::from_str(include_str!("../manifest/core.v1.json")).unwrap();
    let contract = &manifest["protocols"]["audioBridgePlugin"]["audioAnalysis"];
    assert_eq!(contract["version"], AUDIO_ANALYSIS_VERSION);
    assert_eq!(
        contract["capabilitiesTarget"],
        AUDIO_ANALYSIS_CAPABILITIES_TARGET
    );
    assert_eq!(contract["analyzeTarget"], AUDIO_ANALYZE_TARGET);
    assert_eq!(contract["schema"], "schema/audio-analysis.v1.schema.json");
    assert_eq!(
        contract["fixtures"],
        "fixtures/audio-analysis.conformance.json"
    );
    assert_eq!(contract["contract"], "contract/audio-analysis-v1.md");
    assert_eq!(
        core_api::node::contracts::first_party_core_protocol_version("audiobridge"),
        Some(3)
    );
    let capabilities = AudioAnalysisCapabilities::new(true, false, model());
    capabilities.validate().unwrap();
    let value = serde_json::to_value(&capabilities).unwrap();
    assert!(schema("AudioAnalysisCapabilities").is_valid(&value));
    let definitions: Value =
        serde_json::from_str(include_str!("../schema/audio-analysis.v1.schema.json")).unwrap();
    for field in ["format", "limits"] {
        let name = if field == "format" {
            "AudioAnalysisFormat"
        } else {
            "AudioAnalysisLimits"
        };
        for (key, value) in value[field].as_object().unwrap() {
            assert_eq!(
                &definitions["$defs"][name]["properties"][key]["const"],
                value
            );
        }
    }
    let mut invalid = capabilities;
    invalid.loaded = true;
    invalid.supported = false;
    assert!(invalid.validate().is_err());
}
