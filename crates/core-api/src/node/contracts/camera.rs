use serde::{Deserialize, Serialize};

pub const VERSION: &str = crate::node::settings::VERSION;
pub const NODE_TYPE: &str = "camera";
pub const SETTINGS_STATUS: &str = crate::node::settings::STATUS;
pub const SETTINGS_UPDATE: &str = crate::node::settings::UPDATE;
pub const SETTINGS_REVERT: &str = crate::node::settings::REVERT;
pub const SETTINGS_RETRY: &str = crate::node::settings::RETRY;
pub const RECOGNITION_SECTION: &str = "recognition";

pub type SettingsStatusRequest = crate::node::settings::StatusRequest;
pub type RecognitionUpdateRequest = crate::node::settings::UpdateRequest<RecognitionSettings>;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RecognitionSettings {
    pub enabled: bool,
}

pub type RecognitionCommandRequest = crate::node::settings::SectionCommandRequest;
