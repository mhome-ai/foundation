//! IP transport for the same Security 2 handshake and encrypted messages as BLE.
//! IPs come from local DNS-SD; redirects and ambient HTTP proxies are forbidden.
use std::net::SocketAddr;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use std::time::Duration;

use crate::platform::{Link, LinkError};
use app_facade_api::pod::{POD_CREDENTIAL_ENDPOINT, POD_INFO_ENDPOINT};
use async_trait::async_trait;

const MAX_RESPONSE: usize = 64 * 1024;

pub(crate) struct HttpLink {
    client: reqwest::Client,
    address: SocketAddr,
    session: String,
    open: AtomicBool,
}

impl HttpLink {
    pub fn new(address: SocketAddr) -> Result<Arc<dyn Link>, LinkError> {
        if address.port() == 0 || address.ip().is_unspecified() || address.ip().is_multicast() {
            return Err(LinkError::NotFound("invalid Pod LAN address".into()));
        }
        let client = reqwest::Client::builder()
            .no_proxy()
            // Resolve directly to the discovered socket, retaining IPv6 scope IDs.
            .resolve("pod-init.invalid", address)
            .redirect(reqwest::redirect::Policy::none())
            .connect_timeout(Duration::from_secs(5))
            .timeout(Duration::from_secs(20))
            .build()
            .map_err(|e| LinkError::Unavailable(e.to_string()))?;
        Ok(Arc::new(Self {
            client,
            address,
            session: uuid::Uuid::new_v4().to_string(),
            open: AtomicBool::new(true),
        }))
    }
}

#[async_trait]
impl Link for HttpLink {
    async fn exchange(&self, endpoint: &str, request: &[u8]) -> Result<Vec<u8>, LinkError> {
        if !self.open.load(Ordering::SeqCst) {
            return Err(LinkError::Disconnected("LAN session closed".into()));
        }
        if !matches!(
            endpoint,
            "proto-ver" | "prov-session" | POD_INFO_ENDPOINT | POD_CREDENTIAL_ENDPOINT
        ) {
            return Err(LinkError::Rejected(
                "endpoint is not part of Pod LAN initialization".into(),
            ));
        }
        if request.len() > MAX_RESPONSE {
            return Err(LinkError::Rejected("Pod request is too large".into()));
        }
        let mut response = self
            .client
            .post(format!(
                "http://pod-init.invalid:{}/{endpoint}",
                self.address.port()
            ))
            .header("X-Meow-Session", &self.session)
            .header("Content-Type", "application/octet-stream")
            .body(request.to_vec())
            .send()
            .await
            .map_err(|e| LinkError::Disconnected(e.to_string()))?;
        if !response.status().is_success() {
            return Err(LinkError::Rejected(format!(
                "Pod returned {}",
                response.status()
            )));
        }
        let mut bytes = Vec::new();
        while let Some(chunk) = response
            .chunk()
            .await
            .map_err(|e| LinkError::Disconnected(e.to_string()))?
        {
            if bytes.len().saturating_add(chunk.len()) > MAX_RESPONSE {
                return Err(LinkError::Rejected("Pod response is too large".into()));
            }
            bytes.extend_from_slice(&chunk);
        }
        Ok(bytes)
    }

    async fn is_connected(&self) -> bool {
        self.open.load(Ordering::SeqCst)
    }
    async fn disconnect(&self) {
        self.open.store(false, Ordering::SeqCst);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    #[tokio::test]
    async fn direct_ip_transport_uses_fresh_session_ids_and_refuses_redirects() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let server = tokio::spawn(async move {
            let mut ids = Vec::new();
            for attempt in 0..2 {
                let (mut socket, _) = listener.accept().await.unwrap();
                let mut data = Vec::new();
                let header_end = loop {
                    let mut chunk = [0u8; 1024];
                    let n = socket.read(&mut chunk).await.unwrap();
                    assert!(n > 0);
                    data.extend_from_slice(&chunk[..n]);
                    if let Some(end) = data.windows(4).position(|w| w == b"\r\n\r\n") {
                        break end + 4;
                    }
                };
                let headers = String::from_utf8_lossy(&data[..header_end]).to_lowercase();
                assert!(headers.starts_with("post /pod-info http/1.1\r\n"));
                let id = headers
                    .lines()
                    .find_map(|line| line.strip_prefix("x-meow-session: "))
                    .unwrap()
                    .to_string();
                assert!(uuid::Uuid::parse_str(&id).is_ok());
                ids.push(id);
                if attempt == 0 {
                    socket
                        .write_all(
                            b"HTTP/1.1 200 OK\r\nContent-Length: 3\r\nConnection: close\r\n\r\nabc",
                        )
                        .await
                        .unwrap();
                } else {
                    socket.write_all(b"HTTP/1.1 302 Found\r\nLocation: http://127.0.0.1:1/pod-info\r\nContent-Length: 0\r\nConnection: close\r\n\r\n").await.unwrap();
                }
            }
            assert_ne!(ids[0], ids[1]);
        });
        let first = HttpLink::new(address).unwrap();
        assert_eq!(
            first.exchange("pod-info", b"ciphertext").await.unwrap(),
            b"abc"
        );
        assert!(first.exchange("prov-config", b"wifi").await.is_err());
        first.disconnect().await;
        assert!(first.exchange("pod-info", b"ciphertext").await.is_err());
        let second = HttpLink::new(address).unwrap();
        assert!(matches!(
            second.exchange("pod-info", b"ciphertext").await,
            Err(LinkError::Rejected(_))
        ));
        server.await.unwrap();
    }
}
