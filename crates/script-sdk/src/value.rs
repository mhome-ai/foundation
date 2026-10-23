use crate::ScriptError;
use chrono::{NaiveDate, NaiveTime};
use std::collections::BTreeMap;

pub type Object = BTreeMap<String, Value>;
#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Null,
    Bool(bool),
    Number(f64),
    String(String),
    Timestamp(Timestamp),
    Duration(Duration),
    LocalDate(NaiveDate),
    LocalTime(NaiveTime),
    SpecialDate {
        year: u16,
        month: u8,
        day: u8,
        weekday: Option<u8>,
    },
    RelativeDate(u8),
    SpecialTime {
        event: u8,
        unit: u8,
        offset_millis: i32,
    },
    Quantity(Quantity),
    Array(Vec<Value>),
    Object(Object),
}
/// Values are ALWAYS in base units; unit only controls display.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Quantity {
    pub kind: u8,
    pub unit: u8,
    pub value: f64,
}
impl Quantity {
    pub fn validate(self) -> Result<Self, ScriptError> {
        let valid = match self.kind {
            2 => matches!(self.unit, 1..=2),
            3 => matches!(self.unit, 4..=5),
            5 => matches!(self.unit, 0x10..=0x14),
            6 => matches!(self.unit, 0x30..=0x37),
            7 => matches!(self.unit, 0x40..=0x41),
            _ => false,
        };
        if valid {
            Ok(self)
        } else {
            Err(ScriptError::new(
                "INVALID_VALUE",
                "invalid quantity kind/unit",
            ))
        }
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Timestamp {
    pub epoch_millis: i64,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Duration {
    pub millis: i64,
    pub display_unit: u8,
}
impl Duration {
    pub fn validate(self) -> Result<Self, ScriptError> {
        if matches!(self.display_unit, 0x20..=0x24) {
            Ok(self)
        } else {
            Err(ScriptError::new("INVALID_VALUE", "invalid duration unit"))
        }
    }
    pub fn from_millis(millis: i64) -> Self {
        Self {
            millis,
            display_unit: 0x20,
        }
    }
}
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Temperature {
    pub celsius: f64,
    pub display_unit: u8,
}
impl Temperature {
    pub fn from_celsius(celsius: f64) -> Self {
        Self {
            celsius,
            display_unit: 2,
        }
    }
}
/// Artifact reference only. Reading bytes requires a later, scoped host API.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Image {
    pub uri: String,
}

impl Value {
    pub fn as_str(&self) -> Result<&str, ScriptError> {
        match self {
            Self::String(v) => Ok(v),
            _ => Err(type_error("String")),
        }
    }
    pub fn as_number(&self) -> Result<f64, ScriptError> {
        match self {
            Self::Number(v) => Ok(*v),
            _ => Err(type_error("Number")),
        }
    }
    pub fn get(&self, key: &str) -> Result<&Value, ScriptError> {
        match self {
            Self::Object(o) => o
                .get(key)
                .ok_or_else(|| ScriptError::new("MISSING_FIELD", key)),
            _ => Err(type_error("Object")),
        }
    }
}
fn type_error(expected: &str) -> ScriptError {
    ScriptError::new("INVALID_ARGUMENT", format!("expected {expected}"))
}
macro_rules! scalar {
    ($ty:ty, $variant:ident) => {
        impl From<$ty> for Value {
            fn from(v: $ty) -> Self {
                Self::$variant(v)
            }
        }
        impl TryFrom<Value> for $ty {
            type Error = ScriptError;
            fn try_from(v: Value) -> Result<Self, Self::Error> {
                match v {
                    Value::$variant(v) => Ok(v),
                    _ => Err(type_error(stringify!($variant))),
                }
            }
        }
    };
}
scalar!(f64, Number);
scalar!(bool, Bool);
scalar!(String, String);
scalar!(Object, Object);
scalar!(Timestamp, Timestamp);
impl From<&str> for Value {
    fn from(v: &str) -> Self {
        Self::String(v.into())
    }
}
impl<T: Into<Value>> From<Vec<T>> for Value {
    fn from(v: Vec<T>) -> Self {
        Self::Array(v.into_iter().map(Into::into).collect())
    }
}
impl<T: TryFrom<Value>> TryFrom<Value> for Vec<T>
where
    T::Error: Into<ScriptError>,
{
    type Error = ScriptError;
    fn try_from(v: Value) -> Result<Self, Self::Error> {
        match v {
            Value::Array(v) => v
                .into_iter()
                .map(|v| v.try_into().map_err(Into::into))
                .collect(),
            _ => Err(type_error("Array")),
        }
    }
}
impl From<Temperature> for Value {
    fn from(v: Temperature) -> Self {
        Self::Quantity(Quantity {
            kind: 2,
            unit: v.display_unit,
            value: v.celsius,
        })
    }
}
impl TryFrom<Value> for Temperature {
    type Error = ScriptError;
    fn try_from(v: Value) -> Result<Self, Self::Error> {
        match v {
            Value::Quantity(q) if q.kind == 2 => {
                q.validate()?;
                Ok(Self {
                    celsius: q.value,
                    display_unit: q.unit,
                })
            }
            _ => Err(type_error("Temperature")),
        }
    }
}
impl From<Duration> for Value {
    fn from(v: Duration) -> Self {
        Self::Duration(v)
    }
}
impl TryFrom<Value> for Duration {
    type Error = ScriptError;
    fn try_from(v: Value) -> Result<Self, Self::Error> {
        match v {
            Value::Duration(v) => {
                v.validate()?;
                Ok(v)
            }
            _ => Err(type_error("Duration")),
        }
    }
}
impl TryFrom<Value> for Image {
    type Error = ScriptError;
    fn try_from(v: Value) -> Result<Self, Self::Error> {
        Ok(Self {
            uri: v.get("uri")?.as_str()?.to_owned(),
        })
    }
}
impl From<Image> for Value {
    fn from(v: Image) -> Self {
        Self::Object(crate::object! { "uri" => v.uri })
    }
}
