use mhome_script_api::{ScriptValue, ValueLimits};
use rquickjs::{Array, BigInt, Coerced, Ctx, Exception, FromJs, IntoJs, Object, TypedArray, Value};
use std::collections::BTreeMap;

const MAX_SAFE: i64 = 9_007_199_254_740_991;

pub(crate) fn to_js<'js>(ctx: &Ctx<'js>, value: &ScriptValue) -> rquickjs::Result<Value<'js>> {
    match value {
        ScriptValue::Null => Ok(Value::new_null(ctx.clone())),
        ScriptValue::Bool(v) => v.into_js(ctx),
        ScriptValue::Number(v) => v.into_js(ctx),
        ScriptValue::I64(v) if (-MAX_SAFE..=MAX_SAFE).contains(v) => (*v as f64).into_js(ctx),
        ScriptValue::I64(v) => BigInt::from_i64(ctx.clone(), *v).map(BigInt::into_value),
        ScriptValue::U64(v) => BigInt::from_u64(ctx.clone(), *v).map(BigInt::into_value),
        ScriptValue::String(v) => v.into_js(ctx),
        ScriptValue::Bytes(v) => {
            TypedArray::<u8>::new_copy(ctx.clone(), v).map(TypedArray::into_value)
        }
        ScriptValue::List(values) => {
            let result = Array::new(ctx.clone())?;
            for (index, value) in values.iter().enumerate() {
                result.set(index, to_js(ctx, value)?)?;
            }
            Ok(result.into_value())
        }
        ScriptValue::Object(values) => {
            // Null prototype prevents __proto__ from becoming a setter or inherited key.
            let result = Object::new_proto(ctx.clone(), None)?;
            for (name, value) in values {
                result.set(name.as_str(), to_js(ctx, value)?)?;
            }
            Ok(result.into_value())
        }
    }
}

pub(crate) fn from_js(value: Value<'_>, limits: ValueLimits) -> rquickjs::Result<ScriptValue> {
    Reader {
        remaining: limits,
        ancestors: Vec::new(),
    }
    .read(value, 0)
}

struct Reader<'js> {
    remaining: ValueLimits,
    ancestors: Vec<Value<'js>>,
}

impl<'js> Reader<'js> {
    fn bytes(&mut self, ctx: &Ctx<'js>, count: usize) -> rquickjs::Result<()> {
        self.remaining.bytes = self
            .remaining
            .bytes
            .checked_sub(count)
            .ok_or_else(|| Exception::throw_range(ctx, "script value exceeds byte limit"))?;
        Ok(())
    }

    fn read(&mut self, value: Value<'js>, depth: usize) -> rquickjs::Result<ScriptValue> {
        let ctx = value.ctx().clone();
        if depth > self.remaining.depth || self.remaining.nodes == 0 {
            return Err(Exception::throw_range(
                &ctx,
                "script value exceeds node/depth limit",
            ));
        }
        self.remaining.nodes -= 1;
        self.bytes(&ctx, 8)?;
        if value.is_null() {
            return Ok(ScriptValue::Null);
        }
        if let Some(v) = value.as_bool() {
            return Ok(ScriptValue::Bool(v));
        }
        if let Some(v) = value.as_number() {
            if !v.is_finite() {
                return Err(Exception::throw_type(&ctx, "script number must be finite"));
            }
            return Ok(ScriptValue::Number(v));
        }
        if let Some(v) = value.as_string() {
            let text = v.to_string()?;
            self.bytes(&ctx, text.len())?;
            return Ok(ScriptValue::String(text));
        }
        if value.is_big_int() {
            // JS_ToInt64Ext wraps large BigInts. Decimal parsing checks the range instead.
            let text = Coerced::<String>::from_js(&ctx, value)?.0;
            return text
                .parse::<i64>()
                .map(ScriptValue::I64)
                .or_else(|_| text.parse::<u64>().map(ScriptValue::U64))
                .map_err(|_| Exception::throw_range(&ctx, "BigInt exceeds i64/u64 range"));
        }
        if value.is_function() || value.is_promise() || value.is_proxy() || value.is_error() {
            return Err(Exception::throw_type(
                &ctx,
                "script result must contain data values",
            ));
        }
        if let Some(object) = value.as_object() {
            if object.is_typed_array::<u8>() {
                let array = TypedArray::<u8>::from_object(object.clone())?;
                let bytes = array
                    .as_raw()
                    .ok_or_else(|| Exception::throw_type(&ctx, "byte buffer is detached"))?;
                self.bytes(&ctx, bytes.len())?;
                // SAFETY: no JS is executed while this slice exists; the runtime
                // is thread-local and host buffers are copied, never shared with JS.
                return Ok(ScriptValue::Bytes(unsafe { bytes.as_ref() }.to_vec()));
            }
        }
        if let Some(array) = value.as_array() {
            if array.len() > self.remaining.nodes {
                return Err(Exception::throw_range(
                    &ctx,
                    "script array exceeds node limit",
                ));
            }
            self.enter(&value)?;
            let result = array
                .iter::<Value>()
                .map(|v| self.read(v?, depth + 1))
                .collect::<rquickjs::Result<Vec<_>>>()?;
            self.ancestors.pop();
            return Ok(ScriptValue::List(result));
        }
        if let Some(object) = value.as_object() {
            // Date, Map, Set, typed arrays and class instances must be explicitly
            // converted by the SDK. Do not silently turn them into empty objects.
            if let Some(proto) = object.get_prototype() {
                if proto.get_prototype().is_some() {
                    return Err(Exception::throw_type(
                        &ctx,
                        "script result must contain plain objects",
                    ));
                }
            }
            self.enter(&value)?;
            let mut result = BTreeMap::new();
            for property in object.props::<String, Value>() {
                let (name, value) = property?;
                self.bytes(&ctx, name.len())?;
                result.insert(name, self.read(value, depth + 1)?);
            }
            self.ancestors.pop();
            return Ok(ScriptValue::Object(result));
        }
        Err(Exception::throw_type(
            &ctx,
            "undefined and symbols are not script data",
        ))
    }

    fn enter(&mut self, value: &Value<'js>) -> rquickjs::Result<()> {
        if self.ancestors.contains(value) {
            return Err(Exception::throw_type(
                value.ctx(),
                "cyclic script data is not supported",
            ));
        }
        self.ancestors.push(value.clone());
        Ok(())
    }
}
