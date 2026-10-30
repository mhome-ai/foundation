//! Explicit business timestamp paths and units. Never inspect arbitrary state/log values.
use chrono::{DateTime, Utc};
use serde_json::Value;
use std::sync::LazyLock;
static PATHS: LazyLock<Value> = LazyLock::new(|| {
    serde_json::from_str(include_str!("../../manifest/presentation-times.v1.json"))
        .expect("timestamp manifest")
});
pub fn requires_timezone(target: &str) -> bool {
    PATHS.get(target).is_some()
}
/// Use the Home IANA zone, falling back to UTC for missing or invalid configuration.
pub fn timezone(name: &str) -> chrono_tz::Tz {
    let area = name.split('/').next().unwrap_or("");
    if matches!(name, "UTC" | "GMT")
        || matches!(
            area,
            "Africa"
                | "America"
                | "Antarctica"
                | "Arctic"
                | "Asia"
                | "Atlantic"
                | "Australia"
                | "Europe"
                | "Indian"
                | "Pacific"
                | "Etc"
        )
    {
        name.parse().unwrap_or(chrono_tz::UTC)
    } else {
        chrono_tz::UTC
    }
}
pub fn format_millis(value: i64, home_timezone: &str) -> Option<String> {
    let zone = timezone(home_timezone);
    DateTime::<Utc>::from_timestamp_millis(value).map(|instant| {
        format!(
            "{} ({zone})",
            instant.with_timezone(&zone).format("%Y-%m-%d %H:%M:%S %:z")
        )
    })
}
pub fn localize(target: &str, data: &mut Value, home_timezone: &str) {
    let Some(units) = PATHS.get(target).and_then(Value::as_object) else {
        return;
    };
    for (unit, paths) in units {
        for path in paths.as_array().expect("timestamp paths") {
            visit(
                data,
                &path
                    .as_str()
                    .expect("timestamp path")
                    .split('.')
                    .collect::<Vec<_>>(),
                if unit == "sec" { 1000 } else { 1 },
                unit == "typedMs",
                home_timezone,
            );
        }
    }
}
fn visit(value: &mut Value, path: &[&str], multiplier: i64, typed: bool, zone: &str) {
    let Some((key, remaining)) = path.split_first() else {
        if typed {
            localize_typed(value, zone);
            return;
        }
        if let Some(timestamp) = value
            .as_i64()
            .filter(|v| *v > 0)
            .and_then(|v| v.checked_mul(multiplier))
            .and_then(|v| format_millis(v, zone))
        {
            *value = Value::String(timestamp);
        }
        return;
    };
    if *key == "*" {
        if let Some(items) = value.as_array_mut() {
            for item in items {
                visit(item, remaining, multiplier, typed, zone);
            }
        }
    } else if let Some(item) = value.get_mut(*key) {
        visit(item, remaining, multiplier, typed, zone);
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    #[test]
    fn explicit_paths_units_dst_and_payload_isolation() {
        let mut activity = json!({"activities":[{"timestamp":1730615400000_i64,"state":{"timestamp":1730615400000_i64}}]});
        localize(
            "/app/automation/activity/recent/list",
            &mut activity,
            "America/New_York",
        );
        assert_eq!(
            activity["activities"][0]["timestamp"],
            "2024-11-03 01:30:00 -05:00 (America/New_York)"
        );
        assert_eq!(
            activity["activities"][0]["state"]["timestamp"],
            1730615400000_i64
        );
        let mut widget = json!({"history":[{"timestamp":1730615400,"value":{"timestamp":1}},{"timestamp":null},{"timestamp":0}]});
        localize("/app/widget/history/list", &mut widget, "Asia/Shanghai");
        assert_eq!(
            widget["history"][0]["timestamp"],
            "2024-11-03 14:30:00 +08:00 (Asia/Shanghai)"
        );
        assert_eq!(widget["history"][0]["value"]["timestamp"], 1);
        assert!(widget["history"][1]["timestamp"].is_null());
        assert_eq!(widget["history"][2]["timestamp"], 0);
        assert_eq!(
            format_millis(1730615400000, "invalid"),
            format_millis(1730615400000, "UTC")
        );
    }
}

// Only explicitly typed business values are dates. Numeric values and durations stay intact.
fn localize_typed(value: &mut Value, zone: &str) {
    if let Some(items) = value.as_array_mut() {
        for item in items {
            localize_typed(item, zone);
        }
    } else if value["_type"] == "Timestamp" {
        if let Some(text) = value["value"]
            .as_i64()
            .and_then(|millis| format_millis(millis, zone))
        {
            value["value"] = Value::String(text);
        }
    }
}

#[cfg(test)]
mod widget_tests {
    use super::*;
    use serde_json::json;
    #[test]
    fn widget_dates_keep_units_zone_and_business_type_without_converting_numbers() {
        let mut value = json!({"items":[{"resource":{"nextTimestamp":1730615400000_i64,
          "value":[{"_type":"Timestamp","value":1730615400000_i64},
                   {"_type":"Duration","value":30000,"unit":"ms"},1730615400000_i64]}}]});
        localize("/app/widget/value/get", &mut value, "America/New_York");
        let resource = &value["items"][0]["resource"];
        assert_eq!(
            resource["nextTimestamp"],
            "2024-11-03 01:30:00 -05:00 (America/New_York)"
        );
        assert_eq!(resource["value"][0]["value"], resource["nextTimestamp"]);
        assert_eq!(resource["value"][1]["value"], 30000);
        assert_eq!(resource["value"][2], 1730615400000_i64);
    }
}
