//! Bounded, type-specific Widget reads for App presentation.
use crate::resource_read::ResourceIdsInput;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;

pub const CONFORMANCE: &str = include_str!("../fixtures/widget-presentation.v1.json");

pub const LIST: &str = "/app/presentation/widget/list";
pub const NAMES: &str = "/app/presentation/widget/names/get";
pub const DEFAULTS: &str = "/app/presentation/widget/defaults/get";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum WidgetType {
    Data,
    Button,
    Circadian,
}
impl WidgetType {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Data => "data",
            Self::Button => "button",
            Self::Circadian => "circadian",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ListInput {
    pub r#type: WidgetType,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DefaultsInput {}

/// An omitted type retains the existing shared summary catalog.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SummaryInput {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub r#type: Option<WidgetType>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub exclude_sources: Vec<String>,
}
impl SummaryInput {
    pub fn validate(&self) -> Result<(), &'static str> {
        if self.exclude_sources.iter().any(|source| {
            !matches!(
                source.as_str(),
                "widget" | "automation" | "special" | "twin" | "weather"
            )
        }) {
            return Err("excludeSources contains an unsupported source");
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListOutput {
    /// Whitelisted control fields, revision and only the active Circadian curve preview.
    pub items: Vec<Value>,
    pub automation_names: BTreeMap<String, String>,
}
pub type NamesInput = ResourceIdsInput;
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct NamesOutput {
    pub names: BTreeMap<String, String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn list_requires_one_known_type_and_summary_filters_are_explicit() {
        for input in [
            r#"{}"#,
            r#"{"type":"all"}"#,
            r#"{"type":"data","history":true}"#,
        ] {
            assert!(serde_json::from_str::<ListInput>(input).is_err());
        }
        assert!(serde_json::from_str::<DefaultsInput>(r#"{"history":true}"#).is_err());
        for kind in ["data", "button", "circadian"] {
            let input: ListInput =
                serde_json::from_value(serde_json::json!({"type":kind})).unwrap();
            assert_eq!(input.r#type.as_str(), kind);
        }
        let input: SummaryInput =
            serde_json::from_str(r#"{"type":"data","excludeSources":["automation"]}"#).unwrap();
        input.validate().unwrap();
        assert!(SummaryInput {
            exclude_sources: vec!["typo".into()],
            ..Default::default()
        }
        .validate()
        .is_err());
    }
}
