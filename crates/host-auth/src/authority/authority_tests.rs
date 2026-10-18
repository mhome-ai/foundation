use super::keys::KeySnapshot;
use super::*;
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use core_api::{
    host::auth::{SigningKey as CloudKey, SigningKeySet},
    runtime_space::SpaceIdentity,
};
use rsa::{pkcs8::EncodePrivateKey, traits::PublicKeyParts, RsaPrivateKey};
use std::collections::BTreeSet;

struct Fixture {
    root: tempfile::TempDir,
    host: Arc<Authority>,
    issuer: RsaPrivateKey,
}

#[tokio::test]
async fn local_operator_can_manage_admins_without_impersonating_a_user_or_removing_the_last_admin()
{
    let f = Fixture::admin().await;
    let actor = ManagementActor::LocalUnix { uid: 123 };
    let current = f.host.administrators(&actor).unwrap();
    assert!(current["currentUserId"].is_null());
    let pending = f
        .host
        .administrator_application("administratorRequest", f.applicant("bob"))
        .await
        .unwrap();
    f.host
        .decide_administrator(&actor, pending["requestId"].as_str().unwrap().into(), true)
        .await
        .unwrap();
    f.host.remove_administrator(&actor, "alice").await.unwrap();
    assert!(f.host.remove_administrator(&actor, "bob").await.is_err());
    assert!(f
        .host
        .administrators(&ManagementActor::CloudUser("alice".into()))
        .is_err());
}
impl Fixture {
    async fn new(mode: HostMode) -> Self {
        let root = tempfile::tempdir().unwrap();
        let issuer = RsaPrivateKey::new(&mut rand::thread_rng(), 2048).unwrap();
        let keys = KeySnapshot {
            key_set: Some(SigningKeySet {
                version: 1,
                keys: vec![CloudKey {
                    kid: "cloud".into(),
                    status: SigningKeyStatus::Active,
                    n: URL_SAFE_NO_PAD.encode(issuer.n().to_bytes_be()),
                    e: URL_SAFE_NO_PAD.encode(issuer.e().to_bytes_be()),
                }],
            }),
            checked_at: unix_seconds(),
        };
        crate::secure_file::write_private_export(
            &root.path().join("signing-keys.json"),
            &serde_json::to_vec(&keys).unwrap(),
        )
        .unwrap();
        let host = if mode == HostMode::SpaceDep {
            Authority::open_dependent(
                root.path().into(),
                "host".into(),
                RuntimeEnv::Prod,
                "https://127.0.0.1:1/api/v1",
                root.path().join("device-binding.json"),
            )
            .await
        } else {
            Authority::open(
                root.path().into(),
                "host".into(),
                mode,
                RuntimeEnv::Prod,
                "https://127.0.0.1:1/api/v1",
            )
            .await
        }
        .unwrap();
        Self { root, host, issuer }
    }
    fn sign(&self, kind: &str, claims: &impl Serialize) -> String {
        let mut header = jsonwebtoken::Header::new(jsonwebtoken::Algorithm::RS256);
        header.kid = Some("cloud".into());
        header.typ = Some(kind.into());
        let pem = self
            .issuer
            .to_pkcs8_pem(rsa::pkcs8::LineEnding::LF)
            .unwrap();
        jsonwebtoken::encode(
            &header,
            claims,
            &jsonwebtoken::EncodingKey::from_rsa_pem(pem.as_bytes()).unwrap(),
        )
        .unwrap()
    }
    fn binding(&self, user: &str, space: Option<SpaceIdentity>) -> AccessBinding {
        let s = self.host.status().unwrap();
        AccessBinding {
            host_id: s.host_id,
            host_key_id: s.host_key_id,
            env: s.env,
            generation: s.generation,
            user_id: user.into(),
            space,
        }
    }
    async fn claim(&self, user: &str, space: Option<SpaceIdentity>) -> ClaimRequest {
        let secret = random_id();
        let hash = access::exchange_digest(&secret).unwrap();
        let challenge = self
            .host
            .challenge(
                "Host".into(),
                ChallengeRequest {
                    probe_id: random_id(),
                    exchange_key_hash: Some(hash.clone()),
                },
            )
            .await
            .unwrap();
        let now = unix_seconds();
        let proof = self.sign(
            CLAIM_TYPE,
            &ClaimClaims {
                iss: CLOUD_ISSUER.into(),
                aud: ACCESS_AUDIENCE.into(),
                sub: user.into(),
                jti: random_id(),
                iat: now,
                exp: now + 300,
                binding: self.binding(user, space),
                challenge,
                exchange_key_hash: hash,
                display_name: user.into(),
                email: format!("{user}@example.test"),
            },
        );
        ClaimRequest {
            env: RuntimeEnv::Prod,
            proof,
            exchange_key: secret,
            claim_token: None,
        }
    }
    fn access(&self, user: &str, space: Option<SpaceIdentity>) -> AccessClaims {
        let now = unix_seconds();
        AccessClaims {
            iss: CLOUD_ISSUER.into(),
            aud: ACCESS_AUDIENCE.into(),
            sub: user.into(),
            jti: random_id(),
            iat: now,
            exp: now + 300,
            binding: self.binding(user, space),
            permissions: BTreeSet::from([
                Permission::Read,
                Permission::ManageRuntime,
                Permission::ManageAdministrators,
                Permission::Reset,
            ]),
        }
    }
}
#[tokio::test]
async fn first_claim_is_durable_idempotent_and_requires_the_direct_exchange_secret() {
    let f = Fixture::new(HostMode::Admin).await;
    let request = f.claim("alice", None).await;
    let mut wrong = request.clone();
    wrong.exchange_key = random_id();
    assert!(f.host.complete_claim(wrong).await.is_err());
    assert!(f.host.status().unwrap().claim.is_none());
    let receipt = f.host.complete_claim(request.clone()).await.unwrap();
    assert_eq!(f.host.complete_claim(request).await.unwrap(), receipt);
    let read = f.sign(ACCESS_TYPE, &f.access("alice", None));
    assert_eq!(
        f.host.authenticate(&read, Permission::Read).await.unwrap(),
        "alice"
    );
    let second = f.claim("bob", None).await;
    assert!(f.host.complete_claim(second).await.is_err());
    let reopened = Authority::open(
        f.root.path().into(),
        "host".into(),
        HostMode::Admin,
        RuntimeEnv::Prod,
        "https://127.0.0.1:1/api/v1",
    )
    .await
    .unwrap();
    assert_eq!(f.host.status().unwrap(), reopened.status().unwrap());
    assert!(reopened.authenticate(&read, Permission::Read).await.is_ok());
    assert!(Authority::open(
        f.root.path().into(),
        "host".into(),
        HostMode::SpaceSelf,
        RuntimeEnv::Prod,
        "https://127.0.0.1:1/api/v1"
    )
    .await
    .is_err());
}
#[tokio::test]
async fn space_host_uses_current_cloud_owner_without_persisting_an_owner_account() {
    let f = Fixture::new(HostMode::SpaceSelf).await;
    assert!(f.host.runtime_policy().unwrap().is_none());
    let space = SpaceIdentity {
        tenant_id: "tenant".into(),
        scope_id: "home".into(),
    };
    f.host
        .complete_claim(f.claim("old-owner", Some(space.clone())).await)
        .await
        .unwrap();
    assert!(f
        .host
        .runtime_policy()
        .unwrap()
        .unwrap()
        .admit("tenant", "home")
        .is_ok());
    assert!(f
        .host
        .runtime_policy()
        .unwrap()
        .unwrap()
        .admit("tenant", "other")
        .is_err());
    let fresh_owner = f.sign(ACCESS_TYPE, &f.access("new-owner", Some(space)));
    assert!(f
        .host
        .authenticate(&fresh_owner, Permission::ManageRuntime)
        .await
        .is_ok());
    assert!(f
        .host
        .authenticate(&fresh_owner, Permission::ManageAdministrators)
        .await
        .is_err());
    assert!(f
        .host
        .administrators(&ManagementActor::CloudUser("new-owner".into()))
        .is_err());
    assert!(f
        .host
        .administrator_application("administratorRequest", f.applicant("bob"))
        .await
        .is_err());
    assert!(f
        .host
        .authenticate(
            &f.sign(ACCESS_TYPE, &f.access("owner", None)),
            Permission::Read
        )
        .await
        .is_err());
}
#[tokio::test]
async fn signature_type_target_expiry_and_current_acl_are_enforced() {
    let f = Fixture::new(HostMode::Admin).await;
    f.host
        .complete_claim(f.claim("alice", None).await)
        .await
        .unwrap();
    let claims = f.access("alice", None);
    for change in 0..6 {
        let mut bad = claims.clone();
        match change {
            0 => bad.exp = bad.iat + 301,
            1 => {
                bad.iat -= 600;
                bad.exp -= 600;
            }
            2 => bad.binding.host_key_id = random_id(),
            3 => bad.binding.generation = random_id(),
            4 => bad.sub = "bob".into(),
            _ => bad.binding.env = RuntimeEnv::Dev,
        }
        assert!(
            f.host
                .authenticate(&f.sign(ACCESS_TYPE, &bad), Permission::Read)
                .await
                .is_err(),
            "case {change}"
        );
    }
    assert!(f
        .host
        .authenticate(&f.sign(CLAIM_TYPE, &claims), Permission::Read)
        .await
        .is_err());
    let proof = f.applicant("bob");
    let request = f
        .host
        .administrator_application("administratorRequest", proof)
        .await
        .unwrap();
    f.host
        .decide_administrator(
            &ManagementActor::CloudUser("alice".into()),
            request["requestId"].as_str().unwrap().into(),
            true,
        )
        .await
        .unwrap();
    let old = f.sign(ACCESS_TYPE, &claims);
    f.host
        .remove_administrator(&ManagementActor::CloudUser("bob".into()), "alice")
        .await
        .unwrap();
    assert!(f.host.authenticate(&old, Permission::Read).await.is_err());
    assert!(f
        .host
        .remove_administrator(&ManagementActor::CloudUser("bob".into()), "bob")
        .await
        .is_err());
}

impl Fixture {
    fn applicant(&self, user: &str) -> String {
        let now = unix_seconds();
        self.sign(
            ADMIN_APPLICANT_TYPE,
            &AdministratorApplicantClaims {
                iss: CLOUD_ISSUER.into(),
                aud: ACCESS_AUDIENCE.into(),
                sub: user.into(),
                jti: random_id(),
                iat: now,
                exp: now + 600,
                binding: self.binding(user, None),
                display_name: format!("Name {user}"),
                email: format!("{user}@example.test"),
            },
        )
    }
    async fn admin() -> Self {
        let f = Self::new(HostMode::Admin).await;
        f.host
            .complete_claim(f.claim("alice", None).await)
            .await
            .unwrap();
        f
    }
    async fn add(&self, user: &str) -> serde_json::Value {
        let request = self
            .host
            .administrator_application("administratorRequest", self.applicant(user))
            .await
            .unwrap();
        self.host
            .decide_administrator(
                &ManagementActor::CloudUser("alice".into()),
                request["requestId"].as_str().unwrap().into(),
                true,
            )
            .await
            .unwrap()
    }
}
#[tokio::test]
async fn applicant_identity_is_not_management_and_only_current_admin_can_approve() {
    let f = Fixture::admin().await;
    let proof = f.applicant("bob");
    for permission in [
        Permission::Read,
        Permission::ManageRuntime,
        Permission::ManageAdministrators,
        Permission::Reset,
    ] {
        assert!(f.host.authenticate(&proof, permission).await.is_err());
    }
    let request = f
        .host
        .administrator_application("administratorRequest", proof.clone())
        .await
        .unwrap();
    assert_eq!(request["status"], "pending");
    assert!(f
        .host
        .administrators(&ManagementActor::CloudUser("bob".into()))
        .is_err());
    let id = request["requestId"].as_str().unwrap().to_owned();
    assert!(f
        .host
        .decide_administrator(&ManagementActor::CloudUser("bob".into()), id.clone(), true)
        .await
        .is_err());
    assert!(f
        .host
        .remove_administrator(&ManagementActor::CloudUser("bob".into()), "alice")
        .await
        .is_err());
    let approved = f
        .host
        .decide_administrator(
            &ManagementActor::CloudUser("alice".into()),
            id.clone(),
            true,
        )
        .await
        .unwrap();
    assert_eq!(approved["status"], "approved");
    let receipt: AuthorizationReceipt = verify_jws(
        f.host.key.verifying_key(),
        RECEIPT_TYPE,
        approved["receipt"].as_str().unwrap(),
    )
    .unwrap();
    assert_eq!(receipt.proof, proof);
    assert_eq!(receipt.user_id, "bob");
    assert_eq!(
        f.host
            .decide_administrator(
                &ManagementActor::CloudUser("alice".into()),
                id.clone(),
                true
            )
            .await
            .unwrap(),
        approved
    );
    assert!(f
        .host
        .decide_administrator(&ManagementActor::CloudUser("alice".into()), id, false)
        .await
        .is_err());
    assert!(f
        .host
        .authenticate(
            &f.sign(ACCESS_TYPE, &f.access("bob", None)),
            Permission::Read
        )
        .await
        .is_ok());
}
#[tokio::test]
async fn repeated_requests_keep_deadline_and_applicants_see_only_their_own_request() {
    let f = Fixture::admin().await;
    let first = f
        .host
        .administrator_application("administratorRequest", f.applicant("bob"))
        .await
        .unwrap();
    assert_eq!(
        f.host
            .administrator_application("administratorRequest", f.applicant("bob"))
            .await
            .unwrap(),
        first
    );
    assert_eq!(
        f.host
            .administrator_application("administratorRequestStatus", f.applicant("charlie"))
            .await
            .unwrap()["status"],
        "none"
    );
    assert_eq!(
        f.host
            .administrators(&ManagementActor::CloudUser("alice".into()))
            .unwrap()["requests"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
    let bytes = std::fs::read(f.root.path().join("identity.json")).unwrap();
    assert_eq!(
        f.host
            .administrator_application("administratorRequestStatus", f.applicant("bob"))
            .await
            .unwrap(),
        first
    );
    assert_eq!(
        std::fs::read(f.root.path().join("identity.json")).unwrap(),
        bytes
    );
    let reopened = Authority::open(
        f.root.path().into(),
        "host".into(),
        HostMode::Admin,
        RuntimeEnv::Prod,
        "https://127.0.0.1:1/api/v1",
    )
    .await
    .unwrap();
    assert_eq!(
        reopened
            .administrators(&ManagementActor::CloudUser("alice".into()))
            .unwrap()["requests"][0],
        first
    );
}
#[tokio::test]
async fn reject_cancel_expire_and_retry_do_not_grant_or_resurrect_access() {
    let f = Fixture::admin().await;
    let proof = f.applicant("bob");
    let first = f
        .host
        .administrator_application("administratorRequest", proof.clone())
        .await
        .unwrap();
    let id = first["requestId"].as_str().unwrap().to_owned();
    assert_eq!(
        f.host
            .decide_administrator(
                &ManagementActor::CloudUser("alice".into()),
                id.clone(),
                false
            )
            .await
            .unwrap()["status"],
        "rejected"
    );
    assert_eq!(
        f.host
            .administrator_application("administratorRequest", proof)
            .await
            .unwrap()["status"],
        "rejected"
    );
    let proof = f.applicant("bob");
    let second = f
        .host
        .administrator_application("administratorRequest", proof.clone())
        .await
        .unwrap();
    assert_ne!(first["requestId"], second["requestId"]);
    assert_eq!(
        f.host
            .administrator_application("administratorRequestCancel", proof.clone())
            .await
            .unwrap()["status"],
        "cancelled"
    );
    assert!(f
        .host
        .decide_administrator(
            &ManagementActor::CloudUser("alice".into()),
            second["requestId"].as_str().unwrap().into(),
            true
        )
        .await
        .is_err());
    assert_eq!(
        f.host
            .administrator_application("administratorRequest", proof)
            .await
            .unwrap()["status"],
        "cancelled"
    );
    let fresh = f
        .host
        .administrator_application("administratorRequest", f.applicant("bob"))
        .await
        .unwrap();
    f.host
        .mutate(|state| {
            state
                .administrator_requests
                .get_mut("bob")
                .unwrap()
                .expires_at = unix_seconds();
            Ok(())
        })
        .await
        .unwrap();
    assert!(f
        .host
        .decide_administrator(
            &ManagementActor::CloudUser("alice".into()),
            fresh["requestId"].as_str().unwrap().into(),
            true
        )
        .await
        .is_err());
    assert_eq!(
        f.host
            .administrator_application("administratorRequestStatus", f.applicant("bob"))
            .await
            .unwrap()["status"],
        "none"
    );
    assert_eq!(
        f.host
            .administrator_application("administratorRequest", f.applicant("bob"))
            .await
            .unwrap()["status"],
        "pending"
    );
}
#[tokio::test]
async fn concurrent_decisions_have_one_result_and_concurrent_removals_keep_an_admin() {
    let f = Fixture::admin().await;
    f.add("bob").await;
    let request = f
        .host
        .administrator_application("administratorRequest", f.applicant("charlie"))
        .await
        .unwrap();
    let id = request["requestId"].as_str().unwrap().to_owned();
    let alice = ManagementActor::CloudUser("alice".into());
    let bob = ManagementActor::CloudUser("bob".into());
    let (a, b) = tokio::join!(
        f.host.decide_administrator(&alice, id.clone(), true),
        f.host.decide_administrator(&bob, id, false)
    );
    assert_ne!(a.is_ok(), b.is_ok());
    if a.is_ok() {
        f.host
            .remove_administrator(&ManagementActor::CloudUser("alice".into()), "charlie")
            .await
            .unwrap();
    }
    let (a, b) = tokio::join!(
        f.host.remove_administrator(&alice, "bob"),
        f.host.remove_administrator(&bob, "alice")
    );
    assert_ne!(a.is_ok(), b.is_ok());
    let Some(Claim::Admin { user_ids }) = f.host.status().unwrap().claim else {
        panic!()
    };
    assert_eq!(user_ids.len(), 1);
}
#[tokio::test]
async fn removed_admin_cannot_approve_and_old_approved_request_cannot_readd_them() {
    let f = Fixture::admin().await;
    let proof = f.applicant("bob");
    let request = f
        .host
        .administrator_application("administratorRequest", proof.clone())
        .await
        .unwrap();
    let id = request["requestId"].as_str().unwrap().to_owned();
    f.host
        .decide_administrator(
            &ManagementActor::CloudUser("alice".into()),
            id.clone(),
            true,
        )
        .await
        .unwrap();
    f.host
        .remove_administrator(&ManagementActor::CloudUser("alice".into()), "bob")
        .await
        .unwrap();
    assert!(f
        .host
        .decide_administrator(&ManagementActor::CloudUser("bob".into()), id.clone(), true)
        .await
        .is_err());
    assert!(f
        .host
        .decide_administrator(&ManagementActor::CloudUser("alice".into()), id, true)
        .await
        .is_err());
    let result = f
        .host
        .administrator_application("administratorRequest", proof)
        .await
        .unwrap();
    assert_eq!(result["status"], "removed");
    assert!(result["receipt"].is_null());
    assert_eq!(
        f.host
            .administrator_application("administratorRequest", f.applicant("bob"))
            .await
            .unwrap()["status"],
        "pending"
    );
}

#[tokio::test]
async fn dependent_host_reads_client_binding_and_reset_intent_immediately_invalidates_tokens() {
    use crate::device_binding::{DeviceComponentIdentity, Writer};
    let f = Fixture::new(HostMode::SpaceDep).await;
    assert!(f.host.status().is_err());
    assert!(f.host.runtime_policy().is_err());
    let path = f.root.path().join("device-binding.json");
    let writer = Writer::new(path.clone()).unwrap();
    let client = DeviceComponentIdentity {
        id: "pod".into(),
        key_id: "pod-key".into(),
    };
    let initial = writer
        .initialize(
            RuntimeEnv::Prod,
            client.clone(),
            Some(f.host.component_identity()),
        )
        .unwrap();
    assert!(f.host.status().unwrap().claim.is_none());
    assert!(f.host.runtime_policy().unwrap().is_none());
    let space = SpaceIdentity {
        tenant_id: "tenant".into(),
        scope_id: "home".into(),
    };
    let direct = f.claim("owner", Some(space.clone())).await;
    assert!(f.host.complete_claim(direct.clone()).await.is_err());
    assert!(f.host.device_claim_receipt(&direct).await.is_err());
    let approved = f.host.verify_device_claim(&direct).await.unwrap();
    assert_eq!(approved.space, Some(space.clone()));
    let mut wrong_secret = direct.clone();
    wrong_secret.exchange_key = random_id();
    assert!(f.host.verify_device_claim(&wrong_secret).await.is_err());
    writer
        .bind_verified(&initial, approved.space.unwrap())
        .unwrap();
    let receipt = f.host.device_claim_receipt(&direct).await.unwrap();
    let decoded: AuthorizationReceipt =
        verify_jws(f.host.key.verifying_key(), RECEIPT_TYPE, &receipt).unwrap();
    assert_eq!(decoded.proof, direct.proof);
    assert_eq!(decoded.status, f.host.status().unwrap());
    assert!(f
        .host
        .runtime_policy()
        .unwrap()
        .unwrap()
        .admit("tenant", "home")
        .is_ok());
    assert!(f
        .host
        .runtime_policy()
        .unwrap()
        .unwrap()
        .admit("tenant", "elsewhere")
        .is_err());
    let token = f.sign(ACCESS_TYPE, &f.access("owner", Some(space)));
    assert!(f
        .host
        .authenticate(&token, Permission::ManageRuntime)
        .await
        .is_ok());
    assert!(f
        .host
        .authenticate(&token, Permission::ManageAdministrators)
        .await
        .is_err());
    let saved: serde_json::Value =
        serde_json::from_slice(&std::fs::read(f.root.path().join("identity.json")).unwrap())
            .unwrap();
    assert_eq!(
        saved["authorization"],
        serde_json::json!({"source":"client","env":"prod"})
    );
    writer
        .reader()
        .request_reset(RuntimeEnv::Prod, f.host.component_identity())
        .unwrap();
    assert!(f.host.authenticate(&token, Permission::Read).await.is_err());
    assert!(f.host.verify_device_claim(&direct).await.is_err());
    assert!(f.host.device_claim_receipt(&direct).await.is_err());
    assert!(f.host.runtime_policy().is_err());
    std::fs::remove_file(path).unwrap();
    assert!(f.host.status().is_err());
    assert!(f.host.authenticate(&token, Permission::Read).await.is_err());
}
