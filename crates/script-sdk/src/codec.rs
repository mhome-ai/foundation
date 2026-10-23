//! Existing MeowCore constant tags and little-endian payloads, with bounded decoding.
use crate::{Object, Quantity, ScriptError, ScriptResult, Timestamp, Value};
use chrono::{Datelike, NaiveDate, NaiveTime, Timelike};

#[derive(Clone, Copy, Debug)]
pub struct Limits {
    pub bytes: usize,
    pub depth: usize,
    pub values: usize,
}
impl Default for Limits {
    fn default() -> Self {
        Self {
            bytes: 1024 * 1024,
            depth: 64,
            values: 65536,
        }
    }
}
fn invalid(message: &str) -> ScriptError {
    ScriptError::new("INVALID_WIRE_VALUE", message)
}

pub fn encode(value: &Value, limits: Limits) -> Result<Vec<u8>, ScriptError> {
    let mut writer = Writer {
        out: Vec::new(),
        limits,
        remaining: limits.values,
    };
    writer.value(value, 0)?;
    Ok(writer.out)
}
pub fn decode(bytes: &[u8], limits: Limits) -> Result<Value, ScriptError> {
    if bytes.len() > limits.bytes {
        return Err(invalid("byte limit exceeded"));
    }
    let (value, used) = decode_prefix(bytes, limits)?;
    if used != bytes.len() {
        return Err(invalid("trailing bytes"));
    }
    Ok(value)
}
/// For instructions containing multiple constants; scripts must use strict `decode`.
pub fn decode_prefix(bytes: &[u8], limits: Limits) -> Result<(Value, usize), ScriptError> {
    let mut reader = Reader {
        bytes,
        pos: 0,
        limits,
        remaining: limits.values,
    };
    let value = reader.value(0)?;
    Ok((value, reader.pos))
}
fn budget(depth: usize, limits: Limits, remaining: &mut usize) -> Result<(), ScriptError> {
    if depth > limits.depth || *remaining == 0 {
        return Err(invalid("value/depth limit exceeded"));
    }
    *remaining -= 1;
    Ok(())
}
struct Writer {
    out: Vec<u8>,
    limits: Limits,
    remaining: usize,
}
impl Writer {
    fn put(&mut self, bytes: &[u8]) -> Result<(), ScriptError> {
        if bytes.len() > self.limits.bytes.saturating_sub(self.out.len()) {
            return Err(invalid("byte limit exceeded"));
        }
        self.out.extend_from_slice(bytes);
        Ok(())
    }
    fn sized(&mut self, tag: u8, len: usize) -> Result<(), ScriptError> {
        if len <= 255 {
            self.put(&[tag, len as u8])
        } else if len <= 65535 {
            self.put(&[tag | 0x40])?;
            self.put(&(len as u16).to_le_bytes())
        } else {
            let len = u32::try_from(len).map_err(|_| invalid("length overflow"))?;
            self.put(&[tag | 0x80])?;
            self.put(&len.to_le_bytes())
        }
    }
    fn value(&mut self, v: &Value, depth: usize) -> Result<(), ScriptError> {
        budget(depth, self.limits, &mut self.remaining)?;
        match v {
            Value::Null => self.put(&[0]),
            Value::Bool(b) => self.put(&[if *b { 2 } else { 3 }]),
            Value::Number(n) => {
                self.put(&[4])?;
                self.put(&n.to_le_bytes())
            }
            Value::String(s) => {
                self.sized(1, s.len())?;
                self.put(s.as_bytes())
            }
            Value::Timestamp(t) => {
                self.put(&[8])?;
                self.put(&t.epoch_millis.to_le_bytes())
            }
            Value::Duration(d) => {
                d.validate()?;
                self.put(&[10, 1, d.display_unit])?;
                self.put(&d.millis.to_le_bytes())
            }
            Value::Quantity(q) => {
                q.validate()?;
                self.put(&[10, q.kind, q.unit])?;
                self.put(&(q.value as f32).to_le_bytes())
            }
            Value::LocalTime(t) => {
                self.put(&[13, t.hour() as u8, t.minute() as u8, t.second() as u8])
            }
            Value::LocalDate(d) => {
                let year =
                    u16::try_from(d.year()).map_err(|_| invalid("date year outside wire range"))?;
                self.put(&[12, d.month() as u8, d.day() as u8])?;
                self.put(&year.to_le_bytes())
            }
            Value::SpecialTime {
                event,
                unit,
                offset_millis,
            } => {
                validate_special_time(*event, *unit)?;
                self.put(&[13, event | 128, *unit])?;
                self.put(&offset_millis.to_le_bytes())
            }
            Value::RelativeDate(v) => {
                validate_relative(*v)?;
                self.put(&[12, v | 64])
            }
            Value::SpecialDate {
                year,
                month,
                day,
                weekday,
            } => {
                validate_special_date(*month, *day, *weekday)?;
                self.put(&[12, month | 128, weekday.map_or(*day, |w| w | 128)])?;
                self.put(&year.to_le_bytes())
            }
            Value::Array(a) => {
                if a.len() > self.remaining {
                    return Err(invalid("value limit exceeded"));
                }
                self.sized(19, a.len())?;
                for v in a {
                    self.value(v, depth + 1)?;
                }
                Ok(())
            }
            Value::Object(o) => {
                if o.len() > self.remaining {
                    return Err(invalid("value limit exceeded"));
                }
                self.sized(16, o.len())?;
                for (k, v) in o {
                    let len = u8::try_from(k.len())
                        .map_err(|_| invalid("object key exceeds 255 UTF-8 bytes"))?;
                    self.put(&[len])?;
                    self.put(k.as_bytes())?;
                    self.value(v, depth + 1)?;
                }
                Ok(())
            }
        }
    }
}
struct Reader<'a> {
    bytes: &'a [u8],
    pos: usize,
    limits: Limits,
    remaining: usize,
}
impl<'a> Reader<'a> {
    fn read(&mut self, len: usize) -> Result<&'a [u8], ScriptError> {
        let end = self
            .pos
            .checked_add(len)
            .ok_or_else(|| invalid("length overflow"))?;
        if end > self.bytes.len() || end > self.limits.bytes {
            return Err(invalid("truncated value or byte limit exceeded"));
        }
        let v = &self.bytes[self.pos..end];
        self.pos = end;
        Ok(v)
    }
    fn byte(&mut self) -> Result<u8, ScriptError> {
        Ok(self.read(1)?[0])
    }
    fn fixed<const N: usize>(&mut self) -> Result<[u8; N], ScriptError> {
        Ok(self.read(N)?.try_into().unwrap())
    }
    fn len(&mut self, bits: u8) -> Result<usize, ScriptError> {
        match bits {
            0 => Ok(self.byte()? as usize),
            1 => Ok(u16::from_le_bytes(self.fixed()?) as usize),
            2 => Ok(u32::from_le_bytes(self.fixed()?) as usize),
            _ => Err(invalid("reserved length flags")),
        }
    }
    fn string(&mut self, len: usize) -> Result<String, ScriptError> {
        std::str::from_utf8(self.read(len)?)
            .map(str::to_owned)
            .map_err(|_| invalid("invalid UTF-8"))
    }
    fn value(&mut self, depth: usize) -> Result<Value, ScriptError> {
        budget(depth, self.limits, &mut self.remaining)?;
        let head = self.byte()?;
        let tag = head & 63;
        let bits = head >> 6;
        if bits != 0 && !matches!(tag, 1 | 18 | 16 | 19) {
            return Err(invalid("unexpected length flags"));
        }
        Ok(match tag {
            0 => Value::Null,
            2 => Value::Bool(true),
            3 => Value::Bool(false),
            4 => Value::Number(f64::from_le_bytes(self.fixed()?)),
            1 | 18 => {
                let len = self.len(bits)?;
                Value::String(self.string(len)?)
            }
            8 => Value::Timestamp(Timestamp {
                epoch_millis: i64::from_le_bytes(self.fixed()?),
            }),
            10 => {
                let kind = self.byte()?;
                let unit = self.byte()?;
                if kind == 1 {
                    let d = crate::Duration {
                        millis: i64::from_le_bytes(self.fixed()?),
                        display_unit: unit,
                    };
                    Value::Duration(d.validate()?)
                } else {
                    Quantity {
                        kind,
                        unit,
                        value: 0.0,
                    }
                    .validate()?;
                    let value = f32::from_le_bytes(self.fixed()?) as f64;
                    Value::Quantity(Quantity { kind, unit, value })
                }
            }
            13 => {
                let first = self.byte()?;
                if first & 128 != 0 {
                    let event = first & 127;
                    let unit = self.byte()?;
                    validate_special_time(event, unit)?;
                    Value::SpecialTime {
                        event,
                        unit,
                        offset_millis: i32::from_le_bytes(self.fixed()?),
                    }
                } else {
                    let m = self.byte()?;
                    let s = self.byte()?;
                    Value::LocalTime(
                        NaiveTime::from_hms_opt(first as u32, m as u32, s as u32)
                            .ok_or_else(|| invalid("invalid time"))?,
                    )
                }
            }
            12 => {
                let first = self.byte()?;
                if first & 128 != 0 {
                    let month = first & 127;
                    let d = self.byte()?;
                    let year = u16::from_le_bytes(self.fixed()?);
                    let weekday = if d & 128 != 0 { Some(d & 127) } else { None };
                    let day = if weekday.is_some() { 0 } else { d };
                    validate_special_date(month, day, weekday)?;
                    Value::SpecialDate {
                        year,
                        month,
                        day,
                        weekday,
                    }
                } else if first & 64 != 0 {
                    let day = first & 63;
                    validate_relative(day)?;
                    Value::RelativeDate(day)
                } else {
                    let day = self.byte()?;
                    let year = u16::from_le_bytes(self.fixed()?);
                    Value::LocalDate(
                        NaiveDate::from_ymd_opt(year as i32, first as u32, day as u32)
                            .ok_or_else(|| invalid("invalid date"))?,
                    )
                }
            }
            16 | 19 => {
                let len = self.len(bits)?;
                if len > self.remaining || len > self.bytes.len().saturating_sub(self.pos) {
                    return Err(invalid("collection length exceeds input or value limit"));
                }
                if tag == 19 {
                    let mut a = Vec::new();
                    for _ in 0..len {
                        a.push(self.value(depth + 1)?);
                    }
                    Value::Array(a)
                } else {
                    let mut o = Object::new();
                    for _ in 0..len {
                        let n = self.byte()? as usize;
                        let k = self.string(n)?;
                        if o.contains_key(&k) {
                            return Err(invalid("duplicate object key"));
                        }
                        o.insert(k, self.value(depth + 1)?);
                    }
                    Value::Object(o)
                }
            }
            _ => return Err(invalid("unknown constant tag")),
        })
    }
}
fn validate_relative(v: u8) -> Result<(), ScriptError> {
    if matches!(v, 0..=4 | 51..=54) {
        Ok(())
    } else {
        Err(invalid("invalid relative date"))
    }
}
fn validate_special_time(event: u8, unit: u8) -> Result<(), ScriptError> {
    if event <= 6 && matches!(unit, 0x20..=0x24) {
        Ok(())
    } else {
        Err(invalid("invalid special time"))
    }
}
fn validate_special_date(month: u8, day: u8, weekday: Option<u8>) -> Result<(), ScriptError> {
    if month <= 12 && day <= 31 && weekday.is_none_or(|w| w <= 7) {
        Ok(())
    } else {
        Err(invalid("invalid special date"))
    }
}

/// v1 + status (0 success, 1 business error) + one complete constant value.
pub fn encode_result(result: &ScriptResult, limits: Limits) -> Result<Vec<u8>, ScriptError> {
    let error_value;
    let (status, value) = match result {
        Ok(v) => (0, v),
        Err(e) => {
            error_value = Value::Object(
                crate::object! {"code" => e.code.clone(), "message" => e.message.clone()},
            );
            (1, &error_value)
        }
    };
    let body_limits = Limits {
        bytes: limits
            .bytes
            .checked_sub(2)
            .ok_or_else(|| invalid("result byte limit"))?,
        ..limits
    };
    let mut out = vec![1, status];
    out.extend(encode(value, body_limits)?);
    Ok(out)
}
pub fn decode_result(bytes: &[u8], limits: Limits) -> Result<ScriptResult, ScriptError> {
    if bytes.len() < 3 || bytes.len() > limits.bytes || bytes[0] != 1 {
        return Err(invalid("invalid result version/length"));
    }
    let v = decode(&bytes[2..], limits)?;
    match bytes[1] {
        0 => Ok(Ok(v)),
        1 => {
            let mut args = crate::Arguments::new(v)?;
            let code: String = args.take("code")?;
            let message: String = args.take("message")?;
            args.finish()?;
            Ok(Err(ScriptError::new(code, message)))
        }
        _ => Err(invalid("invalid result status")),
    }
}
