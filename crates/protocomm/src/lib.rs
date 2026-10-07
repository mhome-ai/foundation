//! ESP-IDF protocomm security 2 (SRP6a-3072/SHA-512 + AES-256-GCM) and the
//! Wi-Fi provisioning endpoints, for MeowLink device commissioning.
pub mod client;
pub mod proto;
pub mod sec2;
pub mod server;
pub mod srp;

pub use client::{
    ProtocommClient, ProtocommError, ScanEntry, Transport, TransportError, VersionInfo,
    WifiFailure, WifiStatus,
};
pub use server::{SessionError, SessionResponder};

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::Mutex;

    use async_trait::async_trait;

    use super::*;

    struct Device {
        session: Mutex<SessionResponder>,
        drop_next_echo: AtomicBool,
    }

    #[async_trait]
    impl Transport for Device {
        async fn exchange(
            &self,
            endpoint: &str,
            request: &[u8],
        ) -> Result<Vec<u8>, TransportError> {
            let mut session = self.session.lock().unwrap();
            let rejected = |error: SessionError| TransportError::Rejected(error.to_string());
            match endpoint {
                client::SESSION_ENDPOINT => session.handle(request).map_err(rejected),
                "echo" => {
                    let response = session
                        .respond(request, |plain| plain.to_vec())
                        .map_err(rejected)?;
                    if self.drop_next_echo.swap(false, Ordering::SeqCst) {
                        return Err(TransportError::Disconnected("response lost".into()));
                    }
                    Ok(response)
                }
                _ => Err(TransportError::Rejected(format!("no endpoint {endpoint}"))),
            }
        }
    }

    fn device(code: &str) -> Device {
        let (salt, verifier) = srp::generate_salt_verifier("meow", code);
        Device {
            session: Mutex::new(SessionResponder::new("meow", salt, verifier)),
            drop_next_echo: AtomicBool::new(false),
        }
    }

    #[tokio::test]
    async fn handshake_then_encrypted_calls() {
        let client = ProtocommClient::new(device("123456"));
        client
            .establish_sec2("meow", "123456", server::SECURITY_PATCH_VERSION)
            .await
            .unwrap();
        assert!(client.is_secured().await);
        for body in [&br#"{"op":"status"}"#[..], b"second", b""] {
            assert_eq!(client.call("echo", body).await.unwrap(), body);
        }
    }

    #[tokio::test]
    async fn a_wrong_code_can_be_retried_on_the_same_link() {
        let client = ProtocommClient::new(device("123456"));
        assert_eq!(
            client.establish_sec2("meow", "654321", 1).await,
            Err(ProtocommError::CodeRejected)
        );
        assert_eq!(
            client.call("echo", b"x").await,
            Err(ProtocommError::NotSecured)
        );
        client.establish_sec2("meow", "123456", 1).await.unwrap();
        assert_eq!(client.call("echo", b"x").await.unwrap(), b"x");
    }

    #[tokio::test]
    async fn a_new_handshake_restarts_an_established_session() {
        let client = ProtocommClient::new(device("123456"));
        client.establish_sec2("meow", "123456", 1).await.unwrap();
        client.call("echo", b"one").await.unwrap();
        client.establish_sec2("meow", "123456", 1).await.unwrap();
        assert_eq!(client.call("echo", b"two").await.unwrap(), b"two");
    }

    #[tokio::test]
    async fn a_failed_call_ends_the_secure_session() {
        let device = device("123456");
        device.drop_next_echo.store(true, Ordering::SeqCst);
        let client = ProtocommClient::new(device);
        client.establish_sec2("meow", "123456", 1).await.unwrap();
        assert!(matches!(
            client.call("echo", b"lost").await,
            Err(ProtocommError::Disconnected(_))
        ));
        assert!(!client.is_secured().await);
        assert_eq!(
            client.call("echo", b"again").await,
            Err(ProtocommError::NotSecured)
        );
        client.establish_sec2("meow", "123456", 1).await.unwrap();
        assert_eq!(client.call("echo", b"again").await.unwrap(), b"again");
    }

    #[tokio::test]
    async fn a_device_error_on_a_call_also_ends_the_session() {
        let client = ProtocommClient::new(device("123456"));
        client.establish_sec2("meow", "123456", 1).await.unwrap();
        assert!(matches!(
            client.call("missing", b"x").await,
            Err(ProtocommError::Rejected(_))
        ));
        assert_eq!(
            client.call("echo", b"x").await,
            Err(ProtocommError::NotSecured)
        );
    }

    #[tokio::test]
    async fn rejects_unknown_patch_versions_before_talking() {
        let client = ProtocommClient::new(device("123456"));
        assert!(matches!(
            client.establish_sec2("meow", "123456", 2).await,
            Err(ProtocommError::UnsupportedSecurity(_))
        ));
    }

    #[test]
    fn the_responder_enforces_the_esp_idf_message_rules() {
        use prost::Message;
        use proto::sec2_payload::Payload;

        let (salt, verifier) = srp::generate_salt_verifier("meow", "123456");
        let mut responder = SessionResponder::new("meow", salt, verifier);
        let message = |payload| {
            proto::SessionData {
                sec_ver: proto::SecSchemeVersion::SecScheme2 as i32,
                proto: Some(proto::session_data::Proto::Sec2(proto::Sec2Payload {
                    msg: 0,
                    payload: Some(payload),
                })),
            }
            .encode_to_vec()
        };
        let short_key = message(Payload::Sc0(proto::S2SessionCmd0 {
            client_username: b"meow".to_vec(),
            client_pubkey: vec![1; 383],
        }));
        assert_eq!(
            responder.handle(&short_key),
            Err(SessionError::InvalidPublicKey)
        );
        let early_proof = message(Payload::Sc1(proto::S2SessionCmd1 {
            client_proof: vec![0; 64],
        }));
        assert_eq!(
            responder.handle(&early_proof),
            Err(SessionError::UnexpectedMessage)
        );
        let client = srp::SrpClient::new("meow", "123456");
        let start = message(Payload::Sc0(proto::S2SessionCmd0 {
            client_username: b"meow".to_vec(),
            client_pubkey: client.public_key(),
        }));
        responder.handle(&start).unwrap();
        let short_proof = message(Payload::Sc1(proto::S2SessionCmd1 {
            client_proof: vec![0; 32],
        }));
        assert_eq!(responder.handle(&short_proof), Err(SessionError::Malformed));
        assert_eq!(
            responder.handle(&early_proof),
            Err(SessionError::ProofRejected)
        );
        assert_eq!(
            responder.handle(&early_proof),
            Err(SessionError::UnexpectedMessage)
        );
    }
}
