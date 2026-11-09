//! Best-effort attribution snapshots. Array order is execution order.
use crate::SourceKind;
use serde::{Deserialize, Serialize};

pub type Trace = Vec<TraceItem>;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TraceItem {
    pub timestamp: i64,
    #[serde(flatten)]
    pub kind: TraceKind,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SourceMetadata {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub app_client_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub page: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub card_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub surface_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub provider: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "SCREAMING_SNAKE_CASE")]
pub enum TraceKind {
    User {
        #[serde(rename = "userId")]
        user_id: String,
        source: SourceKind,
        #[serde(flatten)]
        metadata: SourceMetadata,
    },
    Space {
        #[serde(rename = "spaceId")]
        space_id: String,
        source: SourceKind,
        #[serde(flatten)]
        metadata: SourceMetadata,
    },
    AutomationIgnite {
        #[serde(rename = "automationId")]
        automation_id: String,
    },
    AutomationTrigger {
        #[serde(rename = "automationId")]
        automation_id: String,
    },
    AutomationResume {
        #[serde(rename = "automationId")]
        automation_id: String,
    },
    Widget {
        #[serde(rename = "widgetId")]
        widget_id: String,
    },
    Group {
        #[serde(rename = "groupId")]
        group_id: String,
    },
    QuickWord {
        #[serde(rename = "quickWord")]
        quick_word: String,
    },
    Schedule {
        #[serde(
            default,
            rename = "scheduleId",
            skip_serializing_if = "Option::is_none"
        )]
        schedule_id: Option<String>,
    },
    External {
        #[serde(
            default,
            rename = "integrationId",
            skip_serializing_if = "Option::is_none"
        )]
        integration_id: Option<String>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TraceGetInput {
    pub trace_ids: Vec<String>,
}

pub const TRACE_GET_TARGET: &str = "/app/trace/get";

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn shared_schema_rejects_wrong_type_fields_and_source_values() {
        let schema: serde_json::Value =
            serde_json::from_str(include_str!("../schema/trace.v1.schema.json")).unwrap();
        let validator = jsonschema::validator_for(&schema).unwrap();
        let mut value: serde_json::Value =
            serde_json::from_str(include_str!("../fixtures/trace/linear.json")).unwrap();
        assert!(validator.is_valid(&value));
        value[0]["source"] = serde_json::json!("skill");
        assert!(!validator.is_valid(&value));
        value[0]["source"] = serde_json::json!("ui");
        value[1]["userId"] = serde_json::json!("wrong-field");
        assert!(!validator.is_valid(&value));
    }
    #[test]
    fn flat_uppercase_items_preserve_millisecond_times_and_order() {
        let json = include_str!("../fixtures/trace/linear.json");
        let trace: Trace = serde_json::from_str(json).unwrap();
        assert_eq!(
            serde_json::to_value(&trace).unwrap(),
            serde_json::from_str::<serde_json::Value>(json).unwrap()
        );
        assert_eq!(trace[0].timestamp, 1794225600000);
        assert!(matches!(trace[2].kind, TraceKind::AutomationResume { .. }));
        let invalid = json.replace("AUTOMATION_RESUME", "automationResume");
        assert!(serde_json::from_str::<Trace>(&invalid).is_err());
        assert!(
            serde_json::from_str::<Trace>(&json.replace("\"automationId\"", "\"target\"")).is_err()
        );
    }
}
