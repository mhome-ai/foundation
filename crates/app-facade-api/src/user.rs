//! Cloud-owned account profile.
//!
//! The display name is the account `name`. Avatar bytes stay in private object
//! storage: the account stores the object key, prepare returns a short-lived
//! upload URL, and get returns a short-lived read URL. Neither value is a
//! Space setting.

use serde::{Deserialize, Serialize};

pub const PROFILE_GET: &str = "/app/user/profile/get";
pub const PROFILE_UPDATE: &str = "/app/user/profile/update";
pub const AVATAR_PREPARE: &str = "/app/user/profile/avatar/prepare";
pub const AVATAR_COMMIT: &str = "/app/user/profile/avatar/commit";
pub const AVATAR_CLEAR: &str = "/app/user/profile/avatar/clear";
pub const CONTRACT: &str = "mhome.user.profile.v1";

pub const REQUEST_TARGETS: &[&str] = &[
    PROFILE_GET,
    PROFILE_UPDATE,
    AVATAR_PREPARE,
    AVATAR_COMMIT,
    AVATAR_CLEAR,
];

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ProfileGetRequest {}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ProfileUpdateRequest {
    pub name: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PrepareAvatarRequest {
    pub content_type: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CommitAvatarRequest {
    pub object_key: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ClearAvatarRequest {}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UserProfile {
    pub name: String,
    pub email: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub avatar_url: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub avatar_expires_at_ms: Option<i64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PreparedAvatarUpload {
    pub object_key: String,
    pub upload_url: String,
    pub expires_at_ms: i64,
    pub content_type: String,
    pub max_bytes: i64,
}
