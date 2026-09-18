//! Pure invocation policy shared by deployments. No discovery or provider routing.
use std::collections::BTreeSet;

use artifact_api::{ArtifactKind, ArtifactReference};
use serde::{Deserialize, Serialize};

use crate::{ContentPart, Message, ModelConstraints};

/// Closed vocabulary for non-text model input. `text` is implied and must not be listed.
pub const INPUT_IMAGE: &str = "image";
/// Video input.
pub const INPUT_VIDEO: &str = "video";
/// Audio input.
pub const INPUT_AUDIO: &str = "audio";
/// Generic file input.
pub const INPUT_FILE: &str = "file";

/// Confirmed model capabilities. Missing information never establishes support.
#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct ModelCapabilities {
    /// Closed vocabulary: `image`, `video`, `audio`, `file`. `text` is implied.
    #[serde(default)]
    pub input: Vec<String>,
    pub tool_calling: bool,
    pub structured_output: bool,
}

impl ModelConstraints {
    /// Checks declarations only, including requirements on requests with no attachments/tools.
    pub fn validate(&self, capabilities: &ModelCapabilities) -> Result<(), &'static str> {
        let required = normalize_constraint_input(&self.input)?;
        let supported = normalize_capability_input(&capabilities.input)?;
        for token in &required {
            if !supported.iter().any(|item| item == token) {
                return Err(static_input_token(token));
            }
        }
        if self.tool_calling && !capabilities.tool_calling {
            return Err("tool_calling");
        }
        if self.structured_output && !capabilities.structured_output {
            return Err("structured_output");
        }
        Ok(())
    }

    /// Payload modalities must be a subset of the caller-declared `input` list.
    pub fn validate_payload<'a>(
        &self,
        messages: impl IntoIterator<Item = &'a Message>,
    ) -> Result<(), &'static str> {
        let declared = normalize_constraint_input(&self.input)?;
        for token in payload_input_modalities(messages)? {
            if !declared.iter().any(|item| item == &token) {
                return Err(static_input_token(&token));
            }
        }
        Ok(())
    }
}

/// Ordered from least to most thinking. Unknown tokens are not on this ladder.
pub const REASONING_EFFORT_LADDER: &[&str] = &[
    "none", "minimal", "low", "medium", "high", "xhigh", "max", "ultra",
];

/// Internal per-response output cap. Not a user setting. Adapters omit it when
/// the model snapshot says `maxTokens` is false.
pub const DEFAULT_MAX_OUTPUT_TOKENS: u32 = 32_768;

/// Deployment-owned generation settings, never supplied through Agent constraints.
/// None leaves the setting unspecified. These are desired preferences. Adapters
/// omit unsupported fields and clamp reasoning intensity onto the model's list.
/// Syntax validation does not establish model/provider support.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GenerationParameters {
    pub reasoning_effort: Option<String>,
    pub temperature: Option<f64>,
    pub thinking: Option<bool>,
    pub fast_mode: Option<bool>,
}

/// Maps a desired effort onto `supported`. Exact matches are kept. Otherwise the nearest
/// ladder neighbor is used; ties pick the lower effort. An empty supported list, or no
/// desired value, omits the field. Values not on the ladder are ignored.
pub fn clamp_reasoning_effort(
    desired: Option<&str>,
    supported: &[impl AsRef<str>],
) -> Option<String> {
    let desired = desired?;
    let supported: Vec<&str> = supported
        .iter()
        .map(AsRef::as_ref)
        .filter(|value| !value.is_empty())
        .collect();
    if supported.is_empty() {
        return None;
    }
    if supported.iter().any(|item| *item == desired) {
        return Some(desired.to_string());
    }
    let want = effort_rank(desired)?;
    let mut best: Option<(&str, usize, usize)> = None;
    for item in supported {
        let Some(rank) = effort_rank(item) else {
            continue;
        };
        let dist = rank.abs_diff(want);
        match best {
            None => best = Some((item, dist, rank)),
            Some((_, best_dist, best_rank))
                if dist < best_dist || (dist == best_dist && rank < best_rank) =>
            {
                best = Some((item, dist, rank));
            }
            _ => {}
        }
    }
    best.map(|(item, _, _)| item.to_string())
}

/// Middle intensity on the ladder among `supported`. Empty or unranked lists omit.
/// Even-length lists pick the lower-middle rank.
pub fn middle_reasoning_effort(supported: &[impl AsRef<str>]) -> Option<String> {
    let mut ranked: Vec<(usize, String)> = supported
        .iter()
        .filter_map(|value| {
            let effort = value.as_ref();
            if effort.is_empty() || effort == "none" {
                return None;
            }
            effort_rank(effort).map(|rank| (rank, effort.to_string()))
        })
        .collect();
    ranked.sort_by_key(|(rank, _)| *rank);
    ranked.dedup_by(|(left, _), (right, _)| left == right);
    if ranked.is_empty() {
        return None;
    }
    let index = (ranked.len() - 1) / 2;
    ranked.into_iter().nth(index).map(|(_, effort)| effort)
}

fn effort_rank(effort: &str) -> Option<usize> {
    REASONING_EFFORT_LADDER
        .iter()
        .position(|item| *item == effort)
}

impl GenerationParameters {
    pub fn validate(&self) -> Result<(), &'static str> {
        if let Some(effort) = self.reasoning_effort.as_deref() {
            if !matches!(
                effort,
                "none" | "minimal" | "low" | "medium" | "high" | "xhigh" | "max" | "ultra"
            ) {
                return Err("invalid reasoning_effort");
            }
        }
        if self
            .temperature
            .is_some_and(|value| !value.is_finite() || value < 0.0)
        {
            return Err("temperature must be finite and nonnegative");
        }
        Ok(())
    }
}

/// Maps one catalog/constraint token. `text` is dropped. `vision` and unknown values fail.
pub fn normalize_input_token(raw: &str) -> Result<Option<&'static str>, &'static str> {
    let token = raw.trim().to_ascii_lowercase();
    if token.is_empty() || token == "text" {
        return Ok(None);
    }
    match token.as_str() {
        INPUT_IMAGE => Ok(Some(INPUT_IMAGE)),
        INPUT_VIDEO => Ok(Some(INPUT_VIDEO)),
        INPUT_AUDIO => Ok(Some(INPUT_AUDIO)),
        INPUT_FILE => Ok(Some(INPUT_FILE)),
        "vision" => Err("vision"),
        _ => Err("unknown input modality"),
    }
}

/// Normalizes catalog capability tokens. `text` is ignored; unknown tokens fail.
pub fn normalize_capability_input<I, S>(raw: I) -> Result<Vec<String>, &'static str>
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    let mut set = BTreeSet::new();
    for item in raw {
        if let Some(token) = normalize_input_token(item.as_ref())? {
            set.insert(token.to_string());
        }
    }
    Ok(set.into_iter().collect())
}

/// Normalizes caller-declared constraint tokens. `text` is illegal here.
pub fn normalize_constraint_input<I, S>(raw: I) -> Result<Vec<String>, &'static str>
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    let mut set = BTreeSet::new();
    for item in raw {
        match normalize_input_token(item.as_ref())? {
            None => return Err("text"),
            Some(token) => {
                set.insert(token.to_string());
            }
        }
    }
    Ok(set.into_iter().collect())
}

/// Closed-vocabulary token for an artifact kind.
#[must_use]
pub const fn input_modality_for_kind(kind: ArtifactKind) -> &'static str {
    match kind {
        ArtifactKind::Image => INPUT_IMAGE,
        ArtifactKind::Video => INPUT_VIDEO,
        ArtifactKind::Audio => INPUT_AUDIO,
        ArtifactKind::File => INPUT_FILE,
    }
}

/// Closed-vocabulary token for a MIME type. Non-media types map to `file`.
#[must_use]
pub fn input_modality_for_mime(mime_type: &str) -> &'static str {
    let mime = mime_type.trim().to_ascii_lowercase();
    if mime.starts_with("image/") {
        INPUT_IMAGE
    } else if mime.starts_with("video/") {
        INPUT_VIDEO
    } else if mime.starts_with("audio/") {
        INPUT_AUDIO
    } else {
        INPUT_FILE
    }
}

/// Distinct payload modalities required by `Artifact` and `Image` parts.
pub fn payload_input_modalities<'a>(
    messages: impl IntoIterator<Item = &'a Message>,
) -> Result<Vec<String>, &'static str> {
    let mut set = BTreeSet::new();
    for message in messages {
        for part in &message.content {
            if let Some(token) = part_input_modality(part)? {
                set.insert(token.to_string());
            }
        }
    }
    Ok(set.into_iter().collect())
}

fn part_input_modality(part: &ContentPart) -> Result<Option<&'static str>, &'static str> {
    match part {
        ContentPart::Image { .. } => Ok(Some(INPUT_IMAGE)),
        ContentPart::Artifact { uri, mime_type } => {
            artifact_input_modality(uri, mime_type).map(Some)
        }
        _ => Ok(None),
    }
}

fn artifact_input_modality(uri: &str, mime_type: &str) -> Result<&'static str, &'static str> {
    let reference = ArtifactReference::parse(uri).map_err(|_| "invalid artifact reference")?;
    if reference.metadata().mime_type() != mime_type {
        return Err("artifact mime type does not match the message");
    }
    let from_kind = input_modality_for_kind(reference.metadata().kind());
    let from_mime = input_modality_for_mime(mime_type);
    if from_kind != from_mime {
        return Err("artifact kind does not match mime type");
    }
    Ok(from_kind)
}

fn static_input_token(token: &str) -> &'static str {
    match token {
        INPUT_IMAGE => INPUT_IMAGE,
        INPUT_VIDEO => INPUT_VIDEO,
        INPUT_AUDIO => INPUT_AUDIO,
        INPUT_FILE => INPUT_FILE,
        "text" => "text",
        "vision" => "vision",
        "unknown input modality" => "unknown input modality",
        _ => "undeclared input",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{MessageRole, ModelConstraints};
    use artifact_api::ArtifactMetadata;

    fn image_uri() -> (String, String) {
        let artifact = ArtifactReference::new(
            "tenant-1",
            "scope-1",
            "a".repeat(64),
            ArtifactMetadata::image("image/png", 100, 10, 10).expect("valid image metadata"),
        )
        .expect("valid image artifact");
        (artifact.uri().expect("uri"), "image/png".to_owned())
    }

    fn audio_uri() -> (String, String) {
        let artifact = ArtifactReference::new(
            "tenant-1",
            "scope-1",
            "b".repeat(64),
            ArtifactMetadata::audio("audio/mpeg", 100, Some(1_000)).expect("valid audio metadata"),
        )
        .expect("valid audio artifact");
        (artifact.uri().expect("uri"), "audio/mpeg".to_owned())
    }

    #[test]
    fn declarations_are_requirements_not_prohibitions() {
        let supported = ModelCapabilities {
            input: vec![INPUT_IMAGE.into()],
            tool_calling: true,
            structured_output: true,
        };
        assert!(ModelConstraints::default().validate(&supported).is_ok());
        assert!(ModelConstraints::default()
            .validate(&ModelCapabilities::default())
            .is_ok());
        for constraints in [
            ModelConstraints {
                input: vec![INPUT_IMAGE.into()],
                ..Default::default()
            },
            ModelConstraints {
                tool_calling: true,
                ..Default::default()
            },
            ModelConstraints {
                structured_output: true,
                ..Default::default()
            },
        ] {
            assert!(constraints.validate(&supported).is_ok());
            assert!(constraints.validate(&ModelCapabilities::default()).is_err());
        }
    }

    #[test]
    fn input_lists_use_closed_vocabulary_and_ignore_catalog_text() {
        assert_eq!(
            normalize_capability_input(["text", "IMAGE", "image"]).unwrap(),
            vec![INPUT_IMAGE.to_string()]
        );
        assert!(normalize_constraint_input(["text"]).is_err());
        assert!(normalize_input_token("vision").is_err());
        assert!(normalize_input_token("unknown").is_err());
        let constraints = ModelConstraints {
            input: vec![INPUT_VIDEO.into()],
            ..Default::default()
        };
        assert_eq!(
            constraints
                .validate(&ModelCapabilities {
                    input: vec![INPUT_IMAGE.into()],
                    ..Default::default()
                })
                .unwrap_err(),
            INPUT_VIDEO
        );
    }

    #[test]
    fn payload_must_be_declared_and_kind_must_match_mime() {
        let (image_uri, image_mime) = image_uri();
        let (audio_uri, audio_mime) = audio_uri();
        let image_message = Message {
            role: MessageRole::User,
            content: vec![ContentPart::Artifact {
                uri: image_uri,
                mime_type: image_mime,
            }],
            continuation: None,
        };
        let audio_message = Message {
            role: MessageRole::User,
            content: vec![ContentPart::Artifact {
                uri: audio_uri.clone(),
                mime_type: audio_mime,
            }],
            continuation: None,
        };
        assert_eq!(
            payload_input_modalities([&image_message]).unwrap(),
            vec![INPUT_IMAGE.to_string()]
        );
        let declared = ModelConstraints {
            input: vec![INPUT_IMAGE.into()],
            ..Default::default()
        };
        assert!(declared.validate_payload([&image_message]).is_ok());
        assert_eq!(
            declared.validate_payload([&audio_message]).unwrap_err(),
            INPUT_AUDIO
        );
        let mismatched = Message {
            role: MessageRole::User,
            content: vec![ContentPart::Artifact {
                uri: audio_uri,
                mime_type: "image/png".to_owned(),
            }],
            continuation: None,
        };
        assert!(payload_input_modalities([&mismatched]).is_err());
    }

    #[test]
    fn retired_generation_control_is_not_silently_ignored() {
        assert!(
            serde_json::from_value::<ModelConstraints>(serde_json::json!({
                "vision": false, "tool_calling": false, "structured_output": false
            }))
            .is_err()
        );
        assert!(serde_json::from_value::<ModelConstraints>(serde_json::json!({
            "input": ["image"], "tool_calling": false, "structured_output": false, "reasoning": "high"
        }))
        .is_err());
    }

    #[test]
    fn clamps_desired_effort_onto_supported_list() {
        let flash = ["max", "high", "low"];
        assert_eq!(
            clamp_reasoning_effort(Some("none"), &flash).as_deref(),
            Some("low")
        );
        assert_eq!(
            clamp_reasoning_effort(Some("low"), &flash).as_deref(),
            Some("low")
        );
        assert_eq!(
            clamp_reasoning_effort(Some("ultra"), &flash).as_deref(),
            Some("max")
        );
        assert_eq!(
            clamp_reasoning_effort(Some("medium"), &["low", "high"]).as_deref(),
            Some("low")
        );
        assert_eq!(
            clamp_reasoning_effort(Some("none"), &["high", "max"]).as_deref(),
            Some("high")
        );
        assert_eq!(clamp_reasoning_effort(Some("none"), &[] as &[&str]), None);
        assert_eq!(clamp_reasoning_effort(None, &flash), None);
    }

    #[test]
    fn middle_effort_picks_lower_middle_of_ranked_list() {
        assert_eq!(
            middle_reasoning_effort(&["low", "medium", "high", "xhigh", "max"]).as_deref(),
            Some("high")
        );
        assert_eq!(
            middle_reasoning_effort(&["low", "high", "max"]).as_deref(),
            Some("high")
        );
        assert_eq!(
            middle_reasoning_effort(&["none", "low", "medium", "high"]).as_deref(),
            Some("medium")
        );
        assert_eq!(middle_reasoning_effort(&[] as &[&str]), None);
    }

    #[test]
    fn validates_explicit_generation_settings() {
        for effort in ["none", "low", "high", "max", "ultra"] {
            assert!(GenerationParameters {
                reasoning_effort: Some(effort.into()),
                ..Default::default()
            }
            .validate()
            .is_ok());
        }
        assert!(GenerationParameters {
            reasoning_effort: Some("unknown".into()),
            ..Default::default()
        }
        .validate()
        .is_err());
        for temperature in [-1.0, f64::NAN, f64::INFINITY] {
            assert!(GenerationParameters {
                temperature: Some(temperature),
                ..Default::default()
            }
            .validate()
            .is_err());
        }
    }
}
