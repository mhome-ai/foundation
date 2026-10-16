use conversation_api::talk::*;
use serde::de::DeserializeOwned;
use serde_json::Value;

fn decode<T: DeserializeOwned>(body: Value) {
    serde_json::from_value::<T>(body).unwrap();
}

#[test]
fn talk_fixtures_match_schema_and_rust_without_extending_chat() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let schema: Value =
        serde_json::from_str(include_str!("../schema/pod-talk.v1.schema.json")).unwrap();
    let validator = jsonschema::validator_for(&schema).unwrap();
    let manifest: Value =
        serde_json::from_str(include_str!("../manifest/targets.v1.json")).unwrap();
    assert_eq!(
        manifest["podTalk"]["requestTargets"],
        serde_json::json!(REQUEST_TARGETS)
    );
    assert_eq!(
        manifest["podTalk"]["eventTargets"],
        serde_json::json!([CHANGED])
    );
    for file in std::fs::read_dir(root.join("fixtures/talk")).unwrap() {
        let file = file.unwrap();
        let value: Value =
            serde_json::from_str(&std::fs::read_to_string(file.path()).unwrap()).unwrap();
        let errors: Vec<_> = validator
            .iter_errors(&value)
            .map(|e| e.to_string())
            .collect();
        assert!(errors.is_empty(), "{}: {errors:?}", file.path().display());
        let target = value["target"].as_str().unwrap();
        assert!(!conversation_api::is_conversation_request_target(target));
        let body = value["body"].clone();
        assert!(encoded_payload_fits(
            &body,
            if target == CHANGED {
                CHANGED_BYTES
            } else {
                FRAME_BYTES
            }
        ));
        match (target, value["direction"].as_str().unwrap()) {
            (VIEW_GET, "request") => decode::<ViewGet>(body),
            (VIEW_GET, "response") => decode::<View>(body),
            (TURN_SUBMIT, "request") => decode::<TurnSubmit>(body),
            (TURN_SUBMIT, "response") => decode::<Receipt>(body),
            (REQUEST_GET, "request") => decode::<RequestGet>(body),
            (REQUEST_GET, "response") => decode::<RequestResult>(body),
            (REQUEST_CANCEL, "request") => decode::<RequestCancel>(body),
            (REQUEST_CANCEL, "response") => decode::<conversation_api::RequestCancelResponse>(body),
            (REPLY_READ, "request") => decode::<ReplyRead>(body),
            (REPLY_READ, "response") => decode::<Reply>(body),
            (INTERACTION_ANSWER, "request") => decode::<InteractionAnswer>(body),
            (INTERACTION_ANSWER, "response") => decode::<AnswerReceipt>(body),
            (AUDIO_PREPARE, "request") => decode::<AudioPrepare>(body),
            (AUDIO_PREPARE, "response") => decode::<UploadGrant>(body),
            (AUDIO_TRANSCRIBE, "request") => decode::<AudioTranscribe>(body),
            (AUDIO_TRANSCRIBE, "response") => decode::<Transcript>(body),
            (SPEECH_PREPARE, "request") => decode::<SpeechPrepare>(body),
            (SPEECH_PREPARE, "response") => decode::<Speech>(body),
            (MEDIA_RESOLVE, "request") => decode::<MediaResolve>(body),
            (MEDIA_RESOLVE, "response") => {
                let media: ResolvedMedia = serde_json::from_value(body).unwrap();
                media.descriptor.validate().unwrap();
            }
            (CHANGED, "event") => decode::<Changed>(body),
            _ => panic!("unexpected Talk fixture"),
        }
    }
}
