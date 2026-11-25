//! Space-scoped Plugin routing. Business payloads belong to the destination Plugin.
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashSet;

pub const REQUEST_TARGET: &str = "/plugin/request";
pub const EVENT_TARGET: &str = "/plugin/event";
pub const INSTANCE_LIST_TARGET: &str = "/plugin/instance/list";
pub const ACCESS_DETAIL: &str = "pluginAccess";
pub const DEFAULT_TIMEOUT_MS: u64 = 30_000;
pub const MAX_TIMEOUT_MS: u64 = 60_000;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PluginAddress {
    pub node_type: String,
    pub node_id: String,
}

impl PluginAddress {
    pub fn validate(&self) -> Result<(), &'static str> {
        if self.node_type.is_empty()
            || self.node_type.len() > 128
            || !self
                .node_type
                .bytes()
                .all(|c| c.is_ascii_alphanumeric() || matches!(c, b'.' | b'-' | b'_'))
            || self.node_id.is_empty()
            || self.node_id.len() > 256
            || self
                .node_id
                .chars()
                .any(|c| c.is_control() || c.is_whitespace())
        {
            return Err("Invalid Plugin instance address");
        }
        Ok(())
    }
}

pub fn validate_target(target: &str) -> Result<(), &'static str> {
    if !target.starts_with("/plugin/")
        || target.len() > 512
        || target[1..].split('/').any(|s| {
            s.is_empty()
                || s == "."
                || s == ".."
                || !s
                    .bytes()
                    .all(|c| c.is_ascii_alphanumeric() || matches!(c, b'.' | b'-' | b'_'))
        })
    {
        return Err("Invalid Plugin route");
    }
    Ok(())
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PluginAccess {
    #[serde(default)]
    pub requests: Vec<String>,
    #[serde(default)]
    pub events: Vec<String>,
}

impl PluginAccess {
    /// A typed, optional extension of the existing authenticated Node description.
    pub fn from_description(description: &crate::NodeDescription) -> Result<Self, String> {
        let access: Self = description
            .details
            .get(ACCESS_DETAIL)
            .cloned()
            .map(serde_json::from_value)
            .transpose()
            .map_err(|e| format!("Invalid Plugin access: {e}"))?
            .unwrap_or_default();
        for routes in [&access.requests, &access.events] {
            if routes.len() > 256 {
                return Err("Too many Plugin routes".into());
            }
            let mut seen = HashSet::new();
            for route in routes {
                validate_target(route).map_err(str::to_owned)?;
                if !seen.insert(route) || !description.routes.contains(route) {
                    return Err(format!(
                        "Plugin access route is duplicated or unadvertised: {route}"
                    ));
                }
            }
        }
        Ok(access)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PluginRequest {
    pub to: PluginAddress,
    pub target: String,
    pub version: String,
    pub payload: Value,
    #[serde(default = "default_timeout")]
    pub timeout_ms: u64,
}

fn default_timeout() -> u64 {
    DEFAULT_TIMEOUT_MS
}

impl PluginRequest {
    pub fn validate(&self) -> Result<(), &'static str> {
        self.to.validate()?;
        validate_target(&self.target)?;
        validate_version(&self.version)?;
        if self.timeout_ms == 0 || self.timeout_ms > MAX_TIMEOUT_MS {
            return Err("Plugin timeout must be between 1 and 60000 milliseconds");
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PluginEvent {
    pub to: PluginAddress,
    pub target: String,
    pub version: String,
    pub payload: Value,
}

impl PluginEvent {
    pub fn validate(&self) -> Result<(), &'static str> {
        self.to.validate()?;
        validate_target(&self.target)?;
        validate_version(&self.version)
    }
}

pub fn validate_version(version: &str) -> Result<(), &'static str> {
    if version.is_empty()
        || version.len() > 64
        || !version
            .bytes()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, b'.' | b'-' | b'_'))
    {
        return Err("Invalid Plugin protocol version");
    }
    Ok(())
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PluginInstanceListRequest {
    pub node_type: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PluginInstance {
    pub address: PluginAddress,
    pub access: PluginAccess,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PluginInstanceList {
    pub instances: Vec<PluginInstance>,
}

/// Core-authenticated metadata, outside the business payload. Never accepted from a sender.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PluginCallContext {
    pub space_id: String,
    pub from: PluginAddress,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub timeout_ms: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum PluginControl {
    PluginCall(PluginCallContext),
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn request_rejects_spoofed_context_and_invalid_routing() {
        let input = json!({"to":{"nodeType":"dev.example.matter","nodeId":"matter-1"},
            "target":"/plugin/camera/source/list","version":"v1","payload":{"nested":[0,false,null]}});
        let request: PluginRequest = serde_json::from_value(input.clone()).unwrap();
        request.validate().unwrap();
        assert_eq!(request.timeout_ms, DEFAULT_TIMEOUT_MS);
        for key in ["from", "spaceId", "context", "control"] {
            let mut bad = input.clone();
            bad[key] = json!({});
            assert!(
                serde_json::from_value::<PluginRequest>(bad).is_err(),
                "{key}"
            );
        }
        for target in [
            "/app/plugin/remove",
            "/plugin/../remove",
            "/plugin//list",
            "/plugin/list?x=1",
            "/plugin/",
        ] {
            let mut bad = request.clone();
            bad.target = target.into();
            assert!(bad.validate().is_err(), "{target}");
        }
        for timeout in [0, MAX_TIMEOUT_MS + 1] {
            let mut bad = request.clone();
            bad.timeout_ms = timeout;
            assert!(bad.validate().is_err());
        }
    }

    #[test]
    fn directed_events_have_no_request_timeout_or_sender_identity() {
        let input = json!({"to":{"nodeType":"camera","nodeId":"c"},"target":"/plugin/source/changed","version":"v1","payload":{"revision":8}});
        serde_json::from_value::<PluginEvent>(input.clone())
            .unwrap()
            .validate()
            .unwrap();
        let mut bad = input;
        bad["timeoutMs"] = json!(1000);
        assert!(serde_json::from_value::<PluginEvent>(bad).is_err());
    }

    #[test]
    fn access_is_explicit_typed_and_bound_to_advertised_routes() {
        let mut description: crate::NodeDescription = serde_json::from_value(json!({
            "schemaVersion":crate::NODE_DESCRIBE_SCHEMA_VERSION,"nodeType":"example",
            "serviceId":"example","serviceVersion":"1.0.0","instancePolicy":"multiple",
            "routes":["/plugin/source/list","/plugin/source/changed"],"appAccess":{},
            "capabilities":[],"details":{}
        }))
        .unwrap();
        assert_eq!(
            PluginAccess::from_description(&description).unwrap(),
            PluginAccess::default()
        );
        description.details.insert(
            ACCESS_DETAIL.into(),
            json!({"requests":["/plugin/source/list"],"events":["/plugin/source/changed"]}),
        );
        let access = PluginAccess::from_description(&description).unwrap();
        assert_eq!(access.requests, ["/plugin/source/list"]);
        assert_eq!(access.events, ["/plugin/source/changed"]);
        for bad in [
            json!({"requests":["/plugin/missing"]}),
            json!({"requests":["/plugin/source/list","/plugin/source/list"]}),
            json!({"requests":["/app/plugin/remove"]}),
            json!({"request":[]}),
            Value::Null,
        ] {
            description
                .details
                .insert(ACCESS_DETAIL.into(), bad.clone());
            assert!(
                PluginAccess::from_description(&description).is_err(),
                "{bad}"
            );
        }
    }
}
