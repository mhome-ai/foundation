use serde::{Deserialize, Serialize};

pub const VERSION: &str = crate::settings::VERSION;
pub const NODE_TYPE: &str = "camera";
pub const SETTINGS_STATUS: &str = crate::settings::STATUS;
pub const SETTINGS_UPDATE: &str = crate::settings::UPDATE;
pub const SETTINGS_REVERT: &str = crate::settings::REVERT;
pub const SETTINGS_RETRY: &str = crate::settings::RETRY;
pub const RECOGNITION_SECTION: &str = "recognition";

pub type SettingsStatusRequest = crate::settings::StatusRequest;
pub type RecognitionUpdateRequest = crate::settings::UpdateRequest<RecognitionSettings>;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RecognitionSettings {
    pub enabled: bool,
}

pub type RecognitionCommandRequest = crate::settings::SectionCommandRequest;
