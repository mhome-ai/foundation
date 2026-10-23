//! Rust-owned, flat Input/Output contracts. Nested objects remain dynamic.
use crate::{
    codec, object, Arguments, Duration, Image, Object, ScriptError, Temperature, Timestamp, Value,
};
use std::collections::BTreeSet;

pub const CONTRACT_VERSION: u32 = 1;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FieldType {
    Bool,
    Number,
    String,
    Timestamp,
    Duration,
    Temperature,
    Image,
    Object,
    Value,
    List(Box<FieldType>),
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Field {
    pub name: String,
    pub ty: FieldType,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScriptContract {
    pub sdk_version: String,
    pub input: Vec<Field>,
    pub output: Vec<Field>,
}

/// Implemented by `#[derive(ScriptInput)]` on a named-field struct.
/// Every field is required, including `Value` (which can explicitly be Null).
///
/// ```compile_fail
/// use mhome_script_sdk::prelude::*;
/// #[derive(ScriptInput)]
/// struct Input { optional: Option<f64> }
/// ```
/// ```compile_fail
/// use mhome_script_sdk::prelude::*;
/// #[derive(ScriptInput)]
/// struct Input(Vec<f64>);
/// ```
/// ```compile_fail
/// use mhome_script_sdk::prelude::*;
/// #[derive(ScriptInput)]
/// struct Input<T> { value: T }
/// ```
/// ```compile_fail
/// use mhome_script_sdk::prelude::*;
/// #[derive(ScriptInput)]
/// struct Input;
/// ```
/// ```compile_fail
/// use mhome_script_sdk::prelude::*;
/// #[derive(ScriptInput)]
/// enum Input { A, B }
/// ```
pub trait ScriptInput: Sized {
    fn fields() -> Vec<Field>;
    fn from_value(value: Value) -> Result<Self, ScriptError>;
}
/// Implemented by `#[derive(ScriptOutput)]`; the result is always an Object.
/// A business list is represented by a field such as `items: Vec<f64>`.
///
/// ```compile_fail
/// use mhome_script_sdk::prelude::*;
/// #[derive(ScriptOutput)]
/// struct Output { count: i64 }
/// ```
/// ```compile_fail
/// use mhome_script_sdk::prelude::*;
/// #[derive(ScriptOutput)]
/// struct Child { x: f64 }
/// #[derive(ScriptOutput)]
/// struct Output { child: Child } // Use Object for nested business objects.
/// ```
pub trait ScriptOutput {
    fn fields() -> Vec<Field>;
    fn into_value(self) -> Value;
}

mod sealed {
    pub trait Sealed {}
}
/// Supported field types only. Aliases and qualified paths resolve through Rust,
/// not through textual matching of source type names.
pub trait ScriptField: sealed::Sealed {
    fn field_type() -> FieldType;
}
macro_rules! field_types {
    ($($ty:ty => $variant:ident),* $(,)?) => { $(
        impl sealed::Sealed for $ty {}
        impl ScriptField for $ty {
            fn field_type() -> FieldType { FieldType::$variant }
        }
    )*};
}
field_types! {
    bool => Bool, f64 => Number, String => String,
    Timestamp => Timestamp, Duration => Duration, Temperature => Temperature,
    Image => Image, Object => Object, Value => Value,
}
impl<T: ScriptField> sealed::Sealed for Vec<T> {}
impl<T: ScriptField> ScriptField for Vec<T> {
    fn field_type() -> FieldType {
        FieldType::List(Box::new(T::field_type()))
    }
}

impl ScriptContract {
    pub fn for_script<I: ScriptInput, O: ScriptOutput>() -> Self {
        Self {
            sdk_version: crate::SDK_VERSION.into(),
            input: I::fields(),
            output: O::fields(),
        }
    }
    /// Versioned metadata uses the same bounded binary codec as script values.
    pub fn encode(&self, limits: codec::Limits) -> Result<Vec<u8>, ScriptError> {
        codec::encode(
            &object! {
                "version" => CONTRACT_VERSION as f64,
                "sdkVersion" => self.sdk_version.clone(),
                "input" => encode_fields(&self.input),
                "output" => encode_fields(&self.output),
            }
            .into(),
            limits,
        )
    }
    pub fn decode(bytes: &[u8], limits: codec::Limits) -> Result<Self, ScriptError> {
        Self::from_value(codec::decode(bytes, limits)?)
            .map_err(|e| ScriptError::new("INVALID_CONTRACT", e.message))
    }
    fn from_value(value: Value) -> Result<Self, ScriptError> {
        let mut root = Arguments::new(value)?;
        if root.take::<f64>("version")? != CONTRACT_VERSION as f64 {
            return Err(invalid("unsupported script contract version"));
        }
        let sdk_version: String = root.take("sdkVersion")?;
        if sdk_version.is_empty() || sdk_version.len() > 64 {
            return Err(invalid("invalid script SDK version"));
        }
        let input = decode_fields(root.take("input")?)?;
        let output = decode_fields(root.take("output")?)?;
        root.finish()?;
        Ok(Self {
            sdk_version,
            input,
            output,
        })
    }
}
fn encode_fields(fields: &[Field]) -> Vec<Value> {
    fields
        .iter()
        .map(|f| object! {"name" => f.name.clone(), "type" => f.ty.to_value()}.into())
        .collect()
}
fn decode_fields(fields: Vec<Value>) -> Result<Vec<Field>, ScriptError> {
    let mut names = BTreeSet::new();
    fields
        .into_iter()
        .map(|value| {
            let mut field = Arguments::new(value)?;
            let name: String = field.take("name")?;
            if name.is_empty() || name.len() > 255 || !names.insert(name.clone()) {
                return Err(invalid("empty, oversized or duplicate script field name"));
            }
            let ty = FieldType::from_value(field.take_value("type")?)?;
            field.finish()?;
            Ok(Field { name, ty })
        })
        .collect()
}
impl FieldType {
    fn to_value(&self) -> Value {
        match self {
            Self::List(element) => object! { "list" => element.to_value() }.into(),
            Self::Bool => "bool".into(),
            Self::Number => "number".into(),
            Self::String => "string".into(),
            Self::Timestamp => "timestamp".into(),
            Self::Duration => "duration".into(),
            Self::Temperature => "temperature".into(),
            Self::Image => "image".into(),
            Self::Object => "object".into(),
            Self::Value => "value".into(),
        }
    }
    fn from_value(value: Value) -> Result<Self, ScriptError> {
        if let Value::String(name) = value {
            return match name.as_str() {
                "bool" => Ok(Self::Bool),
                "number" => Ok(Self::Number),
                "string" => Ok(Self::String),
                "timestamp" => Ok(Self::Timestamp),
                "duration" => Ok(Self::Duration),
                "temperature" => Ok(Self::Temperature),
                "image" => Ok(Self::Image),
                "object" => Ok(Self::Object),
                "value" => Ok(Self::Value),
                _ => Err(invalid("unknown script field type")),
            };
        }
        let mut list = Arguments::new(value)?;
        let element = Self::from_value(list.take_value("list")?)?;
        list.finish()?;
        Ok(Self::List(Box::new(element)))
    }
}
fn invalid(message: &str) -> ScriptError {
    ScriptError::new("INVALID_CONTRACT", message)
}
