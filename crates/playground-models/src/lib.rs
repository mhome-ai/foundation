mod laundry_models;
pub use laundry_models::{DRYER_V1, WASHER_DRYER_V1, WASHER_V1};
mod humidistat_model;
pub use humidistat_model::HUMIDISTAT_V1;
mod chime_model;
pub use chime_model::CHIME_V1;
mod alarm_models;
pub use alarm_models::{DEVICE_ALARM_V1, SECURITY_SYSTEM_V1, SIREN_V1};

use anyhow::{anyhow, ensure, Result};
use serde::{Deserialize, Serialize};
use serde_json::{json, Map, Value};

pub const SPEAKER_MEDIA_BROADCAST_V1: &str = "playground.speaker.media-broadcast.v1";
pub const LIGHT_RGB_DIMMER_V1: &str = "playground.light.rgb-dimmer.v1";
pub const PLUG_ENERGY_V1: &str = "playground.plug.energy.v1";
pub const COVER_SHADE_V1: &str = "playground.cover.shade.v1";
pub const COVER_GARAGE_DOOR_V1: &str = "playground.cover.garage-door.v1";
pub const LOCK_DEADBOLT_V1: &str = "playground.lock.deadbolt.v1";
pub const CONTACT_SENSOR_V1: &str = "playground.sensor.contact.v1";
pub const MOTION_SENSOR_V1: &str = "playground.sensor.motion.v1";
pub const ILLUMINANCE_SENSOR_V1: &str = "playground.sensor.illuminance.v1";
pub const PRESENCE_SENSOR_V1: &str = "playground.sensor.presence.v1";
pub const LEAK_SENSOR_V1: &str = "playground.sensor.leak.v1";
pub const AIR_MONITOR_V1: &str = "playground.sensor.air-monitor.v1";
pub const SMOKE_CO_ALARM_V1: &str = "playground.alarm.smoke-co.v1";
pub const THERMOSTAT_FAN_V1: &str = "playground.thermostat.fan.v1";
pub const FAN_PURIFIER_V1: &str = "playground.fan.air-purifier.v1";
pub const BUTTON_SCENE_V1: &str = "playground.button.scene.v1";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceInstance {
    pub id: String,
    #[serde(rename = "displayName")]
    pub display_name: String,
    #[serde(rename = "modelId")]
    pub model_id: String,
    #[serde(default)]
    pub attributes: Map<String, Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandHistoryEntry {
    pub id: i64,
    #[serde(rename = "deviceId")]
    pub device_id: String,
    pub capability: String,
    pub request: Value,
    pub response: Value,
    pub status: String,
    #[serde(rename = "createdAt")]
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct DeviceSnapshot {
    pub id: String,
    #[serde(rename = "displayName")]
    pub display_name: String,
    #[serde(rename = "modelId")]
    pub model_id: String,
    pub capabilities: Vec<String>,
    pub attributes: Map<String, Value>,
    #[serde(rename = "capabilityData")]
    pub capability_data: Map<String, Value>,
    #[serde(rename = "commandHistory")]
    pub command_history: Vec<CommandHistoryEntry>,
}

#[derive(Debug, Clone)]
pub struct CommandEvent {
    pub capability: String,
    pub payload: Value,
}

#[derive(Debug, Clone)]
pub struct CommandOutcome {
    pub response: Value,
    pub events: Vec<CommandEvent>,
}

impl CommandOutcome {
    pub fn no_response() -> Self {
        Self {
            response: Value::Null,
            events: Vec::new(),
        }
    }

    pub fn with_event(capability: impl Into<String>, payload: Value) -> Self {
        Self {
            response: Value::Null,
            events: vec![CommandEvent {
                capability: capability.into(),
                payload,
            }],
        }
    }
}

pub fn known_model_ids() -> &'static [&'static str] {
    &[
        WASHER_V1,
        DRYER_V1,
        WASHER_DRYER_V1,
        HUMIDISTAT_V1,
        CHIME_V1,
        DEVICE_ALARM_V1,
        SIREN_V1,
        SECURITY_SYSTEM_V1,
        LIGHT_RGB_DIMMER_V1,
        PLUG_ENERGY_V1,
        COVER_SHADE_V1,
        COVER_GARAGE_DOOR_V1,
        LOCK_DEADBOLT_V1,
        CONTACT_SENSOR_V1,
        MOTION_SENSOR_V1,
        ILLUMINANCE_SENSOR_V1,
        PRESENCE_SENSOR_V1,
        LEAK_SENSOR_V1,
        AIR_MONITOR_V1,
        SMOKE_CO_ALARM_V1,
        THERMOSTAT_FAN_V1,
        FAN_PURIFIER_V1,
        SPEAKER_MEDIA_BROADCAST_V1,
        BUTTON_SCENE_V1,
    ]
}

pub fn ensure_known_model(model_id: &str) -> Result<()> {
    if known_model_ids().contains(&model_id) {
        Ok(())
    } else {
        Err(anyhow!("unknown playground modelId: {}", model_id))
    }
}

pub fn default_attributes(model_id: &str) -> Result<Map<String, Value>> {
    if laundry_models::supported(model_id) {
        return Ok(laundry_models::defaults(model_id));
    }
    if model_id == HUMIDISTAT_V1 {
        return Ok(humidistat_model::defaults());
    }
    if model_id == CHIME_V1 {
        return Ok(chime_model::defaults());
    }
    if let Some(value) = alarm_models::defaults(model_id) {
        return Ok(value);
    }
    match model_id {
        LIGHT_RGB_DIMMER_V1 => object_map(json!({
            "online": true,
            "on": false,
            "brightness": 55,
            "colorTemperature": 3200,
            "minColorTemperature": 2200,
            "maxColorTemperature": 6500,
            "colorHue": 35,
            "colorSaturation": 70,
            "power": 6.4,
            "importedEnergy": 1.8
        })),
        PLUG_ENERGY_V1 => object_map(json!({
            "online": true,
            "on": false,
            "power": 0,
            "importedEnergy": 1.2
        })),
        COVER_SHADE_V1 => object_map(json!({
            "online": true,
            "coverState": "closed",
            "coverMotionState": "stopped",
            "coverPosition": 0,
            "coverDeviceType": "shade",
            "canSetPosition": true,
            "canGetPosition": true
        })),
        COVER_GARAGE_DOOR_V1 => object_map(json!({
            "online": true,
            "coverState": "closed",
            "coverMotionState": "stopped",
            "coverPosition": 0,
            "coverDeviceType": "garageDoor",
            "canSetPosition": true,
            "canGetPosition": true,
            "contactState": "closed"
        })),
        LOCK_DEADBOLT_V1 => object_map(json!({
            "online": true,
            "lockState": "locked",
            "batteryLevel": 88,
            "chargingState": "not charging"
        })),
        CONTACT_SENSOR_V1 => object_map(json!({
            "online": true,
            "contactState": "closed",
            "batteryLevelState": "normal",
            "tamperState": "clear"
        })),
        MOTION_SENSOR_V1 => object_map(json!({
            "online": true,
            "motionState": "no motion",
            "batteryLevel": 84,
            "chargingState": "not charging",
            "tamperState": "clear"
        })),
        ILLUMINANCE_SENSOR_V1 => object_map(json!({
            "online": true,
            "illuminanceLux": 120,
            "batteryLevel": 86,
            "chargingState": "not charging"
        })),
        PRESENCE_SENSOR_V1 => object_map(json!({
            "online": true,
            "presenceState": "not present",
            "batteryLevel": 77,
            "chargingState": "not charging"
        })),
        LEAK_SENSOR_V1 => object_map(json!({
            "online": true,
            "waterState": "dry",
            "batteryLevel": 93,
            "chargingState": "not charging",
            "tamperState": "clear"
        })),
        AIR_MONITOR_V1 => object_map(json!({
            "online": true,
            "temperature": temperature(22.4, "C"),
            "humidity": 45,
            "qualityState": "good",
            "index": 32,
            "co2": 520,
            "co2State": "low",
            "pm25": 7,
            "pm25State": "low",
            "vocPpb": 180,
            "vocState": "low",
            "batteryLevel": 100,
            "chargingState": "full"
        })),
        SMOKE_CO_ALARM_V1 => object_map(json!({
            "online": true,
            "smokeState": "clear",
            "coAlarmState": "clear",
            "coConcentration": 0,
            "coConcentrationState": "low",
            "smokeSeverity": null,
            "coSeverity": null,
            "detectorFault": "normal",
            "detectorService": "normal",
            "detectorTest": "idle",
            "detectorTestResult": null,
            "detectorMute": "unmuted",
            "smokeSensitivity": "standard",
            "batteryLevel": 96,
            "chargingState": "not charging",
            "tamperState": "clear"
        })),
        THERMOSTAT_FAN_V1 => object_map(json!({
            "online": true,
            "temperature": temperature(22.0, "C"),
            "hvacMode": "auto",
            "presetMode": "home",
            "runningState": "idle",
            "supportCool": true,
            "supportHeat": true,
            "coolingSetpoint": temperature(25.0, "C"),
            "heatingSetpoint": temperature(20.0, "C"),
            "minCoolingSetpoint": temperature(18.0, "C"),
            "maxCoolingSetpoint": temperature(30.0, "C"),
            "minHeatingSetpoint": temperature(10.0, "C"),
            "maxHeatingSetpoint": temperature(26.0, "C"),
            "availableHvacModes": ["off", "heat", "cool", "auto"],
            "availablePresetModes": ["home", "away", "sleep"],
            "fanMode": "auto",
            "fanSpeed": 35,
            "availableFanModes": ["auto", "low", "medium", "high"],
            "fanOscillation": "off",
            "availableFanOscillationModes": ["off","on"]
        })),
        FAN_PURIFIER_V1 => object_map(json!({
            "online": true,
            "on": false,
            "fanMode": "auto",
            "fanSpeed": 30,
            "availableFanModes": ["auto", "sleep", "low", "medium", "high", "turbo"],
            "fanOscillation": "off",
            "availableFanOscillationModes": ["off","on"],
            "qualityState": "good",
            "index": 35,
            "pm25": 8,
            "pm25State": "low"
        })),
        SPEAKER_MEDIA_BROADCAST_V1 => object_map(json!({
            "online": true,
            "volume": 35,
            "muted": false,
            "playbackState": "stopped",
            "elapsedTime": 0,
            "totalTime": 0,
            "track": {
                "title": "",
                "artist": "",
                "album": "",
                "mediaSource": "local"
            },
            "mediaItems": [
                {
                    "mediaContentId": "playlist:dinner",
                    "mediaContentType": "playlist",
                    "title": "Dinner Playlist",
                    "subtitle": "Warm evening music",
                    "source": "favorites",
                    "canPlay": true,
                    "canExpand": false
                },
                {
                    "mediaContentId": "station:news",
                    "mediaContentType": "station",
                    "title": "Daily News",
                    "subtitle": "Live radio",
                    "source": "favorites",
                    "canPlay": true,
                    "canExpand": false
                }
            ],
            "supportedAudioInputs": ["url", "base64"],
            "supportedMimeTypes": ["audio/mpeg", "audio/wav", "audio/mp4"],
            "broadcastDuration": "7s",
            "restorePreviousPlayback": true,
            "lastAudio": null,
            "lastVolume": null,
            "broadcastCount": 0
        })),
        BUTTON_SCENE_V1 => object_map(json!({
            "online": true,
            "batteryLevel": 82,
            "chargingState": "not charging",
            "supportedPressTypes": ["single", "double", "long"],
            "lastPressType": "single"
        })),
        _ => Err(anyhow!("unknown playground modelId: {}", model_id)),
    }
}

pub fn normalize_instance(mut instance: DeviceInstance) -> Result<DeviceInstance> {
    instance.id = instance.id.trim().to_string();
    instance.display_name = instance.display_name.trim().to_string();
    instance.model_id = instance.model_id.trim().to_string();
    if instance.id.is_empty() {
        return Err(anyhow!("playground device id is required"));
    }
    if instance.display_name.is_empty() {
        return Err(anyhow!(
            "playground device {} displayName is required",
            instance.id
        ));
    }
    ensure_known_model(&instance.model_id)?;
    let mut attributes = default_attributes(&instance.model_id)?;
    merge_object(&mut attributes, instance.attributes);
    validate_attributes(&instance.model_id, &attributes)?;
    instance.attributes = attributes;
    Ok(instance)
}

pub fn capabilities(model_id: &str) -> Result<Vec<String>> {
    if laundry_models::supported(model_id) {
        return Ok(laundry_models::capabilities());
    }
    if model_id == HUMIDISTAT_V1 {
        return Ok(humidistat_model::capabilities());
    }
    if model_id == CHIME_V1 {
        return Ok(chime_model::capabilities());
    }
    if let Some(values) = alarm_models::capabilities(model_id) {
        return Ok(values.iter().map(|v| v.to_string()).collect());
    }
    let values = match model_id {
        LIGHT_RGB_DIMMER_V1 => &[
            "state.onOff",
            "command.onOff",
            "state.brightness",
            "command.setBrightness",
            "state.colorTemperature",
            "command.setColorTemperature",
            "state.colorRGB",
            "command.setColorRGB",
            "state.power",
            "state.energy",
        ][..],
        PLUG_ENERGY_V1 => &[
            "state.onOff",
            "command.onOff",
            "state.power",
            "state.energy",
        ][..],
        COVER_SHADE_V1 => &[
            "state.cover",
            "command.coverControl",
            "data.coverAttributes",
        ][..],
        COVER_GARAGE_DOOR_V1 => &[
            "state.cover",
            "command.coverControl",
            "data.coverAttributes",
            "state.contactSensor",
        ][..],
        LOCK_DEADBOLT_V1 => &[
            "state.doorLock",
            "state.battery",
            "command.doorLock",
            "data.doorLockAttributes",
        ][..],
        CONTACT_SENSOR_V1 => &["state.contactSensor", "state.battery", "state.tamper"][..],
        MOTION_SENSOR_V1 => &["state.motion", "state.battery", "state.tamper"][..],
        ILLUMINANCE_SENSOR_V1 => &["state.illuminance", "state.battery"][..],
        PRESENCE_SENSOR_V1 => &["state.presence", "state.battery"][..],
        LEAK_SENSOR_V1 => &["state.water", "state.battery", "state.tamper"][..],
        AIR_MONITOR_V1 => &[
            "state.temperature",
            "state.humidity",
            "state.airCompositionSensor",
            "data.airCompositionSensorAttributes",
            "state.airQualitySensor",
            "data.airQualitySensorAttributes",
            "state.battery",
        ][..],
        SMOKE_CO_ALARM_V1 => &[
            "state.smokeDetector",
            "state.carbonMonoxideDetector",
            "state.smokeDetectorSetting",
            "data.smokeDetectorAttributes",
            "data.carbonMonoxideDetectorAttributes",
            "command.testSmokeDetector",
            "command.testCarbonMonoxideDetector",
            "command.setSmokeDetectorSensitivity",
            "command.setSmokeDetectorMute",
            "command.setCarbonMonoxideDetectorMute",
            "state.battery",
            "state.tamper",
        ][..],
        THERMOSTAT_FAN_V1 => &[
            "state.temperature",
            "state.thermostat",
            "command.setThermostatMode",
            "command.setThermostatTemperature",
            "state.fan",
            "command.setFan",
        ][..],
        FAN_PURIFIER_V1 => &[
            "state.onOff",
            "command.onOff",
            "state.fan",
            "command.setFan",
            "state.airCompositionSensor",
            "data.airCompositionSensorAttributes",
            "state.airQualitySensor",
            "data.airQualitySensorAttributes",
        ][..],
        SPEAKER_MEDIA_BROADCAST_V1 => &[
            "state.audioMute",
            "state.audioVolume",
            "state.mediaPlayback",
            "state.mediaTrack",
            "data.mediaBrowse",
            "command.setAudioVolume",
            "command.setAudioMute",
            "command.playMedia",
            "command.controlMediaPlayback",
            "command.broadcastAudio",
        ][..],
        BUTTON_SCENE_V1 => &["state.battery", "event.buttonPress"][..],
        _ => return Err(anyhow!("unknown playground modelId: {}", model_id)),
    };
    let mut capabilities: Vec<String> = values.iter().map(|value| (*value).to_string()).collect();
    if capabilities.iter().any(|c| c == "state.battery") {
        capabilities.push("data.batteryAttributes".into());
    }
    Ok(capabilities)
}

pub fn exposes_capability(model_id: &str, capability: &str) -> bool {
    capabilities(model_id)
        .map(|items| items.iter().any(|item| item == capability))
        .unwrap_or(false)
}

fn detector_fields(capability: &str) -> Vec<(&'static str, &'static str)> {
    let smoke = capability == "state.smokeDetector";
    let mut fields = vec![
        (
            "alarmState",
            if smoke { "smokeState" } else { "coAlarmState" },
        ),
        (
            "alarmSeverity",
            if smoke { "smokeSeverity" } else { "coSeverity" },
        ),
        ("faultState", "detectorFault"),
        ("serviceState", "detectorService"),
        ("testState", "detectorTest"),
        ("testResult", "detectorTestResult"),
        ("muteState", "detectorMute"),
    ];
    if !smoke {
        fields.extend([
            ("concentration", "coConcentration"),
            ("concentrationState", "coConcentrationState"),
        ]);
    }
    fields
}
fn detector_state(attrs: &Map<String, Value>, capability: &str) -> Map<String, Value> {
    detector_fields(capability)
        .iter()
        .map(|(field, attr)| {
            let value = attrs
                .get(*attr)
                .filter(|v| match *field {
                    "concentration" => v.as_f64().is_some_and(|v| v.is_finite() && v >= 0.0),
                    _ => v.as_str().is_some_and(|s| match *field {
                        "alarmState" => ["clear", "detected"].contains(&s),
                        "alarmSeverity" => ["warning", "critical"].contains(&s),
                        "faultState" => ["normal", "fault"].contains(&s),
                        "serviceState" => ["normal", "expired"].contains(&s),
                        "testState" => ["idle", "testing"].contains(&s),
                        "testResult" => ["passed", "failed"].contains(&s),
                        "muteState" => ["muted", "unmuted"].contains(&s),
                        _ => ["low", "medium", "high", "critical"].contains(&s),
                    }),
                })
                .cloned()
                .unwrap_or(Value::Null);
            ((*field).into(), value)
        })
        .collect()
}
fn detector_command(
    attrs: &mut Map<String, Value>,
    capability: &str,
    req: &Map<String, Value>,
) -> Result<CommandOutcome> {
    if capability == "command.setSmokeDetectorSensitivity" {
        let value = required_string(req, "sensitivity")?;
        if !["low", "standard", "high"].contains(&value.as_str()) {
            return Err(anyhow!("Unsupported smoke sensitivity"));
        }
        attrs.insert("smokeSensitivity".into(), json!(value));
        return Ok(CommandOutcome::with_event(
            "state.smokeDetectorSetting",
            json!({"sensitivity":value}),
        ));
    }
    let caps = ["state.smokeDetector", "state.carbonMonoxideDetector"];
    let mut events = Vec::new();
    if capability.starts_with("command.test") {
        attrs.insert("detectorTest".into(), json!("testing"));
        attrs.insert("detectorTestResult".into(), Value::Null);
        for cap in caps {
            events.push(CommandEvent {
                capability: cap.into(),
                payload: json!(detector_state(attrs, cap)),
            });
        }
        attrs.insert("detectorTest".into(), json!("idle"));
        attrs.insert("detectorTestResult".into(), json!("passed"));
    } else {
        let value = required_string(req, "muteState")?;
        if !["muted", "unmuted"].contains(&value.as_str()) {
            return Err(anyhow!("muteState must be muted or unmuted"));
        }
        attrs.insert("detectorMute".into(), json!(value));
    }
    for cap in caps {
        events.push(CommandEvent {
            capability: cap.into(),
            payload: json!(detector_state(attrs, cap)),
        });
    }
    Ok(CommandOutcome {
        response: Value::Null,
        events,
    })
}

fn all_air_fields(model_id: &str) -> &'static [&'static str] {
    match model_id {
        AIR_MONITOR_V1 => &[
            "qualityState",
            "index",
            "co2",
            "co2State",
            "pm25",
            "pm25State",
            "vocPpb",
            "vocState",
        ],
        FAN_PURIFIER_V1 => &["qualityState", "index", "pm25", "pm25State"],
        _ => &[],
    }
}
fn air_sensor_fields(model_id: &str, capability: &str) -> Vec<&'static str> {
    let quality = capability == "state.airQualitySensor";
    all_air_fields(model_id)
        .iter()
        .copied()
        .filter(|field| matches!(*field, "qualityState" | "index") == quality)
        .collect()
}
fn air_options(field: &str) -> &'static [&'static str] {
    match field {
        "qualityState" => &[
            "good",
            "fair",
            "moderate",
            "poor",
            "very poor",
            "extremely poor",
        ],
        _ => &["low", "medium", "high", "critical"],
    }
}

fn battery_fields(model: &str) -> &'static [&'static str] {
    match model {
        CONTACT_SENSOR_V1 => &["levelState"],
        PRESENCE_SENSOR_V1 => &["chargingState"],
        LOCK_DEADBOLT_V1
        | MOTION_SENSOR_V1
        | ILLUMINANCE_SENSOR_V1
        | LEAK_SENSOR_V1
        | AIR_MONITOR_V1
        | SMOKE_CO_ALARM_V1 => &["level", "chargingState"],
        BUTTON_SCENE_V1 => &["level"],
        _ => &[],
    }
}
fn battery_attribute(field: &str) -> &str {
    match field {
        "level" => "batteryLevel",
        "levelState" => "batteryLevelState",
        _ => "chargingState",
    }
}
fn battery_discovery(model: &str) -> Result<Map<String, Value>> {
    let fields = battery_fields(model);
    if fields.is_empty() {
        return Err(anyhow!("Model has no battery observations"));
    }
    let mut result = object_map(json!({"supportedStates":fields}))?;
    if fields.contains(&"levelState") {
        result.insert("levelStates".into(), json!(["normal", "low"]));
    }
    Ok(result)
}
fn battery_state(model: &str, attrs: &Map<String, Value>) -> Result<Map<String, Value>> {
    let mut result = Map::new();
    for field in battery_fields(model) {
        let value = attrs
            .get(battery_attribute(field))
            .cloned()
            .unwrap_or(Value::Null);
        let valid = value.is_null()
            || match *field {
                "level" => value
                    .as_f64()
                    .is_some_and(|n| n.is_finite() && (0.0..=100.0).contains(&n)),
                "levelState" => value
                    .as_str()
                    .is_some_and(|s| ["normal", "low"].contains(&s)),
                _ => value.as_str().is_some_and(|s| {
                    ["charging", "discharging", "full", "not charging"].contains(&s)
                }),
            };
        if !valid {
            return Err(anyhow!("Invalid battery {field}"));
        }
        result.insert((*field).into(), value);
    }
    if result.is_empty() {
        return Err(anyhow!("Model has no battery observations"));
    }
    Ok(result)
}

pub fn project_state(
    model_id: &str,
    attrs: &Map<String, Value>,
    capability: &str,
) -> Result<Map<String, Value>> {
    if laundry_models::supported(model_id) {
        return laundry_models::state(model_id, attrs, capability);
    }
    if model_id == HUMIDISTAT_V1 {
        return humidistat_model::state(attrs, capability);
    }
    if model_id == CHIME_V1 {
        return chime_model::state(attrs, capability);
    }
    if let Some(value) = alarm_models::state(model_id, attrs, capability) {
        return value;
    }

    match capability {
        "state.onOff" => Ok(object_map(json!({
            "state": if bool_attr(attrs, "on")? { "on" } else { "off" }
        }))?),
        "state.brightness" => Ok(object_map(json!({
            "brightness": number_attr(attrs, "brightness")?
        }))?),
        "state.colorTemperature" => Ok(object_map(json!({
            "colorTemperature": number_attr(attrs, "colorTemperature")?
        }))?),
        "state.colorRGB" => Ok(object_map(json!({
            "hue": number_attr(attrs, "colorHue")?,
            "saturation": number_attr(attrs, "colorSaturation")?
        }))?),
        "state.power" => Ok(object_map(json!({
            "power": nullable_meter(attrs, "power", false)?
        }))?),
        "state.energy" => Ok(object_map(json!({
            "importedEnergy": nullable_meter(attrs, "importedEnergy", true)?
        }))?),
        "state.cover" => cover_snapshot(attrs),
        "data.coverAttributes" => cover_discovery(attrs),
        "state.contactSensor" => Ok(object_map(json!({
            "state": string_attr(attrs, "contactState")?
        }))?),
        "data.doorLockAttributes" => project_discovery(model_id, attrs, "state.doorLock", "$f1"),
        "state.doorLock" => Ok(object_map(json!({
            "lockState": string_attr(attrs, "lockState")?
        }))?),
        "data.batteryAttributes" => battery_discovery(model_id),
        "state.battery" => battery_state(model_id, attrs),
        "state.tamper" => Ok(object_map(json!({
            "state": string_attr(attrs, "tamperState")?
        }))?),
        "state.motion" => Ok(object_map(json!({
            "state": string_attr(attrs, "motionState")?
        }))?),
        "state.illuminance" => Ok(object_map(json!({
            "illuminanceLux": number_attr(attrs, "illuminanceLux")?
        }))?),
        "state.presence" => Ok(object_map(json!({
            "state": string_attr(attrs, "presenceState")?
        }))?),
        "state.water" => Ok(object_map(json!({
            "state": string_attr(attrs, "waterState")?
        }))?),
        "state.temperature" => Ok(object_map(json!({
            "temperature": value_attr(attrs, "temperature")?
        }))?),
        "state.humidity" => Ok(object_map(json!({
            "humidity": number_attr(attrs, "humidity")?
        }))?),
        "data.airCompositionSensorAttributes" => {
            project_discovery(model_id, attrs, "state.airCompositionSensor", "$f1")
        }
        "data.airQualitySensorAttributes" => {
            project_discovery(model_id, attrs, "state.airQualitySensor", "$f1")
        }
        "state.airCompositionSensor" | "state.airQualitySensor" => {
            Ok(air_sensor_fields(model_id, capability)
                .iter()
                .map(|field| {
                    let value = attrs
                        .get(*field)
                        .filter(|v| {
                            if field.ends_with("State") {
                                v.as_str().is_some_and(|s| air_options(field).contains(&s))
                            } else {
                                v.as_f64().is_some_and(|n| n.is_finite() && n >= 0.0)
                            }
                        })
                        .cloned()
                        .unwrap_or(Value::Null);
                    ((*field).into(), value)
                })
                .collect())
        }
        "state.smokeDetector" | "state.carbonMonoxideDetector" => {
            Ok(detector_state(attrs, capability))
        }
        "state.smokeDetectorSetting" => Ok(object_map(
            json!({"sensitivity":attrs.get("smokeSensitivity").filter(|v|v.as_str().is_some_and(|s|["low","standard","high"].contains(&s))).cloned().unwrap_or(Value::Null)}),
        )?),
        "data.smokeDetectorAttributes" => {
            project_discovery(model_id, attrs, "state.smokeDetector", "$f1")
        }
        "data.carbonMonoxideDetectorAttributes" => {
            project_discovery(model_id, attrs, "state.carbonMonoxideDetector", "$f1")
        }
        "state.fan" => fan_state(attrs),
        "state.thermostat" => Ok(object_map(json!({
            "hvacMode": string_attr(attrs, "hvacMode")?,
            "presetMode": string_attr(attrs, "presetMode")?,
            "runningState": string_attr(attrs, "runningState")?,
            "coolingSetpoint": value_attr(attrs, "coolingSetpoint")?,
            "heatingSetpoint": value_attr(attrs, "heatingSetpoint")?
        }))?),
        "state.audioMute" => Ok(object_map(json!({
            "state": if bool_attr(attrs, "muted")? { "mute" } else { "unmute" }
        }))?),
        "state.audioVolume" => Ok(object_map(json!({
            "volume": number_attr(attrs, "volume")?
        }))?),
        "state.mediaPlayback" => Ok(object_map(json!({
            "state": string_attr(attrs, "playbackState")?,
            "elapsedTime": number_attr(attrs, "elapsedTime")?,
            "totalTime": number_attr(attrs, "totalTime")?
        }))?),
        "state.mediaTrack" => object_attr(attrs, "track"),
        "data.mediaBrowse" => Ok(object_map(json!({
            "items": attrs.get("mediaItems").cloned().unwrap_or_else(|| Value::Array(Vec::new()))
        }))?),
        "event.buttonPress" => Ok(object_map(json!({
            "occurred": true,
            "pressedType": string_attr(attrs, "lastPressType")?
        }))?),
        _ => Err(anyhow!(
            "playground model {} cannot project state for capability {}",
            model_id,
            capability
        )),
    }
}

pub fn project_discovery(
    model_id: &str,
    attrs: &Map<String, Value>,
    capability: &str,
    _alias: &str,
) -> Result<Map<String, Value>> {
    if laundry_models::supported(model_id) {
        ensure!(
            laundry_models::capabilities()
                .iter()
                .any(|c| c == capability),
            "Unsupported laundry discovery"
        );
        return laundry_models::discovery(model_id, attrs);
    }
    if model_id == HUMIDISTAT_V1 {
        return humidistat_model::discovery(attrs, capability);
    }
    if model_id == CHIME_V1 {
        return chime_model::discovery(attrs, capability);
    }
    if let Some(value) = alarm_models::discovery(model_id, attrs, capability, _alias) {
        return value;
    }

    match capability {
        "state.battery" | "data.batteryAttributes" => battery_discovery(model_id),
        "state.smokeDetector"
        | "command.setSmokeDetectorSensitivity"
        | "command.setSmokeDetectorMute" => Ok(object_map(
            json!({"supportedStates":["alarmSeverity","faultState","serviceState","testState","testResult","muteState"],"availableSensitivities":["low","standard","high"],"availableMuteStates":["muted","unmuted"]}),
        )?),
        "state.carbonMonoxideDetector" | "command.setCarbonMonoxideDetectorMute" => Ok(object_map(
            json!({"supportedStates":["concentration","concentrationState","alarmState","alarmSeverity","faultState","serviceState","testState","testResult","muteState"],"concentrationStates":["low","medium","high","critical"],"availableMuteStates":["muted","unmuted"]}),
        )?),
        "state.power" => Ok(object_map(
            json!({"supportedMeasurements":["activePower"]}),
        )?),
        "state.energy" => Ok(object_map(json!({"supportedCounters":["importedEnergy"]}))?),
        "state.airCompositionSensor" | "state.airQualitySensor" => {
            let fields = air_sensor_fields(model_id, capability);
            if fields.is_empty() {
                return Err(anyhow!("Model has no observations for {capability}"));
            }
            let mut out = object_map(json!({"supportedStates":fields}))?;
            if fields.contains(&"index") {
                out.insert("indexScale".into(), json!("playground:air-index-v1"));
            }
            for field in fields.iter().filter(|field| field.ends_with("State")) {
                out.insert(format!("{field}s"), json!(air_options(field)));
            }
            Ok(out)
        }

        "state.colorTemperature" | "command.setColorTemperature" => Ok(object_map(json!({
            "min": number_attr(attrs, "minColorTemperature")?,
            "max": number_attr(attrs, "maxColorTemperature")?
        }))?),
        "state.doorLock" | "command.doorLock" => Ok(object_map(json!({
            "supportedCommands": ["lock", "unlock", "unlatch"],
            "canGetDoorState": false,
            "pinCodeRequirement": "unsupported"
        }))?),
        "state.cover" | "command.coverControl" => cover_discovery(attrs),
        "state.fan" | "command.setFan" => Ok(object_map(json!({
            "availableModes": attrs.get("availableFanModes").cloned().unwrap_or_else(|| Value::Array(Vec::new())),
            "supportedStates": ["mode","speed","oscillation"],
            "supportedSettings": ["mode","speed","oscillation"],
            "availableOscillationModes": value_attr(attrs,"availableFanOscillationModes")?
        }))?),
        "state.thermostat" | "command.setThermostatMode" | "command.setThermostatTemperature" => {
            Ok(object_map(json!({
                "supportCool": bool_attr(attrs, "supportCool")?,
                "supportHeat": bool_attr(attrs, "supportHeat")?,
                "minCoolingSetpoint": value_attr(attrs, "minCoolingSetpoint")?,
                "maxCoolingSetpoint": value_attr(attrs, "maxCoolingSetpoint")?,
                "minHeatingSetpoint": value_attr(attrs, "minHeatingSetpoint")?,
                "maxHeatingSetpoint": value_attr(attrs, "maxHeatingSetpoint")?,
                "availableHvacModes": attrs.get("availableHvacModes").cloned().unwrap_or_else(|| Value::Array(Vec::new())),
                "availablePresetModes": attrs.get("availablePresetModes").cloned().unwrap_or_else(|| Value::Array(Vec::new()))
            }))?)
        }
        "event.buttonPress" => Ok(object_map(json!({
            "supportedPressTypes": value_attr(attrs, "supportedPressTypes")?
        }))?),
        "command.broadcastAudio" => Ok(object_map(json!({
            "supportedAudioInputs": attrs.get("supportedAudioInputs").cloned().unwrap_or_else(|| Value::Array(Vec::new())),
            "supportedMimeTypes": attrs.get("supportedMimeTypes").cloned().unwrap_or_else(|| Value::Array(Vec::new())),
            "defaultDuration": attrs.get("broadcastDuration").cloned().unwrap_or(Value::Null),
            "restorePreviousPlayback": attrs.get("restorePreviousPlayback").cloned().unwrap_or(Value::Bool(false))
        }))?),
        _ => Ok(Map::new()),
    }
}

pub fn project_capability_data(
    model_id: &str,
    attrs: &Map<String, Value>,
) -> Result<Map<String, Value>> {
    let mut data = Map::new();
    for capability in capabilities(model_id)? {
        if alarm_models::parameterized_data(&capability) {
            continue;
        }
        let mut entry = Map::new();
        if capability.starts_with("state.")
            || capability.starts_with("data.")
            || capability.starts_with("event.")
        {
            entry.insert(
                "states".to_string(),
                Value::Object(project_state(model_id, attrs, &capability)?),
            );
        }
        let discovery = project_discovery(model_id, attrs, &capability, "$f1")?;
        if !discovery.is_empty() {
            entry.insert("attributes".to_string(), Value::Object(discovery));
        }
        if !entry.is_empty() {
            data.insert(capability, Value::Object(entry));
        }
    }
    Ok(data)
}

pub fn apply_observed_payload(
    model_id: &str,
    attrs: &mut Map<String, Value>,
    capability: &str,
    payload: &Map<String, Value>,
) -> Result<()> {
    if laundry_models::supported(model_id) {
        return laundry_models::observe(model_id, attrs, capability, payload);
    }
    if model_id == HUMIDISTAT_V1 {
        return humidistat_model::observe(attrs, capability, payload);
    }
    if model_id == CHIME_V1 {
        return chime_model::observe(attrs, capability, payload);
    }
    if alarm_models::capabilities(model_id).is_some() {
        ensure!(
            exposes_capability(model_id, capability),
            "Unsupported model observation"
        );
        return alarm_models::observe(model_id, attrs, capability, payload).unwrap();
    }

    if !exposes_capability(model_id, capability) {
        return Err(anyhow!(
            "playground model {} does not expose capability {}",
            model_id,
            capability
        ));
    }
    match capability {
        "state.audioVolume" => copy_payload_field(attrs, payload, "volume", "volume")?,
        "state.audioMute" => {
            let value = required_string(payload, "state")?;
            attrs.insert("muted".to_string(), Value::Bool(value == "mute"));
        }
        "state.mediaPlayback" => {
            copy_optional_payload_fields(
                attrs,
                payload,
                &[
                    ("state", "playbackState"),
                    ("elapsedTime", "elapsedTime"),
                    ("totalTime", "totalTime"),
                ],
            );
        }
        "state.mediaTrack" => {
            attrs.insert("track".to_string(), Value::Object(payload.clone()));
        }
        "data.mediaBrowse" => {
            if let Some(value) = payload.get("items") {
                attrs.insert("mediaItems".to_string(), value.clone());
            }
        }
        "state.onOff" => {
            let value = required_string(payload, "state")?;
            attrs.insert("on".to_string(), Value::Bool(value == "on"));
        }
        "state.brightness" => copy_payload_field(attrs, payload, "brightness", "brightness")?,
        "state.colorTemperature" => {
            copy_payload_field(attrs, payload, "colorTemperature", "colorTemperature")?
        }
        "state.colorRGB" => {
            copy_payload_field(attrs, payload, "hue", "colorHue")?;
            copy_payload_field(attrs, payload, "saturation", "colorSaturation")?;
        }
        "state.power" => copy_payload_field(attrs, payload, "power", "power")?,
        "state.energy" => copy_payload_field(attrs, payload, "importedEnergy", "importedEnergy")?,
        "state.cover" => {
            let mut next = attrs.clone();
            let mut allowed = vec!["openingState", "motionState"];
            if bool_attr(attrs, "canGetPosition")? {
                allowed.push("opening");
            }
            ensure!(
                !payload.is_empty() && payload.keys().all(|key| allowed.contains(&key.as_str())),
                "Unsupported cover observation"
            );
            copy_optional_payload_fields(
                &mut next,
                payload,
                &[
                    ("openingState", "coverState"),
                    ("motionState", "coverMotionState"),
                    ("opening", "coverPosition"),
                ],
            );
            cover_snapshot(&next)?;
            *attrs = next;
        }
        "state.contactSensor" => copy_payload_field(attrs, payload, "state", "contactState")?,
        "state.doorLock" => copy_payload_field(attrs, payload, "lockState", "lockState")?,
        "state.battery" => {
            let mut staged = attrs.clone();
            let fields = battery_fields(model_id);
            if payload.is_empty() || payload.keys().any(|key| !fields.contains(&key.as_str())) {
                return Err(anyhow!("Unsupported battery observation"));
            }
            for field in fields {
                if let Some(value) = payload.get(*field) {
                    staged.insert(battery_attribute(field).into(), value.clone());
                }
            }
            battery_state(model_id, &staged)?;
            *attrs = staged;
        }
        "state.tamper" => copy_payload_field(attrs, payload, "state", "tamperState")?,
        "state.motion" => copy_payload_field(attrs, payload, "state", "motionState")?,
        "state.illuminance" => {
            copy_payload_field(attrs, payload, "illuminanceLux", "illuminanceLux")?
        }
        "state.presence" => copy_payload_field(attrs, payload, "state", "presenceState")?,
        "state.water" => copy_payload_field(attrs, payload, "state", "waterState")?,
        "state.temperature" => copy_payload_field(attrs, payload, "temperature", "temperature")?,
        "state.humidity" => copy_payload_field(attrs, payload, "humidity", "humidity")?,
        "state.airCompositionSensor" | "state.airQualitySensor" => {
            for field in air_sensor_fields(model_id, capability) {
                if let Some(value) = payload.get(field) {
                    attrs.insert(field.into(), value.clone());
                }
            }
        }
        "state.smokeDetector" | "state.carbonMonoxideDetector" => {
            for (field, attr) in detector_fields(capability) {
                if let Some(value) = payload.get(field) {
                    attrs.insert(attr.into(), value.clone());
                }
            }
        }
        "state.smokeDetectorSetting" => {
            copy_payload_field(attrs, payload, "sensitivity", "smokeSensitivity")?
        }
        "state.fan" => {
            let mut next = attrs.clone();
            ensure!(
                !payload.is_empty()
                    && payload
                        .keys()
                        .all(|k| ["mode", "speed", "oscillation"].contains(&k.as_str())),
                "Invalid fan observation"
            );
            copy_optional_payload_fields(
                &mut next,
                payload,
                &[
                    ("mode", "fanMode"),
                    ("speed", "fanSpeed"),
                    ("oscillation", "fanOscillation"),
                ],
            );
            fan_state(&next)?;
            *attrs = next;
        }
        "state.thermostat" => {
            copy_optional_payload_fields(
                attrs,
                payload,
                &[
                    ("hvacMode", "hvacMode"),
                    ("presetMode", "presetMode"),
                    ("runningState", "runningState"),
                    ("coolingSetpoint", "coolingSetpoint"),
                    ("heatingSetpoint", "heatingSetpoint"),
                ],
            );
        }
        "event.buttonPress" => {
            copy_payload_field(attrs, payload, "pressedType", "lastPressType")?;
        }
        _ => {
            return Err(anyhow!(
                "playground observed payload does not support capability {}",
                capability
            ))
        }
    }
    Ok(())
}

pub fn snapshot_device(
    instance: &DeviceInstance,
    history: Vec<CommandHistoryEntry>,
) -> Result<DeviceSnapshot> {
    Ok(DeviceSnapshot {
        id: instance.id.clone(),
        display_name: instance.display_name.clone(),
        model_id: instance.model_id.clone(),
        capabilities: capabilities(&instance.model_id)?,
        attributes: instance.attributes.clone(),
        capability_data: project_capability_data(&instance.model_id, &instance.attributes)?,
        command_history: history,
    })
}

pub fn handle_command(
    model_id: &str,
    attrs: &mut Map<String, Value>,
    capability: &str,
    request: &Map<String, Value>,
) -> Result<CommandOutcome> {
    if laundry_models::supported(model_id) {
        return laundry_models::command(model_id, attrs, capability, request);
    }
    if model_id == HUMIDISTAT_V1 {
        return humidistat_model::command(attrs, capability, request);
    }
    if model_id == CHIME_V1 {
        return chime_model::command(attrs, capability, request);
    }
    if alarm_models::capabilities(model_id).is_some() {
        ensure!(
            exposes_capability(model_id, capability),
            "Unsupported model command"
        );
        return alarm_models::command(model_id, attrs, capability, request).unwrap();
    }

    if !exposes_capability(model_id, capability) {
        return Err(anyhow!(
            "playground model {} does not expose capability {}",
            model_id,
            capability
        ));
    }
    match capability {
        "command.testSmokeDetector"
        | "command.testCarbonMonoxideDetector"
        | "command.setSmokeDetectorSensitivity"
        | "command.setSmokeDetectorMute"
        | "command.setCarbonMonoxideDetectorMute" => detector_command(attrs, capability, request),
        "command.onOff" => set_on_off(attrs, request),
        "command.setBrightness" => set_brightness(attrs, request),
        "command.setColorTemperature" => set_color_temperature(attrs, request),
        "command.setColorRGB" => set_color_rgb(attrs, request),
        "command.coverControl" => control_cover(attrs, request),
        "command.doorLock" => door_lock(attrs, request),
        "command.setFan" => set_fan(attrs, request),
        "command.setThermostatMode" => set_thermostat_mode(attrs, request),
        "command.setThermostatTemperature" => set_thermostat_temperature(attrs, request),
        "command.setAudioVolume" => speaker_set_audio_volume(attrs, request),
        "command.setAudioMute" => speaker_set_audio_mute(attrs, request),
        "command.playMedia" => speaker_play_media(attrs, request),
        "command.controlMediaPlayback" => speaker_control_media_playback(attrs, request),
        "command.broadcastAudio" => speaker_broadcast_audio(attrs, request),
        _ => Err(anyhow!(
            "unsupported playground command capability {} for model {}",
            capability,
            model_id
        )),
    }
}

fn speaker_set_audio_volume(
    attrs: &mut Map<String, Value>,
    request: &Map<String, Value>,
) -> Result<CommandOutcome> {
    let volume = required_number(request, "volume")?.clamp(0.0, 100.0);
    attrs.insert("volume".to_string(), json!(volume));
    let payload = project_state(SPEAKER_MEDIA_BROADCAST_V1, attrs, "state.audioVolume")?;
    Ok(CommandOutcome::with_event(
        "state.audioVolume",
        Value::Object(payload),
    ))
}

fn set_on_off(
    attrs: &mut Map<String, Value>,
    request: &Map<String, Value>,
) -> Result<CommandOutcome> {
    let command = required_string(request, "command")?;
    let on = match command.as_str() {
        "on" => true,
        "off" => false,
        _ => return Err(anyhow!("unsupported onOff command: {}", command)),
    };
    attrs.insert("on".to_string(), Value::Bool(on));
    if attrs.contains_key("power") {
        attrs.insert("power".to_string(), json!(if on { 8.0 } else { 0.0 }));
    }
    projected_event(attrs, "state.onOff")
}

fn set_brightness(
    attrs: &mut Map<String, Value>,
    request: &Map<String, Value>,
) -> Result<CommandOutcome> {
    let brightness = required_number(request, "brightness")?.clamp(0.0, 100.0);
    attrs.insert("brightness".to_string(), json!(brightness));
    projected_event(attrs, "state.brightness")
}

fn set_color_temperature(
    attrs: &mut Map<String, Value>,
    request: &Map<String, Value>,
) -> Result<CommandOutcome> {
    let min = attrs
        .get("minColorTemperature")
        .and_then(Value::as_f64)
        .unwrap_or(1000.0);
    let max = attrs
        .get("maxColorTemperature")
        .and_then(Value::as_f64)
        .unwrap_or(10000.0);
    let value = required_number(request, "colorTemperature")?.clamp(min, max);
    attrs.insert("colorTemperature".to_string(), json!(value));
    projected_event(attrs, "state.colorTemperature")
}

fn set_color_rgb(
    attrs: &mut Map<String, Value>,
    request: &Map<String, Value>,
) -> Result<CommandOutcome> {
    let component = |key: &str, old: &str, max: f64| -> Result<f64> {
        let value = request
            .get(key)
            .filter(|v| !v.is_null())
            .or_else(|| attrs.get(old));
        value
            .and_then(Value::as_f64)
            .filter(|v| v.is_finite() && (0.0..=max).contains(v))
            .ok_or_else(|| anyhow!("{key} is invalid or unavailable"))
    };
    ensure!(
        request.get("hue").is_some_and(|v| !v.is_null())
            || request.get("saturation").is_some_and(|v| !v.is_null()),
        "hue or saturation is required"
    );
    let hue = component("hue", "colorHue", 360.0)?;
    let saturation = component("saturation", "colorSaturation", 100.0)?;
    attrs.insert("colorHue".into(), json!(hue));
    attrs.insert("colorSaturation".into(), json!(saturation));
    projected_event(attrs, "state.colorRGB")
}

fn cover_snapshot(attrs: &Map<String, Value>) -> Result<Map<String, Value>> {
    let mut report = Map::new();
    for (field, attr, choices) in [
        (
            "openingState",
            "coverState",
            &["open", "closed", "partially open"][..],
        ),
        (
            "motionState",
            "coverMotionState",
            &["opening", "closing", "stopped"][..],
        ),
    ] {
        let value = attrs.get(attr).cloned().unwrap_or(Value::Null);
        ensure!(
            value.is_null() || value.as_str().is_some_and(|v| choices.contains(&v)),
            "Invalid cover {field}"
        );
        report.insert(field.into(), value);
    }
    if bool_attr(attrs, "canGetPosition")? {
        let value = attrs.get("coverPosition").cloned().unwrap_or(Value::Null);
        ensure!(
            value.is_null()
                || value
                    .as_f64()
                    .is_some_and(|v| v.is_finite() && (0.0..=100.0).contains(&v)),
            "Invalid cover opening"
        );
        report.insert("opening".into(), value);
    }
    Ok(report)
}

fn cover_discovery(attrs: &Map<String, Value>) -> Result<Map<String, Value>> {
    let mut states = vec!["openingState", "motionState"];
    if bool_attr(attrs, "canGetPosition")? {
        states.push("opening");
    }
    object_map(json!({
        "supportedStates": states,
        "supportedSettings": if bool_attr(attrs, "canSetPosition")? { vec!["opening"] } else { vec![] },
        "supportedCommands": ["open", "close", "stop"],
        "requiredSettings": [],
        "openingStates": ["open", "closed", "partially open"],
        "motionStates": ["opening", "closing", "stopped"],
        "deviceType": string_attr(attrs,"coverDeviceType")?
    }))
}

fn control_cover(
    attrs: &mut Map<String, Value>,
    request: &Map<String, Value>,
) -> Result<CommandOutcome> {
    ensure!(
        request
            .keys()
            .all(|k| ["device", "command", "opening", "transitionDuration"].contains(&k.as_str())),
        "Unsupported cover parameter"
    );
    let target = request
        .get("opening")
        .filter(|v| !v.is_null())
        .map(|v| {
            ensure!(
                bool_attr(attrs, "canSetPosition")?,
                "cover does not support setting opening"
            );
            v.as_f64()
                .filter(|n| n.is_finite() && (0.0..=100.0).contains(n))
                .ok_or_else(|| anyhow!("opening must be between 0 and 100"))
        })
        .transpose()?;
    let action = request.get("command").filter(|v| !v.is_null());
    ensure!(
        action.is_none_or(|v| matches!(v.as_str(), Some("open" | "close" | "stop"))),
        "unsupported cover command"
    );
    ensure!(
        target.is_some() || action.is_some(),
        "cover requires command or opening"
    );
    ensure!(
        target.is_none() || action.is_none(),
        "A cover action cannot accompany an opening target"
    );
    if let Some(value) = request.get("transitionDuration").filter(|v| !v.is_null()) {
        ensure!(
            value["_type"] == "Duration"
                && matches!(
                    value["unit"].as_str(),
                    Some("ms" | "sec" | "min" | "hour" | "day")
                )
                && value["value"].as_f64() == Some(0.0),
            "cover does not support transitionDuration; omit or use zero"
        );
        ensure!(
            target.is_some() || matches!(action.and_then(Value::as_str), Some("open" | "close")),
            "transitionDuration requires an opening movement"
        );
    }
    let mut next = attrs.clone();
    if action.and_then(Value::as_str) != Some("stop") {
        let position = target.unwrap_or(if action.and_then(Value::as_str) == Some("open") {
            100.0
        } else {
            0.0
        });
        let state = if position == 0.0 {
            "closed"
        } else if position == 100.0 {
            "open"
        } else {
            "partially open"
        };
        next.insert("coverState".into(), json!(state));
        next.insert("coverPosition".into(), json!(position));
        if next.contains_key("contactState") {
            next.insert(
                "contactState".into(),
                json!(if state == "closed" { "closed" } else { "open" }),
            );
        }
    }
    next.insert("coverMotionState".into(), json!("stopped"));
    cover_snapshot(&next)?;
    let outcome = projected_event(&next, "state.cover")?;
    *attrs = next;
    Ok(outcome)
}

fn door_lock(
    attrs: &mut Map<String, Value>,
    request: &Map<String, Value>,
) -> Result<CommandOutcome> {
    ensure!(
        request.get("pinCode").is_none_or(Value::is_null),
        "This door lock does not accept operation PINs"
    );
    let command = required_string(request, "command")?;
    let lock_state = match command.as_str() {
        "lock" => "locked",
        "unlock" => "unlocked",
        "unlatch" => "unlatched",
        _ => return Err(anyhow!("unsupported doorLock command: {}", command)),
    };
    attrs.insert(
        "lockState".to_string(),
        Value::String(lock_state.to_string()),
    );
    projected_event(attrs, "state.doorLock")
}

fn fan_state(attrs: &Map<String, Value>) -> Result<Map<String, Value>> {
    let mut out = Map::new();
    for (field, key, catalog) in [
        ("mode", "fanMode", Some("availableFanModes")),
        ("speed", "fanSpeed", None),
        (
            "oscillation",
            "fanOscillation",
            Some("availableFanOscillationModes"),
        ),
    ] {
        let value = value_attr(attrs, key)?;
        if !value.is_null() {
            if let Some(catalog) = catalog {
                let name = value
                    .as_str()
                    .ok_or_else(|| anyhow!("Invalid fan {field}"))?;
                ensure_in_string_list(attrs, catalog, name)?;
            } else {
                ensure!(
                    value
                        .as_f64()
                        .is_some_and(|n| n.is_finite() && (0.0..=100.0).contains(&n)),
                    "Invalid fan percentage"
                );
            }
        }
        out.insert(field.into(), value);
    }
    Ok(out)
}

fn set_fan(attrs: &mut Map<String, Value>, request: &Map<String, Value>) -> Result<CommandOutcome> {
    ensure!(request.len() == 1, "Specify exactly one fan setting");
    let (field, value) = request.iter().next().unwrap();
    let (key, next) = match field.as_str() {
        "mode" | "oscillation" => {
            let name = value
                .as_str()
                .ok_or_else(|| anyhow!("Fan setting requires a string"))?;
            ensure_in_string_list(
                attrs,
                if field == "mode" {
                    "availableFanModes"
                } else {
                    "availableFanOscillationModes"
                },
                name,
            )?;
            (
                if field == "mode" {
                    "fanMode"
                } else {
                    "fanOscillation"
                },
                value.clone(),
            )
        }
        "speed" => {
            let n = value
                .as_f64()
                .filter(|n| n.is_finite() && (0.0..=100.0).contains(n))
                .ok_or_else(|| anyhow!("Invalid fan percentage"))?;
            ("fanSpeed", json!(n))
        }
        _ => return Err(anyhow!("Unsupported fan setting")),
    };
    let mut updated = attrs.clone();
    updated.insert(key.into(), next);
    let payload = Value::Object(fan_state(&updated)?);
    *attrs = updated;
    Ok(CommandOutcome::with_event("state.fan", payload))
}

fn set_thermostat_mode(
    attrs: &mut Map<String, Value>,
    request: &Map<String, Value>,
) -> Result<CommandOutcome> {
    if let Some(mode) = request.get("hvacMode").and_then(Value::as_str) {
        ensure_in_string_list(attrs, "availableHvacModes", mode)?;
        attrs.insert("hvacMode".to_string(), Value::String(mode.to_string()));
        attrs.insert(
            "runningState".to_string(),
            Value::String(if mode == "off" { "off" } else { "idle" }.to_string()),
        );
    }
    if let Some(mode) = request.get("presetMode").and_then(Value::as_str) {
        ensure_in_string_list(attrs, "availablePresetModes", mode)?;
        attrs.insert("presetMode".to_string(), Value::String(mode.to_string()));
    }
    projected_event(attrs, "state.thermostat")
}

fn set_thermostat_temperature(
    attrs: &mut Map<String, Value>,
    request: &Map<String, Value>,
) -> Result<CommandOutcome> {
    // The simulator's automatic thermostat requires a non-overlapping target pair.
    // Validate the complete resulting pair before changing either stored target.
    let cooling = request
        .get("coolingSetpoint")
        .or_else(|| attrs.get("coolingSetpoint"));
    let heating = request
        .get("heatingSetpoint")
        .or_else(|| attrs.get("heatingSetpoint"));
    let celsius = |field: &str, value: Option<&Value>| -> Result<f64> {
        let value = value.ok_or_else(|| anyhow!("Thermostat {field} is unavailable"))?;
        let number = value
            .get("value")
            .and_then(Value::as_f64)
            .ok_or_else(|| anyhow!("Thermostat {field} must have a numeric value"))?;
        let result = match value.get("unit").and_then(Value::as_str) {
            Some("C") => number,
            Some("F") => (number - 32.0) / 9.0 * 5.0,
            _ => return Err(anyhow!("Thermostat {field} requires C or F units")),
        };
        if !result.is_finite() {
            return Err(anyhow!("Thermostat {field} is not finite"));
        }
        Ok(result)
    };
    let heat = celsius("heatingSetpoint", heating)?;
    let cool = celsius("coolingSetpoint", cooling)?;
    // Unit conversion may differ by a few floating-point rounding bits.
    let tolerance = 4.0 * f64::EPSILON * heat.abs().max(cool.abs()).max(1.0);
    if heat - cool > tolerance {
        return Err(anyhow!(
            "heatingSetpoint ({heat} C) must not exceed coolingSetpoint ({cool} C)"
        ));
    }
    if let Some(value) = request.get("coolingSetpoint") {
        attrs.insert("coolingSetpoint".to_string(), value.clone());
    }
    if let Some(value) = request.get("heatingSetpoint") {
        attrs.insert("heatingSetpoint".to_string(), value.clone());
    }
    projected_event(attrs, "state.thermostat")
}

fn speaker_set_audio_mute(
    attrs: &mut Map<String, Value>,
    request: &Map<String, Value>,
) -> Result<CommandOutcome> {
    let command = required_string(request, "command")?;
    let muted = match command.as_str() {
        "mute" => true,
        "unmute" => false,
        _ => return Err(anyhow!("unsupported setAudioMute command: {}", command)),
    };
    attrs.insert("muted".to_string(), Value::Bool(muted));
    let payload = project_state(SPEAKER_MEDIA_BROADCAST_V1, attrs, "state.audioMute")?;
    Ok(CommandOutcome::with_event(
        "state.audioMute",
        Value::Object(payload),
    ))
}

fn speaker_play_media(
    attrs: &mut Map<String, Value>,
    request: &Map<String, Value>,
) -> Result<CommandOutcome> {
    let media_content_id = required_string(request, "mediaContentId")?;
    attrs.insert(
        "playbackState".to_string(),
        Value::String("playing".to_string()),
    );
    attrs.insert("elapsedTime".to_string(), json!(0));
    attrs.insert("totalTime".to_string(), json!(1800));
    attrs.insert(
        "track".to_string(),
        json!({
            "title": media_content_id,
            "artist": "Playground",
            "album": "Bench Media",
            "mediaSource": "local"
        }),
    );
    let payload = project_state(SPEAKER_MEDIA_BROADCAST_V1, attrs, "state.mediaPlayback")?;
    Ok(CommandOutcome::with_event(
        "state.mediaPlayback",
        Value::Object(payload),
    ))
}

fn speaker_control_media_playback(
    attrs: &mut Map<String, Value>,
    request: &Map<String, Value>,
) -> Result<CommandOutcome> {
    let command = required_string(request, "command")?;
    let state = match command.as_str() {
        "play" | "nextTrack" | "previousTrack" => "playing",
        "pause" => "paused",
        "stop" => "stopped",
        _ => {
            return Err(anyhow!(
                "unsupported controlMediaPlayback command: {}",
                command
            ))
        }
    };
    attrs.insert(
        "playbackState".to_string(),
        Value::String(state.to_string()),
    );
    let payload = project_state(SPEAKER_MEDIA_BROADCAST_V1, attrs, "state.mediaPlayback")?;
    Ok(CommandOutcome::with_event(
        "state.mediaPlayback",
        Value::Object(payload),
    ))
}

fn speaker_broadcast_audio(
    attrs: &mut Map<String, Value>,
    request: &Map<String, Value>,
) -> Result<CommandOutcome> {
    let audio = request
        .get("audio")
        .and_then(Value::as_object)
        .ok_or_else(|| anyhow!("Missing audio object in broadcastAudio request"))?;
    let volume = request
        .get("volume")
        .and_then(Value::as_f64)
        .map(|value| value.clamp(0.0, 100.0));
    attrs.insert(
        "lastAudio".to_string(),
        Value::Object(normalized_audio(audio)),
    );
    attrs.insert(
        "lastVolume".to_string(),
        volume.map(Value::from).unwrap_or(Value::Null),
    );
    let count = attrs
        .get("broadcastCount")
        .and_then(Value::as_f64)
        .unwrap_or(0.0)
        + 1.0;
    attrs.insert("broadcastCount".to_string(), json!(count));
    attrs.insert(
        "playbackState".to_string(),
        Value::String("playing".to_string()),
    );
    attrs.insert("elapsedTime".to_string(), json!(0));
    attrs.insert(
        "totalTime".to_string(),
        json!(duration_seconds(attrs.get("broadcastDuration")).unwrap_or(7.0)),
    );
    attrs.insert(
        "track".to_string(),
        json!({
            "title": audio_title(audio),
            "artist": "Playground",
            "album": "Broadcasts",
            "mediaSource": "announcement"
        }),
    );
    let payload = project_state(SPEAKER_MEDIA_BROADCAST_V1, attrs, "state.mediaPlayback")?;
    Ok(CommandOutcome::with_event(
        "state.mediaPlayback",
        Value::Object(payload),
    ))
}

fn validate_attributes(model_id: &str, attrs: &Map<String, Value>) -> Result<()> {
    ensure_known_model(model_id)?;
    for capability in capabilities(model_id)? {
        if alarm_models::parameterized_data(&capability) {
            continue;
        }
        if capability.starts_with("state.")
            || capability.starts_with("data.")
            || capability.starts_with("event.")
        {
            project_state(model_id, attrs, &capability)?;
        }
    }
    Ok(())
}

fn merge_object(target: &mut Map<String, Value>, overrides: Map<String, Value>) {
    for (key, value) in overrides {
        target.insert(key, value);
    }
}

fn object_map(value: Value) -> Result<Map<String, Value>> {
    value
        .as_object()
        .cloned()
        .ok_or_else(|| anyhow!("expected JSON object"))
}

fn object_attr(attrs: &Map<String, Value>, key: &str) -> Result<Map<String, Value>> {
    attrs
        .get(key)
        .and_then(Value::as_object)
        .cloned()
        .ok_or_else(|| anyhow!("playground attribute {} must be an object", key))
}

fn value_attr(attrs: &Map<String, Value>, key: &str) -> Result<Value> {
    attrs
        .get(key)
        .cloned()
        .ok_or_else(|| anyhow!("playground attribute {} is required", key))
}

fn bool_attr(attrs: &Map<String, Value>, key: &str) -> Result<bool> {
    attrs
        .get(key)
        .and_then(Value::as_bool)
        .ok_or_else(|| anyhow!("playground attribute {} must be a bool", key))
}

fn string_attr(attrs: &Map<String, Value>, key: &str) -> Result<String> {
    attrs
        .get(key)
        .and_then(Value::as_str)
        .map(ToString::to_string)
        .ok_or_else(|| anyhow!("playground attribute {} must be a string", key))
}

fn nullable_meter(attrs: &Map<String, Value>, key: &str, nonnegative: bool) -> Result<Value> {
    let value = attrs
        .get(key)
        .ok_or_else(|| anyhow!("{} is required", key))?;
    if value.is_null() {
        return Ok(Value::Null);
    }
    let number = value
        .as_f64()
        .ok_or_else(|| anyhow!("{} must be a number or null", key))?;
    ensure!(
        number.is_finite() && (!nonnegative || number >= 0.0),
        "invalid electrical reading {}",
        key
    );
    Ok(json!(number))
}

fn number_attr(attrs: &Map<String, Value>, key: &str) -> Result<Value> {
    let value = required_number(attrs, key)?;
    Ok(json!(value))
}

fn required_number(map: &Map<String, Value>, key: &str) -> Result<f64> {
    map.get(key)
        .and_then(Value::as_f64)
        .ok_or_else(|| anyhow!("{} must be a number", key))
}

fn required_string(map: &Map<String, Value>, key: &str) -> Result<String> {
    map.get(key)
        .and_then(Value::as_str)
        .map(ToString::to_string)
        .ok_or_else(|| anyhow!("{} must be a string", key))
}

fn copy_payload_field(
    attrs: &mut Map<String, Value>,
    payload: &Map<String, Value>,
    payload_key: &str,
    attr_key: &str,
) -> Result<()> {
    let value = payload
        .get(payload_key)
        .cloned()
        .ok_or_else(|| anyhow!("{} requires {}", payload_key, attr_key))?;
    attrs.insert(attr_key.to_string(), value);
    Ok(())
}

fn copy_optional_payload_fields(
    attrs: &mut Map<String, Value>,
    payload: &Map<String, Value>,
    fields: &[(&str, &str)],
) {
    for (payload_key, attr_key) in fields {
        if let Some(value) = payload.get(*payload_key) {
            attrs.insert((*attr_key).to_string(), value.clone());
        }
    }
}

fn projected_event(attrs: &Map<String, Value>, capability: &'static str) -> Result<CommandOutcome> {
    let payload = project_state("", attrs, capability)?;
    Ok(CommandOutcome::with_event(
        capability,
        Value::Object(payload),
    ))
}

fn ensure_in_string_list(attrs: &Map<String, Value>, key: &str, value: &str) -> Result<()> {
    let Some(values) = attrs.get(key).and_then(Value::as_array) else {
        return Ok(());
    };
    if values.iter().any(|item| item.as_str() == Some(value)) {
        return Ok(());
    }
    Err(anyhow!("{} does not support value {}", key, value))
}

fn temperature(value: f64, unit: &str) -> Value {
    json!({
        "_type": "Temperature",
        "value": value,
        "unit": unit
    })
}

fn normalized_audio(audio: &Map<String, Value>) -> Map<String, Value> {
    let mut normalized = Map::new();
    for key in ["url", "base64", "mimeType", "createdAt"] {
        if let Some(value) = audio.get(key) {
            normalized.insert(key.to_string(), value.clone());
        }
    }
    normalized
}

fn audio_title(audio: &Map<String, Value>) -> String {
    if let Some(url) = audio
        .get("url")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        return format!("Audio announcement ({})", url);
    }
    if audio
        .get("base64")
        .and_then(Value::as_str)
        .map(str::trim)
        .is_some_and(|value| !value.is_empty())
    {
        return "Audio announcement (base64)".to_string();
    }
    "Audio announcement".to_string()
}

fn duration_seconds(value: Option<&Value>) -> Option<f64> {
    match value? {
        Value::Number(number) => number.as_f64(),
        Value::String(text) => parse_duration_seconds(text),
        Value::Object(map) => {
            let raw = map.get("value")?.as_f64()?;
            let unit = map.get("unit")?.as_str()?;
            duration_unit_multiplier(unit).map(|multiplier| raw * multiplier)
        }
        _ => None,
    }
}

fn parse_duration_seconds(text: &str) -> Option<f64> {
    let trimmed = text.trim();
    let split_at = trimmed
        .find(|ch: char| !(ch.is_ascii_digit() || ch == '.'))
        .unwrap_or(trimmed.len());
    let value = trimmed.get(..split_at)?.parse::<f64>().ok()?;
    let unit = trimmed.get(split_at..)?.trim();
    duration_unit_multiplier(unit).map(|multiplier| value * multiplier)
}

fn duration_unit_multiplier(unit: &str) -> Option<f64> {
    match unit {
        "ms" | "millisecond" | "milliseconds" => Some(0.001),
        "s" | "sec" | "second" | "seconds" => Some(1.0),
        "min" | "minute" | "minutes" => Some(60.0),
        "h" | "hr" | "hour" | "hours" => Some(3600.0),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn door_lock_inventory_discovery_commands_and_state_share_the_new_contract() {
        let mut attrs = default_attributes(LOCK_DEADBOLT_V1).unwrap();
        let caps = capabilities(LOCK_DEADBOLT_V1).unwrap();
        assert!(caps.contains(&"data.doorLockAttributes".to_string()));
        assert!(!caps
            .iter()
            .any(|s| s == "state.lock" || s == "command.lockUnlock"));
        let facts = project_discovery(LOCK_DEADBOLT_V1, &attrs, "state.doorLock", "$f1").unwrap();
        assert_eq!(facts,object_map(json!({"supportedCommands":["lock","unlock","unlatch"],"canGetDoorState":false,"pinCodeRequirement":"unsupported"})).unwrap());
        assert_eq!(
            project_state(LOCK_DEADBOLT_V1, &attrs, "data.doorLockAttributes").unwrap(),
            facts
        );
        for (action, state) in [
            ("lock", "locked"),
            ("unlock", "unlocked"),
            ("unlatch", "unlatched"),
        ] {
            let outcome = handle_command(
                LOCK_DEADBOLT_V1,
                &mut attrs,
                "command.doorLock",
                &object_map(json!({"command":action})).unwrap(),
            )
            .unwrap();
            assert_eq!(outcome.events[0].payload["lockState"], state);
            assert_eq!(outcome.events[0].capability, "state.doorLock");
        }
        let before = attrs.clone();
        assert!(handle_command(
            LOCK_DEADBOLT_V1,
            &mut attrs,
            "command.doorLock",
            &object_map(json!({"command":"unlock","pinCode":"0012"})).unwrap()
        )
        .is_err());
        assert_eq!(attrs, before);
    }

    #[test]
    fn cover_stop_retains_position_and_does_not_move_the_simulated_device() {
        for model in [COVER_SHADE_V1, COVER_GARAGE_DOOR_V1] {
            let mut attrs = default_attributes(model).unwrap();
            let discovery =
                project_discovery(model, &attrs, "command.coverControl", "$f1").unwrap();
            assert_eq!(
                discovery["supportedCommands"],
                json!(["open", "close", "stop"])
            );
            for (position, state) in [(0, "closed"), (35, "partially open"), (100, "open")] {
                attrs.insert("coverPosition".into(), json!(position));
                attrs.insert("coverState".into(), json!(state));
                attrs.insert("coverMotionState".into(), json!("closing"));
                let outcome = handle_command(
                    model,
                    &mut attrs,
                    "command.coverControl",
                    &object_map(json!({"command":"stop"})).unwrap(),
                )
                .unwrap();
                assert_eq!(attrs["coverPosition"].as_f64(), Some(f64::from(position)));
                assert_eq!(
                    outcome.events[0].payload["opening"].as_f64(),
                    Some(f64::from(position))
                );
                assert_eq!(outcome.events[0].payload["openingState"], json!(state));
                assert_eq!(outcome.events[0].payload["motionState"], "stopped");
            }
            let before = attrs["coverPosition"].as_f64();
            assert!(handle_command(
                model,
                &mut attrs,
                "command.coverControl",
                &object_map(json!({"command":"stop","opening":0})).unwrap(),
            )
            .is_err());
            assert_eq!(attrs["coverPosition"].as_f64(), before);
            let before = attrs.clone();
            for request in [
                json!({"command":"open","opening":101}),
                json!({"command":"open","tiltPosition":50}),
                json!({"command":"open","transitionDuration":{"_type":"Duration","value":1,"unit":"sec"}}),
            ] {
                assert!(handle_command(
                    model,
                    &mut attrs,
                    "command.coverControl",
                    &object_map(request).unwrap()
                )
                .is_err());
                assert_eq!(attrs, before);
            }
        }
    }

    #[test]
    fn cover_snapshots_and_discovery_preserve_independent_feedback_and_reject_old_inputs() {
        for model in [COVER_SHADE_V1, COVER_GARAGE_DOOR_V1] {
            let mut attrs = default_attributes(model).unwrap();
            assert!(capabilities(model)
                .unwrap()
                .contains(&"data.coverAttributes".into()));
            assert!(!capabilities(model)
                .unwrap()
                .contains(&"command.controlCover".into()));
            assert_eq!(
                project_state(model, &attrs, "data.coverAttributes").unwrap(),
                project_discovery(model, &attrs, "state.cover", "$f1").unwrap()
            );
            let payload = object_map(
                json!({"openingState":"partially open","motionState":"closing","opening":35.5}),
            )
            .unwrap();
            apply_observed_payload(model, &mut attrs, "state.cover", &payload).unwrap();
            assert_eq!(
                project_state(model, &attrs, "state.cover").unwrap(),
                payload
            );
            let cleared =
                object_map(json!({"openingState":null,"motionState":null,"opening":null})).unwrap();
            apply_observed_payload(model, &mut attrs, "state.cover", &cleared).unwrap();
            assert_eq!(
                project_state(model, &attrs, "state.cover").unwrap(),
                cleared
            );
            let outcome = handle_command(model, &mut attrs, "command.coverControl", &object_map(json!({"opening":52.25,"transitionDuration":{"_type":"Duration","unit":"ms","value":0}})).unwrap()).unwrap();
            assert_eq!(
                outcome.events[0].payload,
                json!({"openingState":"partially open","motionState":"stopped","opening":52.25})
            );
            let before = attrs.clone();
            for request in [
                json!({"position":20}),
                json!({"command":"open","opening":20}),
                json!({"command":"open","latchState":"unlatched"}),
                json!({"command":"open","tiltOpening":20}),
                json!({"command":"open","speed":"high"}),
                json!({"command":"open","positionTransitionDuration":0}),
            ] {
                assert!(handle_command(
                    model,
                    &mut attrs,
                    "command.coverControl",
                    request.as_object().unwrap()
                )
                .is_err());
                assert_eq!(attrs, before);
            }
            assert!(apply_observed_payload(
                model,
                &mut attrs,
                "state.cover",
                &object_map(json!({"opening":101,"motionState":"opening"})).unwrap()
            )
            .is_err());
            assert_eq!(attrs, before);
            attrs.insert("canGetPosition".into(), json!(false));
            let facts = project_discovery(model, &attrs, "state.cover", "$f1").unwrap();
            assert_eq!(
                facts["supportedStates"],
                json!(["openingState", "motionState"])
            );
            assert_eq!(facts["supportedSettings"], json!(["opening"]));
            assert!(!project_state(model, &attrs, "state.cover")
                .unwrap()
                .contains_key("opening"));
        }
    }

    #[test]
    fn button_catalog_and_events_use_the_canonical_contract() {
        let device = normalize_instance(DeviceInstance {
            id: "button".into(),
            display_name: "Button".into(),
            model_id: BUTTON_SCENE_V1.into(),
            attributes: object_map(json!({"supportedPressTypes": ["single", "double"]})).unwrap(),
        })
        .unwrap();
        let mut attrs = device.attributes;
        let catalog =
            project_discovery(BUTTON_SCENE_V1, &attrs, "event.buttonPress", "$f1").unwrap();
        assert_eq!(
            catalog,
            object_map(json!({"supportedPressTypes": ["single", "double"]})).unwrap()
        );
        let payload = object_map(json!({"occurred": true, "pressedType": "double"})).unwrap();
        apply_observed_payload(BUTTON_SCENE_V1, &mut attrs, "event.buttonPress", &payload).unwrap();
        assert_eq!(
            project_state(BUTTON_SCENE_V1, &attrs, "event.buttonPress").unwrap(),
            payload
        );
        assert_eq!(
            project_discovery(BUTTON_SCENE_V1, &attrs, "event.buttonPress", "$f1").unwrap(),
            catalog
        );
        attrs.remove("supportedPressTypes");
        assert!(project_discovery(BUTTON_SCENE_V1, &attrs, "event.buttonPress", "$f1").is_err());
    }

    #[test]
    fn thermostat_feedback_and_ambient_temperature_have_separate_contracts() {
        let mut attrs = default_attributes(THERMOSTAT_FAN_V1).unwrap();
        let before = project_state(THERMOSTAT_FAN_V1, &attrs, "state.thermostat").unwrap();
        assert!(!before.contains_key("currentTemperature"));
        assert!(!before.contains_key("runningMode"));
        assert_eq!(before["runningState"], "idle");
        let facts =
            project_discovery(THERMOSTAT_FAN_V1, &attrs, "state.thermostat", "$f1").unwrap();
        assert_eq!(facts["supportCool"], true);
        assert_eq!(facts["supportHeat"], true);
        attrs.insert("temperature".into(), temperature(19.0, "C"));
        assert_eq!(
            project_state(THERMOSTAT_FAN_V1, &attrs, "state.thermostat").unwrap(),
            before
        );
        assert_eq!(
            project_state(THERMOSTAT_FAN_V1, &attrs, "state.temperature").unwrap()["temperature"],
            temperature(19.0, "C")
        );
        handle_command(
            THERMOSTAT_FAN_V1,
            &mut attrs,
            "command.setThermostatMode",
            &object_map(json!({"hvacMode":"auto"})).unwrap(),
        )
        .unwrap();
        assert_eq!(
            project_state(THERMOSTAT_FAN_V1, &attrs, "state.thermostat").unwrap()["runningState"],
            "idle"
        );
    }

    #[test]
    fn thermostat_rejects_crossed_targets_before_mutating_state() {
        for request in [
            json!({"heatingSetpoint":temperature(25.0,"C"),"coolingSetpoint":temperature(20.0,"C")}),
            json!({"heatingSetpoint":temperature(26.0,"C")}),
            json!({"coolingSetpoint":temperature(19.0,"C")}),
            json!({"heatingSetpoint":temperature(78.8,"F"),"coolingSetpoint":temperature(25.0,"C")}),
            json!({"heatingSetpoint":temperature(20.430000001,"C"),"coolingSetpoint":temperature(68.774,"F")}),
        ] {
            let mut attrs = default_attributes(THERMOSTAT_FAN_V1).unwrap();
            let before = attrs.clone();
            let result = handle_command(
                THERMOSTAT_FAN_V1,
                &mut attrs,
                "command.setThermostatTemperature",
                &object_map(request).unwrap(),
            );
            assert!(result.is_err(), "crossed targets must fail");
            assert_eq!(attrs, before, "rejection must leave both targets unchanged");
        }
    }

    #[test]
    fn thermostat_single_target_preserves_the_other_target_and_accepts_equal_temperatures() {
        let mut attrs = default_attributes(THERMOSTAT_FAN_V1).unwrap();
        let old_cooling = attrs["coolingSetpoint"].clone();
        let outcome = handle_command(
            THERMOSTAT_FAN_V1,
            &mut attrs,
            "command.setThermostatTemperature",
            &object_map(json!({"heatingSetpoint":temperature(77.0,"F")})).unwrap(),
        )
        .unwrap();
        assert_eq!(attrs["coolingSetpoint"], old_cooling);
        assert_eq!(attrs["heatingSetpoint"], temperature(77.0, "F"));
        assert_eq!(
            outcome.events[0].payload["heatingSetpoint"],
            temperature(77.0, "F")
        );
        for (heating, cooling) in [
            (temperature(20.43, "C"), temperature(68.774, "F")),
            (temperature(68.774, "F"), temperature(20.43, "C")),
        ] {
            handle_command(
                THERMOSTAT_FAN_V1,
                &mut attrs,
                "command.setThermostatTemperature",
                &object_map(json!({"heatingSetpoint":heating,"coolingSetpoint":cooling})).unwrap(),
            )
            .unwrap();
        }
    }

    #[test]
    fn thermostat_coupled_validation_requires_known_valid_target_units() {
        for other in [Value::Null, json!({"value":20,"unit":"K"}), json!("20C")] {
            let mut attrs = default_attributes(THERMOSTAT_FAN_V1).unwrap();
            attrs.insert("heatingSetpoint".into(), other);
            let before = attrs.clone();
            assert!(handle_command(
                THERMOSTAT_FAN_V1,
                &mut attrs,
                "command.setThermostatTemperature",
                &object_map(json!({"coolingSetpoint":temperature(22.0,"C")})).unwrap()
            )
            .is_err());
            assert_eq!(attrs, before);
        }
        let mut attrs = default_attributes(THERMOSTAT_FAN_V1).unwrap();
        attrs.insert("heatingSetpoint".into(), Value::Null);
        assert!(handle_command(THERMOSTAT_FAN_V1, &mut attrs,
            "command.setThermostatTemperature",
            &object_map(json!({"heatingSetpoint":temperature(20.0,"C"),"coolingSetpoint":temperature(22.0,"C")})).unwrap()).is_ok());
    }

    #[test]
    fn partial_rgb_preserves_the_other_component_and_validates_before_mutation() {
        let model = LIGHT_RGB_DIMMER_V1;
        let mut attrs = default_attributes(model).unwrap();
        attrs.insert("colorHue".into(), json!(35));
        attrs.insert("colorSaturation".into(), json!(70));
        for (request, expected) in [
            (json!({"hue":120.5}), json!({"hue":120.5,"saturation":70.0})),
            (
                json!({"saturation":0}),
                json!({"hue":120.5,"saturation":0.0}),
            ),
        ] {
            handle_command(
                model,
                &mut attrs,
                "command.setColorRGB",
                request.as_object().unwrap(),
            )
            .unwrap();
            assert_eq!(
                json!(project_state(model, &attrs, "state.colorRGB").unwrap()),
                expected
            );
        }
        for request in [
            json!({}),
            json!({"hue":null}),
            json!({"hue":10,"saturation":-1}),
            json!({"hue":361}),
        ] {
            let before = attrs.clone();
            assert!(handle_command(
                model,
                &mut attrs,
                "command.setColorRGB",
                request.as_object().unwrap()
            )
            .is_err());
            assert_eq!(attrs, before);
        }
        attrs.insert("colorHue".into(), Value::Null);
        let before = attrs.clone();
        assert!(handle_command(
            model,
            &mut attrs,
            "command.setColorRGB",
            json!({"saturation":20}).as_object().unwrap()
        )
        .is_err());
        assert_eq!(attrs, before);
    }

    #[test]
    fn fan_atomic_settings_preserve_other_fields_unknowns_and_validate_before_mutation() {
        let mut attrs = default_attributes(FAN_PURIFIER_V1).unwrap();
        let before = attrs.clone();
        for req in [
            json!({}),
            json!({"speed":101}),
            json!({"speed":"50"}),
            json!({"speed":50,"mode":"auto"}),
            json!({"oscillation":"other"}),
        ] {
            assert!(handle_command(
                FAN_PURIFIER_V1,
                &mut attrs,
                "command.setFan",
                req.as_object().unwrap()
            )
            .is_err());
            assert_eq!(attrs, before);
        }
        let out = handle_command(
            FAN_PURIFIER_V1,
            &mut attrs,
            "command.setFan",
            json!({"oscillation":"on"}).as_object().unwrap(),
        )
        .unwrap();
        assert_eq!(out.events[0].payload["oscillation"], "on");
        assert_eq!(attrs["fanSpeed"], before["fanSpeed"]);
        apply_observed_payload(
            FAN_PURIFIER_V1,
            &mut attrs,
            "state.fan",
            json!({"speed":null}).as_object().unwrap(),
        )
        .unwrap();
        assert!(project_state(FAN_PURIFIER_V1, &attrs, "state.fan").unwrap()["speed"].is_null());
    }

    #[test]
    fn immutable_catalogs_survive_device_controls() {
        for (model, capability, command, request) in [
            (
                LIGHT_RGB_DIMMER_V1,
                "state.colorTemperature",
                "command.setColorTemperature",
                json!({"colorTemperature": 5000}),
            ),
            (
                COVER_SHADE_V1,
                "state.cover",
                "command.coverControl",
                json!({"command": "open"}),
            ),
            (
                FAN_PURIFIER_V1,
                "state.fan",
                "command.setFan",
                json!({"mode": "turbo"}),
            ),
            (
                THERMOSTAT_FAN_V1,
                "state.thermostat",
                "command.setThermostatMode",
                json!({"hvacMode": "heat"}),
            ),
            (
                THERMOSTAT_FAN_V1,
                "state.thermostat",
                "command.setThermostatMode",
                json!({"hvacMode": "cool"}),
            ),
            (
                THERMOSTAT_FAN_V1,
                "state.thermostat",
                "command.setThermostatMode",
                json!({"hvacMode": "off"}),
            ),
        ] {
            let mut attrs = default_attributes(model).unwrap();
            let before = project_discovery(model, &attrs, capability, "$f1").unwrap();
            handle_command(model, &mut attrs, command, &object_map(request).unwrap()).unwrap();
            assert_eq!(
                project_discovery(model, &attrs, capability, "$f1").unwrap(),
                before,
                "{model}: {command}"
            );
        }
        for (model, capability, payload) in [
            (
                AIR_MONITOR_V1,
                "state.airCompositionSensor",
                json!({"pm25": 90, "qualityState": "poor"}),
            ),
            (
                SECURITY_SYSTEM_V1,
                "state.securitySystem",
                json!({"state": "armedAway"}),
            ),
        ] {
            let mut attrs = default_attributes(model).unwrap();
            let before = project_discovery(model, &attrs, capability, "$f1").unwrap();
            apply_observed_payload(model, &mut attrs, capability, &object_map(payload).unwrap())
                .unwrap();
            assert_eq!(
                project_discovery(model, &attrs, capability, "$f1").unwrap(),
                before
            );
        }
    }

    #[test]
    fn speaker_defaults_project_basic_state() {
        let attrs = default_attributes(SPEAKER_MEDIA_BROADCAST_V1).unwrap();
        assert_eq!(
            project_state(SPEAKER_MEDIA_BROADCAST_V1, &attrs, "state.audioVolume")
                .unwrap()
                .get("volume")
                .and_then(Value::as_f64),
            Some(35.0)
        );
        assert_eq!(
            project_state(SPEAKER_MEDIA_BROADCAST_V1, &attrs, "state.audioMute")
                .unwrap()
                .get("state")
                .and_then(Value::as_str),
            Some("unmute")
        );
    }

    #[test]
    fn speaker_commands_mutate_canonical_attributes() {
        let mut attrs = default_attributes(SPEAKER_MEDIA_BROADCAST_V1).unwrap();
        handle_command(
            SPEAKER_MEDIA_BROADCAST_V1,
            &mut attrs,
            "command.setAudioVolume",
            &object_map(json!({ "volume": 22 })).unwrap(),
        )
        .unwrap();
        handle_command(
            SPEAKER_MEDIA_BROADCAST_V1,
            &mut attrs,
            "command.setAudioMute",
            &object_map(json!({ "command": "mute" })).unwrap(),
        )
        .unwrap();
        handle_command(
            SPEAKER_MEDIA_BROADCAST_V1,
            &mut attrs,
            "command.playMedia",
            &object_map(json!({ "mediaContentId": "playlist:dinner" })).unwrap(),
        )
        .unwrap();

        assert_eq!(attrs.get("volume").and_then(Value::as_f64), Some(22.0));
        assert_eq!(attrs.get("muted").and_then(Value::as_bool), Some(true));
        assert_eq!(
            attrs
                .get("track")
                .and_then(Value::as_object)
                .and_then(|track| track.get("title"))
                .and_then(Value::as_str),
            Some("playlist:dinner")
        );
    }

    #[test]
    fn broadcast_audio_uses_audio_request_and_updates_verifiable_attributes() {
        let mut attrs = default_attributes(SPEAKER_MEDIA_BROADCAST_V1).unwrap();
        let outcome = handle_command(
            SPEAKER_MEDIA_BROADCAST_V1,
            &mut attrs,
            "command.broadcastAudio",
            &object_map(json!({
                "audio": {
                    "url": "playground://audio/dinner.mp3",
                    "mimeType": "audio/mpeg",
                    "createdAt": "2099-01-01T00:00:00Z"
                },
                "volume": 64
            }))
            .unwrap(),
        )
        .unwrap();

        assert_eq!(outcome.events.len(), 1);
        assert_eq!(
            attrs.get("broadcastCount").and_then(Value::as_f64),
            Some(1.0)
        );
        assert_eq!(attrs.get("lastVolume").and_then(Value::as_f64), Some(64.0));
        assert_eq!(
            attrs
                .get("lastAudio")
                .and_then(Value::as_object)
                .and_then(|audio| audio.get("url"))
                .and_then(Value::as_str),
            Some("playground://audio/dinner.mp3")
        );
        assert_eq!(
            project_state(SPEAKER_MEDIA_BROADCAST_V1, &attrs, "state.mediaPlayback")
                .unwrap()
                .get("state")
                .and_then(Value::as_str),
            Some("playing")
        );
    }

    #[test]
    fn observed_payload_updates_canonical_attributes() {
        let mut attrs = default_attributes(LIGHT_RGB_DIMMER_V1).unwrap();
        apply_observed_payload(
            LIGHT_RGB_DIMMER_V1,
            &mut attrs,
            "state.colorRGB",
            &object_map(json!({
                "hue": 180,
                "saturation": 60
            }))
            .unwrap(),
        )
        .unwrap();

        assert_eq!(attrs.get("colorHue").and_then(Value::as_f64), Some(180.0));
        assert_eq!(
            attrs.get("colorSaturation").and_then(Value::as_f64),
            Some(60.0)
        );
        assert_eq!(
            project_state(LIGHT_RGB_DIMMER_V1, &attrs, "state.colorRGB")
                .unwrap()
                .get("hue")
                .and_then(Value::as_f64),
            Some(180.0)
        );
    }
    #[test]
    fn both_air_sensor_contracts_keep_their_own_catalogs_states_and_updates() {
        for model in [AIR_MONITOR_V1, FAN_PURIFIER_V1] {
            let mut attrs = default_attributes(model).unwrap();
            let catalog = project_capability_data(model, &attrs).unwrap();
            for cap in ["airCompositionSensor", "airQualitySensor"] {
                let state_id = format!("state.{cap}");
                let data_id = format!("data.{cap}Attributes");
                assert!(exposes_capability(model, &state_id));
                assert!(exposes_capability(model, &data_id));
                assert_eq!(
                    catalog[&data_id]["states"],
                    catalog[&state_id]["attributes"]
                );
                let discovery = project_discovery(model, &attrs, &state_id, "$f1").unwrap();
                assert_eq!(
                    discovery["supportedStates"].as_array().unwrap().len(),
                    air_sensor_fields(model, &state_id).len()
                );
            }
            apply_observed_payload(
                model,
                &mut attrs,
                "state.airCompositionSensor",
                &object_map(json!({"pm25":null,"qualityState":"poor"})).unwrap(),
            )
            .unwrap();
            let state = project_state(model, &attrs, "state.airCompositionSensor").unwrap();
            assert!(state["pm25"].is_null());
            assert_eq!(state["pm25State"], "low");
            assert!(!state.contains_key("qualityState"));
            assert_eq!(
                project_state(model, &attrs, "state.airQualitySensor").unwrap()["qualityState"],
                "good"
            );
            apply_observed_payload(
                model,
                &mut attrs,
                "state.airQualitySensor",
                &object_map(json!({"qualityState":"poor","index":null,"co2":999})).unwrap(),
            )
            .unwrap();
            let quality = project_state(model, &attrs, "state.airQualitySensor").unwrap();
            assert_eq!(quality["qualityState"], "poor");
            assert!(quality["index"].is_null());
            assert!(!quality.contains_key("co2"));
            assert!(!exposes_capability(model, "state.air"));
            assert!(!exposes_capability(model, "data.airAttributes"));
        }
    }
    #[test]
    fn playground_detectors_preserve_independent_alarms_measurements_and_combined_controls() {
        let model = SMOKE_CO_ALARM_V1;
        let mut attrs = default_attributes(model).unwrap();
        assert!(!exposes_capability(model, "state.air"));
        assert!(!exposes_capability(model, "state.smoke"));
        let catalog = project_capability_data(model, &attrs).unwrap();
        assert_eq!(
            catalog["data.carbonMonoxideDetectorAttributes"]["states"],
            catalog["state.carbonMonoxideDetector"]["attributes"]
        );
        apply_observed_payload(model,&mut attrs,"state.carbonMonoxideDetector",&object_map(json!({"concentration":0.5,"concentrationState":"critical","alarmState":"detected"})).unwrap()).unwrap();
        assert_eq!(
            project_state(model, &attrs, "state.carbonMonoxideDetector").unwrap()["concentration"],
            0.5
        );
        assert_eq!(
            project_state(model, &attrs, "state.smokeDetector").unwrap()["alarmState"],
            "clear"
        );
        let result =
            handle_command(model, &mut attrs, "command.testSmokeDetector", &Map::new()).unwrap();
        assert_eq!(result.events.len(), 4);
        assert_eq!(result.events[0].payload["testState"], "testing");
        assert_eq!(
            project_state(model, &attrs, "state.carbonMonoxideDetector").unwrap()["alarmState"],
            "detected"
        );
        assert_eq!(
            project_state(model, &attrs, "state.smokeDetector").unwrap()["testResult"],
            "passed"
        );
        handle_command(
            model,
            &mut attrs,
            "command.setCarbonMonoxideDetectorMute",
            &object_map(json!({"muteState":"muted"})).unwrap(),
        )
        .unwrap();
        assert_eq!(
            project_state(model, &attrs, "state.smokeDetector").unwrap()["muteState"],
            "muted"
        );
        assert!(handle_command(
            model,
            &mut attrs,
            "command.setSmokeDetectorSensitivity",
            &object_map(json!({"sensitivity":"auto"})).unwrap()
        )
        .is_err());
        handle_command(
            model,
            &mut attrs,
            "command.setSmokeDetectorSensitivity",
            &object_map(json!({"sensitivity":"high"})).unwrap(),
        )
        .unwrap();
        assert_eq!(
            project_state(model, &attrs, "state.smokeDetectorSetting").unwrap()["sensitivity"],
            "high"
        );
        apply_observed_payload(
            model,
            &mut attrs,
            "state.carbonMonoxideDetector",
            &object_map(json!({"alarmState":null})).unwrap(),
        )
        .unwrap();
        let state = project_state(model, &attrs, "state.carbonMonoxideDetector").unwrap();
        assert!(state["alarmState"].is_null());
        assert_eq!(state["concentration"], 0.5);
    }

    #[test]
    fn electrical_discovery_and_counter_reads_preserve_zero_unknowns_and_resets() {
        for model in [LIGHT_RGB_DIMMER_V1, PLUG_ENERGY_V1] {
            let mut attrs = default_attributes(model).unwrap();
            assert_eq!(
                project_discovery(model, &attrs, "state.energy", "$f1").unwrap(),
                object_map(json!({"supportedCounters":["importedEnergy"]})).unwrap()
            );
            assert_eq!(
                project_discovery(model, &attrs, "state.power", "$f1").unwrap(),
                object_map(json!({"supportedMeasurements":["activePower"]})).unwrap()
            );
            for value in [json!(12.5), json!(0), json!(0.25), Value::Null] {
                apply_observed_payload(
                    model,
                    &mut attrs,
                    "state.energy",
                    &object_map(json!({"importedEnergy":value})).unwrap(),
                )
                .unwrap();
                let state = project_state(model, &attrs, "state.energy").unwrap();
                assert_eq!(state.len(), 1);
                assert_eq!(state["importedEnergy"].as_f64(), value.as_f64());
                assert_eq!(state["importedEnergy"].is_null(), value.is_null());
            }
            assert!(apply_observed_payload(
                model,
                &mut attrs,
                "state.energy",
                &object_map(json!({"energy":1})).unwrap()
            )
            .is_err());
            attrs.insert("importedEnergy".into(), json!(-1));
            assert!(project_state(model, &attrs, "state.energy").is_err());
        }
    }
}

#[cfg(test)]
mod cover_contract_migration_tests {
    use super::*;
    #[test]
    fn unsupported_latch_does_not_move_or_mutate_a_simulated_cover() {
        for latch in [false, true] {
            let mut attrs = Map::new();
            let original = attrs.clone();
            assert!(control_cover(
                &mut attrs,
                &object_map(json!({"command":"open","latch":latch})).unwrap()
            )
            .is_err());
            assert_eq!(attrs, original);
        }
    }
}

#[cfg(test)]
mod battery_contract_tests {
    use super::*;
    #[test]
    fn models_expose_independent_readings_and_discovery_even_when_unknown() {
        for (model, fields) in [
            (CONTACT_SENSOR_V1, vec!["levelState"]),
            (PRESENCE_SENSOR_V1, vec!["chargingState"]),
            (BUTTON_SCENE_V1, vec!["level"]),
            (LOCK_DEADBOLT_V1, vec!["level", "chargingState"]),
        ] {
            assert!(exposes_capability(model, "data.batteryAttributes"));
            let mut attrs = default_attributes(model).unwrap();
            let facts = project_state(model, &attrs, "data.batteryAttributes").unwrap();
            assert_eq!(facts["supportedStates"], json!(fields));
            for field in &fields {
                attrs.insert(battery_attribute(field).into(), Value::Null);
            }
            let unknown = project_state(model, &attrs, "state.battery").unwrap();
            assert_eq!(unknown.len(), fields.len());
            assert!(unknown.values().all(Value::is_null));
            assert_eq!(
                project_discovery(model, &attrs, "state.battery", "$f1").unwrap(),
                facts
            );
        }
    }
    #[test]
    fn partial_updates_are_atomic_and_preserve_decimals_zero_and_null() {
        let model = LOCK_DEADBOLT_V1;
        let mut attrs = default_attributes(model).unwrap();
        for value in [json!(83.5), json!(0), Value::Null] {
            apply_observed_payload(
                model,
                &mut attrs,
                "state.battery",
                &object_map(json!({"level":value})).unwrap(),
            )
            .unwrap();
            let state = project_state(model, &attrs, "state.battery").unwrap();
            assert_eq!(state["level"], value);
            assert_eq!(state["chargingState"], "not charging");
        }
        for bad in [
            json!({"level":101}),
            json!({"level":50,"chargingState":"unknown"}),
            json!({"levelState":"low"}),
        ] {
            let before = attrs.clone();
            assert!(apply_observed_payload(
                model,
                &mut attrs,
                "state.battery",
                bad.as_object().unwrap()
            )
            .is_err());
            assert_eq!(attrs, before);
        }
    }
}
