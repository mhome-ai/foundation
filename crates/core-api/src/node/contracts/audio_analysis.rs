//! Optional finite Artifact analysis. These observations never authorize a
//! business action or decide whether a speaker addressed the application.
use artifact_api::{ArtifactKind, ArtifactReference, MediaReference};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{fmt, io};

pub const AUDIO_ANALYSIS_CAPABILITIES_TARGET: &str = "/audiobridge/audio/analysis/capabilities";
pub const AUDIO_ANALYZE_TARGET: &str = "/audiobridge/audio/analyze";
pub const AUDIO_ANALYSIS_VERSION: u32 = 1;
pub const AUDIO_ANALYSIS_SAMPLE_RATE_HZ: u32 = 16_000;
pub const AUDIO_ANALYSIS_HEADER_BYTES: u32 = 44;
pub const AUDIO_ANALYSIS_MIN_SAMPLES: u32 = 4_800;
pub const AUDIO_ANALYSIS_MAX_SAMPLES: u32 = 480_000;
pub const AUDIO_ANALYSIS_MAX_WAV_BYTES: u32 = 960_044;
pub const AUDIO_ANALYSIS_WINDOW_SAMPLES: u32 = 64_000;
pub const AUDIO_ANALYSIS_HOP_SAMPLES: u32 = 32_000;
pub const AUDIO_ANALYSIS_MAX_WINDOWS: u32 = 14;
pub const AUDIO_ANALYSIS_MAX_CLASSES: u32 = 1_024;
pub const AUDIO_ANALYSIS_MAX_LABEL_BYTES: u32 = 256;
pub const AUDIO_ANALYSIS_MAX_RESPONSE_BYTES: u32 = 512 * 1_024;

pub type AudioAnalysisCapabilitiesRequest = super::EmptyRequest;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AudioAnalysisValidationError {
    InvalidReference,
    UnsupportedAudio,
    InvalidAudio,
    InsufficientAudio,
    InputTooLarge,
    InvalidVersion,
    InvalidModel,
    InvalidClasses,
    InvalidCoverage,
    IncompleteScores,
    InvalidCapabilities,
    InputMismatch,
    ResponseTooLarge,
    InvalidJson,
}

impl fmt::Display for AudioAnalysisValidationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::InvalidReference => "analysis requires an Audio Artifact reference",
            Self::UnsupportedAudio => "analysis requires canonical PCM16 mono 16 kHz WAV",
            Self::InvalidAudio => "audio bytes, SHA-256, header, or metadata disagree",
            Self::InsufficientAudio => "analysis requires at least 4800 actual samples",
            Self::InputTooLarge => "analysis input exceeds 480000 actual samples",
            Self::InvalidVersion => "unsupported audio analysis version",
            Self::InvalidModel => "invalid or unexpected analysis model identity",
            Self::InvalidClasses => "analysis requires the complete ordered class catalog",
            Self::InvalidCoverage => "analysis windows do not exactly cover the input",
            Self::IncompleteScores => "analysis requires complete finite scores in [0,1]",
            Self::InvalidCapabilities => "inconsistent audio analysis capabilities",
            Self::InputMismatch => "analysis does not match the exact input reference",
            Self::ResponseTooLarge => "analysis response exceeds 512 KiB",
            Self::InvalidJson => "invalid audio analysis JSON",
        })
    }
}

impl std::error::Error for AudioAnalysisValidationError {}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AudioAnalysisRequest {
    pub audio: MediaReference,
}

impl AudioAnalysisRequest {
    /// Authenticated tenant/scope authorization and delivery remain with the
    /// caller. Reject inadmissible metadata before fetching any Artifact body.
    pub fn reference(&self) -> Result<ArtifactReference, AudioAnalysisValidationError> {
        let reference = self
            .audio
            .reference(ArtifactKind::Audio)
            .map_err(|_| AudioAnalysisValidationError::InvalidReference)?;
        let metadata = reference.metadata();
        if metadata.mime_type() != "audio/wav" {
            return Err(AudioAnalysisValidationError::UnsupportedAudio);
        }
        let size = metadata.size_bytes();
        if size > u64::from(AUDIO_ANALYSIS_MAX_WAV_BYTES) {
            return Err(AudioAnalysisValidationError::InputTooLarge);
        }
        if size < u64::from(AUDIO_ANALYSIS_HEADER_BYTES)
            || (size - u64::from(AUDIO_ANALYSIS_HEADER_BYTES)) % 2 != 0
        {
            return Err(AudioAnalysisValidationError::InvalidAudio);
        }
        let samples = ((size - u64::from(AUDIO_ANALYSIS_HEADER_BYTES)) / 2) as u32;
        validate_sample_count(samples)?;
        if let Some(duration) = metadata.duration_millis() {
            if duration != u64::from(samples) * 1_000 / u64::from(AUDIO_ANALYSIS_SAMPLE_RATE_HZ) {
                return Err(AudioAnalysisValidationError::InvalidAudio);
            }
        }
        Ok(reference)
    }

    pub fn validate(&self) -> Result<(), AudioAnalysisValidationError> {
        self.reference().map(|_| ())
    }

    pub fn sample_count(&self) -> Result<u32, AudioAnalysisValidationError> {
        let reference = self.reference()?;
        Ok(
            ((reference.metadata().size_bytes() - u64::from(AUDIO_ANALYSIS_HEADER_BYTES)) / 2)
                as u32,
        )
    }

    /// Validate canonical bytes and content identity; return the original PCM
    /// byte slice without allocating samples or padding. No recovery/skipping.
    pub fn validate_wav<'a>(
        &self,
        bytes: &'a [u8],
    ) -> Result<&'a [u8], AudioAnalysisValidationError> {
        let reference = self.reference()?;
        if bytes.len() as u64 != reference.metadata().size_bytes() {
            return Err(AudioAnalysisValidationError::InvalidAudio);
        }
        // The checked metadata minimum above proves all fixed header offsets.
        let u16_at = |at| u16::from_le_bytes(bytes[at..at + 2].try_into().unwrap());
        let u32_at = |at| u32::from_le_bytes(bytes[at..at + 4].try_into().unwrap());
        if &bytes[0..4] != b"RIFF"
            || &bytes[8..12] != b"WAVE"
            || &bytes[12..16] != b"fmt "
            || &bytes[36..40] != b"data"
            || u32_at(4) != bytes.len() as u32 - 8
            || u32_at(16) != 16
            || u16_at(20) != 1
            || u16_at(22) != 1
            || u32_at(24) != AUDIO_ANALYSIS_SAMPLE_RATE_HZ
            || u32_at(28) != AUDIO_ANALYSIS_SAMPLE_RATE_HZ * 2
            || u16_at(32) != 2
            || u16_at(34) != 16
            || u32_at(40) != bytes.len() as u32 - AUDIO_ANALYSIS_HEADER_BYTES
        {
            return Err(AudioAnalysisValidationError::InvalidAudio);
        }
        if format!("{:x}", Sha256::digest(bytes)) != reference.sha256() {
            return Err(AudioAnalysisValidationError::InvalidAudio);
        }
        Ok(&bytes[AUDIO_ANALYSIS_HEADER_BYTES as usize..])
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AudioAnalysisModel {
    pub id: String,
    pub model_sha256: String,
    pub labels_sha256: String,
}

impl AudioAnalysisModel {
    pub fn validate(&self) -> Result<(), AudioAnalysisValidationError> {
        let hash = |value: &str| {
            value.len() == 64
                && value
                    .bytes()
                    .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
        };
        if self.id.trim().is_empty()
            || self.id.len() > 256
            || !hash(&self.model_sha256)
            || !hash(&self.labels_sha256)
        {
            return Err(AudioAnalysisValidationError::InvalidModel);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum AudioAnalysisContainer {
    Wav,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum AudioAnalysisSampleFormat {
    PcmS16le,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AudioAnalysisFormat {
    pub container: AudioAnalysisContainer,
    pub sample_format: AudioAnalysisSampleFormat,
    pub channels: u32,
    pub sample_rate_hz: u32,
    pub header_bytes: u32,
}

impl Default for AudioAnalysisFormat {
    fn default() -> Self {
        Self {
            container: AudioAnalysisContainer::Wav,
            sample_format: AudioAnalysisSampleFormat::PcmS16le,
            channels: 1,
            sample_rate_hz: AUDIO_ANALYSIS_SAMPLE_RATE_HZ,
            header_bytes: AUDIO_ANALYSIS_HEADER_BYTES,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AudioAnalysisLimits {
    pub min_sample_count: u32,
    pub max_sample_count: u32,
    pub max_wav_bytes: u32,
    pub window_samples: u32,
    pub hop_samples: u32,
    pub max_windows: u32,
    pub max_classes: u32,
    pub max_label_bytes: u32,
    pub max_response_bytes: u32,
}

impl Default for AudioAnalysisLimits {
    fn default() -> Self {
        Self {
            min_sample_count: AUDIO_ANALYSIS_MIN_SAMPLES,
            max_sample_count: AUDIO_ANALYSIS_MAX_SAMPLES,
            max_wav_bytes: AUDIO_ANALYSIS_MAX_WAV_BYTES,
            window_samples: AUDIO_ANALYSIS_WINDOW_SAMPLES,
            hop_samples: AUDIO_ANALYSIS_HOP_SAMPLES,
            max_windows: AUDIO_ANALYSIS_MAX_WINDOWS,
            max_classes: AUDIO_ANALYSIS_MAX_CLASSES,
            max_label_bytes: AUDIO_ANALYSIS_MAX_LABEL_BYTES,
            max_response_bytes: AUDIO_ANALYSIS_MAX_RESPONSE_BYTES,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AudioAnalysisCapabilities {
    #[serde(deserialize_with = "deserialize_analysis_version")]
    pub analysis_version: u32,
    /// Interface support, not model health or evidence quality.
    pub supported: bool,
    /// True only after this process verified and initialized the model.
    pub loaded: bool,
    pub expected_model: AudioAnalysisModel,
    pub format: AudioAnalysisFormat,
    pub limits: AudioAnalysisLimits,
}

impl AudioAnalysisCapabilities {
    pub fn new(supported: bool, loaded: bool, expected_model: AudioAnalysisModel) -> Self {
        Self {
            analysis_version: AUDIO_ANALYSIS_VERSION,
            supported,
            loaded,
            expected_model,
            format: AudioAnalysisFormat::default(),
            limits: AudioAnalysisLimits::default(),
        }
    }

    pub fn validate(&self) -> Result<(), AudioAnalysisValidationError> {
        validate_version(self.analysis_version)?;
        self.expected_model.validate()?;
        if (self.loaded && !self.supported)
            || self.format != AudioAnalysisFormat::default()
            || self.limits != AudioAnalysisLimits::default()
        {
            return Err(AudioAnalysisValidationError::InvalidCapabilities);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AudioAnalysisClass {
    pub index: u32,
    pub label: String,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AudioAnalysisCoverage {
    pub offset_samples: u32,
    pub valid_samples: u32,
    pub padding_samples: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AudioAnalysisWindow {
    pub offset_samples: u32,
    pub valid_samples: u32,
    pub padding_samples: u32,
    #[serde(deserialize_with = "deserialize_scores")]
    pub scores: Vec<f32>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum AudioAnalysisUnknownReason {
    Unavailable,
    Busy,
    Timeout,
    UnsupportedAudio,
    InvalidAudio,
    InsufficientAudio,
    InferenceFailed,
    IncompleteScores,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(
    tag = "status",
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub enum AudioAnalysisResponse {
    Analyzed {
        audio: MediaReference,
        #[serde(deserialize_with = "deserialize_analysis_version")]
        analysis_version: u32,
        model: AudioAnalysisModel,
        sample_rate_hz: u32,
        sample_count: u32,
        classes: Vec<AudioAnalysisClass>,
        windows: Vec<AudioAnalysisWindow>,
    },
    Unknown {
        audio: MediaReference,
        #[serde(deserialize_with = "deserialize_analysis_version")]
        analysis_version: u32,
        reason: AudioAnalysisUnknownReason,
    },
}

impl AudioAnalysisResponse {
    pub fn audio(&self) -> &MediaReference {
        match self {
            Self::Analyzed { audio, .. } | Self::Unknown { audio, .. } => audio,
        }
    }

    pub fn validate(&self) -> Result<(), AudioAnalysisValidationError> {
        match self {
            Self::Unknown {
                audio,
                analysis_version,
                ..
            } => {
                validate_version(*analysis_version)?;
                // Unknown may describe an unsupported or too short Audio
                // Artifact. It must still name an Audio Artifact, not a URL.
                audio
                    .reference(ArtifactKind::Audio)
                    .map_err(|_| AudioAnalysisValidationError::InvalidReference)?;
            }
            Self::Analyzed {
                audio,
                analysis_version,
                model,
                sample_rate_hz,
                sample_count,
                classes,
                windows,
            } => {
                validate_version(*analysis_version)?;
                model.validate()?;
                let request = AudioAnalysisRequest {
                    audio: audio.clone(),
                };
                if *sample_rate_hz != AUDIO_ANALYSIS_SAMPLE_RATE_HZ
                    || request.sample_count()? != *sample_count
                {
                    return Err(AudioAnalysisValidationError::InvalidAudio);
                }
                if classes.is_empty()
                    || classes.len() > AUDIO_ANALYSIS_MAX_CLASSES as usize
                    || classes.iter().enumerate().any(|(index, class)| {
                        class.index as usize != index
                            || class.label.trim().is_empty()
                            || class.label.len() > AUDIO_ANALYSIS_MAX_LABEL_BYTES as usize
                    })
                {
                    return Err(AudioAnalysisValidationError::InvalidClasses);
                }
                let expected = audio_analysis_coverage(*sample_count)?;
                if windows.len() != expected.len() {
                    return Err(AudioAnalysisValidationError::InvalidCoverage);
                }
                for (window, expected) in windows.iter().zip(expected) {
                    if window.offset_samples != expected.offset_samples
                        || window.valid_samples != expected.valid_samples
                        || window.padding_samples != expected.padding_samples
                    {
                        return Err(AudioAnalysisValidationError::InvalidCoverage);
                    }
                    if window.scores.len() != classes.len()
                        || window
                            .scores
                            .iter()
                            .any(|score| !score.is_finite() || !(0.0..=1.0).contains(score))
                    {
                        return Err(AudioAnalysisValidationError::IncompleteScores);
                    }
                }
            }
        }
        serde_json::to_writer(SizeLimit(0), self)
            .map_err(|_| AudioAnalysisValidationError::ResponseTooLarge)
    }

    pub fn validate_for(
        &self,
        request: &AudioAnalysisRequest,
    ) -> Result<(), AudioAnalysisValidationError> {
        if self.audio() != &request.audio {
            return Err(AudioAnalysisValidationError::InputMismatch);
        }
        self.validate()
    }

    /// Consumers select the accepted model policy. The shared DTO does not pin
    /// a product model, class semantics, thresholds, or a backchannel decision.
    pub fn validate_for_model(
        &self,
        request: &AudioAnalysisRequest,
        expected: &AudioAnalysisModel,
    ) -> Result<(), AudioAnalysisValidationError> {
        expected.validate()?;
        self.validate_for(request)?;
        if let Self::Analyzed { model, .. } = self {
            if model != expected {
                return Err(AudioAnalysisValidationError::InvalidModel);
            }
        }
        Ok(())
    }

    /// Enforce the wire cap before deserialization as well as after canonical
    /// serialization. This rejects oversized whitespace and duplicate fields.
    pub fn from_json_slice(bytes: &[u8]) -> Result<Self, AudioAnalysisValidationError> {
        if bytes.len() > AUDIO_ANALYSIS_MAX_RESPONSE_BYTES as usize {
            return Err(AudioAnalysisValidationError::ResponseTooLarge);
        }
        let result: Self =
            serde_json::from_slice(bytes).map_err(|_| AudioAnalysisValidationError::InvalidJson)?;
        result.validate()?;
        Ok(result)
    }
}

pub fn audio_analysis_coverage(
    sample_count: u32,
) -> Result<Vec<AudioAnalysisCoverage>, AudioAnalysisValidationError> {
    validate_sample_count(sample_count)?;
    let mut result = Vec::with_capacity(AUDIO_ANALYSIS_MAX_WINDOWS as usize);
    let mut offset = 0;
    loop {
        let valid = (sample_count - offset).min(AUDIO_ANALYSIS_WINDOW_SAMPLES);
        result.push(AudioAnalysisCoverage {
            offset_samples: offset,
            valid_samples: valid,
            padding_samples: AUDIO_ANALYSIS_WINDOW_SAMPLES - valid,
        });
        if offset + valid == sample_count {
            return Ok(result);
        }
        offset += AUDIO_ANALYSIS_HOP_SAMPLES;
    }
}

fn validate_sample_count(samples: u32) -> Result<(), AudioAnalysisValidationError> {
    if samples < AUDIO_ANALYSIS_MIN_SAMPLES {
        return Err(AudioAnalysisValidationError::InsufficientAudio);
    }
    if samples > AUDIO_ANALYSIS_MAX_SAMPLES {
        return Err(AudioAnalysisValidationError::InputTooLarge);
    }
    Ok(())
}

fn validate_version(version: u32) -> Result<(), AudioAnalysisValidationError> {
    if version != AUDIO_ANALYSIS_VERSION {
        return Err(AudioAnalysisValidationError::InvalidVersion);
    }
    Ok(())
}

fn deserialize_analysis_version<'de, D: serde::Deserializer<'de>>(
    deserializer: D,
) -> Result<u32, D::Error> {
    let version = u32::deserialize(deserializer)?;
    validate_version(version).map_err(serde::de::Error::custom)?;
    Ok(version)
}

fn deserialize_scores<'de, D: serde::Deserializer<'de>>(
    deserializer: D,
) -> Result<Vec<f32>, D::Error> {
    // Check JSON's f64 value before narrowing: 1.00000001 and tiny negative
    // values can otherwise round to valid f32 endpoints and evade the range.
    let scores = Vec::<f64>::deserialize(deserializer)?;
    if scores.is_empty()
        || scores.len() > AUDIO_ANALYSIS_MAX_CLASSES as usize
        || scores
            .iter()
            .any(|score| !score.is_finite() || !(0.0..=1.0).contains(score))
    {
        return Err(serde::de::Error::custom(
            AudioAnalysisValidationError::IncompleteScores,
        ));
    }
    Ok(scores.into_iter().map(|score| score as f32).collect())
}

// Count serialization bytes without allocating another large response buffer.
struct SizeLimit(usize);
impl io::Write for SizeLimit {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        self.0 = self.0.saturating_add(bytes.len());
        if self.0 > AUDIO_ANALYSIS_MAX_RESPONSE_BYTES as usize {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "analysis response too large",
            ));
        }
        Ok(bytes.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}
