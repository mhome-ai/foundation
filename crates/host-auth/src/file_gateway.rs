//! Native file URLs for WebViews. Tickets authorize one pinned HTTPS resource (or
//! one Storage session prefix); this is not an arbitrary URL/traffic proxy.
use anyhow::{ensure, Context, Result};
use axum::{
    body::Body,
    extract::{Request, State},
    http::{HeaderValue, Method, StatusCode},
    response::{IntoResponse, Response},
    routing::any,
    Router,
};
use core_api::host::auth::PublicKey;
use futures_util::StreamExt;
use serde::Deserialize;
use std::{
    collections::HashMap,
    sync::Arc,
    time::{SystemTime, UNIX_EPOCH},
};
use tokio::sync::{Mutex, Semaphore};
use tokio_util::sync::CancellationToken;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FileGrant {
    pub url: String,
    pub host_public_key: PublicKey,
    pub expires_at_unix_ms: u64,
    #[serde(default)]
    pub storage_prefix: bool,
    #[serde(default)]
    pub headers: std::collections::BTreeMap<String, String>,
}
struct Ticket {
    url: reqwest::Url,
    client: reqwest::Client,
    expires: u64,
    prefix: bool,
    cancel: CancellationToken,
}
impl Drop for Ticket {
    fn drop(&mut self) {
        self.cancel.cancel();
    }
}
struct GatewayState {
    tickets: Mutex<HashMap<String, Arc<Ticket>>>,
    slots: Arc<Semaphore>,
}
#[derive(Clone)]
pub struct FileGateway {
    state: Arc<GatewayState>,
    base: String,
    shutdown: CancellationToken,
}
fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}
impl FileGateway {
    pub async fn start() -> Result<Self> {
        let tcp = tokio::net::TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, 0)).await?;
        let base = format!("http://127.0.0.1:{}/files/", tcp.local_addr()?.port());
        let state = Arc::new(GatewayState {
            tickets: Mutex::new(HashMap::new()),
            slots: Arc::new(Semaphore::new(32)),
        });
        let router = Router::new()
            .fallback(any(serve_file))
            .with_state(state.clone());
        let shutdown = CancellationToken::new();
        let stopped = shutdown.clone();
        tokio::spawn(async move {
            let _ = axum::serve(tcp, router)
                .with_graceful_shutdown(stopped.cancelled_owned())
                .await;
        });
        Ok(Self {
            state,
            base,
            shutdown,
        })
    }
    pub async fn register(&self, grant: FileGrant) -> Result<String> {
        let url = reqwest::Url::parse(&grant.url)?;
        ensure!(
            url.scheme() == "https"
                && url.username().is_empty()
                && url.password().is_none()
                && url.fragment().is_none(),
            "File endpoint requires HTTPS without userinfo/fragment"
        );
        ensure!(
            grant.expires_at_unix_ms > now(),
            "File authorization expired"
        );
        ensure!(
            !grant.storage_prefix || (url.query().is_none() && url.path().ends_with("/storage/v1")),
            "Invalid Storage session prefix"
        );
        let mut headers = reqwest::header::HeaderMap::new();
        for (name, value) in grant.headers {
            ensure!(
                name.eq_ignore_ascii_case("authorization")
                    || name.eq_ignore_ascii_case("content-type"),
                "Unsupported file grant header"
            );
            headers.insert(
                reqwest::header::HeaderName::from_bytes(name.as_bytes())?,
                HeaderValue::from_str(&value)?,
            );
        }
        let client = reqwest::Client::builder()
            .use_preconfigured_tls(crate::tls::PeerPin::trusted(grant.host_public_key)?.config())
            .no_proxy()
            .redirect(reqwest::redirect::Policy::none())
            .no_gzip()
            .no_brotli()
            .no_deflate()
            .connect_timeout(std::time::Duration::from_secs(10))
            .read_timeout(std::time::Duration::from_secs(60))
            .default_headers(headers)
            .build()?;
        let id = crate::signatures::random_id();
        let mut tickets = self.state.tickets.lock().await;
        tickets.retain(|_, t| {
            let keep = t.expires > now();
            if !keep {
                t.cancel.cancel();
            }
            keep
        });
        ensure!(tickets.len() < 256, "Too many active file authorizations");
        tickets.insert(
            id.clone(),
            Arc::new(Ticket {
                url,
                client,
                expires: grant.expires_at_unix_ms,
                prefix: grant.storage_prefix,
                cancel: CancellationToken::new(),
            }),
        );
        Ok(format!("{}{id}", self.base))
    }
    /// Called on logout/page retirement; also stops in-flight upload/download streams.
    pub async fn clear(&self) {
        let mut tickets = self.state.tickets.lock().await;
        for ticket in tickets.values() {
            ticket.cancel.cancel();
        }
        tickets.clear();
    }
    pub async fn stop(&self) {
        self.clear().await;
        self.shutdown.cancel();
    }
}
async fn serve_file(State(state): State<Arc<GatewayState>>, request: Request) -> Response {
    let result = transfer(state, request).await;
    let mut response = result
        .unwrap_or_else(|_| (StatusCode::BAD_GATEWAY, "File transfer unavailable").into_response());
    let headers = response.headers_mut();
    headers.insert("access-control-allow-origin", HeaderValue::from_static("*"));
    headers.insert(
        "access-control-allow-methods",
        HeaderValue::from_static("GET, HEAD, POST, PUT, PATCH, DELETE, OPTIONS"),
    );
    headers.insert("access-control-allow-headers", HeaderValue::from_static("Authorization, Content-Type, Range, If-Range, If-Match, If-None-Match, Content-Encoding, X-Meow-Content-Sha256"));
    headers.insert(
        "access-control-expose-headers",
        HeaderValue::from_static(
            "Content-Length, Content-Type, Content-Range, Accept-Ranges, ETag, Last-Modified, X-Meow-Object-Id, X-Meow-Content-Sha256",
        ),
    );
    headers.insert("cache-control", HeaderValue::from_static("no-store"));
    response
}
async fn transfer(state: Arc<GatewayState>, request: Request) -> Result<Response> {
    let path = request
        .uri()
        .path()
        .strip_prefix("/files/")
        .context("Unknown file route")?;
    let (id, suffix) = path
        .split_once('/')
        .map_or((path, ""), |(id, rest)| (id, rest));
    let ticket = state
        .tickets
        .lock()
        .await
        .get(id)
        .cloned()
        .context("Unknown file ticket")?;
    ensure!(
        ticket.expires > now() && !ticket.cancel.is_cancelled(),
        "File authorization expired"
    );
    if request.method() == Method::OPTIONS {
        return Ok(StatusCode::NO_CONTENT.into_response());
    }
    ensure!(
        ticket.prefix
            || (suffix.is_empty()
                && (request.method() == Method::GET || request.method() == Method::HEAD)),
        "File ticket does not allow this operation"
    );
    let url = if ticket.prefix {
        let lowered = suffix.to_ascii_lowercase();
        ensure!(
            !lowered.contains("%2f") && !lowered.contains("%5c") && !suffix.contains('\\'),
            "Invalid file path"
        );
        let mut target = reqwest::Url::parse(&format!(
            "{}/{}",
            ticket.url.as_str().trim_end_matches('/'),
            suffix
        ))?;
        ensure!(
            target.origin() == ticket.url.origin()
                && target
                    .path()
                    .starts_with(&format!("{}/", ticket.url.path().trim_end_matches('/'))),
            "File path escaped session"
        );
        target.set_query(request.uri().query());
        target
    } else {
        ensure!(
            request.uri().query().is_none(),
            "File ticket cannot change query"
        );
        ticket.url.clone()
    };
    let permit = state
        .slots
        .clone()
        .try_acquire_owned()
        .context("File transfer capacity exceeded")?;
    let mut outgoing = ticket.client.request(request.method().clone(), url);
    for name in [
        "range",
        "if-range",
        "if-match",
        "if-none-match",
        "content-type",
        "content-length",
        "content-encoding",
        "x-meow-content-sha256",
    ] {
        if let Some(value) = request.headers().get(name) {
            outgoing = outgoing.header(name, value);
        }
    }
    let body = request.into_body().into_data_stream();
    let outgoing = outgoing.body(reqwest::Body::wrap_stream(body)).send();
    let response = tokio::select! { result = outgoing => result?, _ = ticket.cancel.cancelled() => anyhow::bail!("File authorization retired") };
    let mut result = Response::builder().status(response.status());
    for name in [
        "content-type",
        "content-length",
        "content-range",
        "accept-ranges",
        "etag",
        "last-modified",
        "content-encoding",
        "content-disposition",
        "x-meow-object-id",
        "x-meow-content-sha256",
    ] {
        if let Some(value) = response.headers().get(name) {
            result = result.header(name, value);
        }
    }
    // One chunk at a time with backpressure, including Range responses. No whole-file buffer.
    let stream = async_stream::stream! {
        let _permit = permit;
        let mut input = response.bytes_stream();
        loop {
            let next = tokio::select! { chunk = input.next() => chunk, _ = ticket.cancel.cancelled() => None };
            match next { Some(Ok(bytes)) => yield Ok::<_, std::io::Error>(bytes), Some(Err(error)) => { yield Err(std::io::Error::other(error)); break; }, None => break }
        }
    };
    Ok(result.body(Body::from_stream(stream))?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        signatures::{new_key, public_key},
        tls,
    };
    use axum::http::HeaderMap;

    async fn fixture() -> (String, PublicKey, tokio::task::JoinHandle<()>) {
        let key = new_key();
        let pin = public_key(key.verifying_key());
        let (cert, private) = tls::certificate(&key).unwrap();
        let tcp = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let url = format!("https://{}/file?grant=secret", tcp.local_addr().unwrap());
        let listener = tls::Listener::new(
            tcp,
            tls::server_config(cert.as_bytes(), private.as_bytes()).unwrap(),
        );
        let app = Router::new().route(
            "/file",
            any(|method: Method, headers: HeaderMap| async move {
                assert_eq!(method, Method::GET);
                assert_eq!(headers.get("range").unwrap(), "bytes=5-9");
                Response::builder()
                    .status(StatusCode::PARTIAL_CONTENT)
                    .header("content-range", "bytes 5-9/10")
                    .header("content-length", "5")
                    .body(Body::from("56789"))
                    .unwrap()
            }),
        );
        let task = tokio::spawn(async move {
            axum::serve(listener, app).await.unwrap();
        });
        (url, pin, task)
    }
    fn grant(url: String, key: PublicKey) -> FileGrant {
        FileGrant {
            url,
            host_public_key: key,
            expires_at_unix_ms: now() + 60000,
            storage_prefix: false,
            headers: Default::default(),
        }
    }

    #[tokio::test]
    async fn storage_checksum_and_metadata_survive_the_native_gateway() {
        use sha2::{Digest, Sha256};

        // Exercise an authenticated Storage upload contract over real pinned TLS.
        // Missing/incorrect checksums must not become successful uploads in transit.
        async fn upload(headers: HeaderMap, body: axum::body::Bytes) -> Response {
            assert_eq!(headers["authorization"], "Bearer session-token");
            assert!(!headers.contains_key("x-unrelated-header"));
            let digest = format!("{:x}", Sha256::digest(&body));
            if headers
                .get("x-meow-content-sha256")
                .and_then(|h| h.to_str().ok())
                != Some(digest.as_str())
            {
                return (StatusCode::BAD_REQUEST, "content SHA-256 mismatch").into_response();
            }
            Response::builder()
                .status(StatusCode::CREATED)
                .header("x-meow-object-id", "object-1")
                .header("x-meow-content-sha256", digest)
                .body(Body::from("stored"))
                .unwrap()
        }

        let key = new_key();
        let pin = public_key(key.verifying_key());
        let (cert, private) = tls::certificate(&key).unwrap();
        let tcp = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let endpoint = format!("https://{}/storage/v1", tcp.local_addr().unwrap());
        let listener = tls::Listener::new(
            tcp,
            tls::server_config(cert.as_bytes(), private.as_bytes()).unwrap(),
        );
        let app = Router::new().route("/storage/v1/objects/file", axum::routing::put(upload));
        let server = tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
        let gateway = FileGateway::start().await.unwrap();
        let mut grant = grant(endpoint, pin);
        grant.storage_prefix = true;
        grant
            .headers
            .insert("Authorization".into(), "Bearer session-token".into());
        let local = format!("{}/objects/file", gateway.register(grant).await.unwrap());
        let client = reqwest::Client::builder().no_proxy().build().unwrap();
        let preflight = client
            .request(Method::OPTIONS, &local)
            .header("Origin", "http://localhost")
            .header("Access-Control-Request-Method", "PUT")
            .header("Access-Control-Request-Headers", "x-meow-content-sha256")
            .send()
            .await
            .unwrap();
        assert_eq!(preflight.status(), StatusCode::NO_CONTENT);
        assert!(preflight.headers()["access-control-allow-headers"]
            .to_str()
            .unwrap()
            .split(',')
            .any(|h| h.trim().eq_ignore_ascii_case("x-meow-content-sha256")));

        let digest = format!("{:x}", Sha256::digest(b"data"));
        let response = client
            .put(&local)
            .header("x-meow-content-sha256", &digest)
            .header("Authorization", "Bearer must-not-override-session")
            .header("x-unrelated-header", "must-not-pass")
            .body("data")
            .send()
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::CREATED);
        assert_eq!(response.headers()["x-meow-object-id"], "object-1");
        assert_eq!(response.headers()["x-meow-content-sha256"], digest);
        for name in ["x-meow-object-id", "x-meow-content-sha256"] {
            assert!(response.headers()["access-control-expose-headers"]
                .to_str()
                .unwrap()
                .split(',')
                .any(|h| h.trim().eq_ignore_ascii_case(name)));
        }
        assert_eq!(response.text().await.unwrap(), "stored");

        let rejected = client
            .put(&local)
            .header("x-meow-content-sha256", "0".repeat(64))
            .body("data")
            .send()
            .await
            .unwrap();
        assert_eq!(rejected.status(), StatusCode::BAD_REQUEST);
        assert_eq!(rejected.text().await.unwrap(), "content SHA-256 mismatch");
        gateway.stop().await;
        server.abort();
        let _ = server.await;
    }
    #[tokio::test]
    async fn range_uses_pinned_tls_and_logout_retires_ticket() {
        let (url, key, server) = fixture().await;
        let gateway = FileGateway::start().await.unwrap();
        let local = gateway.register(grant(url, key)).await.unwrap();
        let client = reqwest::Client::new();
        let response = client
            .get(&local)
            .header("Range", "bytes=5-9")
            .send()
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::PARTIAL_CONTENT);
        assert_eq!(response.headers()["content-range"], "bytes 5-9/10");
        assert_eq!(response.text().await.unwrap(), "56789");
        assert_eq!(
            client
                .put(&local)
                .body("overwrite")
                .send()
                .await
                .unwrap()
                .status(),
            StatusCode::BAD_GATEWAY
        );
        gateway.clear().await;
        assert_eq!(
            client.get(local).send().await.unwrap().status(),
            StatusCode::BAD_GATEWAY
        );
        gateway.stop().await;
        server.abort();
        let _ = server.await;
    }
    #[tokio::test]
    async fn wrong_host_key_and_plaintext_are_rejected() {
        let (url, _, server) = fixture().await;
        let gateway = FileGateway::start().await.unwrap();
        let key = public_key(new_key().verifying_key());
        assert!(gateway
            .register(grant(url.replace("https:", "http:"), key.clone()))
            .await
            .is_err());
        let direct = reqwest::Client::builder()
            .use_preconfigured_tls(tls::PeerPin::trusted(key.clone()).unwrap().config())
            .build()
            .unwrap();
        let failure = direct.get(&url).send().await.unwrap_err();
        assert!(
            tls::is_identity_error(&failure),
            "certificate rejection lost its classification: {failure:?}"
        );
        let local = gateway.register(grant(url, key)).await.unwrap();
        let response = reqwest::get(local).await.unwrap();
        assert_eq!(response.status(), StatusCode::BAD_GATEWAY);
        gateway.stop().await;
        server.abort();
        let _ = server.await;
    }
}
