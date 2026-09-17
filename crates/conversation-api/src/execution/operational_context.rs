//! Structured operational-context boundary.

use async_trait::async_trait;

use serde::{Deserialize, Serialize};

use crate::execution::{ExternalError, InvocationContext};

/// Deployment-owned generation vector for independently cacheable context sections.
#[derive(Clone, Debug, Default, Eq, Hash, PartialEq, Serialize, Deserialize)]
pub struct ContextGeneration {
    pub user_scope: String,
    pub identity: String,
    pub memory: String,
    pub integration_guide: String,
    pub installed_integrations: String,
    pub scope_integrations: String,
}

impl ContextGeneration {
    /// Creates a vector whose sections share one deployment-owned token.
    #[must_use]
    pub fn new(value: impl Into<String>) -> Self {
        let value = value.into();
        Self {
            user_scope: value.clone(),
            identity: value.clone(),
            memory: value.clone(),
            integration_guide: value.clone(),
            installed_integrations: value.clone(),
            scope_integrations: value,
        }
    }

    /// Returns whether every independently cacheable section has a non-empty token.
    #[must_use]
    pub fn is_complete(&self) -> bool {
        [
            &self.user_scope,
            &self.identity,
            &self.memory,
            &self.integration_guide,
            &self.installed_integrations,
            &self.scope_integrations,
        ]
        .into_iter()
        .all(|value| !value.trim().is_empty())
    }
}

/// Notification that an App Facade mutation changed facts used by the Agent prompt.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ContextInvalidation {
    /// New effective generation after the mutation committed.
    pub generation: ContextGeneration,
}

/// Runtime facts loaded from the App Facade. This type deliberately contains data, not messages:
/// prompt text and rendering order are owned by the Agent implementation.
#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct OperationalContext {
    pub user_scope: String,
    pub identity: String,
    pub memory: String,
    pub integration_guide: String,
    pub installed_integrations: String,
    pub scope_integrations: String,
}

/// Delta returned for sections whose generation differs from the caller's cached vector.
#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct OperationalContextDelta {
    pub generation: ContextGeneration,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub user_scope: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub identity: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub memory: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub integration_guide: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub installed_integrations: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scope_integrations: Option<String>,
}

/// Loads typed facts used by the canonical Agent prompt.
#[async_trait]
pub trait ContextSource: Send + Sync {
    async fn load(&self, context: &InvocationContext) -> Result<OperationalContext, ExternalError>;
}
