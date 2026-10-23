//! Shared constant values and Automation Script ABI v1. No host IO is exposed.
pub mod codec;
pub mod guest;
mod value;
pub use value::*;

pub const ABI_VERSION: i32 = 1;
pub type ScriptResult = Result<Value, ScriptError>;

#[derive(Debug, Clone, PartialEq, Eq)]
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
impl std::fmt::Display for ScriptError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.code, self.message)
    }
}
impl std::error::Error for ScriptError {}
impl From<std::convert::Infallible> for ScriptError {
    fn from(v: std::convert::Infallible) -> Self {
        match v {}
    }
}

/// Used by generated wrappers, with exact names and types from the input contract.
pub struct Arguments(Object);
impl Arguments {
    pub fn new(value: Value) -> Result<Self, ScriptError> {
        match value {
            Value::Object(o) => Ok(Self(o)),
            _ => Err(ScriptError::new(
                "INVALID_ARGUMENT",
                "expected argument object",
            )),
        }
    }
    pub fn take<T: TryFrom<Value>>(&mut self, name: &str) -> Result<T, ScriptError>
    where
        T::Error: Into<ScriptError>,
    {
        self.0
            .remove(name)
            .ok_or_else(|| {
                ScriptError::new("INVALID_ARGUMENT", format!("missing argument {name}"))
            })?
            .try_into()
            .map_err(Into::into)
    }
    pub fn take_value(&mut self, name: &str) -> Result<Value, ScriptError> {
        self.0
            .remove(name)
            .ok_or_else(|| ScriptError::new("INVALID_ARGUMENT", format!("missing argument {name}")))
    }
    pub fn finish(self) -> Result<(), ScriptError> {
        if self.0.is_empty() {
            Ok(())
        } else {
            Err(ScriptError::new("INVALID_ARGUMENT", "unexpected arguments"))
        }
    }
}

#[macro_export]
macro_rules! object {
    ($($key:expr => $value:expr),* $(,)?) => {{
        let mut object = $crate::Object::new();
        $(object.insert($key.into(), $crate::Value::from($value));)*
        object
    }};
}
pub mod prelude {
    pub use crate::{
        object, Arguments, Duration, Image, Object, ScriptError, ScriptResult, Temperature,
        Timestamp, Value,
    };
}
