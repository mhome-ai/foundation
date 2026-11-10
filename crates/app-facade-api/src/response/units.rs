//! Essential-only unit preferences. Untagged numbers and unsupported units are never guessed.
use serde_json::Value;
use std::collections::BTreeMap;

pub fn localize(value: &mut Value, preferences: &BTreeMap<String, String>) {
    if let (Some(kind), Some(unit), Some(number)) = (
        value["_type"].as_str(),
        value["unit"].as_str(),
        value["value"].as_f64(),
    ) {
        let preferred = |key: &str| preferences.get(key).map(String::as_str);
        let target = match kind {
            "Temperature" => preferred("temperature"),
            "TemperatureDiff" => {
                preferred("temperature").map(|u| if u == "F" { "dF" } else { "dC" })
            }
            "Length" => preferred("length").map(|u| if u == "imperial" { "foot" } else { "m" }),
            "Weight" => preferred("weight").map(|u| if u == "imperial" { "lb" } else { "kg" }),
            "Speed" => preferred("length").map(|u| if u == "imperial" { "mph" } else { "kmh" }),
            _ => None,
        };
        if let Some(target) = target.filter(|target| *target != unit) {
            if let Some(converted) = convert(kind, number, unit, target).filter(|n| n.is_finite()) {
                if let Some(number) = serde_json::Number::from_f64(converted) {
                    value["value"] = Value::Number(number);
                    value["unit"] = Value::String(target.into());
                }
            }
        }
    }
    match value {
        Value::Array(items) => {
            for item in items {
                localize(item, preferences);
            }
        }
        Value::Object(fields) => {
            for item in fields.values_mut() {
                localize(item, preferences);
            }
        }
        _ => {}
    }
}
fn factor(kind: &str, unit: &str) -> Option<f64> {
    match (kind, unit) {
        ("Length", "mm") => Some(0.001),
        ("Length", "cm") => Some(0.01),
        ("Length", "m") => Some(1.0),
        ("Length", "km") => Some(1000.0),
        ("Length", "inch") => Some(0.0254),
        ("Length", "foot") => Some(0.3048),
        ("Length", "yard") => Some(0.9144),
        ("Length", "mile") => Some(1609.344),
        ("Weight", "mg") => Some(0.000001),
        ("Weight", "g") => Some(0.001),
        ("Weight", "kg") => Some(1.0),
        ("Weight", "oz") => Some(0.0283495),
        ("Weight", "lb") => Some(0.453592),
        ("Speed", "kmh") => Some(1.0),
        ("Speed", "mph") => Some(1.609344),
        _ => None,
    }
}
fn convert(kind: &str, number: f64, from: &str, to: &str) -> Option<f64> {
    if kind == "Temperature" {
        let celsius = match from {
            "C" => number,
            "F" => (number - 32.0) * 5.0 / 9.0,
            "K" => number - 273.15,
            _ => return None,
        };
        return match to {
            "C" => Some(celsius),
            "F" => Some(celsius * 9.0 / 5.0 + 32.0),
            "K" => Some(celsius + 273.15),
            _ => None,
        };
    }
    if kind == "TemperatureDiff" {
        let celsius = match from {
            "dC" => number,
            "dF" => number * 5.0 / 9.0,
            _ => return None,
        };
        return match to {
            "dC" => Some(celsius),
            "dF" => Some(celsius * 9.0 / 5.0),
            _ => None,
        };
    }
    Some(number * factor(kind, from)? / factor(kind, to)?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    #[test]
    fn preferences_change_only_known_unit_values_and_do_not_mutate_full() {
        let full = json!({"temp":{"_type":"Temperature","value":20,"unit":"C"},
            "diff":{"_type":"TemperatureDiff","value":5,"unit":"dC"},
            "weight":{"_type":"Weight","value":1,"unit":"kg"},
            "distance":{"_type":"Length","value":0.3048,"unit":"m"},
            "speed":{"_type":"Speed","value":1.609344,"unit":"kmh"},"ordinary":20});
        let mut essential = full.clone();
        localize(
            &mut essential,
            &BTreeMap::from([
                ("temperature".into(), "F".into()),
                ("length".into(), "imperial".into()),
                ("weight".into(), "imperial".into()),
            ]),
        );
        assert_eq!(essential["temp"]["value"], 68.0);
        assert_eq!(essential["diff"]["value"], 9.0);
        assert_eq!(essential["distance"]["value"], 1.0);
        assert_eq!(essential["speed"]["value"], 1.0);
        assert_eq!(essential["ordinary"], 20);
        assert_eq!(full["temp"]["unit"], "C");
    }
}
