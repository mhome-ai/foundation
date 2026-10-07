//! Commissioner side of a protocomm session over any request/response transport.
use async_trait::async_trait;
use prost::Message;
use serde_json::Value;
use tokio::sync::Mutex;

use crate::proto::{self, Status};
use crate::sec2::{Sec2Cipher, MAX_PATCH_VERSION};
use crate::srp::SrpClient;

pub const PROTO_VERSION_ENDPOINT: &str = "proto-ver";
pub const SESSION_ENDPOINT: &str = "prov-session";
pub const WIFI_SCAN_ENDPOINT: &str = "prov-scan";
pub const WIFI_CONFIG_ENDPOINT: &str = "prov-config";
pub const WIFI_CTRL_ENDPOINT: &str = "prov-ctrl";

const SCAN_PAGE: u32 = 4;

/// Why an exchange produced no response.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum TransportError {
    /// The device answered with an error, as ESP-IDF does with a GATT/ATT error
    /// when an endpoint handler fails. The link is still up.
    #[error("device rejected the request: {0}")]
    Rejected(String),
    /// The link dropped or the exchange timed out.
    #[error("link lost: {0}")]
    Disconnected(String),
}

/// One request and its response on a named endpoint.
#[async_trait]
pub trait Transport: Send + Sync {
    async fn exchange(&self, endpoint: &str, request: &[u8]) -> Result<Vec<u8>, TransportError>;
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ProtocommError {
    #[error("link lost: {0}")]
    Disconnected(String),
    #[error("device rejected the request: {0}")]
    Rejected(String),
    #[error("protocol: {0}")]
    Protocol(String),
    #[error("device returned {0:?}")]
    Device(Status),
    /// The device refused the proof; a new handshake may follow on the same link.
    #[error("the code was rejected")]
    CodeRejected,
    /// The device refused to start a handshake.
    #[error("the device refused the handshake")]
    HandshakeRefused,
    #[error("unsupported security: {0}")]
    UnsupportedSecurity(String),
    /// No secure session, or the previous one ended with a failed call and its
    /// nonce counters can no longer be trusted.
    #[error("no secure session")]
    NotSecured,
}

impl From<TransportError> for ProtocommError {
    fn from(error: TransportError) -> Self {
        match error {
            TransportError::Rejected(message) => Self::Rejected(message),
            TransportError::Disconnected(message) => Self::Disconnected(message),
        }
    }
}

enum CipherState {
    None,
    Ready(Box<Sec2Cipher>),
    Poisoned,
}

#[derive(Debug, Clone)]
pub struct VersionInfo {
    pub raw: Value,
}

impl VersionInfo {
    pub fn security_version(&self) -> Option<u64> {
        self.raw["prov"]["sec_ver"].as_u64()
    }

    pub fn security_patch_version(&self) -> u32 {
        self.raw["prov"]["sec_patch_ver"].as_u64().unwrap_or(0) as u32
    }

    pub fn has_app_capability(&self, label: &str, capability: &str) -> bool {
        self.raw[label]["cap"]
            .as_array()
            .is_some_and(|caps| caps.iter().any(|cap| cap.as_str() == Some(capability)))
    }

    /// A boolean in the app info object, such as `"locked"`.
    pub fn app_flag(&self, label: &str, flag: &str) -> bool {
        self.raw[label][flag].as_bool() == Some(true)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WifiFailure {
    AuthError,
    NetworkNotFound,
    Other,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WifiStatus {
    Connected { ip4_addr: String },
    Connecting,
    Failed(WifiFailure),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScanEntry {
    pub ssid: String,
    pub rssi: i32,
    pub channel: u32,
    pub auth: proto::WifiAuthMode,
}

pub struct ProtocommClient<T: Transport> {
    transport: T,
    cipher: Mutex<CipherState>,
}

fn decode<M: Message + Default>(bytes: &[u8]) -> Result<M, ProtocommError> {
    M::decode(bytes).map_err(|error| ProtocommError::Protocol(error.to_string()))
}

fn ok(status: i32) -> Result<(), ProtocommError> {
    match Status::try_from(status) {
        Ok(Status::Success) => Ok(()),
        Ok(other) => Err(ProtocommError::Device(other)),
        Err(_) => Err(ProtocommError::Protocol(format!("unknown status {status}"))),
    }
}

impl<T: Transport> ProtocommClient<T> {
    pub fn new(transport: T) -> Self {
        Self {
            transport,
            cipher: Mutex::new(CipherState::None),
        }
    }

    pub fn transport(&self) -> &T {
        &self.transport
    }

    pub async fn version(&self) -> Result<VersionInfo, ProtocommError> {
        let response = self
            .transport
            .exchange(PROTO_VERSION_ENDPOINT, b"---")
            .await?;
        let raw = serde_json::from_slice(&response)
            .map_err(|error| ProtocommError::Protocol(format!("proto-ver: {error}")))?;
        Ok(VersionInfo { raw })
    }

    pub async fn is_secured(&self) -> bool {
        matches!(*self.cipher.lock().await, CipherState::Ready(_))
    }

    /// Runs the security 2 handshake. A rejected `SessionCommand1` (or a device
    /// proof that does not match) is [`ProtocommError::CodeRejected`]; a lost
    /// link stays [`ProtocommError::Disconnected`].
    pub async fn establish_sec2(
        &self,
        username: &str,
        password: &str,
        patch_version: u32,
    ) -> Result<(), ProtocommError> {
        use proto::sec2_payload::Payload;
        use proto::session_data::Proto;

        if patch_version > MAX_PATCH_VERSION {
            return Err(ProtocommError::UnsupportedSecurity(format!(
                "security 2 patch version {patch_version}"
            )));
        }
        let mut state = self.cipher.lock().await;
        *state = CipherState::None;
        let srp = SrpClient::new(username, password);
        let cmd0 = session(
            proto::Sec2MsgType::S2SessionCommand0,
            Payload::Sc0(proto::S2SessionCmd0 {
                client_username: username.as_bytes().to_vec(),
                client_pubkey: srp.public_key(),
            }),
        );
        let response = match self.transport.exchange(SESSION_ENDPOINT, &cmd0).await {
            Ok(response) => response,
            Err(TransportError::Rejected(_)) => return Err(ProtocommError::HandshakeRefused),
            Err(error) => return Err(error.into()),
        };
        let resp0 = match decode::<proto::SessionData>(&response)?.proto {
            Some(Proto::Sec2(proto::Sec2Payload {
                payload: Some(Payload::Sr0(resp)),
                ..
            })) => resp,
            _ => {
                return Err(ProtocommError::Protocol(
                    "expected session response 0".into(),
                ))
            }
        };
        if ok(resp0.status).is_err() {
            return Err(ProtocommError::HandshakeRefused);
        }
        let proof = srp
            .process_challenge(&resp0.device_salt, &resp0.device_pubkey)
            .map_err(|error| ProtocommError::Protocol(error.to_string()))?;

        let cmd1 = session(
            proto::Sec2MsgType::S2SessionCommand1,
            Payload::Sc1(proto::S2SessionCmd1 {
                client_proof: proof.proof.clone(),
            }),
        );
        let response = match self.transport.exchange(SESSION_ENDPOINT, &cmd1).await {
            Ok(response) => response,
            Err(TransportError::Rejected(_)) => return Err(ProtocommError::CodeRejected),
            Err(error) => return Err(error.into()),
        };
        let resp1 = match decode::<proto::SessionData>(&response)?.proto {
            Some(Proto::Sec2(proto::Sec2Payload {
                payload: Some(Payload::Sr1(resp)),
                ..
            })) => resp,
            _ => return Err(ProtocommError::CodeRejected),
        };
        if ok(resp1.status).is_err() {
            return Err(ProtocommError::CodeRejected);
        }
        let key = proof
            .verify(&resp1.device_proof)
            .map_err(|_| ProtocommError::CodeRejected)?;
        let cipher = Sec2Cipher::new(&key, &resp1.device_nonce, patch_version)
            .map_err(|error| ProtocommError::Protocol(error.to_string()))?;
        *state = CipherState::Ready(Box::new(cipher));
        Ok(())
    }

    /// Encrypted request/response on one endpoint. Calls are serialized because
    /// both sides advance the nonce in the same order. Any failed exchange or
    /// record ends the secure session: the device may or may not have advanced
    /// its counter, so later calls return [`ProtocommError::NotSecured`] until a
    /// new handshake.
    pub async fn call(&self, endpoint: &str, plaintext: &[u8]) -> Result<Vec<u8>, ProtocommError> {
        let mut guard = self.cipher.lock().await;
        let CipherState::Ready(cipher) = &mut *guard else {
            return Err(ProtocommError::NotSecured);
        };
        let result = match cipher.encrypt(plaintext) {
            Ok(request) => match self.transport.exchange(endpoint, &request).await {
                Ok(response) => cipher
                    .decrypt(&response)
                    .map_err(|error| ProtocommError::Protocol(error.to_string())),
                Err(error) => Err(error.into()),
            },
            Err(error) => Err(ProtocommError::Protocol(error.to_string())),
        };
        if result.is_err() {
            *guard = CipherState::Poisoned;
        }
        result
    }

    pub async fn call_json(
        &self,
        endpoint: &str,
        request: &Value,
    ) -> Result<Value, ProtocommError> {
        let body = serde_json::to_vec(request)
            .map_err(|error| ProtocommError::Protocol(error.to_string()))?;
        let response = self.call(endpoint, &body).await?;
        serde_json::from_slice(&response)
            .map_err(|error| ProtocommError::Protocol(format!("{endpoint}: {error}")))
    }

    pub async fn wifi_status(&self) -> Result<WifiStatus, ProtocommError> {
        use proto::resp_get_status::State;
        use proto::wifi_config_payload::Payload;

        let response = self
            .wifi_config(
                proto::WiFiConfigMsgType::TypeCmdGetStatus,
                Payload::CmdGetStatus(proto::CmdGetStatus {}),
            )
            .await?;
        let Some(Payload::RespGetStatus(status)) = response.payload else {
            return Err(ProtocommError::Protocol("expected Wi-Fi status".into()));
        };
        ok(status.status)?;
        Ok(match proto::WifiStationState::try_from(status.sta_state) {
            Ok(proto::WifiStationState::Connected) => WifiStatus::Connected {
                ip4_addr: match status.state {
                    Some(State::Connected(connected)) => connected.ip4_addr,
                    _ => String::new(),
                },
            },
            Ok(proto::WifiStationState::Connecting) => WifiStatus::Connecting,
            Ok(proto::WifiStationState::ConnectionFailed) => {
                WifiStatus::Failed(match status.state {
                    Some(State::FailReason(reason)) => {
                        match proto::WifiConnectFailedReason::try_from(reason) {
                            Ok(proto::WifiConnectFailedReason::AuthError) => WifiFailure::AuthError,
                            Ok(proto::WifiConnectFailedReason::NetworkNotFound) => {
                                WifiFailure::NetworkNotFound
                            }
                            Err(_) => WifiFailure::Other,
                        }
                    }
                    _ => WifiFailure::Other,
                })
            }
            _ => WifiStatus::Failed(WifiFailure::Other),
        })
    }

    pub async fn wifi_set_config(
        &self,
        ssid: &str,
        passphrase: &str,
    ) -> Result<(), ProtocommError> {
        use proto::wifi_config_payload::Payload;

        let response = self
            .wifi_config(
                proto::WiFiConfigMsgType::TypeCmdSetConfig,
                Payload::CmdSetConfig(proto::CmdSetConfig {
                    ssid: ssid.as_bytes().to_vec(),
                    passphrase: passphrase.as_bytes().to_vec(),
                    bssid: Vec::new(),
                    channel: 0,
                }),
            )
            .await?;
        match response.payload {
            Some(Payload::RespSetConfig(resp)) => ok(resp.status),
            _ => Err(ProtocommError::Protocol(
                "expected set-config response".into(),
            )),
        }
    }

    pub async fn wifi_apply(&self) -> Result<(), ProtocommError> {
        use proto::wifi_config_payload::Payload;

        let response = self
            .wifi_config(
                proto::WiFiConfigMsgType::TypeCmdApplyConfig,
                Payload::CmdApplyConfig(proto::CmdApplyConfig {}),
            )
            .await?;
        match response.payload {
            Some(Payload::RespApplyConfig(resp)) => ok(resp.status),
            _ => Err(ProtocommError::Protocol(
                "expected apply-config response".into(),
            )),
        }
    }

    /// Clears a failed join so the device accepts new credentials.
    pub async fn wifi_reset(&self) -> Result<(), ProtocommError> {
        use proto::wifi_ctrl_payload::Payload;

        let request = proto::WiFiCtrlPayload {
            msg: proto::WiFiCtrlMsgType::TypeCmdCtrlReset as i32,
            status: Status::Success as i32,
            payload: Some(Payload::CmdCtrlReset(proto::CmdCtrlReset {})),
        };
        let response: proto::WiFiCtrlPayload = decode(
            &self
                .call(WIFI_CTRL_ENDPOINT, &request.encode_to_vec())
                .await?,
        )?;
        ok(response.status)
    }

    pub async fn wifi_scan(&self) -> Result<Vec<ScanEntry>, ProtocommError> {
        use proto::wifi_scan_payload::Payload;

        let start = self
            .wifi_scan_call(
                proto::WiFiScanMsgType::TypeCmdScanStart,
                Payload::CmdScanStart(proto::CmdScanStart {
                    blocking: true,
                    passive: false,
                    group_channels: 0,
                    period_ms: 120,
                }),
            )
            .await?;
        ok(start.status)?;
        let mut count = 0;
        for _ in 0..20 {
            let status = self
                .wifi_scan_call(
                    proto::WiFiScanMsgType::TypeCmdScanStatus,
                    Payload::CmdScanStatus(proto::CmdScanStatus {}),
                )
                .await?;
            ok(status.status)?;
            if let Some(Payload::RespScanStatus(status)) = status.payload {
                if status.scan_finished {
                    count = status.result_count;
                    break;
                }
            }
            tokio::time::sleep(std::time::Duration::from_millis(500)).await;
        }
        let mut entries = Vec::new();
        let mut index = 0;
        while index < count {
            let page = SCAN_PAGE.min(count - index);
            let result = self
                .wifi_scan_call(
                    proto::WiFiScanMsgType::TypeCmdScanResult,
                    Payload::CmdScanResult(proto::CmdScanResult {
                        start_index: index,
                        count: page,
                    }),
                )
                .await?;
            ok(result.status)?;
            let Some(Payload::RespScanResult(result)) = result.payload else {
                break;
            };
            if result.entries.is_empty() {
                break;
            }
            index += result.entries.len() as u32;
            entries.extend(result.entries.into_iter().map(|entry| {
                ScanEntry {
                    ssid: String::from_utf8_lossy(&entry.ssid).into_owned(),
                    rssi: entry.rssi,
                    channel: entry.channel,
                    auth: proto::WifiAuthMode::try_from(entry.auth)
                        .unwrap_or(proto::WifiAuthMode::Wpa2Psk),
                }
            }));
        }
        Ok(entries)
    }

    async fn wifi_config(
        &self,
        msg: proto::WiFiConfigMsgType,
        payload: proto::wifi_config_payload::Payload,
    ) -> Result<proto::WiFiConfigPayload, ProtocommError> {
        let request = proto::WiFiConfigPayload {
            msg: msg as i32,
            payload: Some(payload),
        };
        decode(
            &self
                .call(WIFI_CONFIG_ENDPOINT, &request.encode_to_vec())
                .await?,
        )
    }

    async fn wifi_scan_call(
        &self,
        msg: proto::WiFiScanMsgType,
        payload: proto::wifi_scan_payload::Payload,
    ) -> Result<proto::WiFiScanPayload, ProtocommError> {
        let request = proto::WiFiScanPayload {
            msg: msg as i32,
            status: Status::Success as i32,
            payload: Some(payload),
        };
        decode(
            &self
                .call(WIFI_SCAN_ENDPOINT, &request.encode_to_vec())
                .await?,
        )
    }
}

fn session(msg: proto::Sec2MsgType, payload: proto::sec2_payload::Payload) -> Vec<u8> {
    proto::SessionData {
        sec_ver: proto::SecSchemeVersion::SecScheme2 as i32,
        proto: Some(proto::session_data::Proto::Sec2(proto::Sec2Payload {
            msg: msg as i32,
            payload: Some(payload),
        })),
    }
    .encode_to_vec()
}
