//! Device side of the security 2 handshake (`prov-session`), equivalent to
//! ESP-IDF `protocomm_security2`.
use prost::Message;

use crate::proto::{self, Status};
use crate::sec2::{new_device_nonce, Sec2Cipher};
use crate::srp::{SrpServer, GROUP_BYTES, PROOF_BYTES};

/// The patch level this responder implements: the nonce advances per record.
pub const SECURITY_PATCH_VERSION: u32 = 1;

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum SessionError {
    #[error("malformed session message")]
    Malformed,
    #[error("unexpected session message")]
    UnexpectedMessage,
    #[error("unknown username")]
    UnknownUser,
    #[error("client public key must be {GROUP_BYTES} bytes")]
    InvalidPublicKey,
    #[error("client proof rejected")]
    ProofRejected,
    #[error("session is not established")]
    NotEstablished,
    #[error("record authentication failed")]
    Record,
}

enum State {
    AwaitingCommand0,
    AwaitingCommand1 {
        srp: SrpServer,
        client_pubkey: Vec<u8>,
    },
    Established(Box<Sec2Cipher>),
}

pub struct SessionResponder {
    username: String,
    salt: Vec<u8>,
    verifier: Vec<u8>,
    state: State,
}

impl SessionResponder {
    pub fn new(username: &str, salt: Vec<u8>, verifier: Vec<u8>) -> Self {
        Self {
            username: username.to_string(),
            salt,
            verifier,
            state: State::AwaitingCommand0,
        }
    }

    pub fn is_established(&self) -> bool {
        matches!(self.state, State::Established(_))
    }

    /// Forgets the session, as when the central disconnects.
    pub fn reset(&mut self) {
        self.state = State::AwaitingCommand0;
    }

    /// A `SessionCommand0` restarts the handshake from any state. A rejected
    /// proof returns to awaiting `SessionCommand0`.
    pub fn handle(&mut self, request: &[u8]) -> Result<Vec<u8>, SessionError> {
        use proto::sec2_payload::Payload;
        use proto::session_data::Proto;

        let data = proto::SessionData::decode(request).map_err(|_| SessionError::Malformed)?;
        if data.sec_ver != proto::SecSchemeVersion::SecScheme2 as i32 {
            return Err(SessionError::Malformed);
        }
        let Some(Proto::Sec2(payload)) = data.proto else {
            return Err(SessionError::Malformed);
        };
        match payload.payload {
            Some(Payload::Sc0(cmd)) => {
                self.state = State::AwaitingCommand0;
                if cmd.client_username != self.username.as_bytes() {
                    return Err(SessionError::UnknownUser);
                }
                if cmd.client_pubkey.len() != GROUP_BYTES {
                    return Err(SessionError::InvalidPublicKey);
                }
                let srp = SrpServer::new(&self.username, &self.salt, &self.verifier);
                let response = reply(
                    proto::Sec2MsgType::S2SessionResponse0,
                    Payload::Sr0(proto::S2SessionResp0 {
                        status: Status::Success as i32,
                        device_pubkey: srp.public_key(),
                        device_salt: self.salt.clone(),
                    }),
                );
                self.state = State::AwaitingCommand1 {
                    srp,
                    client_pubkey: cmd.client_pubkey,
                };
                Ok(response)
            }
            Some(Payload::Sc1(cmd)) => {
                let State::AwaitingCommand1 { srp, client_pubkey } = &self.state else {
                    return Err(SessionError::UnexpectedMessage);
                };
                if cmd.client_proof.len() != PROOF_BYTES {
                    return Err(SessionError::Malformed);
                }
                let session = match srp.verify_client(client_pubkey, &cmd.client_proof) {
                    Ok(session) => session,
                    Err(_) => {
                        self.state = State::AwaitingCommand0;
                        return Err(SessionError::ProofRejected);
                    }
                };
                let nonce = new_device_nonce();
                let cipher = Sec2Cipher::new(&session.key, &nonce, SECURITY_PATCH_VERSION)
                    .map_err(|_| SessionError::Malformed)?;
                self.state = State::Established(Box::new(cipher));
                Ok(reply(
                    proto::Sec2MsgType::S2SessionResponse1,
                    Payload::Sr1(proto::S2SessionResp1 {
                        status: Status::Success as i32,
                        device_proof: session.device_proof,
                        device_nonce: nonce.to_vec(),
                    }),
                ))
            }
            _ => Err(SessionError::UnexpectedMessage),
        }
    }

    pub fn decrypt(&mut self, record: &[u8]) -> Result<Vec<u8>, SessionError> {
        match &mut self.state {
            State::Established(cipher) => cipher.decrypt(record).map_err(|_| SessionError::Record),
            _ => Err(SessionError::NotEstablished),
        }
    }

    pub fn encrypt(&mut self, plaintext: &[u8]) -> Result<Vec<u8>, SessionError> {
        match &mut self.state {
            State::Established(cipher) => {
                cipher.encrypt(plaintext).map_err(|_| SessionError::Record)
            }
            _ => Err(SessionError::NotEstablished),
        }
    }

    /// Decrypts a request, runs `handler` on it and encrypts its response.
    pub fn respond(
        &mut self,
        record: &[u8],
        handler: impl FnOnce(&[u8]) -> Vec<u8>,
    ) -> Result<Vec<u8>, SessionError> {
        let request = self.decrypt(record)?;
        self.encrypt(&handler(&request))
    }
}

fn reply(msg: proto::Sec2MsgType, payload: proto::sec2_payload::Payload) -> Vec<u8> {
    proto::SessionData {
        sec_ver: proto::SecSchemeVersion::SecScheme2 as i32,
        proto: Some(proto::session_data::Proto::Sec2(proto::Sec2Payload {
            msg: msg as i32,
            payload: Some(payload),
        })),
    }
    .encode_to_vec()
}
