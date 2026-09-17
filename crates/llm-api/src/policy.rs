//! Pure invocation policy shared by deployments. No discovery or provider routing.
use crate::ModelConstraints;
use serde::{Deserialize, Serialize};

/// Confirmed model capabilities. Missing information never establishes support.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct ModelCapabilities {
    pub vision: bool,
    pub tool_calling: bool,
    pub structured_output: bool,
}

impl ModelConstraints {
    /// Checks declarations only, including requirements on requests with no images/tools.
    pub fn validate(&self, capabilities: &ModelCapabilities) -> Result<(), &'static str> {
        for (required, supported, name) in [
            (self.vision, capabilities.vision, "vision"),
            (self.tool_calling, capabilities.tool_calling, "tool_calling"),
            (
                self.structured_output,
                capabilities.structured_output,
                "structured_output",
            ),
        ] {
            if required && !supported {
                return Err(name);
            }
        }
        Ok(())
    }
}

/// Deployment-owned generation settings, never supplied through Agent constraints.
/// None leaves the setting unspecified. Adapters must apply every explicit value
/// or reject it; this basic validation does not establish model/provider support.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GenerationParameters {
    pub reasoning_effort: Option<String>,
    pub temperature: Option<f64>,
}

impl GenerationParameters {
    pub fn validate(&self) -> Result<(), &'static str> {
        if let Some(effort) = self.reasoning_effort.as_deref() {
            if !matches!(
                effort,
                "none" | "minimal" | "low" | "medium" | "high" | "xhigh" | "max" | "ultra"
            ) {
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
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn declarations_are_requirements_not_prohibitions() {
        let supported = ModelCapabilities {
            vision: true,
            tool_calling: true,
            structured_output: true,
        };
        assert!(ModelConstraints::default().validate(&supported).is_ok());
        assert!(ModelConstraints::default()
            .validate(&ModelCapabilities::default())
            .is_ok());
        for constraints in [
            ModelConstraints {
                vision: true,
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
    fn retired_generation_control_is_not_silently_ignored() {
        assert!(serde_json::from_value::<ModelConstraints>(serde_json::json!({
            "vision": false, "tool_calling": false, "structured_output": false, "reasoning": "high"
        })).is_err());
    }
    #[test]
    fn validates_explicit_generation_settings() {
        for effort in ["none", "low", "high", "max", "ultra"] {
            assert!(GenerationParameters {
                reasoning_effort: Some(effort.into()),
                temperature: None
            }
            .validate()
            .is_ok());
        }
        assert!(GenerationParameters {
            reasoning_effort: Some("unknown".into()),
            temperature: None
        }
        .validate()
        .is_err());
        for temperature in [-1.0, f64::NAN, f64::INFINITY] {
            assert!(GenerationParameters {
                temperature: Some(temperature),
                ..Default::default()
            }
            .validate()
            .is_err());
        }
    }
}
