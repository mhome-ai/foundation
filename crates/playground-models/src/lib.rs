use anyhow::{anyhow, Result};
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
            "energy": 1.8
        })),
        PLUG_ENERGY_V1 => object_map(json!({
            "online": true,
            "on": false,
            "power": 0,
            "energy": 1.2
        })),
        COVER_SHADE_V1 => object_map(json!({
            "online": true,
            "coverState": "closed",
            "coverPosition": 0,
            "coverDeviceType": "shade",
            "supportPosition": true
        })),
        COVER_GARAGE_DOOR_V1 => object_map(json!({
            "online": true,
            "coverState": "closed",
            "coverPosition": 0,
            "coverDeviceType": "garageDoor",
            "supportPosition": true,
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
            "batteryLevel": 91,
            "chargingState": "not charging",
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
            "airQualityLevel": "good",
            "airQualityIndex": 32,
            "co2": 520,
            "co2Level": "low",
            "pm25": 7,
            "pm25Level": "low",
            "voc": 0.18,
            "vocLevel": "low",
            "batteryLevel": 100,
            "chargingState": "full"
        })),
        SMOKE_CO_ALARM_V1 => object_map(json!({
            "online": true,
            "smokeState": "clear",
            "carbonMonoxideState": "clear",
            "carbonMonoxideConcentration": 0,
            "batteryLevel": 96,
            "chargingState": "not charging",
            "tamperState": "clear",
            "alarmMode": "home",
            "availableAlarmModes": ["off", "home", "away", "night"],
            "chimeOccurred": false
        })),
        THERMOSTAT_FAN_V1 => object_map(json!({
            "online": true,
            "temperature": temperature(22.0, "C"),
            "currentTemperature": temperature(22.0, "C"),
            "hvacMode": "auto",
            "presetMode": "home",
            "runningMode": "off",
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
            "supportFanSpeedPercentage": true,
            "supportFanMode": true
        })),
        FAN_PURIFIER_V1 => object_map(json!({
            "online": true,
            "on": false,
            "fanMode": "auto",
            "fanSpeed": 30,
            "availableFanModes": ["auto", "sleep", "low", "medium", "high", "turbo"],
            "supportFanSpeedPercentage": true,
            "supportFanMode": true,
            "airQualityLevel": "good",
            "airQualityIndex": 35,
            "pm25": 8,
            "pm25Level": "low"
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
            "lastButton": "button1",
            "lastPressType": "single",
            "pressCount": 0
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
        COVER_SHADE_V1 => &["state.coverOpenClose", "command.coverOpenClose"][..],
        COVER_GARAGE_DOOR_V1 => &[
            "state.coverOpenClose",
            "command.coverOpenClose",
            "state.contactSensor",
        ][..],
        LOCK_DEADBOLT_V1 => &["state.lock", "state.battery", "command.lockUnlock"][..],
        CONTACT_SENSOR_V1 => &["state.contactSensor", "state.battery", "state.tamper"][..],
        MOTION_SENSOR_V1 => &["state.motion", "state.battery", "state.tamper"][..],
        ILLUMINANCE_SENSOR_V1 => &["state.illuminance", "state.battery"][..],
        PRESENCE_SENSOR_V1 => &["state.presence", "state.battery"][..],
        LEAK_SENSOR_V1 => &["state.water", "state.battery", "state.tamper"][..],
        AIR_MONITOR_V1 => &[
            "state.temperature",
            "state.humidity",
            "state.airQuality",
            "state.co2",
            "state.pm25",
            "state.voc",
            "state.battery",
        ][..],
        SMOKE_CO_ALARM_V1 => &[
            "state.smoke",
            "state.carbonMonoxide",
            "state.battery",
            "state.tamper",
            "state.alarm",
            "command.chime",
            "event.chime",
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
            "state.airQuality",
            "state.pm25",
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
    Ok(values.iter().map(|value| (*value).to_string()).collect())
}

pub fn exposes_capability(model_id: &str, capability: &str) -> bool {
    capabilities(model_id)
        .map(|items| items.iter().any(|item| item == capability))
        .unwrap_or(false)
}

pub fn project_state(
    model_id: &str,
    attrs: &Map<String, Value>,
    capability: &str,
) -> Result<Map<String, Value>> {
    let _ = model_id;
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
            "power": number_attr(attrs, "power")?
        }))?),
        "state.energy" => Ok(object_map(json!({
            "energy": number_attr(attrs, "energy")?
        }))?),
        "state.coverOpenClose" => Ok(object_map(json!({
            "state": string_attr(attrs, "coverState")?,
            "position": number_attr(attrs, "coverPosition")?,
            "deviceType": string_attr(attrs, "coverDeviceType")?
        }))?),
        "state.contactSensor" => Ok(object_map(json!({
            "state": string_attr(attrs, "contactState")?
        }))?),
        "state.lock" => Ok(object_map(json!({
            "lockState": string_attr(attrs, "lockState")?
        }))?),
        "state.battery" => Ok(object_map(json!({
            "level": number_attr(attrs, "batteryLevel")?,
            "chargingState": string_attr(attrs, "chargingState")?
        }))?),
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
        "state.airQuality" => Ok(object_map(json!({
            "level": string_attr(attrs, "airQualityLevel")?,
            "index": number_attr(attrs, "airQualityIndex")?
        }))?),
        "state.co2" => Ok(object_map(json!({
            "concentration": number_attr(attrs, "co2")?,
            "level": string_attr(attrs, "co2Level")?
        }))?),
        "state.pm25" => Ok(object_map(json!({
            "concentration": number_attr(attrs, "pm25")?,
            "level": string_attr(attrs, "pm25Level")?
        }))?),
        "state.voc" => Ok(object_map(json!({
            "concentration": number_attr(attrs, "voc")?,
            "level": string_attr(attrs, "vocLevel")?
        }))?),
        "state.smoke" => Ok(object_map(json!({
            "state": string_attr(attrs, "smokeState")?
        }))?),
        "state.carbonMonoxide" => Ok(object_map(json!({
            "state": string_attr(attrs, "carbonMonoxideState")?,
            "concentration": number_attr(attrs, "carbonMonoxideConcentration")?
        }))?),
        "state.alarm" => Ok(object_map(json!({
            "mode": string_attr(attrs, "alarmMode")?
        }))?),
        "state.fan" => Ok(object_map(json!({
            "mode": string_attr(attrs, "fanMode")?,
            "speed": number_attr(attrs, "fanSpeed")?
        }))?),
        "state.thermostat" => Ok(object_map(json!({
            "currentTemperature": value_attr(attrs, "currentTemperature")?,
            "hvacMode": string_attr(attrs, "hvacMode")?,
            "presetMode": string_attr(attrs, "presetMode")?,
            "runningMode": string_attr(attrs, "runningMode")?,
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
        "event.chime" => Ok(object_map(json!({
            "occurred": bool_attr(attrs, "chimeOccurred")?
        }))?),
        "event.buttonPress" => Ok(object_map(json!({
            "button": string_attr(attrs, "lastButton")?,
            "pressType": string_attr(attrs, "lastPressType")?,
            "count": number_attr(attrs, "pressCount")?
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
    let _ = model_id;
    match capability {
        "state.colorTemperature" | "command.setColorTemperature" => Ok(object_map(json!({
            "min": number_attr(attrs, "minColorTemperature")?,
            "max": number_attr(attrs, "maxColorTemperature")?
        }))?),
        "state.coverOpenClose" | "command.coverOpenClose" => Ok(object_map(json!({
            "supportPosition": bool_attr(attrs, "supportPosition")?
        }))?),
        "state.fan" | "command.setFan" => Ok(object_map(json!({
            "availableModes": attrs.get("availableFanModes").cloned().unwrap_or_else(|| Value::Array(Vec::new())),
            "supportSpeedPercentage": bool_attr(attrs, "supportFanSpeedPercentage")?,
            "supportMode": bool_attr(attrs, "supportFanMode")?
        }))?),
        "state.thermostat" | "command.setThermostatMode" | "command.setThermostatTemperature" => {
            Ok(object_map(json!({
                "minCoolingSetpoint": value_attr(attrs, "minCoolingSetpoint")?,
                "maxCoolingSetpoint": value_attr(attrs, "maxCoolingSetpoint")?,
                "minHeatingSetpoint": value_attr(attrs, "minHeatingSetpoint")?,
                "maxHeatingSetpoint": value_attr(attrs, "maxHeatingSetpoint")?,
                "availableHvacModes": attrs.get("availableHvacModes").cloned().unwrap_or_else(|| Value::Array(Vec::new())),
                "availablePresetModes": attrs.get("availablePresetModes").cloned().unwrap_or_else(|| Value::Array(Vec::new()))
            }))?)
        }
        "state.alarm" => Ok(object_map(json!({
            "availableModes": attrs.get("availableAlarmModes").cloned().unwrap_or_else(|| Value::Array(Vec::new()))
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
        "state.energy" => copy_payload_field(attrs, payload, "energy", "energy")?,
        "state.coverOpenClose" => {
            copy_payload_field(attrs, payload, "state", "coverState")?;
            copy_optional_payload_fields(
                attrs,
                payload,
                &[
                    ("position", "coverPosition"),
                    ("deviceType", "coverDeviceType"),
                ],
            );
        }
        "state.contactSensor" => copy_payload_field(attrs, payload, "state", "contactState")?,
        "state.lock" => copy_payload_field(attrs, payload, "lockState", "lockState")?,
        "state.battery" => {
            copy_payload_field(attrs, payload, "level", "batteryLevel")?;
            copy_optional_payload_fields(attrs, payload, &[("chargingState", "chargingState")]);
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
        "state.airQuality" => {
            copy_payload_field(attrs, payload, "level", "airQualityLevel")?;
            copy_payload_field(attrs, payload, "index", "airQualityIndex")?;
        }
        "state.co2" => {
            copy_payload_field(attrs, payload, "concentration", "co2")?;
            copy_payload_field(attrs, payload, "level", "co2Level")?;
        }
        "state.pm25" => {
            copy_payload_field(attrs, payload, "concentration", "pm25")?;
            copy_payload_field(attrs, payload, "level", "pm25Level")?;
        }
        "state.voc" => {
            copy_payload_field(attrs, payload, "concentration", "voc")?;
            copy_payload_field(attrs, payload, "level", "vocLevel")?;
        }
        "state.smoke" => copy_payload_field(attrs, payload, "state", "smokeState")?,
        "state.carbonMonoxide" => {
            copy_payload_field(attrs, payload, "state", "carbonMonoxideState")?;
            copy_optional_payload_fields(
                attrs,
                payload,
                &[("concentration", "carbonMonoxideConcentration")],
            );
        }
        "state.alarm" => copy_payload_field(attrs, payload, "mode", "alarmMode")?,
        "state.fan" => {
            copy_optional_payload_fields(
                attrs,
                payload,
                &[("mode", "fanMode"), ("speed", "fanSpeed")],
            );
        }
        "state.thermostat" => {
            copy_optional_payload_fields(
                attrs,
                payload,
                &[
                    ("currentTemperature", "currentTemperature"),
                    ("hvacMode", "hvacMode"),
                    ("presetMode", "presetMode"),
                    ("runningMode", "runningMode"),
                    ("coolingSetpoint", "coolingSetpoint"),
                    ("heatingSetpoint", "heatingSetpoint"),
                ],
            );
        }
        "event.chime" => copy_payload_field(attrs, payload, "occurred", "chimeOccurred")?,
        "event.buttonPress" => {
            copy_optional_payload_fields(
                attrs,
                payload,
                &[
                    ("button", "lastButton"),
                    ("pressType", "lastPressType"),
                    ("count", "pressCount"),
                ],
            );
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
    if !exposes_capability(model_id, capability) {
        return Err(anyhow!(
            "playground model {} does not expose capability {}",
            model_id,
            capability
        ));
    }
    match capability {
        "command.onOff" => set_on_off(attrs, request),
        "command.setBrightness" => set_brightness(attrs, request),
        "command.setColorTemperature" => set_color_temperature(attrs, request),
        "command.setColorRGB" => set_color_rgb(attrs, request),
        "command.coverOpenClose" => cover_open_close(attrs, request),
        "command.lockUnlock" => lock_unlock(attrs, request),
        "command.chime" => chime(attrs, request),
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
    attrs.insert(
        "colorHue".to_string(),
        json!(required_number(request, "hue")?.clamp(0.0, 360.0)),
    );
    attrs.insert(
        "colorSaturation".to_string(),
        json!(required_number(request, "saturation")?.clamp(0.0, 100.0)),
    );
    projected_event(attrs, "state.colorRGB")
}

fn cover_open_close(
    attrs: &mut Map<String, Value>,
    request: &Map<String, Value>,
) -> Result<CommandOutcome> {
    let position = request
        .get("position")
        .and_then(Value::as_f64)
        .map(|value| value.clamp(0.0, 100.0));
    let state = if let Some(command) = request.get("command").and_then(Value::as_str) {
        match command {
            "open" => "open",
            "close" => "closed",
            _ => return Err(anyhow!("unsupported coverOpenClose command: {}", command)),
        }
    } else if let Some(position) = position {
        if position <= 0.0 {
            "closed"
        } else if position >= 100.0 {
            "open"
        } else {
            "partially open"
        }
    } else {
        return Err(anyhow!("coverOpenClose requires command or position"));
    };
    attrs.insert("coverState".to_string(), Value::String(state.to_string()));
    if let Some(position) = position {
        attrs.insert("coverPosition".to_string(), json!(position));
    } else {
        attrs.insert(
            "coverPosition".to_string(),
            json!(if state == "open" { 100.0 } else { 0.0 }),
        );
    }
    if attrs.contains_key("contactState") {
        attrs.insert(
            "contactState".to_string(),
            Value::String(if state == "closed" { "closed" } else { "open" }.to_string()),
        );
    }
    projected_event(attrs, "state.coverOpenClose")
}

fn lock_unlock(
    attrs: &mut Map<String, Value>,
    request: &Map<String, Value>,
) -> Result<CommandOutcome> {
    let command = required_string(request, "command")?;
    let lock_state = match command.as_str() {
        "lock" => "locked",
        "unlock" | "unlatch" => "unlocked",
        _ => return Err(anyhow!("unsupported lockUnlock command: {}", command)),
    };
    attrs.insert(
        "lockState".to_string(),
        Value::String(lock_state.to_string()),
    );
    projected_event(attrs, "state.lock")
}

fn chime(attrs: &mut Map<String, Value>, request: &Map<String, Value>) -> Result<CommandOutcome> {
    let command = required_string(request, "command")?;
    let occurred = match command.as_str() {
        "on" => true,
        "off" => false,
        _ => return Err(anyhow!("unsupported chime command: {}", command)),
    };
    attrs.insert("chimeOccurred".to_string(), Value::Bool(occurred));
    if occurred {
        projected_event(attrs, "event.chime")
    } else {
        Ok(CommandOutcome::no_response())
    }
}

fn set_fan(attrs: &mut Map<String, Value>, request: &Map<String, Value>) -> Result<CommandOutcome> {
    if let Some(mode) = request.get("mode").and_then(Value::as_str) {
        ensure_in_string_list(attrs, "availableFanModes", mode)?;
        attrs.insert("fanMode".to_string(), Value::String(mode.to_string()));
    }
    if let Some(speed) = request.get("speed").and_then(Value::as_f64) {
        attrs.insert("fanSpeed".to_string(), json!(speed.clamp(0.0, 100.0)));
    }
    projected_event(attrs, "state.fan")
}

fn set_thermostat_mode(
    attrs: &mut Map<String, Value>,
    request: &Map<String, Value>,
) -> Result<CommandOutcome> {
    if let Some(mode) = request.get("hvacMode").and_then(Value::as_str) {
        ensure_in_string_list(attrs, "availableHvacModes", mode)?;
        attrs.insert("hvacMode".to_string(), Value::String(mode.to_string()));
        attrs.insert(
            "runningMode".to_string(),
            Value::String(if mode == "off" { "off" } else { mode }.to_string()),
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
}
