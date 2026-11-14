//! IP transport for Security 2 on a native-selected LAN. Every TCP connection
//! is bound before connect; DNS, ambient proxies and redirects are never used.
use std::net::SocketAddr;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use std::time::Duration;

use crate::platform::{LanNetwork, Link, LinkError};
use app_facade_api::pod::{POD_CREDENTIAL_ENDPOINT, POD_INFO_ENDPOINT};
use async_trait::async_trait;
use http_body_util::{BodyExt, Full};
use hyper::body::Bytes;
use hyper_util::rt::TokioIo;
use tokio::net::TcpSocket;

const MAX_RESPONSE: usize = 64 * 1024;

pub(crate) struct HttpLink {
    network: LanNetwork,
    address: SocketAddr,
    session: String,
    open: AtomicBool,
    closed: tokio::sync::Notify,
}

impl HttpLink {
    pub fn connect(address: SocketAddr, network: LanNetwork) -> Result<Arc<dyn Link>, LinkError> {
        if address.port() == 0 || address.ip().is_unspecified() || address.ip().is_multicast() {
            return Err(LinkError::NotFound("invalid Pod LAN address".into()));
        }
        if network.interface_index == 0
            || network.interface_name.is_empty()
            || network.interface_name.as_bytes().contains(&0)
        {
            return Err(LinkError::Unavailable(
                "Local network identity is missing".into(),
            ));
        }
        if let SocketAddr::V6(v6) = address {
            if v6.scope_id() != 0 && v6.scope_id() != network.interface_index {
                return Err(LinkError::Unavailable(
                    "Device scope does not match the selected LAN".into(),
                ));
            }
        }
        Ok(Arc::new(Self {
            network,
            address,
            session: uuid::Uuid::new_v4().to_string(),
            open: AtomicBool::new(true),
            closed: tokio::sync::Notify::new(),
        }))
    }

    async fn post(&self, endpoint: &str, request: &[u8]) -> Result<Vec<u8>, LinkError> {
        let socket = if self.address.is_ipv4() {
            TcpSocket::new_v4()
        } else {
            TcpSocket::new_v6()
        }
        .map_err(lost)?;
        bind_network(&socket, &self.network, self.address.is_ipv4()).map_err(lost)?;
        let stream = tokio::time::timeout(Duration::from_secs(5), socket.connect(self.address))
            .await
            .map_err(|_| lost("Pod connection timed out"))?
            .map_err(lost)?;
        let (mut sender, connection) = hyper::client::conn::http1::Builder::new()
            .max_buf_size(MAX_RESPONSE)
            .handshake(TokioIo::new(stream))
            .await
            .map_err(lost)?;
        // Each protocol exchange owns one connection. Cancellation/timeout drops
        // the driver as well as the request, so no background write survives it.
        let _driver = Driver(tokio::spawn(async move {
            let _ = connection.await;
        }));
        if !self.open.load(Ordering::SeqCst) {
            return Err(lost("LAN session closed"));
        }
        let message = hyper::Request::post(format!("/{endpoint}"))
            .header("Host", format!("pod-init.invalid:{}", self.address.port()))
            .header("X-Meow-Session", &self.session)
            .header("Content-Type", "application/octet-stream")
            .header("Connection", "close")
            .body(Full::new(Bytes::copy_from_slice(request)))
            .map_err(lost)?;
        let response = sender.send_request(message).await.map_err(lost)?;
        if !response.status().is_success() {
            return Err(LinkError::Rejected(format!(
                "Pod returned {}",
                response.status()
            )));
        }
        let mut body = response.into_body();
        let mut bytes = Vec::new();
        while let Some(frame) = body.frame().await {
            if let Ok(chunk) = frame.map_err(lost)?.into_data() {
                if bytes.len().saturating_add(chunk.len()) > MAX_RESPONSE {
                    return Err(LinkError::Rejected("Pod response is too large".into()));
                }
                bytes.extend_from_slice(&chunk);
            }
        }
        Ok(bytes)
    }
}

fn lost(error: impl std::fmt::Display) -> LinkError {
    LinkError::Disconnected(error.to_string())
}
struct Driver(tokio::task::JoinHandle<()>);
impl Drop for Driver {
    fn drop(&mut self) {
        self.0.abort();
    }
}

#[async_trait]
impl Link for HttpLink {
    async fn exchange(&self, endpoint: &str, request: &[u8]) -> Result<Vec<u8>, LinkError> {
        // Register before checking the state, closing the disconnect race.
        let closed = self.closed.notified();
        tokio::pin!(closed);
        closed.as_mut().enable();
        if !self.open.load(Ordering::SeqCst) {
            return Err(lost("LAN session closed"));
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
        tokio::select! {
            biased;
            _ = closed => Err(lost("LAN session closed")),
            result = tokio::time::timeout(Duration::from_secs(20), self.post(endpoint, request)) =>
                result.map_err(|_| lost("Pod request timed out"))?,
        }
    }
    async fn is_connected(&self) -> bool {
        self.open.load(Ordering::SeqCst)
    }
    async fn disconnect(&self) {
        self.open.store(false, Ordering::SeqCst);
        self.closed.notify_waiters();
    }
}

#[cfg(target_os = "linux")]
fn bind_network(socket: &TcpSocket, network: &LanNetwork, _: bool) -> std::io::Result<()> {
    socket2::SockRef::from(socket).bind_device(Some(network.interface_name.as_bytes()))
}
#[cfg(target_vendor = "apple")]
fn bind_network(socket: &TcpSocket, network: &LanNetwork, ipv4: bool) -> std::io::Result<()> {
    let index = std::num::NonZeroU32::new(network.interface_index);
    let socket = socket2::SockRef::from(socket);
    if ipv4 {
        socket.bind_device_by_index_v4(index)
    } else {
        socket.bind_device_by_index_v6(index)
    }
}
#[cfg(target_os = "android")]
fn bind_network(socket: &TcpSocket, network: &LanNetwork, _: bool) -> std::io::Result<()> {
    use std::os::fd::AsRawFd;
    #[link(name = "android")]
    unsafe extern "C" {
        fn android_setsocknetwork(network: u64, fd: std::ffi::c_int) -> std::ffi::c_int;
    }
    let handle = network
        .android_network
        .filter(|value| *value != 0)
        .ok_or_else(|| std::io::Error::other("Android LAN Network is missing"))?;
    // The platform selected this Network; the API affects only this new socket.
    if unsafe { android_setsocknetwork(handle, socket.as_raw_fd()) } == 0 {
        Ok(())
    } else {
        Err(std::io::Error::last_os_error())
    }
}
#[cfg(windows)]
fn bind_network(socket: &TcpSocket, network: &LanNetwork, ipv4: bool) -> std::io::Result<()> {
    use std::os::windows::io::AsRawSocket;
    use windows_sys::Win32::Networking::WinSock::{
        setsockopt, WSAGetLastError, IPPROTO_IP, IPPROTO_IPV6, IPV6_UNICAST_IF, IP_UNICAST_IF,
        SOCKET,
    };
    let (level, option, index) = if ipv4 {
        (IPPROTO_IP, IP_UNICAST_IF, network.interface_index.to_be())
    } else {
        (IPPROTO_IPV6, IPV6_UNICAST_IF, network.interface_index)
    };
    // Winsock requires network byte order for IPv4, host byte order for IPv6.
    let result = unsafe {
        setsockopt(
            socket.as_raw_socket() as SOCKET,
            level,
            option,
            (&index as *const u32).cast(),
            std::mem::size_of_val(&index) as i32,
        )
    };
    if result == 0 {
        Ok(())
    } else {
        Err(std::io::Error::from_raw_os_error(unsafe {
            WSAGetLastError()
        }))
    }
}
#[cfg(not(any(
    target_os = "linux",
    target_os = "android",
    target_vendor = "apple",
    windows
)))]
fn bind_network(_: &TcpSocket, _: &LanNetwork, _: bool) -> std::io::Result<()> {
    Err(std::io::Error::new(
        std::io::ErrorKind::Unsupported,
        "LAN interface binding is unavailable",
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    fn loopback_network() -> LanNetwork {
        let name = if cfg!(target_vendor = "apple") {
            "lo0"
        } else {
            "lo"
        };
        #[cfg(unix)]
        let index = {
            unsafe extern "C" {
                fn if_nametoindex(name: *const std::ffi::c_char) -> u32;
            }
            unsafe { if_nametoindex(std::ffi::CString::new(name).unwrap().as_ptr()) }
        };
        #[cfg(windows)]
        let index = 1;
        LanNetwork {
            interface_name: name.into(),
            interface_index: index,
            android_network: None,
        }
    }

    #[tokio::test]
    async fn missing_or_disappeared_network_never_uses_the_default_route() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let mut network = loopback_network();
        network.interface_index = 0;
        assert!(HttpLink::connect(address, network.clone()).is_err());
        network.interface_index = u32::MAX;
        network.interface_name = "no-meow-lan".into();
        let link = HttpLink::connect(address, network).unwrap();
        assert!(link.exchange("pod-info", b"secret").await.is_err());
        assert!(
            tokio::time::timeout(Duration::from_millis(50), listener.accept())
                .await
                .is_err()
        );
        let scoped: SocketAddr = "[fe80::1%42]:18782".parse().unwrap();
        assert!(HttpLink::connect(scoped, loopback_network()).is_err());
    }

    async fn read_request(socket: &mut tokio::net::TcpStream) {
        let mut bytes = Vec::new();
        loop {
            let mut byte = [0];
            assert_eq!(socket.read(&mut byte).await.unwrap(), 1);
            bytes.push(byte[0]);
            if bytes.ends_with(b"\r\n\r\n") {
                return;
            }
            assert!(bytes.len() < 4096);
        }
    }

    #[tokio::test]
    async fn chunked_response_is_bounded_and_ipv6_uses_the_selected_interface() {
        let listener = tokio::net::TcpListener::bind("[::1]:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let server = tokio::spawn(async move {
            for size in [3, MAX_RESPONSE + 1] {
                let (mut socket, _) = listener.accept().await.unwrap();
                read_request(&mut socket).await;
                let header =
                    format!("HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\n\r\n{size:x}\r\n");
                socket.write_all(header.as_bytes()).await.unwrap();
                socket.write_all(&vec![b'a'; size]).await.unwrap();
                let _ = socket.write_all(b"\r\n0\r\n\r\n").await;
            }
        });
        let link = HttpLink::connect(address, loopback_network()).unwrap();
        assert_eq!(link.exchange("pod-info", b"").await.unwrap(), b"aaa");
        assert!(matches!(
            link.exchange("pod-info", b"").await,
            Err(LinkError::Rejected(_))
        ));
        server.await.unwrap();
    }

    #[tokio::test]
    async fn disconnect_cancels_the_request_and_closes_its_tcp_driver() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let link = HttpLink::connect(listener.local_addr().unwrap(), loopback_network()).unwrap();
        let exchange = {
            let link = link.clone();
            tokio::spawn(async move { link.exchange("pod-info", b"").await })
        };
        let (mut socket, _) = listener.accept().await.unwrap();
        read_request(&mut socket).await;
        link.disconnect().await;
        assert!(matches!(
            exchange.await.unwrap(),
            Err(LinkError::Disconnected(_))
        ));
        let mut byte = [0];
        let count = tokio::time::timeout(Duration::from_secs(1), socket.read(&mut byte))
            .await
            .unwrap()
            .unwrap();
        assert_eq!(count, 0);
        assert!(!link.is_connected().await);
    }

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
        let first = HttpLink::connect(address, loopback_network()).unwrap();
        assert_eq!(
            first.exchange("pod-info", b"ciphertext").await.unwrap(),
            b"abc"
        );
        assert!(first.exchange("prov-config", b"wifi").await.is_err());
        first.disconnect().await;
        assert!(first.exchange("pod-info", b"ciphertext").await.is_err());
        let second = HttpLink::connect(address, loopback_network()).unwrap();
        assert!(matches!(
            second.exchange("pod-info", b"ciphertext").await,
            Err(LinkError::Rejected(_))
        ));
        server.await.unwrap();
    }
}
