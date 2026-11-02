use crate::ContractError;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Owned values at the native script boundary. Domain objects retain their
/// `_type`/unit/value or URI fields; adapters interpret them without losing units.
/// The tagged wire representation is only for process boundaries, not local calls.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(
    tag = "type",
    content = "value",
    rename_all = "camelCase",
    deny_unknown_fields
)]
pub enum ScriptValue {
    Null,
    Bool(bool),
    Number(f64),
    I64(#[serde(with = "signed")] i64),
    U64(#[serde(with = "unsigned")] u64),
    String(String),
    Bytes(#[serde(with = "binary")] Vec<u8>),
    List(Vec<ScriptValue>),
    Object(BTreeMap<String, ScriptValue>),
}

macro_rules! decimal {
    ($module:ident, $ty:ty) => {
        mod $module {
            use serde::{Deserialize, Deserializer, Serializer};
            pub fn serialize<S: Serializer>(value: &$ty, serializer: S) -> Result<S::Ok, S::Error> {
                serializer.serialize_str(&value.to_string())
            }
            pub fn deserialize<'de, D: Deserializer<'de>>(
                deserializer: D,
            ) -> Result<$ty, D::Error> {
                let text = String::deserialize(deserializer)?;
                let value: $ty = text.parse().map_err(serde::de::Error::custom)?;
                if value.to_string() != text {
                    return Err(serde::de::Error::custom(
                        "integer must use canonical decimal text",
                    ));
                }
                Ok(value)
            }
        }
    };
}
decimal!(signed, i64);
decimal!(unsigned, u64);

mod binary {
    use base64::{engine::general_purpose::STANDARD, Engine};
    use serde::{Deserialize, Deserializer, Serializer};
    pub fn serialize<S: Serializer>(value: &[u8], serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&STANDARD.encode(value))
    }
    pub fn deserialize<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Vec<u8>, D::Error> {
        STANDARD
            .decode(String::deserialize(deserializer)?)
            .map_err(serde::de::Error::custom)
    }
}

#[derive(Clone, Copy, Debug)]
pub struct ValueLimits {
    pub bytes: usize,
    pub nodes: usize,
    pub depth: usize,
}

impl Default for ValueLimits {
    fn default() -> Self {
        Self {
            bytes: 4 * 1024 * 1024,
            nodes: 65_536,
            depth: 64,
        }
    }
}

impl ScriptValue {
    /// Enforce aggregate bounds before allocation in the JS engine. HTTP adapters
    /// must also bound the incoming body before deserializing it.
    pub fn validate(&self, mut limits: ValueLimits) -> Result<(), ContractError> {
        self.check(&mut limits, 0)
    }

    fn check(&self, limits: &mut ValueLimits, depth: usize) -> Result<(), ContractError> {
        if depth > limits.depth || limits.nodes == 0 {
            return Err(ContractError(
                "script value exceeds node/depth limit".into(),
            ));
        }
        limits.nodes -= 1;
        let size = match self {
            Self::String(value) => value.len().saturating_add(8),
            Self::Bytes(value) => value.len().saturating_add(8),
            Self::Number(value) if !value.is_finite() => {
                return Err(ContractError("script number must be finite".into()));
            }
            _ => 8,
        };
        limits.bytes = limits
            .bytes
            .checked_sub(size)
            .ok_or_else(|| ContractError("script value exceeds byte limit".into()))?;
        match self {
            Self::List(values) => {
                for value in values {
                    value.check(limits, depth + 1)?;
                }
            }
            Self::Object(values) => {
                for (name, value) in values {
                    limits.bytes = limits
                        .bytes
                        .checked_sub(name.len())
                        .ok_or_else(|| ContractError("script value exceeds byte limit".into()))?;
                    value.check(limits, depth + 1)?;
                }
            }
            _ => {}
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn integer_wire_values_never_use_json_numbers_or_accept_rounding() {
        for value in [
            ScriptValue::I64(i64::MIN),
            ScriptValue::I64(i64::MAX),
            ScriptValue::U64(u64::MAX),
        ] {
            let json = serde_json::to_value(&value).unwrap();
            assert!(json["value"].is_string());
            assert_eq!(serde_json::from_value::<ScriptValue>(json).unwrap(), value);
        }
        for invalid in [
            r#"{"type":"u64","value":18446744073709551615}"#,
            r#"{"type":"i64","value":"01"}"#,
            r#"{"type":"i64","value":"+1"}"#,
            r#"{"type":"u64","value":"18446744073709551616"}"#,
        ] {
            assert!(serde_json::from_str::<ScriptValue>(invalid).is_err());
        }
    }

    #[test]
    fn values_have_aggregate_bounds_and_reject_nonfinite_numbers() {
        let value = ScriptValue::List(vec![ScriptValue::String("abcdef".into()); 3]);
        assert!(value
            .validate(ValueLimits {
                bytes: 25,
                ..ValueLimits::default()
            })
            .is_err());
        assert!(value
            .validate(ValueLimits {
                nodes: 3,
                ..ValueLimits::default()
            })
            .is_err());
        assert!(value
            .validate(ValueLimits {
                depth: 0,
                ..ValueLimits::default()
            })
            .is_err());
        assert!(value.validate(ValueLimits::default()).is_ok());
        assert!(ScriptValue::Number(f64::NAN)
            .validate(ValueLimits::default())
            .is_err());
    }
}
