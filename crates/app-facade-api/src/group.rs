//! Public Group contracts. Execution bindings and database records are not public DTOs.
use crate::FacadeProblem;
use serde::{Deserialize, Serialize};

pub const LIST: &str = "/app/group/list";
pub const DETAIL: &str = "/app/group/detail/get";
pub const CREATE: &str = "/app/group/create";
pub const RENAME: &str = "/app/group/rename";
pub const MEMBERS_UPDATE: &str = "/app/group/members/update";
pub const DELETE: &str = "/app/group/delete";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GroupApplyState {
    #[default]
    Applying,
    Ready,
    Failed,
    Deleting,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct GroupSummary {
    pub group_id: String,
    pub name: String,
    #[serde(rename = "entityType")]
    pub member_type: String,
    pub member_count: usize,
    pub revision: i64,
    pub apply_state: GroupApplyState,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct GroupCapability {
    pub id: String,
    pub member_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct GroupMemberProblem {
    #[serde(rename = "entityId")]
    pub member_id: String,
    pub code: String,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GroupDetail {
    #[serde(flatten)]
    pub summary: GroupSummary,
    #[serde(rename = "entityIds")]
    pub member_ids: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub capabilities: Option<Vec<GroupCapability>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub member_problems: Option<Vec<GroupMemberProblem>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub application_problem: Option<crate::FacadeProblem>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ListGroupsInput {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[serde(rename = "entityType")]
    pub member_type: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct GetGroupsInput {
    pub ids: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CreateGroupInput {
    pub creation_id: String,
    pub name: String,
    #[serde(rename = "entityType")]
    pub member_type: String,
    #[serde(rename = "entityIds")]
    pub member_ids: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RenameGroupInput {
    pub group_id: String,
    pub name: String,
    pub expected_revision: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct UpdateGroupMembersInput {
    pub group_id: String,
    #[serde(rename = "entityIds")]
    pub member_ids: Vec<String>,
    pub expected_revision: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DeleteGroupInput {
    pub group_id: String,
    pub expected_revision: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListGroupsOutput {
    pub space_id: String,
    pub groups: Vec<GroupSummary>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GroupDetailItem {
    pub requested_id: String,
    pub found: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub group: Option<GroupDetail>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetGroupsOutput {
    pub space_id: String,
    pub items: Vec<GroupDetailItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GroupMutationOutput {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub application_problem: Option<FacadeProblem>,
    pub group_id: String,
    pub revision: i64,
    pub apply_state: GroupApplyState,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeleteGroupOutput {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub application_problem: Option<FacadeProblem>,
    pub group_id: String,
    pub deleted: bool,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn mutations_have_distinct_strict_contracts() {
        let rename = json!({"groupId":"g:1","name":"Lights","expectedRevision":3});
        assert!(serde_json::from_value::<RenameGroupInput>(rename.clone()).is_ok());
        assert!(serde_json::from_value::<UpdateGroupMembersInput>(rename).is_err());
        assert!(serde_json::from_value::<RenameGroupInput>(
            json!({"groupId":"g:1","name":"Lights"})
        )
        .is_err());
        assert!(serde_json::from_value::<DeleteGroupInput>(
            json!({"groupId":"g:1","entityType":"basic.device.Device","expectedRevision":3})
        )
        .is_err());
    }
}
