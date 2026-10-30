use super::*;
use base64::{engine::general_purpose::STANDARD, Engine};
use ed25519_dalek::Signer;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering::SeqCst};

struct Fake {
    context: Mutex<String>,
    ready: AtomicBool,
    offline: AtomicBool,
    operation: Mutex<Option<api::HubOperation>>,
    cloud_outcome: Mutex<String>,
    cloud_delay: Mutex<Duration>,
    cloud_count: AtomicUsize,
    wrong_peer: AtomicBool,
    fail: AtomicBool,
    lost_response: AtomicBool,
    count: AtomicUsize,
    setup: AtomicUsize,
    connected: AtomicUsize,
    entered: tokio::sync::Notify,
}
impl Fake {
    fn new() -> Arc<Self> {
        Arc::new(Self {
            context: Mutex::new("login-one".into()),
            ready: false.into(),
            offline: false.into(),
            operation: Mutex::new(None),
            cloud_outcome: Mutex::new("COMPLETED".into()),
            cloud_delay: Mutex::new(Duration::ZERO),
            cloud_count: 0.into(),
            wrong_peer: false.into(),
            fail: false.into(),
            lost_response: false.into(),
            count: 0.into(),
            setup: 0.into(),
            connected: 0.into(),
            entered: tokio::sync::Notify::new(),
        })
    }
    fn key() -> ed25519_dalek::SigningKey {
        ed25519_dalek::SigningKey::from_bytes(&[7; 32])
    }
    fn peer() -> PublicKey {
        PublicKey {
            x: "x".into(),
            y: "y".into(),
        }
    }
    fn candidate() -> Candidate {
        Candidate {
            host_id: "host".into(),
            host_name: "Host".into(),
            hub_type: "linux".into(),
            fingerprint: "fingerprint".into(),
            public_key: STANDARD.encode(Self::key().verifying_key().to_bytes()),
            addresses: vec!["192.168.1.2".into()],
            port: 443,
            runtime_space_policy: RuntimeSpacePolicy::Multi {},
            route_context: "lan-one".into(),
        }
    }
}
#[async_trait]
impl Platform for Fake {
    async fn account_context(&self) -> Result<String, String> {
        Ok(self.context.lock().unwrap().clone())
    }
    async fn candidates(&self, _: &str, _: &str) -> Result<Vec<Candidate>, String> {
        if self.offline.load(SeqCst) {
            return Err("LAN unavailable".into());
        }
        Ok(vec![Self::candidate()])
    }
    async fn view(&self, _: &str, scope: &str) -> Result<api::LocalHubState, String> {
        let ready = self.ready.load(SeqCst);
        serde_json::from_value(json!({"scopeId":scope,"configured":ready,"operation":self.operation.lock().unwrap().clone(),"hub":if ready{json!({"id":"hub-one","hostId":"host","type":"linux","isTest":false,"status":"ready","cloud":{},"local":{"discovered":true,"connected":true}})}else{Value::Null}})).map_err(|e|e.to_string())
    }
    async fn cloud(
        &self,
        _: &str,
        _: &str,
        path: &str,
        body: Value,
        _: Duration,
    ) -> Result<Value, String> {
        assert_eq!(path, "/hub/setup/initiate");
        self.cloud_count.fetch_add(1, SeqCst);
        let delay = *self.cloud_delay.lock().unwrap();
        tokio::time::sleep(delay).await;
        if self.fail.load(SeqCst) {
            return Err("Cloud unavailable".into());
        }
        if body["hubId"] == "hub-one" {
            assert_eq!(body["hostId"], "host");
            assert!(body.get("candidateFingerprint").is_none());
            let outcome = self.cloud_outcome.lock().unwrap().clone();
            if outcome == "COMPLETED" {
                *self.operation.lock().unwrap() = None;
                self.ready.store(true, SeqCst);
            }
            return Ok(json!({"hubId":"hub-one","outcome":outcome}));
        }
        Ok(
            json!({"outcome":"AUTHORIZED","hubId":"hub-one","setupToken":"secret","commissionSessionId":"cs","commissionSessionSecret":"cs-secret"}),
        )
    }
    async fn lan(
        &self,
        _: &str,
        c: &Candidate,
        _: &str,
        pin: Option<&PublicKey>,
        path: &str,
        body: Value,
        timeout: Duration,
    ) -> Result<LanResponse, String> {
        let mut peer = Self::peer();
        let value = match path {
            "/commission/challenge" => {
                let nonce = body["nonce"].as_str().unwrap();
                let key = Self::key();
                json!({"env":RuntimeEnv::current(),"runtimeSpacePolicy":c.runtime_space_policy,"hostId":c.host_id,"hostPublicKey":peer,"nonce":nonce,"fingerprint":c.fingerprint,"publicKey":c.public_key,"signature":STANDARD.encode(key.sign(nonce.as_bytes()).to_bytes()),"hostTlsSignature":STANDARD.encode(key.sign(&core_api::hub_commission_tls_payload(nonce,&c.host_id,&peer,RuntimeEnv::current(),&c.runtime_space_policy)).to_bytes())})
            }
            "/commission/pairing/start" => {
                assert!(pin.is_some());
                json!({"pairingSessionId":format!("pair-{}",self.count.fetch_add(1,SeqCst)),"pairingCode":"123456","expiresIn":120})
            }
            "/setup/start" => {
                assert!(pin.is_some());
                assert!(timeout > Duration::from_secs(45));
                self.setup.fetch_add(1, SeqCst);
                self.entered.notify_one();
                tokio::time::sleep(Duration::from_secs(60)).await;
                if self.lost_response.load(SeqCst) {
                    return Err("Response lost".into());
                }
                json!({"ok":true,"state":"completion_pending"})
            }
            _ => panic!("unexpected LAN path {path}"),
        };
        if self.wrong_peer.load(SeqCst) {
            peer.x = "other".into();
        }
        Ok(LanResponse {
            body: value,
            peer_key: peer,
        })
    }
    async fn connected(&self, _: &str, _: &str, hub: &str, _: &Candidate) -> Result<(), String> {
        assert_eq!(hub, "hub-one");
        self.connected.fetch_add(1, SeqCst);
        Ok(())
    }
}
async fn start(engine: &Arc<Commissioning>) -> String {
    let result = engine
        .start(
            "home",
            api::HubSetupStartRequest {
                host_id: "host".into(),
                endpoint: None,
            },
        )
        .await
        .unwrap();
    match result {
        api::HubSetupStartResponse::PairingRequired { session_id, .. } => session_id,
        _ => panic!("Expected pairing"),
    }
}

#[tokio::test(start_paused = true)]
async fn slow_import_uses_the_setup_deadline_and_waits_for_exact_cloud_commit() {
    let platform = Fake::new();
    let engine = Commissioning::new(platform.clone());
    let pair = start(&engine).await;
    engine.confirm("home", &pair).await.unwrap();
    platform.entered.notified().await;
    tokio::time::advance(Duration::from_secs(46)).await;
    let status = engine.status("home", &pair).await.unwrap();
    assert!(!status.complete);
    assert_eq!(status.pairing.unwrap().state, "RUNNING");
    tokio::time::advance(Duration::from_secs(15)).await;
    tokio::task::yield_now().await;
    assert!(!engine.status("home", &pair).await.unwrap().complete);
    platform.ready.store(true, SeqCst);
    assert!(engine.status("home", &pair).await.unwrap().complete);
    assert_eq!(platform.connected.load(SeqCst), 1);
}
#[tokio::test(start_paused = true)]
async fn retry_replaces_old_job_and_late_work_cannot_replace_new_attempt() {
    let platform = Fake::new();
    let engine = Commissioning::new(platform.clone());
    let old = start(&engine).await;
    engine.confirm("home", &old).await.unwrap();
    platform.entered.notified().await;
    let new = start(&engine).await;
    engine.confirm("home", &new).await.unwrap();
    platform.entered.notified().await;
    tokio::time::advance(Duration::from_secs(65)).await;
    tokio::task::yield_now().await;
    assert!(engine.status("home", &old).await.is_err());
    assert_eq!(
        engine
            .status("home", &new)
            .await
            .unwrap()
            .pairing
            .unwrap()
            .session_id,
        new
    );
}
#[tokio::test]
async fn replacement_login_cannot_confirm_or_read_another_accounts_attempt() {
    let platform = Fake::new();
    let engine = Commissioning::new(platform.clone());
    let pair = start(&engine).await;
    *platform.context.lock().unwrap() = "login-two".into();
    assert!(engine.confirm("home", &pair).await.is_err());
    assert!(engine.cancel("home", &pair).await.is_err());
    assert_eq!(platform.setup.load(SeqCst), 0);
}
#[tokio::test]
async fn tls_peer_must_match_signed_device_proof_before_pairing_or_cloud() {
    let platform = Fake::new();
    platform.wrong_peer.store(true, SeqCst);
    let engine = Commissioning::new(platform.clone());
    assert!(engine
        .start(
            "home",
            api::HubSetupStartRequest {
                host_id: "host".into(),
                endpoint: None
            }
        )
        .await
        .is_err());
    assert_eq!(platform.count.load(SeqCst), 0);
}
#[tokio::test(start_paused = true)]
async fn expiry_does_not_extend_on_status_polls_and_cloud_failure_allows_fresh_retry() {
    let platform = Fake::new();
    let engine = Commissioning::new(platform.clone());
    platform.fail.store(true, SeqCst);
    let pair = start(&engine).await;
    engine.confirm("home", &pair).await.unwrap();
    tokio::task::yield_now().await;
    assert_eq!(
        engine
            .status("home", &pair)
            .await
            .unwrap()
            .pairing
            .unwrap()
            .state,
        "FAILED"
    );
    platform.fail.store(false, SeqCst);
    let pair = start(&engine).await;
    engine.confirm("home", &pair).await.unwrap();
    platform.entered.notified().await;
    tokio::time::advance(Duration::from_secs(601)).await;
    tokio::task::yield_now().await;
    assert_eq!(
        engine
            .status("home", &pair)
            .await
            .unwrap()
            .pairing
            .unwrap()
            .state,
        "FAILED"
    );
}

#[tokio::test(start_paused = true)]
async fn committed_hub_reconciles_a_lost_response_and_repeated_status_is_idempotent() {
    let platform = Fake::new();
    platform.lost_response.store(true, SeqCst);
    let engine = Commissioning::new(platform.clone());
    let pair = start(&engine).await;
    engine.confirm("home", &pair).await.unwrap();
    platform.entered.notified().await;
    platform.ready.store(true, SeqCst);
    assert!(engine.status("home", &pair).await.unwrap().complete);
    tokio::time::advance(Duration::from_secs(65)).await;
    tokio::task::yield_now().await;
    assert!(engine.status("home", &pair).await.unwrap().complete);
    assert_eq!(platform.connected.load(SeqCst), 1);
    assert!(engine.view("home").await.unwrap().pairing.is_none());
}
#[tokio::test]
async fn signed_proof_rejects_changed_host_space_tls_key_nonce_and_signature() {
    let platform = Fake::new();
    let candidate = Fake::candidate();
    let response = platform
        .lan(
            "login-one",
            &candidate,
            "https://192.168.1.2",
            None,
            "/commission/challenge",
            json!({"nonce":"nonce"}),
            READ_TIMEOUT,
        )
        .await
        .unwrap();
    assert!(proof::verify(&response.body, &response.peer_key, &candidate, "nonce").is_ok());
    for (field, value) in [
        ("hostId", json!("other")),
        ("nonce", json!("other")),
        ("signature", json!(STANDARD.encode([0u8; 64]))),
        ("hostTlsSignature", json!(STANDARD.encode([0u8; 64]))),
        ("hostPublicKey", json!({"x":"other","y":"y"})),
        (
            "runtimeSpacePolicy",
            json!({"mode":"single","space":{"tenantId":"tenant","scopeId":"other"}}),
        ),
    ] {
        let mut changed = response.body.clone();
        changed[field] = value;
        assert!(
            proof::verify(&changed, &response.peer_key, &candidate, "nonce").is_err(),
            "{field}"
        );
    }
}

#[tokio::test(start_paused = true)]
async fn product_setup_uses_existing_hub_or_verifies_its_bound_host_before_pairing() {
    let platform = Fake::new();
    let engine = Commissioning::new(platform.clone());
    let mut wrong = Fake::peer();
    wrong.x = "wrong".into();
    assert!(engine.setup_product("home", "host", &wrong).await.is_err());
    assert_eq!(platform.count.load(SeqCst), 0);
    platform.ready.store(true, SeqCst);
    engine
        .setup_product("home", "host", &Fake::peer())
        .await
        .unwrap();
    assert_eq!(platform.count.load(SeqCst), 0);
    assert_eq!(platform.setup.load(SeqCst), 0);
}

#[tokio::test(start_paused = true)]
async fn product_setup_waits_for_cloud_commit_before_credential_issuance() {
    let platform = Fake::new();
    let engine = Commissioning::new(platform.clone());
    let task =
        tokio::spawn(async move { engine.setup_product("home", "host", &Fake::peer()).await });
    platform.entered.notified().await;
    tokio::time::advance(Duration::from_secs(65)).await;
    tokio::task::yield_now().await;
    assert!(!task.is_finished());
    platform.ready.store(true, SeqCst);
    task.await.unwrap().unwrap();
    assert_eq!(platform.count.load(SeqCst), 1);
    assert_eq!(platform.setup.load(SeqCst), 1);
}

fn finalizing() -> api::HubOperation {
    serde_json::from_value(json!({"kind":"SETUP","state":"FAILED","phase":"FINALIZING",
        "hubId":"hub-one","hostId":"host","startedAt":1,
        "failure":{"reason":"Finalization interrupted"}}))
    .unwrap()
}

#[tokio::test(start_paused = true)]
async fn committed_retry_skips_lan_and_pairing_even_when_hub_is_already_ready() {
    let platform = Fake::new();
    platform.ready.store(true, SeqCst);
    platform.offline.store(true, SeqCst);
    *platform.operation.lock().unwrap() = Some(finalizing());
    *platform.cloud_delay.lock().unwrap() = Duration::from_secs(20);
    let engine = Commissioning::new(platform.clone());
    let result = engine
        .start(
            "home",
            api::HubSetupStartRequest {
                host_id: "host".into(),
                endpoint: None,
            },
        )
        .await
        .unwrap();
    assert!(
        matches!(result, api::HubSetupStartResponse::Completed { hub_id, .. } if hub_id == "hub-one")
    );
    assert_eq!(platform.count.load(SeqCst), 0);
    assert_eq!(platform.setup.load(SeqCst), 0);
    assert!(engine.view("home").await.unwrap().operation.is_none());
    // Response loss is retried against the same ready Hub without issuing credentials.
    let again = engine
        .start(
            "home",
            api::HubSetupStartRequest {
                host_id: "host".into(),
                endpoint: None,
            },
        )
        .await
        .unwrap();
    assert!(matches!(
        again,
        api::HubSetupStartResponse::Completed { .. }
    ));
}

#[tokio::test(start_paused = true)]
async fn committed_retry_timeout_or_error_keeps_the_saved_operation_retryable() {
    let platform = Fake::new();
    platform.ready.store(true, SeqCst);
    platform.offline.store(true, SeqCst);
    *platform.operation.lock().unwrap() = Some(finalizing());
    *platform.cloud_delay.lock().unwrap() = Duration::from_secs(61);
    let engine = Commissioning::new(platform.clone());
    assert!(engine
        .start(
            "home",
            api::HubSetupStartRequest {
                host_id: "host".into(),
                endpoint: None
            }
        )
        .await
        .is_err());
    assert!(engine.view("home").await.unwrap().operation.is_some());
    assert_eq!(platform.count.load(SeqCst), 0);
    *platform.cloud_delay.lock().unwrap() = Duration::ZERO;
    platform.fail.store(true, SeqCst);
    assert!(engine
        .start(
            "home",
            api::HubSetupStartRequest {
                host_id: "host".into(),
                endpoint: None
            }
        )
        .await
        .is_err());
    assert!(engine.view("home").await.unwrap().operation.is_some());
}

#[tokio::test]
async fn committed_retry_rejects_a_different_host_without_cloud_or_lan_work() {
    let platform = Fake::new();
    platform.ready.store(true, SeqCst);
    *platform.operation.lock().unwrap() = Some(finalizing());
    let engine = Commissioning::new(platform.clone());
    assert!(engine
        .start(
            "home",
            api::HubSetupStartRequest {
                host_id: "other".into(),
                endpoint: None
            }
        )
        .await
        .is_err());
    assert_eq!(platform.cloud_count.load(SeqCst), 0);
    assert_eq!(platform.count.load(SeqCst), 0);
}

#[tokio::test(start_paused = true)]
async fn product_setup_continues_committed_work_without_waiting_for_lan() {
    let platform = Fake::new();
    platform.offline.store(true, SeqCst);
    *platform.operation.lock().unwrap() = Some(finalizing());
    let engine = Commissioning::new(platform.clone());
    engine
        .setup_product("home", "host", &Fake::peer())
        .await
        .unwrap();
    assert_eq!(platform.cloud_count.load(SeqCst), 1);
    assert_eq!(platform.count.load(SeqCst), 0);
}

#[tokio::test]
async fn running_finalization_returns_a_cloud_operation_without_a_pairing_session() {
    let platform = Fake::new();
    platform.offline.store(true, SeqCst);
    *platform.operation.lock().unwrap() = Some(finalizing());
    *platform.cloud_outcome.lock().unwrap() = "RUNNING".into();
    let engine = Commissioning::new(platform.clone());
    let result = engine
        .start(
            "home",
            api::HubSetupStartRequest {
                host_id: "host".into(),
                endpoint: None,
            },
        )
        .await
        .unwrap();
    assert!(matches!(result, api::HubSetupStartResponse::Running { .. }));
    assert!(engine.state.lock().unwrap().sessions.is_empty());
}

#[tokio::test(start_paused = true)]
async fn committed_completion_retires_the_old_local_pairing_failure() {
    let platform = Fake::new();
    let engine = Commissioning::new(platform.clone());
    let session = start(&engine).await;
    platform.fail.store(true, SeqCst);
    engine.confirm("home", &session).await.unwrap();
    tokio::task::yield_now().await;
    let key = ("login-one".to_owned(), "home".to_owned());
    engine.update(&key, &session, |operation| {
        operation.view.state = "FAILED".into();
        operation.view.hub_id = Some("hub-one".into());
    });
    platform.fail.store(false, SeqCst);
    *platform.operation.lock().unwrap() = Some(finalizing());
    let result = engine
        .start(
            "home",
            api::HubSetupStartRequest {
                host_id: "host".into(),
                endpoint: None,
            },
        )
        .await
        .unwrap();
    assert!(matches!(
        result,
        api::HubSetupStartResponse::Completed { .. }
    ));
    assert!(engine.view("home").await.unwrap().pairing.is_none());
    assert!(engine.status("home", &session).await.unwrap().complete);
    assert_eq!(platform.connected.load(SeqCst), 1);
}

#[tokio::test(start_paused = true)]
async fn running_continuation_is_reconciled_by_later_cloud_state_reads() {
    let platform = Fake::new();
    let engine = Commissioning::new(platform.clone());
    let session = start(&engine).await;
    platform.fail.store(true, SeqCst);
    engine.confirm("home", &session).await.unwrap();
    tokio::task::yield_now().await;
    engine.update(
        &("login-one".into(), "home".into()),
        &session,
        |operation| {
            operation.view.state = "FAILED".into();
            operation.view.hub_id = Some("hub-one".into());
        },
    );
    platform.fail.store(false, SeqCst);
    platform.offline.store(true, SeqCst);
    *platform.operation.lock().unwrap() = Some(finalizing());
    *platform.cloud_outcome.lock().unwrap() = "RUNNING".into();
    assert!(matches!(
        engine
            .start(
                "home",
                api::HubSetupStartRequest {
                    host_id: "host".into(),
                    endpoint: None,
                }
            )
            .await
            .unwrap(),
        api::HubSetupStartResponse::Running { .. }
    ));
    assert!(engine.view("home").await.unwrap().operation.is_some());
    *platform.operation.lock().unwrap() = None;
    platform.ready.store(true, SeqCst);
    assert!(engine.view("home").await.unwrap().pairing.is_none());
    assert!(engine.view("home").await.unwrap().pairing.is_none());
    assert_eq!(platform.connected.load(SeqCst), 1);
    assert!(engine.status("home", &session).await.unwrap().complete);
}
