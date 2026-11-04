//! Shared contracts for the TypeScript/JavaScript script migration.
//!
//! These models do not authorize an invocation or implement an interpreter.
//! Deserialize, validate, and then resolve ownership against trusted caller context.
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::fmt;

mod value;
pub use value::{ScriptValue, ValueLimits};

pub const CONTRACT_VERSION: u32 = 1;
pub const ARTIFACT_FORMAT: &str = "javascript-esm";
/// New instruction layout; the former Wasm 0x13 and 0x1c layouts are not aliases.
pub const SCRIPT_CALL_OPCODE: u8 = 0x1d;
pub const SCRIPT_CALL_OPERANDS: usize = 4;
pub const MAX_FUNCTIONS: usize = 100;
pub const MAX_SOURCE_BYTES: usize = 256 * 1024;
pub const MAX_YAML_BYTES: usize = 256 * 1024;
pub const MAX_ARTIFACT_BYTES: usize = 4 * 1024 * 1024;
pub const MAX_CONTRACT_DEPTH: usize = 64;
pub const MAX_CONTRACT_FIELDS: usize = 65_536;
pub const MAX_SAFE_VERSION: u64 = 9_007_199_254_740_991;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ContractError(pub String);

impl fmt::Display for ContractError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for ContractError {}

fn invalid(message: impl Into<String>) -> ContractError {
    ContractError(message.into())
}

pub fn is_identifier(value: &str, max_bytes: usize) -> bool {
    let mut bytes = value.bytes();
    value.len() <= max_bytes
        && bytes
            .next()
            .is_some_and(|b| b.is_ascii_alphabetic() || b == b'_')
        && bytes.all(|b| b.is_ascii_alphanumeric() || b == b'_')
}

pub fn validate_sha256(value: &str) -> Result<(), ContractError> {
    if value.len() != 64
        || !value
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    {
        return Err(invalid(
            "sha256 must contain exactly 64 lowercase hexadecimal digits",
        ));
    }
    Ok(())
}

fn validate_owner(value: &str) -> Result<(), ContractError> {
    if value.is_empty()
        || value.len() > 256
        || value.trim() != value
        || value.chars().any(char::is_control)
    {
        return Err(invalid(
            "resource owner must be nonempty bounded text without controls",
        ));
    }
    Ok(())
}

fn validate_version(value: u64) -> Result<(), ContractError> {
    if !(1..=MAX_SAFE_VERSION).contains(&value) {
        return Err(invalid("resource version must be a positive safe integer"));
    }
    Ok(())
}

/// Exact immutable resource identity. Tenant and Space are deliberately not caller fields.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub enum ScriptRef {
    Integration {
        package_id: String,
        version: u64,
        function: String,
        sha256: String,
    },
    Automation {
        automation_id: String,
        revision: u64,
        function: String,
        sha256: String,
    },
}

impl ScriptRef {
    pub fn validate(&self) -> Result<(), ContractError> {
        let (owner, version, function, sha256) = match self {
            Self::Integration {
                package_id,
                version,
                function,
                sha256,
            } => (package_id, *version, function, sha256),
            Self::Automation {
                automation_id,
                revision,
                function,
                sha256,
            } => (automation_id, *revision, function, sha256),
        };
        validate_owner(owner)?;
        validate_version(version)?;
        if !is_identifier(function, 128) {
            return Err(invalid(
                "function must be an ASCII identifier without a path or extension",
            ));
        }
        validate_sha256(sha256)
    }

    pub fn sha256(&self) -> &str {
        match self {
            Self::Integration { sha256, .. } | Self::Automation { sha256, .. } => sha256,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ScalarType {
    Bool,
    Number,
    String,
    Timestamp,
    Duration,
    Temperature,
    Image,
    Object,
    Value,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ListType {
    pub list: Box<FieldType>,
}

/// Scalar tags or recursively nested {"list": ...}. Objects remain intentionally dynamic.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum FieldType {
    Scalar(ScalarType),
    List(ListType),
}

impl FieldType {
    pub fn list(element: Self) -> Self {
        Self::List(ListType {
            list: Box::new(element),
        })
    }

    fn validate(&self, depth: usize) -> Result<(), ContractError> {
        if depth > MAX_CONTRACT_DEPTH {
            return Err(invalid("script contract exceeds the nesting limit"));
        }
        match self {
            Self::Scalar(_) => Ok(()),
            Self::List(list) => list.list.validate(depth + 1),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Field {
    pub name: String,
    #[serde(rename = "type")]
    pub ty: FieldType,
}

/// Build-generated metadata from defineScript. Every top-level field is required.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ScriptContract {
    pub version: u32,
    pub sdk_version: String,
    pub input: Vec<Field>,
    pub output: Vec<Field>,
}

impl ScriptContract {
    pub fn validate_input(&self, value: &ScriptValue) -> Result<(), ContractError> {
        self.validate()?;
        validate_fields(&self.input, value, NumberRepresentation::CoreInput)
    }

    pub fn validate_output(&self, value: &ScriptValue) -> Result<(), ContractError> {
        self.validate()?;
        validate_fields(&self.output, value, NumberRepresentation::JavaScriptOutput)
    }

    pub fn validate(&self) -> Result<(), ContractError> {
        if self.version != CONTRACT_VERSION {
            return Err(invalid("unsupported script contract version"));
        }
        validate_sdk_version(&self.sdk_version)?;
        for fields in [&self.input, &self.output] {
            if fields.len() > MAX_CONTRACT_FIELDS {
                return Err(invalid("script contract has too many fields"));
            }
            let mut names = BTreeSet::new();
            for field in fields {
                if !is_identifier(&field.name, 255) || !names.insert(&field.name) {
                    return Err(invalid(
                        "script contract field names must be unique identifiers",
                    ));
                }
                field.ty.validate(0)?;
            }
        }
        Ok(())
    }
}

// Core's safe i64 input becomes a JS Number. On output, I64/U64 came from
// JS BigInt, even when small: accepting it as t.number would hide a type error.
#[derive(Clone, Copy)]
enum NumberRepresentation {
    CoreInput,
    JavaScriptOutput,
}

fn validate_fields(
    fields: &[Field],
    value: &ScriptValue,
    numbers: NumberRepresentation,
) -> Result<(), ContractError> {
    let ScriptValue::Object(values) = value else {
        return Err(invalid("script input/output root must be an object"));
    };
    if fields.len() != values.len() {
        return Err(invalid(
            "script input/output fields must exactly match the contract",
        ));
    }
    for field in fields {
        let value = values
            .get(&field.name)
            .ok_or_else(|| invalid(format!("missing script field: {}", field.name)))?;
        if !matches_type(&field.ty, value, numbers) {
            return Err(invalid(format!(
                "script field does not match its contract: {}",
                field.name
            )));
        }
    }
    Ok(())
}

fn matches_type(ty: &FieldType, value: &ScriptValue, numbers: NumberRepresentation) -> bool {
    use ScriptValue as V;
    match ty {
        FieldType::List(element) => match value {
            V::List(values) => values
                .iter()
                .all(|value| matches_type(&element.list, value, numbers)),
            _ => false,
        },
        FieldType::Scalar(ty) => match ty {
            ScalarType::Value => true,
            ScalarType::Bool => matches!(value, V::Bool(_)),
            // t.number is JS Number. Exact integers needing BigInt require t.value.
            ScalarType::Number => match value {
                V::Number(n) => n.is_finite(),
                V::I64(n) => {
                    matches!(numbers, NumberRepresentation::CoreInput)
                        && n.unsigned_abs() <= MAX_SAFE_VERSION
                }
                _ => false,
            },
            ScalarType::String => matches!(value, V::String(_)),
            ScalarType::Object => matches!(value, V::Object(_)),
            ScalarType::Image => match value {
                V::Object(fields) => {
                    matches!(fields.get("uri"), Some(V::String(uri)) if !uri.is_empty())
                }
                _ => false,
            },
            ScalarType::Timestamp | ScalarType::Duration | ScalarType::Temperature => {
                let V::Object(fields) = value else {
                    return false;
                };
                let (tag, units): (&str, &[&str]) = match ty {
                    ScalarType::Timestamp => ("Timestamp", &[]),
                    ScalarType::Duration => ("Duration", &["ms", "sec", "min", "hour", "day"]),
                    _ => ("Temperature", &["C", "F", "K"]),
                };
                if !matches!(fields.get("_type"), Some(V::String(actual)) if actual == tag) {
                    return false;
                }
                if *ty == ScalarType::Timestamp {
                    matches!(fields.get("value"), Some(V::I64(_)))
                        || matches!(fields.get("value"), Some(V::U64(n)) if *n <= i64::MAX as u64)
                        || matches!(fields.get("value"), Some(V::Number(n)) if n.is_finite() && n.fract() == 0.0 && n.abs() <= MAX_SAFE_VERSION as f64)
                } else {
                    matches!(fields.get("unit"), Some(V::String(unit)) if units.contains(&unit.as_str()))
                        && fields.get("value").is_some_and(|v| {
                            matches_type(&FieldType::Scalar(ScalarType::Number), v, numbers)
                        })
                }
            }
        },
    }
}

pub fn validate_sdk_version(value: &str) -> Result<(), ContractError> {
    let parts: Vec<_> = value.split('.').collect();
    if value.len() > 64
        || parts.len() != 3
        || parts.iter().any(|part| {
            part.is_empty()
                || (part.len() > 1 && part.starts_with('0'))
                || !part.bytes().all(|b| b.is_ascii_digit())
        })
    {
        return Err(invalid(
            "sdkVersion must be an exact stable major.minor.patch version",
        ));
    }
    Ok(())
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ArtifactFormat {
    #[serde(rename = "javascript-esm")]
    JavascriptEsm,
}

/// Metadata and source must be bound by the same verified manifest.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ScriptArtifact {
    pub format: ArtifactFormat,
    pub sha256: String,
    pub size: usize,
    pub sdk_version: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub contract: Option<ScriptContract>,
}

impl ScriptArtifact {
    /// Verifies immutable source content. Callers still validate metadata against
    /// the authorized reference with `validate_for`, including on cache hits.
    pub fn verify_source<'a>(&self, bytes: &'a [u8]) -> Result<&'a str, ContractError> {
        use sha2::{Digest, Sha256};
        if !(1..=MAX_ARTIFACT_BYTES).contains(&self.size)
            || bytes.len() != self.size
            || format!("{:x}", Sha256::digest(bytes)) != self.sha256
        {
            return Err(invalid("Script content does not match its manifest"));
        }
        let source =
            std::str::from_utf8(bytes).map_err(|_| invalid("Script source is not UTF-8"))?;
        if source.contains('\0') {
            return Err(invalid("Invalid JavaScript source"));
        }
        Ok(source)
    }

    /// Checks metadata consistency, not caller authorization or the source digest itself.
    pub fn validate_for(&self, reference: &ScriptRef) -> Result<(), ContractError> {
        reference.validate()?;
        validate_sha256(&self.sha256)?;
        validate_sdk_version(&self.sdk_version)?;
        if self.sha256 != reference.sha256() || !(1..=MAX_ARTIFACT_BYTES).contains(&self.size) {
            return Err(invalid("script artifact identity or size mismatch"));
        }
        match (&self.contract, reference) {
            (Some(contract), ScriptRef::Automation { .. }) => {
                contract.validate()?;
                if contract.sdk_version != self.sdk_version {
                    return Err(invalid("script contract SDK does not match the artifact"));
                }
            }
            (None, ScriptRef::Integration { .. }) => {}
            _ => {
                return Err(invalid(
                    "only Automation artifacts carry a defineScript contract",
                ))
            }
        }
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ScriptError {
    pub code: String,
    pub message: String,
}

impl ScriptError {
    pub fn new(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
        }
    }
}

impl fmt::Display for ScriptError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.code, self.message)
    }
}

impl std::error::Error for ScriptError {}

/// Internal outcome only. TypeScript functions return data or throw ScriptError.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "camelCase", deny_unknown_fields)]
pub enum ScriptOutcome<T> {
    Success { data: T },
    Failure { error: ScriptError },
}
