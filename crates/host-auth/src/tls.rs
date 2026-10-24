//! LAN TLS authenticates a Host P-256 key, not an IP address or a public CA.
//! Certificates are containers for that key; their dates are not an offline
//! authorization lease. TLS CertificateVerify is always verified by rustls.
use crate::signatures::{public_key, verifying_key};
use anyhow::{ensure, Context, Result};
use core_api::host::auth::PublicKey;
use p256::{ecdsa::SigningKey, pkcs8::EncodePrivateKey};
use rustls::{
    client::danger::{HandshakeSignatureValid, ServerCertVerified, ServerCertVerifier},
    crypto::{ring, verify_tls12_signature, verify_tls13_signature},
    pki_types::{pem::PemObject, CertificateDer, PrivateKeyDer, ServerName, UnixTime},
    ClientConfig, DigitallySignedStruct, Error, ServerConfig, SignatureScheme,
};
use std::{
    path::Path,
    sync::{Arc, Mutex},
};

pub const CERT_FILE: &str = "host-cert.pem";
pub const KEY_FILE: &str = "host-key.pem";

/// Generate from the existing Host identity; never generate a second identity.
pub fn certificate(key: &SigningKey) -> Result<(String, String)> {
    let pem = key.to_pkcs8_pem(p256::pkcs8::LineEnding::LF)?.to_string();
    let pair = rcgen::KeyPair::from_pem(&pem)?;
    let mut params = rcgen::CertificateParams::new(vec!["meow-host.local".into()])?;
    params
        .distinguished_name
        .push(rcgen::DnType::CommonName, "MeowLink Host");
    params.key_usages = vec![rcgen::KeyUsagePurpose::DigitalSignature];
    params.extended_key_usages = vec![rcgen::ExtendedKeyUsagePurpose::ServerAuth];
    Ok((params.self_signed(&pair)?.pem(), pem))
}

/// Child services consume the identity already created by their Host. They
/// never initialize or repair Host security state themselves.
pub fn require_host_identity(root: &Path, host_id: &str) -> Result<()> {
    let state: serde_json::Value =
        serde_json::from_slice(&std::fs::read(root.join("identity.json"))?)?;
    ensure!(
        state["host_id"].as_str() == Some(host_id),
        "Host identity mismatch"
    );
    let key =
        crate::signatures::decode_key(state["private_key"].as_str().context("Host key missing")?)?;
    ensure!(
        load_public_key(root)? == crate::signatures::public_key(key.verifying_key()),
        "Host TLS identity mismatch"
    );
    load_server(root)?;
    Ok(())
}

pub fn certificate_public_key(der: &[u8]) -> Result<PublicKey> {
    let (rest, cert) = x509_parser::parse_x509_certificate(der)
        .map_err(|_| anyhow::anyhow!("invalid Host certificate"))?;
    ensure!(rest.is_empty(), "trailing certificate data");
    let spki = cert.public_key();
    ensure!(
        spki.algorithm.algorithm.to_id_string() == "1.2.840.10045.2.1",
        "Host certificate must use EC"
    );
    let curve = spki
        .algorithm
        .parameters
        .as_ref()
        .context("missing EC curve")?
        .as_oid()
        .map_err(|_| anyhow::anyhow!("invalid EC curve"))?;
    ensure!(
        curve.to_id_string() == "1.2.840.10045.3.1.7",
        "Host certificate must use P-256"
    );
    let key = p256::ecdsa::VerifyingKey::from_sec1_bytes(&spki.subject_public_key.data)?;
    let usages = cert
        .extended_key_usage()
        .map_err(|_| anyhow::anyhow!("invalid certificate usages"))?;
    ensure!(
        usages.is_some_and(|u| u.value.server_auth),
        "Host certificate must allow server authentication"
    );
    if let Some(usage) = cert
        .key_usage()
        .map_err(|_| anyhow::anyhow!("invalid certificate key usage"))?
    {
        ensure!(
            usage.value.digital_signature(),
            "Host certificate must allow signing"
        );
    }
    Ok(public_key(&key))
}

/// Distinguish a rejected peer identity from an offline or unreachable Host.
/// Consumers surface explicit reauthorization only for certificate failures.
pub fn is_identity_error(error: &(dyn std::error::Error + 'static)) -> bool {
    let mut current = Some(error);
    while let Some(error) = current {
        if let Some(error) = error.downcast_ref::<rustls::Error>() {
            return matches!(
                error,
                Error::InvalidCertificate(_) | Error::InvalidMessage(_) | Error::PeerMisbehaved(_)
            ) || matches!(error, Error::General(message) if message.starts_with("Host TLS") || message.starts_with("invalid Host TLS"));
        }
        if let Some(io) = error.downcast_ref::<std::io::Error>() {
            if io.get_ref().is_some_and(|inner| is_identity_error(inner)) {
                return true;
            }
        }
        current = error.source();
    }
    false
}

/// A bootstrap pin belongs to ONE explicit operation. Merely observing a key
/// does not make it trusted: callers must authenticate the challenge/cloud grant
/// and call require_key before sending credentials or persisting the pin.
#[derive(Debug, Clone)]
pub struct PeerPin(Arc<Mutex<Option<PublicKey>>>);

impl PeerPin {
    pub fn trusted(key: PublicKey) -> Result<Self> {
        verifying_key(&key)?;
        Ok(Self(Arc::new(Mutex::new(Some(key)))))
    }

    pub fn bootstrap() -> Self {
        Self(Arc::new(Mutex::new(None)))
    }

    pub fn require_key(&self, trusted: &PublicKey) -> Result<()> {
        let expected = verifying_key(trusted)?;
        let observed = self
            .0
            .lock()
            .map_err(|_| anyhow::anyhow!("Host TLS pin poisoned"))?;
        let observed = observed
            .as_ref()
            .context("Host TLS handshake has not completed")?;
        ensure!(
            verifying_key(observed)? == expected,
            "Host TLS identity mismatch"
        );
        Ok(())
    }

    pub fn config(&self) -> ClientConfig {
        ClientConfig::builder_with_provider(Arc::new(ring::default_provider()))
            .with_safe_default_protocol_versions()
            .expect("TLS protocols")
            .dangerous()
            .with_custom_certificate_verifier(Arc::new(self.clone()))
            .with_no_client_auth()
    }
}

impl ServerCertVerifier for PeerPin {
    fn verify_server_cert(
        &self,
        end: &CertificateDer<'_>,
        _: &[CertificateDer<'_>],
        _: &ServerName<'_>,
        _: &[u8],
        _: UnixTime,
    ) -> std::result::Result<ServerCertVerified, Error> {
        let key = certificate_public_key(end.as_ref())
            .map_err(|_| Error::General("invalid Host TLS certificate".into()))?;
        let mut pin = self
            .0
            .lock()
            .map_err(|_| Error::General("Host TLS pin unavailable".into()))?;
        match pin.as_ref() {
            Some(expected) if expected != &key => {
                return Err(Error::General("Host TLS identity mismatch".into()))
            }
            None => *pin = Some(key),
            _ => {}
        }
        Ok(ServerCertVerified::assertion())
    }
    fn verify_tls12_signature(
        &self,
        msg: &[u8],
        cert: &CertificateDer<'_>,
        sig: &DigitallySignedStruct,
    ) -> std::result::Result<HandshakeSignatureValid, Error> {
        verify_tls12_signature(
            msg,
            cert,
            sig,
            &ring::default_provider().signature_verification_algorithms,
        )
    }
    fn verify_tls13_signature(
        &self,
        msg: &[u8],
        cert: &CertificateDer<'_>,
        sig: &DigitallySignedStruct,
    ) -> std::result::Result<HandshakeSignatureValid, Error> {
        verify_tls13_signature(
            msg,
            cert,
            sig,
            &ring::default_provider().signature_verification_algorithms,
        )
    }
    fn supported_verify_schemes(&self) -> Vec<SignatureScheme> {
        vec![SignatureScheme::ECDSA_NISTP256_SHA256]
    }
}

pub fn server_config(cert_pem: &[u8], key_pem: &[u8]) -> Result<ServerConfig> {
    let certs = CertificateDer::pem_slice_iter(cert_pem).collect::<Result<Vec<_>, _>>()?;
    let cert = certs.first().context("Host certificate is missing")?;
    certificate_public_key(cert.as_ref())?;
    let key = PrivateKeyDer::pem_slice_iter(key_pem)
        .next()
        .transpose()?
        .context("Host private key is missing")?;
    Ok(
        ServerConfig::builder_with_provider(Arc::new(ring::default_provider()))
            .with_safe_default_protocol_versions()?
            .with_no_client_auth()
            .with_single_cert(certs, key)?,
    )
}

pub fn load_server(root: &Path) -> Result<ServerConfig> {
    server_config(
        &std::fs::read(root.join(CERT_FILE))?,
        &std::fs::read(root.join(KEY_FILE))?,
    )
}

pub fn load_public_key(root: &Path) -> Result<PublicKey> {
    let pem = std::fs::read(root.join(CERT_FILE))?;
    let cert = CertificateDer::pem_slice_iter(&pem)
        .next()
        .context("Host certificate missing")??;
    certificate_public_key(cert.as_ref())
}

/// Bounded concurrent handshakes: a silent TCP peer must not block every service.
#[cfg(feature = "tls-server")]
pub struct Listener {
    tcp: tokio::net::TcpListener,
    acceptor: tokio_rustls::TlsAcceptor,
    pending: tokio::task::JoinSet<
        std::io::Result<(
            tokio_rustls::server::TlsStream<tokio::net::TcpStream>,
            std::net::SocketAddr,
        )>,
    >,
}
#[cfg(feature = "tls-server")]
impl Listener {
    pub fn new(tcp: tokio::net::TcpListener, config: ServerConfig) -> Self {
        Self {
            tcp,
            acceptor: tokio_rustls::TlsAcceptor::from(Arc::new(config)),
            pending: tokio::task::JoinSet::new(),
        }
    }
}
#[cfg(feature = "tls-server")]
impl axum::serve::Listener for Listener {
    type Io = tokio_rustls::server::TlsStream<tokio::net::TcpStream>;
    type Addr = std::net::SocketAddr;
    async fn accept(&mut self) -> (Self::Io, Self::Addr) {
        loop {
            tokio::select! {
                result = self.pending.join_next(), if !self.pending.is_empty() => {
                    if let Some(Ok(Ok(ready))) = result { return ready; }
                }
                result = self.tcp.accept(), if self.pending.len() < 128 => {
                    match result {
                        Ok((stream, address)) => {
                            let acceptor = self.acceptor.clone();
                            self.pending.spawn(async move {
                                let tls = tokio::time::timeout(std::time::Duration::from_secs(5), acceptor.accept(stream)).await??;
                                Ok((tls, address))
                            });
                        }
                        Err(_) => tokio::time::sleep(std::time::Duration::from_millis(100)).await,
                    }
                }
            }
        }
    }
    fn local_addr(&self) -> std::io::Result<Self::Addr> {
        self.tcp.local_addr()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::signatures::new_key;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    // Real handshakes verify possession and pinning before application bytes.
    async fn handshake(server: SigningKey, pin: PeerPin) -> bool {
        let (cert, key) = certificate(&server).unwrap();
        let acceptor = tokio_rustls::TlsAcceptor::from(Arc::new(
            server_config(cert.as_bytes(), key.as_bytes()).unwrap(),
        ));
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let task = tokio::spawn(async move {
            let (io, _) = listener.accept().await.unwrap();
            if let Ok(mut tls) = acceptor.accept(io).await {
                let _ = tls.write_all(b"ok").await;
            }
        });
        let result = async {
            let io = tokio::net::TcpStream::connect(addr).await?;
            let mut tls = tokio_rustls::TlsConnector::from(Arc::new(pin.config()))
                .connect(
                    ServerName::try_from("192.168.99.42").unwrap().to_owned(),
                    io,
                )
                .await?;
            let mut bytes = [0; 2];
            tls.read_exact(&mut bytes).await?;
            Ok::<_, anyhow::Error>(bytes == *b"ok")
        }
        .await
        .unwrap_or(false);
        task.await.unwrap();
        result
    }
    #[tokio::test]
    async fn pins_key_across_addresses_and_rejects_another_host() {
        let key = new_key();
        let pin = PeerPin::trusted(public_key(key.verifying_key())).unwrap();
        assert!(handshake(key, pin.clone()).await);
        assert!(!handshake(new_key(), pin).await);
    }
    #[tokio::test]
    async fn bootstrap_is_operation_scoped_and_must_match_authenticated_key() {
        let key = new_key();
        let public = public_key(key.verifying_key());
        let pin = PeerPin::bootstrap();
        assert!(pin.require_key(&public).is_err());
        assert!(handshake(key, pin.clone()).await);
        pin.require_key(&public).unwrap();
        assert!(pin
            .require_key(&public_key(new_key().verifying_key()))
            .is_err());
        assert!(!handshake(new_key(), pin).await);
    }
    #[test]
    fn child_services_require_host_identity_and_never_create_one() {
        let dir = tempfile::tempdir().unwrap();
        assert!(require_host_identity(dir.path(), "host").is_err());
        assert!(!dir.path().join("identity.json").exists());
        let key = new_key();
        let (cert, pem) = certificate(&key).unwrap();
        std::fs::write(dir.path().join(KEY_FILE), pem).unwrap();
        std::fs::write(dir.path().join(CERT_FILE), cert).unwrap();
        assert!(require_host_identity(dir.path(), "host").is_err());
        std::fs::write(dir.path().join("identity.json"), serde_json::to_vec(&serde_json::json!({"host_id":"host", "private_key":crate::signatures::encode_key(&key).unwrap()})).unwrap()).unwrap();
        require_host_identity(dir.path(), "host").unwrap();
        assert!(require_host_identity(dir.path(), "other").is_err());
        let (other_cert, _) = certificate(&new_key()).unwrap();
        std::fs::write(dir.path().join(CERT_FILE), other_cert).unwrap();
        assert!(require_host_identity(dir.path(), "host").is_err());
    }
    #[test]
    fn mismatched_certificate_and_private_key_cannot_start_a_listener() {
        let (cert, _) = certificate(&new_key()).unwrap();
        let (_, key) = certificate(&new_key()).unwrap();
        assert!(server_config(cert.as_bytes(), key.as_bytes()).is_err());
    }
}
