//! Wire messages of ESP-IDF protocomm security 2 and Wi-Fi provisioning
//! (`protocomm/proto`, `wifi_provisioning/proto`). Field tags are protocol.
use prost::{Enumeration, Message};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Enumeration)]
#[repr(i32)]
pub enum Status {
    Success = 0,
    InvalidSecScheme = 1,
    InvalidProto = 2,
    TooManySessions = 3,
    InvalidArgument = 4,
    InternalError = 5,
    CryptoError = 6,
    InvalidSession = 7,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Enumeration)]
#[repr(i32)]
pub enum SecSchemeVersion {
    SecScheme0 = 0,
    SecScheme1 = 1,
    SecScheme2 = 2,
}

#[derive(Clone, PartialEq, Message)]
pub struct SessionData {
    #[prost(enumeration = "SecSchemeVersion", tag = "2")]
    pub sec_ver: i32,
    #[prost(oneof = "session_data::Proto", tags = "12")]
    pub proto: Option<session_data::Proto>,
}

pub mod session_data {
    use prost::Oneof;

    #[derive(Clone, PartialEq, Oneof)]
    pub enum Proto {
        #[prost(message, tag = "12")]
        Sec2(super::Sec2Payload),
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Enumeration)]
#[repr(i32)]
pub enum Sec2MsgType {
    S2SessionCommand0 = 0,
    S2SessionResponse0 = 1,
    S2SessionCommand1 = 2,
    S2SessionResponse1 = 3,
}

#[derive(Clone, PartialEq, Message)]
pub struct S2SessionCmd0 {
    #[prost(bytes = "vec", tag = "1")]
    pub client_username: Vec<u8>,
    #[prost(bytes = "vec", tag = "2")]
    pub client_pubkey: Vec<u8>,
}

#[derive(Clone, PartialEq, Message)]
pub struct S2SessionResp0 {
    #[prost(enumeration = "Status", tag = "1")]
    pub status: i32,
    #[prost(bytes = "vec", tag = "2")]
    pub device_pubkey: Vec<u8>,
    #[prost(bytes = "vec", tag = "3")]
    pub device_salt: Vec<u8>,
}

#[derive(Clone, PartialEq, Message)]
pub struct S2SessionCmd1 {
    #[prost(bytes = "vec", tag = "1")]
    pub client_proof: Vec<u8>,
}

#[derive(Clone, PartialEq, Message)]
pub struct S2SessionResp1 {
    #[prost(enumeration = "Status", tag = "1")]
    pub status: i32,
    #[prost(bytes = "vec", tag = "2")]
    pub device_proof: Vec<u8>,
    #[prost(bytes = "vec", tag = "3")]
    pub device_nonce: Vec<u8>,
}

#[derive(Clone, PartialEq, Message)]
pub struct Sec2Payload {
    #[prost(enumeration = "Sec2MsgType", tag = "1")]
    pub msg: i32,
    #[prost(oneof = "sec2_payload::Payload", tags = "20, 21, 22, 23")]
    pub payload: Option<sec2_payload::Payload>,
}

pub mod sec2_payload {
    use prost::Oneof;

    #[derive(Clone, PartialEq, Oneof)]
    pub enum Payload {
        #[prost(message, tag = "20")]
        Sc0(super::S2SessionCmd0),
        #[prost(message, tag = "21")]
        Sr0(super::S2SessionResp0),
        #[prost(message, tag = "22")]
        Sc1(super::S2SessionCmd1),
        #[prost(message, tag = "23")]
        Sr1(super::S2SessionResp1),
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Enumeration)]
#[repr(i32)]
pub enum WifiStationState {
    Connected = 0,
    Connecting = 1,
    Disconnected = 2,
    ConnectionFailed = 3,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Enumeration)]
#[repr(i32)]
pub enum WifiConnectFailedReason {
    AuthError = 0,
    NetworkNotFound = 1,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Enumeration)]
#[repr(i32)]
pub enum WifiAuthMode {
    Open = 0,
    Wep = 1,
    WpaPsk = 2,
    Wpa2Psk = 3,
    WpaWpa2Psk = 4,
    Wpa2Enterprise = 5,
    Wpa3Psk = 6,
    Wpa2Wpa3Psk = 7,
}

#[derive(Clone, PartialEq, Message)]
pub struct WifiAttemptFailed {
    #[prost(uint32, tag = "1")]
    pub attempts_remaining: u32,
}

#[derive(Clone, PartialEq, Message)]
pub struct WifiConnectedState {
    #[prost(string, tag = "1")]
    pub ip4_addr: String,
    #[prost(enumeration = "WifiAuthMode", tag = "2")]
    pub auth_mode: i32,
    #[prost(bytes = "vec", tag = "3")]
    pub ssid: Vec<u8>,
    #[prost(bytes = "vec", tag = "4")]
    pub bssid: Vec<u8>,
    #[prost(int32, tag = "5")]
    pub channel: i32,
}

#[derive(Clone, PartialEq, Message)]
pub struct CmdGetStatus {}

#[derive(Clone, PartialEq, Message)]
pub struct RespGetStatus {
    #[prost(enumeration = "Status", tag = "1")]
    pub status: i32,
    #[prost(enumeration = "WifiStationState", tag = "2")]
    pub sta_state: i32,
    #[prost(oneof = "resp_get_status::State", tags = "10, 11, 12")]
    pub state: Option<resp_get_status::State>,
}

pub mod resp_get_status {
    use prost::Oneof;

    #[derive(Clone, PartialEq, Oneof)]
    pub enum State {
        #[prost(enumeration = "super::WifiConnectFailedReason", tag = "10")]
        FailReason(i32),
        #[prost(message, tag = "11")]
        Connected(super::WifiConnectedState),
        #[prost(message, tag = "12")]
        AttemptFailed(super::WifiAttemptFailed),
    }
}

#[derive(Clone, PartialEq, Message)]
pub struct CmdSetConfig {
    #[prost(bytes = "vec", tag = "1")]
    pub ssid: Vec<u8>,
    #[prost(bytes = "vec", tag = "2")]
    pub passphrase: Vec<u8>,
    #[prost(bytes = "vec", tag = "3")]
    pub bssid: Vec<u8>,
    #[prost(int32, tag = "4")]
    pub channel: i32,
}

#[derive(Clone, PartialEq, Message)]
pub struct RespSetConfig {
    #[prost(enumeration = "Status", tag = "1")]
    pub status: i32,
}

#[derive(Clone, PartialEq, Message)]
pub struct CmdApplyConfig {}

#[derive(Clone, PartialEq, Message)]
pub struct RespApplyConfig {
    #[prost(enumeration = "Status", tag = "1")]
    pub status: i32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Enumeration)]
#[repr(i32)]
pub enum WiFiConfigMsgType {
    TypeCmdGetStatus = 0,
    TypeRespGetStatus = 1,
    TypeCmdSetConfig = 2,
    TypeRespSetConfig = 3,
    TypeCmdApplyConfig = 4,
    TypeRespApplyConfig = 5,
}

#[derive(Clone, PartialEq, Message)]
pub struct WiFiConfigPayload {
    #[prost(enumeration = "WiFiConfigMsgType", tag = "1")]
    pub msg: i32,
    #[prost(
        oneof = "wifi_config_payload::Payload",
        tags = "10, 11, 12, 13, 14, 15"
    )]
    pub payload: Option<wifi_config_payload::Payload>,
}

pub mod wifi_config_payload {
    use prost::Oneof;

    #[derive(Clone, PartialEq, Oneof)]
    pub enum Payload {
        #[prost(message, tag = "10")]
        CmdGetStatus(super::CmdGetStatus),
        #[prost(message, tag = "11")]
        RespGetStatus(super::RespGetStatus),
        #[prost(message, tag = "12")]
        CmdSetConfig(super::CmdSetConfig),
        #[prost(message, tag = "13")]
        RespSetConfig(super::RespSetConfig),
        #[prost(message, tag = "14")]
        CmdApplyConfig(super::CmdApplyConfig),
        #[prost(message, tag = "15")]
        RespApplyConfig(super::RespApplyConfig),
    }
}

#[derive(Clone, PartialEq, Message)]
pub struct CmdScanStart {
    #[prost(bool, tag = "1")]
    pub blocking: bool,
    #[prost(bool, tag = "2")]
    pub passive: bool,
    #[prost(uint32, tag = "3")]
    pub group_channels: u32,
    #[prost(uint32, tag = "4")]
    pub period_ms: u32,
}

#[derive(Clone, PartialEq, Message)]
pub struct RespScanStart {}

#[derive(Clone, PartialEq, Message)]
pub struct CmdScanStatus {}

#[derive(Clone, PartialEq, Message)]
pub struct RespScanStatus {
    #[prost(bool, tag = "1")]
    pub scan_finished: bool,
    #[prost(uint32, tag = "2")]
    pub result_count: u32,
}

#[derive(Clone, PartialEq, Message)]
pub struct CmdScanResult {
    #[prost(uint32, tag = "1")]
    pub start_index: u32,
    #[prost(uint32, tag = "2")]
    pub count: u32,
}

#[derive(Clone, PartialEq, Message)]
pub struct WiFiScanResult {
    #[prost(bytes = "vec", tag = "1")]
    pub ssid: Vec<u8>,
    #[prost(uint32, tag = "2")]
    pub channel: u32,
    #[prost(int32, tag = "3")]
    pub rssi: i32,
    #[prost(bytes = "vec", tag = "4")]
    pub bssid: Vec<u8>,
    #[prost(enumeration = "WifiAuthMode", tag = "5")]
    pub auth: i32,
}

#[derive(Clone, PartialEq, Message)]
pub struct RespScanResult {
    #[prost(message, repeated, tag = "1")]
    pub entries: Vec<WiFiScanResult>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Enumeration)]
#[repr(i32)]
pub enum WiFiScanMsgType {
    TypeCmdScanStart = 0,
    TypeRespScanStart = 1,
    TypeCmdScanStatus = 2,
    TypeRespScanStatus = 3,
    TypeCmdScanResult = 4,
    TypeRespScanResult = 5,
}

#[derive(Clone, PartialEq, Message)]
pub struct WiFiScanPayload {
    #[prost(enumeration = "WiFiScanMsgType", tag = "1")]
    pub msg: i32,
    #[prost(enumeration = "Status", tag = "2")]
    pub status: i32,
    #[prost(oneof = "wifi_scan_payload::Payload", tags = "10, 11, 12, 13, 14, 15")]
    pub payload: Option<wifi_scan_payload::Payload>,
}

pub mod wifi_scan_payload {
    use prost::Oneof;

    #[derive(Clone, PartialEq, Oneof)]
    pub enum Payload {
        #[prost(message, tag = "10")]
        CmdScanStart(super::CmdScanStart),
        #[prost(message, tag = "11")]
        RespScanStart(super::RespScanStart),
        #[prost(message, tag = "12")]
        CmdScanStatus(super::CmdScanStatus),
        #[prost(message, tag = "13")]
        RespScanStatus(super::RespScanStatus),
        #[prost(message, tag = "14")]
        CmdScanResult(super::CmdScanResult),
        #[prost(message, tag = "15")]
        RespScanResult(super::RespScanResult),
    }
}

#[derive(Clone, PartialEq, Message)]
pub struct CmdCtrlReset {}

#[derive(Clone, PartialEq, Message)]
pub struct RespCtrlReset {}

#[derive(Clone, PartialEq, Message)]
pub struct CmdCtrlReprov {}

#[derive(Clone, PartialEq, Message)]
pub struct RespCtrlReprov {}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Enumeration)]
#[repr(i32)]
pub enum WiFiCtrlMsgType {
    TypeCtrlReserved = 0,
    TypeCmdCtrlReset = 1,
    TypeRespCtrlReset = 2,
    TypeCmdCtrlReprov = 3,
    TypeRespCtrlReprov = 4,
}

#[derive(Clone, PartialEq, Message)]
pub struct WiFiCtrlPayload {
    #[prost(enumeration = "WiFiCtrlMsgType", tag = "1")]
    pub msg: i32,
    #[prost(enumeration = "Status", tag = "2")]
    pub status: i32,
    #[prost(oneof = "wifi_ctrl_payload::Payload", tags = "11, 12, 13, 14")]
    pub payload: Option<wifi_ctrl_payload::Payload>,
}

pub mod wifi_ctrl_payload {
    use prost::Oneof;

    #[derive(Clone, PartialEq, Oneof)]
    pub enum Payload {
        #[prost(message, tag = "11")]
        CmdCtrlReset(super::CmdCtrlReset),
        #[prost(message, tag = "12")]
        RespCtrlReset(super::RespCtrlReset),
        #[prost(message, tag = "13")]
        CmdCtrlReprov(super::CmdCtrlReprov),
        #[prost(message, tag = "14")]
        RespCtrlReprov(super::RespCtrlReprov),
    }
}
