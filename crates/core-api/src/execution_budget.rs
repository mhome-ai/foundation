//! Transport metadata, independent of business inputs and Facade authorization control.
//! Each hop sends remaining milliseconds and anchors them to its own monotonic clock.
//! Zero means expired; absence preserves the receiver's existing local policy.
use serde::{Deserialize, Serialize};
use std::time::Duration;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "WireBudget", into = "WireBudget")]
pub struct ExecutionBudget(u32);

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct WireBudget {
    remaining_ms: u64,
}

impl TryFrom<WireBudget> for ExecutionBudget {
    type Error = &'static str;
    fn try_from(value: WireBudget) -> Result<Self, Self::Error> {
        if value.remaining_ms > i32::MAX as u64 {
            return Err("executionBudget.remainingMs exceeds its limit");
        }
        Ok(Self(value.remaining_ms as u32))
    }
}

impl From<ExecutionBudget> for WireBudget {
    fn from(value: ExecutionBudget) -> Self {
        Self {
            remaining_ms: value.0 as u64,
        }
    }
}

impl ExecutionBudget {
    /// Round down, never adding time at a transport boundary.
    pub fn from_remaining(remaining: Duration) -> Self {
        Self(remaining.as_millis().min(i32::MAX as u128) as u32)
    }

    pub fn remaining(self) -> Duration {
        Duration::from_millis(self.0 as u64)
    }
}

/// Adds an optional top-level field without changing the source API of existing envelopes.
/// The wrapped object must not itself define `executionBudget`.
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Budgeted<T> {
    #[serde(flatten)]
    pub message: T,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub execution_budget: Option<ExecutionBudget>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn optional_budget_round_trips_without_changing_the_business_envelope() {
        #[derive(Debug, Serialize, Deserialize, PartialEq)]
        struct Input {
            target: String,
        }
        let wire = r#"{"target":"/example","executionBudget":{"remainingMs":0}}"#;
        let decoded: Budgeted<Input> = serde_json::from_str(wire).unwrap();
        assert_eq!(
            decoded.execution_budget.unwrap().remaining(),
            Duration::ZERO
        );
        assert_eq!(
            serde_json::to_value(&decoded).unwrap(),
            serde_json::from_str::<serde_json::Value>(wire).unwrap()
        );
        let legacy: Budgeted<Input> = serde_json::from_str(r#"{"target":"/example"}"#).unwrap();
        assert!(legacy.execution_budget.is_none());
        assert!(serde_json::to_value(legacy)
            .unwrap()
            .get("executionBudget")
            .is_none());
    }

    #[test]
    fn invalid_budgets_fail_closed_and_rounding_never_extends_time() {
        for value in [
            r#"{}"#,
            r#"{"remainingMs":-1}"#,
            r#"{"remainingMs":1.5}"#,
            r#"{"remainingMs":2147483648}"#,
            r#"{"remainingMs":null}"#,
            r#"{"remainingMs":5,"extra":1}"#,
        ] {
            assert!(
                serde_json::from_str::<ExecutionBudget>(value).is_err(),
                "{value}"
            );
        }
        assert_eq!(
            ExecutionBudget::from_remaining(Duration::from_micros(1999)).remaining(),
            Duration::from_millis(1)
        );
    }
}
