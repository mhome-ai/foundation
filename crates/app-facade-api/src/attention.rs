//! Deterministic, viewer-specific household attention contract.
use serde::{Deserialize, Serialize};

pub const LIST_TARGET: &str = "/app/now/attention/list";
pub const POLICY_JSON: &str = include_str!("../manifest/attention-policy.v1.json");

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "mode", rename_all = "camelCase", deny_unknown_fields)]
pub enum AttentionTime {
    Decay {
        #[serde(rename = "occurredAt")]
        occurred_at: i64,
        #[serde(rename = "halfLifeMs")]
        half_life_ms: i64,
    },
    Window {
        start: i64,
        peak: i64,
        deadline: i64,
    },
    Persistent,
}

impl AttentionTime {
    pub fn relevance(&self, now: i64) -> f64 {
        match *self {
            Self::Decay {
                occurred_at,
                half_life_ms,
            } if occurred_at > 0 && half_life_ms > 0 && now >= occurred_at => {
                2_f64.powf(-((now - occurred_at) as f64) / half_life_ms as f64)
            }
            Self::Window {
                start,
                peak,
                deadline,
            } if start >= 0
                && start < peak
                && peak < deadline
                && now >= start
                && now < deadline =>
            {
                ((now - start) as f64 / (peak - start) as f64).min(1.0)
            }
            Self::Persistent => 1.0,
            _ => 0.0,
        }
    }

    pub fn next_boundary(&self, now: i64) -> Option<i64> {
        match *self {
            Self::Window {
                start,
                peak,
                deadline,
            } => [start, peak, deadline]
                .into_iter()
                .filter(|t| *t > now)
                .min(),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "kind", rename_all = "camelCase", deny_unknown_fields)]
pub enum AttentionAssociation {
    Household,
    Personal {
        #[serde(rename = "ownerUserId")]
        owner_user_id: String,
    },
}

impl AttentionAssociation {
    pub fn relation(&self, user_id: Option<&str>) -> f64 {
        match self {
            Self::Household => 0.8,
            Self::Personal { owner_user_id }
                if !owner_user_id.is_empty() && user_id == Some(owner_user_id.as_str()) =>
            {
                1.0
            }
            _ => 0.0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AttentionRule {
    pub impact: f64,
    pub necessity: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AttentionPolicy {
    pub version: String,
    pub threshold: f64,
    pub impact_weight: f64,
    pub necessity_weight: f64,
    pub default_half_life_ms: i64,
    pub refresh_interval_ms: i64,
    pub rules: std::collections::BTreeMap<String, AttentionRule>,
}

impl AttentionPolicy {
    pub fn builtin() -> &'static Self {
        static POLICY: std::sync::OnceLock<AttentionPolicy> = std::sync::OnceLock::new();
        POLICY.get_or_init(|| serde_json::from_str(POLICY_JSON).expect("valid attention policy"))
    }

    pub fn score(
        &self,
        reason: &str,
        eligible: bool,
        association: &AttentionAssociation,
        user_id: Option<&str>,
        time: &AttentionTime,
        now: i64,
    ) -> f64 {
        let Some(rule) = self.rules.get(reason) else {
            return 0.0;
        };
        if !eligible {
            return 0.0;
        }
        let score = 100.0
            * association.relation(user_id)
            * time.relevance(now)
            * (self.impact_weight * rule.impact + self.necessity_weight * rule.necessity);
        if score.is_finite() {
            (score.clamp(0.0, 100.0) * 10_000.0).round() / 10_000.0
        } else {
            0.0
        }
    }
}

/// Business meaning, distinct from notification recipients. Producers may omit this only
/// when they cannot establish personal ownership or a more specific time model.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AttentionContext {
    pub reason_code: String,
    pub association: AttentionAssociation,
    pub time: AttentionTime,
}

impl AttentionContext {
    pub fn valid_for_dialog(&self) -> bool {
        if !matches!(
            self.reason_code.as_str(),
            "dialog.pending" | "task.userDecision" | "schedule.decisionRequired"
        ) {
            return false;
        }
        if matches!(&self.association, AttentionAssociation::Personal { owner_user_id } if owner_user_id.trim().is_empty())
        {
            return false;
        }
        match self.time {
            AttentionTime::Persistent => true,
            AttentionTime::Decay {
                occurred_at,
                half_life_ms,
            } => occurred_at > 0 && half_life_ms > 0,
            AttentionTime::Window {
                start,
                peak,
                deadline,
            } => start >= 0 && start < peak && peak < deadline,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AttentionSource {
    pub kind: String,
    pub id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub capability_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AttentionItem {
    pub id: String,
    pub reason_code: String,
    pub title: String,
    pub summary: String,
    pub source: AttentionSource,
    pub score: f64,
    pub time: AttentionTime,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub valid_until: Option<i64>,
    pub source_updated_at: i64,
    pub verification_required: bool,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ListAttentionInput {}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AttentionSourceError {
    pub source: String,
    pub code: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ListAttentionOutput {
    pub items: Vec<AttentionItem>,
    pub evaluated_at: i64,
    pub next_refresh_at: i64,
    pub policy_version: String,
    pub source_errors: Vec<AttentionSourceError>,
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn shared_conformance() {
        let cases: serde_json::Value =
            serde_json::from_str(include_str!("../fixtures/attention/conformance.json")).unwrap();
        for case in cases.as_array().unwrap() {
            let association = serde_json::from_value(case["association"].clone()).unwrap();
            let time = serde_json::from_value(case["time"].clone()).unwrap();
            let score = AttentionPolicy::builtin().score(
                case["reasonCode"].as_str().unwrap(),
                case["eligible"].as_bool().unwrap(),
                &association,
                case["userId"].as_str(),
                &time,
                case["now"].as_i64().unwrap(),
            );
            assert!(
                (score - case["score"].as_f64().unwrap()).abs() < 0.0001,
                "{}: {score}",
                case["name"]
            );
        }
    }
    #[test]
    fn family_is_exactly_eighty_percent_and_recipients_are_not_owners() {
        let policy = AttentionPolicy::builtin();
        let personal = AttentionAssociation::Personal {
            owner_user_id: "me".into(),
        };
        let score = |association: &AttentionAssociation, user| {
            policy.score(
                "dialog.pending",
                true,
                association,
                user,
                &AttentionTime::Persistent,
                1,
            )
        };
        assert_eq!(score(&personal, Some("me")), 62.5);
        assert_eq!(score(&AttentionAssociation::Household, Some("me")), 50.0);
        assert_eq!(score(&personal, Some("other")), 0.0);
        assert_eq!(score(&personal, None), 0.0);
    }
}
