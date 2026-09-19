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

/// Reasoning intensity ordered from least to most. Turning thinking off is the `thinking`
/// switch, never a listed intensity, so `none` is not on this ladder.
pub const REASONING_EFFORT_LADDER: &[&str] =
    &["minimal", "low", "medium", "high", "xhigh", "max", "ultra"];

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
fn clamp_reasoning_effort(desired: Option<&str>, supported: &[impl AsRef<str>]) -> Option<String> {
    let desired = desired?;
    let supported: Vec<&str> = supported
        .iter()
        .map(AsRef::as_ref)
        .filter(|value| !value.is_empty())
        .collect();
    if supported.is_empty() {
        return None;
    }
    if supported.contains(&desired) {
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

fn effort_rank(effort: &str) -> Option<usize> {
    REASONING_EFFORT_LADDER
        .iter()
        .position(|item| *item == effort)
}

impl GenerationParameters {
    pub fn validate(&self) -> Result<(), &'static str> {
        if let Some(effort) = self.reasoning_effort.as_deref() {
            if effort_rank(effort).is_none() {
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

    /// Stored preferences are wishes, not a contract. Retired `none` means thinking off;
    /// any other illegal value is dropped so a request can still run.
    #[must_use]
    pub fn normalize_stored(mut self) -> Self {
        if self.reasoning_effort.as_deref() == Some("none") {
            self.reasoning_effort = None;
            if self.thinking.is_none() {
                self.thinking = Some(false);
            }
        }
        if self
            .reasoning_effort
            .as_deref()
            .is_some_and(|effort| effort_rank(effort).is_none())
        {
            self.reasoning_effort = None;
        }
        if self
            .temperature
            .is_some_and(|value| !value.is_finite() || value < 0.0)
        {
            self.temperature = None;
        }
        self
    }
}

/// Confirmed per-model parameter support from catalog `metadata.generationSupport`.
/// Missing information never establishes support; it is never inferred from preferences.
/// Unknown catalog keys are ignored so a newer catalog cannot break an older client.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GenerationSupport {
    pub temperature: Option<bool>,
    pub max_tokens: Option<bool>,
    /// True when the model can run with thinking turned off.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub thinking: Option<bool>,
    /// Selectable intensities. `none` is not an intensity; see `thinking`.
    pub reasoning_efforts: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reasoning_effort_default: Option<String>,
    pub temperature_with_reasoning: Option<bool>,
    pub temperature_max: Option<f64>,
    pub max_output_tokens: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fast_mode: Option<bool>,
}

impl GenerationSupport {
    pub fn validate(&self) -> Result<(), &'static str> {
        if self
            .temperature_max
            .is_some_and(|value| !value.is_finite() || value < 0.0)
            || self.max_output_tokens == Some(0)
        {
            return Err("invalid generation support limits");
        }
        for effort in self.efforts() {
            if effort_rank(effort).is_none() {
                return Err("reasoningEfforts must list reasoning intensities");
            }
        }
        if let Some(default) = self.reasoning_effort_default.as_deref() {
            if !self.efforts().any(|effort| effort == default) {
                return Err("reasoningEffortDefault must be listed in reasoningEfforts");
            }
        }
        Ok(())
    }

    fn efforts(&self) -> impl Iterator<Item = &str> {
        self.reasoning_efforts.iter().flatten().map(String::as_str)
    }

    fn thinking_supported(&self) -> bool {
        self.thinking == Some(true)
    }

    fn reasons(&self) -> bool {
        self.thinking_supported() || self.efforts().next().is_some()
    }

    fn thinking_on(&self, desired: Option<bool>) -> bool {
        if self.thinking_supported() {
            desired.unwrap_or(true)
        } else {
            self.efforts().next().is_some()
        }
    }

    fn effective_max_output_tokens(&self) -> Option<u32> {
        if self.max_tokens == Some(false) {
            None
        } else {
            Some(
                self.max_output_tokens
                    .map_or(DEFAULT_MAX_OUTPUT_TOKENS, |max| {
                        DEFAULT_MAX_OUTPUT_TOKENS.min(max)
                    }),
            )
        }
    }
}

/// What one backend protocol can express, independent of any model. These are wire facts
/// owned by the adapter, not catalog facts: a protocol that cannot carry a field makes the
/// field unusable even when the model supports it.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BackendCapability {
    /// Accepts a per-response output token cap.
    pub token_cap: bool,
    /// Accepts a sampling temperature.
    pub temperature: bool,
    /// Can encode the thinking switch, and therefore can turn thinking off.
    pub thinking_switch: bool,
    /// Can encode a reasoning intensity.
    pub intensity: bool,
    /// Can request the fast service tier.
    pub fast: bool,
}

/// Generation parameters that may reach the wire. Fields left `None` are omitted from the
/// payload, which leaves the provider default in effect.
#[derive(Clone, Debug, Default, PartialEq, Serialize)]
pub struct EffectiveGeneration {
    pub temperature: Option<f64>,
    pub max_output_tokens: Option<u32>,
    pub thinking: Option<bool>,
    pub reasoning_effort: Option<String>,
    pub fast_mode: Option<bool>,
}

impl EffectiveGeneration {
    /// Provider payloads carry temperature as f32.
    #[must_use]
    #[allow(clippy::cast_possible_truncation)]
    pub fn temperature_f32(&self) -> Option<f32> {
        self.temperature.map(|value| value as f32)
    }

    /// The internal cap, lowered by an optional per-call output budget. A budget never raises
    /// the cap, and it stays out of the wire identity so it can vary between steps of one run.
    #[must_use]
    pub fn output_cap(&self, budget: Option<u32>) -> Option<u32> {
        self.max_output_tokens
            .map(|ceiling| budget.unwrap_or(ceiling).min(ceiling))
    }
}

/// What a settings UI may expose after intersecting catalog facts with the backend protocol.
/// This is the same judgment `resolve` uses; a hidden control cannot appear on the wire, and
/// a shown control is one the current backend can actually carry.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GenerationControls {
    pub temperature: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature_max: Option<f64>,
    pub thinking: bool,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub reasoning_efforts: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reasoning_effort_default: Option<String>,
    pub fast_mode: bool,
}

/// Settings that can take effect on this backend for this model. Callers must not infer
/// controls from catalog fields alone.
#[must_use]
pub fn controls(support: &GenerationSupport, backend: &BackendCapability) -> GenerationControls {
    let thinking = backend.thinking_switch && support.thinking_supported();
    let reasoning_efforts: Vec<String> = if backend.intensity {
        support.efforts().map(str::to_owned).collect()
    } else {
        Vec::new()
    };
    let can_use_temperature_while_reasoning = support.temperature_with_reasoning.unwrap_or(false);
    let temperature = backend.temperature
        && support.temperature == Some(true)
        && (can_use_temperature_while_reasoning || thinking || !support.reasons());
    GenerationControls {
        temperature_max: temperature.then_some(support.temperature_max).flatten(),
        temperature,
        thinking,
        reasoning_effort_default: reasoning_efforts
            .iter()
            .find(|effort| Some(effort.as_str()) == support.reasoning_effort_default.as_deref())
            .cloned(),
        reasoning_efforts,
        fast_mode: backend.fast && support.fast_mode == Some(true),
    }
}

/// The single resolution of desired generation parameters against model and protocol facts.
/// Every caller resolves exactly once, against facts frozen for the request, so replaying a
/// frozen route always yields the same wire payload.
///
/// An unsupported or undeclared setting is omitted rather than guessed, except for the
/// internal output cap, which is a product default and applies unless the protocol or the
/// model rejects a cap. Illegal *desired* values are dropped the same way; illegal catalog
/// support still fails, because that data is ours.
pub fn resolve(
    desired: &GenerationParameters,
    support: &GenerationSupport,
    backend: &BackendCapability,
) -> Result<EffectiveGeneration, &'static str> {
    support.validate()?;
    let desired = desired.clone().normalize_stored();

    let thinking_on = support.thinking_on(desired.thinking);
    let thinking = (backend.thinking_switch && support.thinking_supported()).then_some(thinking_on);

    let reasoning_effort = if thinking_on && backend.intensity {
        let intensities: Vec<&str> = support.efforts().collect();
        clamp_reasoning_effort(
            desired
                .reasoning_effort
                .as_deref()
                .or(support.reasoning_effort_default.as_deref()),
            &intensities,
        )
    } else {
        None
    };

    let temperature = desired
        .temperature
        .filter(|_| {
            backend.temperature
                && support.temperature == Some(true)
                && (support.temperature_with_reasoning.unwrap_or(false) || !thinking_on)
        })
        .map(|value| support.temperature_max.map_or(value, |max| value.min(max)));

    let max_output_tokens = backend
        .token_cap
        .then(|| support.effective_max_output_tokens())
        .flatten();

    let fast_mode =
        (backend.fast && support.fast_mode == Some(true) && desired.fast_mode == Some(true))
            .then_some(true);

    Ok(EffectiveGeneration {
        temperature,
        max_output_tokens,
        thinking,
        reasoning_effort,
        fast_mode,
    })
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
            clamp_reasoning_effort(Some("minimal"), &["high", "max"]).as_deref(),
            Some("high")
        );
        assert_eq!(clamp_reasoning_effort(Some("low"), &[] as &[&str]), None);
        assert_eq!(clamp_reasoning_effort(None, &flash), None);
    }

    #[test]
    fn validates_explicit_generation_settings() {
        for effort in ["minimal", "low", "high", "max", "ultra"] {
            assert!(GenerationParameters {
                reasoning_effort: Some(effort.into()),
                ..Default::default()
            }
            .validate()
            .is_ok());
        }
        for effort in ["unknown", "none"] {
            assert!(GenerationParameters {
                reasoning_effort: Some(effort.into()),
                ..Default::default()
            }
            .validate()
            .is_err());
        }
        for temperature in [-1.0, f64::NAN, f64::INFINITY] {
            assert!(GenerationParameters {
                temperature: Some(temperature),
                ..Default::default()
            }
            .validate()
            .is_err());
        }
    }

    const FULL: BackendCapability = BackendCapability {
        token_cap: true,
        temperature: true,
        thinking_switch: true,
        intensity: true,
        fast: true,
    };

    fn switchable() -> GenerationSupport {
        GenerationSupport {
            temperature: Some(true),
            max_tokens: Some(true),
            thinking: Some(true),
            reasoning_efforts: Some(vec!["low".into(), "medium".into(), "high".into()]),
            ..Default::default()
        }
    }

    #[test]
    fn thinking_defaults_on_and_the_switch_turns_it_off() {
        let support = switchable();
        let on = resolve(&GenerationParameters::default(), &support, &FULL).unwrap();
        assert_eq!(on.thinking, Some(true));
        let off = resolve(
            &GenerationParameters {
                thinking: Some(false),
                ..Default::default()
            },
            &support,
            &FULL,
        )
        .unwrap();
        assert_eq!(off.thinking, Some(false));
        assert_eq!(off.reasoning_effort, None);
    }

    #[test]
    fn intensity_comes_from_the_request_or_the_catalog_default_and_is_never_invented() {
        let support = switchable();
        assert_eq!(
            resolve(&GenerationParameters::default(), &support, &FULL)
                .unwrap()
                .reasoning_effort,
            None
        );
        assert_eq!(
            resolve(
                &GenerationParameters {
                    reasoning_effort: Some("ultra".into()),
                    ..Default::default()
                },
                &support,
                &FULL
            )
            .unwrap()
            .reasoning_effort
            .as_deref(),
            Some("high")
        );
        let defaulted = GenerationSupport {
            reasoning_effort_default: Some("medium".into()),
            ..switchable()
        };
        assert_eq!(
            resolve(&GenerationParameters::default(), &defaulted, &FULL)
                .unwrap()
                .reasoning_effort
                .as_deref(),
            Some("medium")
        );
    }

    #[test]
    fn a_protocol_that_cannot_express_a_field_omits_it() {
        let support = GenerationSupport {
            fast_mode: Some(true),
            ..switchable()
        };
        let desired = GenerationParameters {
            temperature: Some(0.7),
            reasoning_effort: Some("low".into()),
            thinking: Some(false),
            fast_mode: Some(true),
        };
        let full = resolve(&desired, &support, &FULL).unwrap();
        assert_eq!(
            (full.thinking, full.fast_mode, full.max_output_tokens),
            (Some(false), Some(true), Some(DEFAULT_MAX_OUTPUT_TOKENS))
        );
        let bare = resolve(
            &desired,
            &support,
            &BackendCapability {
                token_cap: false,
                temperature: false,
                thinking_switch: false,
                intensity: false,
                fast: false,
            },
        )
        .unwrap();
        assert_eq!(bare, EffectiveGeneration::default());
    }

    #[test]
    fn undeclared_support_omits_instead_of_guessing() {
        let silent = GenerationSupport::default();
        let effective = resolve(
            &GenerationParameters {
                temperature: Some(0.7),
                reasoning_effort: Some("high".into()),
                thinking: Some(false),
                fast_mode: Some(true),
            },
            &silent,
            &FULL,
        )
        .unwrap();
        assert_eq!(
            effective,
            EffectiveGeneration {
                max_output_tokens: Some(DEFAULT_MAX_OUTPUT_TOKENS),
                ..Default::default()
            }
        );
    }

    #[test]
    fn temperature_needs_declared_support_and_coexistence_with_thinking() {
        let hot = GenerationSupport {
            temperature_max: Some(1.0),
            ..switchable()
        };
        let desired = GenerationParameters {
            temperature: Some(1.5),
            ..Default::default()
        };
        assert_eq!(resolve(&desired, &hot, &FULL).unwrap().temperature, None);
        let coexists = GenerationSupport {
            temperature_with_reasoning: Some(true),
            ..hot.clone()
        };
        assert_eq!(
            resolve(&desired, &coexists, &FULL).unwrap().temperature,
            Some(1.0)
        );
        assert_eq!(
            resolve(
                &GenerationParameters {
                    thinking: Some(false),
                    ..desired
                },
                &hot,
                &FULL
            )
            .unwrap()
            .temperature,
            Some(1.0)
        );
    }

    #[test]
    fn non_switchable_models_reason_whenever_they_declare_intensities() {
        let always = GenerationSupport {
            reasoning_efforts: Some(vec!["low".into(), "high".into()]),
            max_tokens: Some(true),
            ..Default::default()
        };
        let effective = resolve(
            &GenerationParameters {
                thinking: Some(false),
                ..Default::default()
            },
            &always,
            &FULL,
        )
        .unwrap();
        assert_eq!(effective.thinking, None);
        assert_eq!(effective.reasoning_effort, None);
        let none = GenerationSupport::default();
        assert_eq!(
            resolve(&GenerationParameters::default(), &none, &FULL)
                .unwrap()
                .thinking,
            None
        );
    }

    #[test]
    fn resolution_is_stable_when_replayed_against_the_same_facts() {
        let support = GenerationSupport {
            fast_mode: Some(true),
            temperature_with_reasoning: Some(true),
            reasoning_effort_default: Some("medium".into()),
            ..switchable()
        };
        let desired = GenerationParameters {
            temperature: Some(0.4),
            reasoning_effort: Some("ultra".into()),
            thinking: Some(true),
            fast_mode: Some(true),
        };
        let first = resolve(&desired, &support, &FULL).unwrap();
        let replay = resolve(
            &GenerationParameters {
                temperature: first.temperature,
                reasoning_effort: first.reasoning_effort.clone(),
                thinking: first.thinking,
                fast_mode: first.fast_mode,
            },
            &support,
            &FULL,
        )
        .unwrap();
        assert_eq!(first, replay);
    }

    #[test]
    fn support_rejects_disable_as_an_intensity_and_unlisted_defaults() {
        assert!(GenerationSupport {
            reasoning_efforts: Some(vec!["none".into()]),
            ..Default::default()
        }
        .validate()
        .is_err());
        assert!(GenerationSupport {
            reasoning_effort_default: Some("max".into()),
            ..switchable()
        }
        .validate()
        .is_err());
        assert!(GenerationSupport {
            max_output_tokens: Some(0),
            ..Default::default()
        }
        .validate()
        .is_err());
    }

    #[test]
    fn the_internal_output_cap_is_a_product_default_not_a_capability() {
        assert_eq!(
            GenerationSupport::default().effective_max_output_tokens(),
            Some(DEFAULT_MAX_OUTPUT_TOKENS)
        );
        assert_eq!(
            GenerationSupport {
                max_output_tokens: Some(4_096),
                ..Default::default()
            }
            .effective_max_output_tokens(),
            Some(4_096)
        );
        assert_eq!(
            GenerationSupport {
                max_tokens: Some(false),
                ..Default::default()
            }
            .effective_max_output_tokens(),
            None
        );
        assert_eq!(
            EffectiveGeneration {
                max_output_tokens: Some(4_096),
                ..Default::default()
            }
            .output_cap(Some(1_024)),
            Some(1_024)
        );
        assert_eq!(
            EffectiveGeneration {
                max_output_tokens: Some(4_096),
                ..Default::default()
            }
            .output_cap(Some(100_000)),
            Some(4_096)
        );
    }

    #[test]
    fn controls_match_what_resolve_can_put_on_the_wire() {
        let switchable = GenerationSupport {
            temperature: Some(true),
            temperature_max: Some(1.0),
            fast_mode: Some(true),
            ..switchable()
        };
        let shown = controls(&switchable, &FULL);
        assert!(shown.temperature);
        assert_eq!(shown.temperature_max, Some(1.0));
        assert!(shown.thinking);
        assert!(shown.fast_mode);
        assert_eq!(
            shown.reasoning_efforts,
            vec!["low".to_string(), "medium".to_string(), "high".to_string()]
        );
        let always = GenerationSupport {
            temperature: Some(true),
            reasoning_efforts: Some(vec!["low".into(), "high".into()]),
            ..Default::default()
        };
        let hidden = controls(&always, &FULL);
        assert!(!hidden.temperature);
        assert!(!hidden.thinking);
        assert_eq!(hidden.reasoning_efforts, vec!["low", "high"]);
        let protocol = controls(
            &switchable,
            &BackendCapability {
                token_cap: true,
                temperature: true,
                thinking_switch: false,
                intensity: true,
                fast: false,
            },
        );
        assert!(!protocol.thinking);
        assert!(!protocol.temperature);
        assert!(!protocol.fast_mode);
    }

    #[test]
    fn retired_and_illegal_preferences_are_dropped_instead_of_failing_the_request() {
        let support = switchable();
        let from_none = resolve(
            &GenerationParameters {
                reasoning_effort: Some("none".into()),
                ..Default::default()
            },
            &support,
            &FULL,
        )
        .unwrap();
        assert_eq!(from_none.thinking, Some(false));
        assert_eq!(from_none.reasoning_effort, None);
        let garbage = resolve(
            &GenerationParameters {
                reasoning_effort: Some("not-a-ladder".into()),
                temperature: Some(f64::NAN),
                thinking: Some(true),
                ..Default::default()
            },
            &support,
            &FULL,
        )
        .unwrap();
        assert_eq!(garbage.thinking, Some(true));
        assert_eq!(garbage.reasoning_effort, None);
        assert_eq!(garbage.temperature, None);
        assert!(GenerationParameters {
            reasoning_effort: Some("none".into()),
            ..Default::default()
        }
        .validate()
        .is_err());
    }

    #[test]
    fn catalog_support_ignores_unknown_keys() {
        let support = serde_json::from_value::<GenerationSupport>(serde_json::json!({
            "temperature": true,
            "thinking": true,
            "reasoningEfforts": ["low"],
            "futureFlag": true
        }))
        .unwrap();
        assert_eq!(support.temperature, Some(true));
        assert!(support.validate().is_ok());
    }
}
