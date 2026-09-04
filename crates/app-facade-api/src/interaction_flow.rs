//! Public Interaction Flow session protocol exposed to MeowLink clients.
//!
//! This is deliberately a current-step projection. It contains no complete
//! definition graph, handler operations, variable references, or Node routes.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

pub const START_TARGET: &str = "/app/interaction-flow/start";
pub const PROTOCOL_SCHEMA: &str = include_str!("../schema/interaction-flow-app.v1.schema.json");
pub const RESOLVE_TARGET: &str = "/app/interaction-flow/resolve";
pub const NEXT_TARGET: &str = "/app/interaction-flow/next";
pub const BACK_TARGET: &str = "/app/interaction-flow/back";
pub const COMPLETE_TARGET: &str = "/app/interaction-flow/complete";
pub const CANCEL_TARGET: &str = "/app/interaction-flow/cancel";

pub const REQUEST_TARGETS: &[&str] = &[
    START_TARGET,
    RESOLVE_TARGET,
    NEXT_TARGET,
    BACK_TARGET,
    COMPLETE_TARGET,
    CANCEL_TARGET,
];

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub enum InteractionFlowSource {
    Plugin {
        node_type: String,
        node_id: String,
        flow_id: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        subject_id: Option<String>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct InteractionFlowStartRequest {
    pub source: InteractionFlowSource,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct InteractionFlowResolveRequest {
    pub session_id: String,
    pub command_id: String,
    pub expected_revision: u64,
    #[serde(default)]
    pub input: HashMap<String, Value>,
    pub vars: Vec<String>,
    #[serde(default)]
    pub force: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct InteractionFlowTransitionRequest {
    pub session_id: String,
    pub command_id: String,
    pub expected_revision: u64,
    #[serde(default)]
    pub input: HashMap<String, Value>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct InteractionFlowCompleteRequest {
    pub session_id: String,
    pub command_id: String,
    pub expected_revision: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct InteractionFlowCancelRequest {
    pub session_id: String,
    pub command_id: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum FlowRendererView {
    TypedForm,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FlowPresentationView {
    pub renderer: FlowRendererView,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum FlowFieldValueTypeView {
    String,
    Boolean,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum FlowFieldControlView {
    Text,
    Password,
    LongText,
    Select,
    Boolean,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FlowFormFieldView {
    pub var: String,
    pub value_type: FlowFieldValueTypeView,
    pub control: FlowFieldControlView,
    pub required: bool,
    pub repeat: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub options: Option<Vec<Value>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub option_unavailable_text: Option<String>,
    pub read_only: bool,
    pub has_value: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub min_rows: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FlowResolutionView {
    pub var: String,
    pub available: bool,
    #[serde(default)]
    pub depends_on: Vec<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FlowFormView {
    #[serde(default)]
    pub fields: Vec<FlowFormFieldView>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ExternalAuthCompletionModeView {
    Callback,
    Poll,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(
    tag = "type",
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub enum FlowStepViewContent {
    Form {
        presentation: FlowPresentationView,
        form: FlowFormView,
    },
    ExternalAuth {
        presentation: FlowPresentationView,
        authorization_url: String,
        completion_mode: ExternalAuthCompletionModeView,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        poll_after_ms: Option<u64>,
    },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct InteractionFlowView {
    pub session_id: String,
    pub revision: u64,
    pub step_id: String,
    pub content: FlowStepViewContent,
    #[serde(default)]
    pub values: HashMap<String, Value>,
    pub resolutions: Vec<FlowResolutionView>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub back_step_id: Option<String>,
    pub ready_to_complete: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct InteractionFlowCompleteResponse {
    pub completed: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub result: Option<Value>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct InteractionFlowCancelResponse {
    pub cancelled: bool,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn public_source_cannot_choose_a_handler_or_route() {
        let request = serde_json::from_value::<InteractionFlowStartRequest>(json!({
            "source": {
                "kind": "plugin",
                "nodeType": "camera",
                "nodeId": "camera-1",
                "flowId": "provider.connect",
                "subjectId": "onvif",
                "handler": "steal",
                "target": "/camera/arbitrary"
            }
        }));
        assert!(request.is_err());
    }

    #[test]
    fn view_is_a_current_step_projection_without_definition_or_handlers() {
        let view = InteractionFlowView {
            session_id: "session-1".into(),
            revision: 1,
            step_id: "credentials".into(),
            content: FlowStepViewContent::Form {
                presentation: FlowPresentationView {
                    renderer: FlowRendererView::TypedForm,
                    title: Some("Connect".into()),
                    description: None,
                    content: None,
                },
                form: FlowFormView::default(),
            },
            values: HashMap::new(),
            resolutions: Vec::new(),
            back_step_id: None,
            ready_to_complete: true,
        };
        let value = serde_json::to_value(view).unwrap();
        assert!(value.get("steps").is_none());
        assert!(value.get("resolvers").is_none());
        assert!(value.to_string().find("handler").is_none());
    }

    #[test]
    fn conformance_fixture_matches_the_published_app_schema() {
        let schema: Value = serde_json::from_str(PROTOCOL_SCHEMA).unwrap();
        let fixture: Value = serde_json::from_str(include_str!(
            "../fixtures/interaction-flow.conformance.json"
        ))
        .unwrap();
        let validator = jsonschema::validator_for(&schema).unwrap();
        let errors = validator
            .iter_errors(&fixture)
            .map(|error| error.to_string())
            .collect::<Vec<_>>();
        assert!(errors.is_empty(), "fixture failed: {errors:?}");
    }
}
