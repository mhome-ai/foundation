pub mod http;
pub mod keys;

// Host-local claim and administrator ACL. Cloud access tokens are never stored.
use self::keys::{unix_seconds, KeyCache};
use crate::secure_file::SecureFile;
use crate::signatures::*;
use crate::{access, policy::HostPolicy};
use anyhow::{ensure, Context, Result};
use core_api::{
    host::{
        access::*,
        auth::{PublicKey, SigningKeyStatus},
    },
    runtime_space::RuntimeSpacePolicy,
    RuntimeEnv,
};
use p256::ecdsa::SigningKey;
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    path::PathBuf,
    sync::{Arc, RwLock},
};
use tokio::sync::Mutex;

#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct AuthorityState {
    host_id: String,
    private_key: String,
    authorization: AuthorizationSource,
    administrator_requests: BTreeMap<String, AdministratorRequest>,
    administrator_profiles: BTreeMap<String, AdministratorProfile>,
    completed_authorizations: BTreeMap<String, Completed>,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(tag = "source", rename_all = "camelCase", deny_unknown_fields)]
enum AuthorizationSource {
    Host { policy: HostPolicy },
    Client { env: RuntimeEnv },
}
impl AuthorityState {
    fn policy(&self) -> Result<&HostPolicy> {
        match &self.authorization {
            AuthorizationSource::Host { policy } => Ok(policy),
            AuthorizationSource::Client { .. } => {
                anyhow::bail!("dependent Host binding is owned by its Client")
            }
        }
    }
    fn policy_mut(&mut self) -> Result<&mut HostPolicy> {
        match &mut self.authorization {
            AuthorizationSource::Host { policy } => Ok(policy),
            AuthorizationSource::Client { .. } => {
                anyhow::bail!("dependent Host binding is owned by its Client")
            }
        }
    }
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Completed {
    expires_at: u64,
    proof_digest: String,
    receipt: String,
    exchange_hash: Option<String>,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct AdministratorProfile {
    display_name: String,
    email: String,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct AdministratorRequest {
    request_id: String,
    user_id: String,
    profile: AdministratorProfile,
    created_at: u64,
    expires_at: u64,
    proof: String,
    status: String,
    receipt: Option<String>,
}
impl AdministratorRequest {
    fn view(&self) -> serde_json::Value {
        serde_json::json!({"requestId":self.request_id,"userId":self.user_id,
            "displayName":self.profile.display_name,"email":self.profile.email,
            "createdAt":self.created_at,"expiresAt":self.expires_at,"status":self.status,"receipt":self.receipt})
    }
}
fn is_administrator(policy: &HostPolicy, user: &str) -> bool {
    matches!(policy.claim(), Some(Claim::Admin { user_ids }) if user_ids.contains(user))
}
/// Created by a trusted transport adapter after cloud-token or OS peer validation.
/// Deliberately not deserializable: a network request cannot choose its authority.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ManagementActor {
    CloudUser(String),
    LocalUnix { uid: u32 },
    LocalWindows { process_id: u32 },
}
impl ManagementActor {
    pub fn user_id(&self) -> Option<&str> {
        match self {
            Self::CloudUser(user) => Some(user),
            Self::LocalUnix { .. } | Self::LocalWindows { .. } => None,
        }
    }
}
fn require_administrator(policy: &HostPolicy, actor: &ManagementActor) -> Result<()> {
    policy.validate(policy.mode())?;
    ensure!(
        matches!(policy.claim(), Some(Claim::Admin { .. }))
            && actor
                .user_id()
                .is_none_or(|user| is_administrator(policy, user)),
        "Current Host administrator required"
    );
    Ok(())
}

pub struct Authority {
    pub host_id: String,
    key: SigningKey,
    store: SecureFile<Option<AuthorityState>>,
    state: Arc<RwLock<AuthorityState>>,
    mutation: Arc<Mutex<()>>,
    pub keys: Arc<KeyCache>,
    binding: Option<crate::device_binding::Reader>,
}
impl Authority {
    pub async fn open(
        root: PathBuf,
        host_id: String,
        mode: HostMode,
        env: RuntimeEnv,
        api_base: &str,
    ) -> Result<Arc<Self>> {
        ensure!(
            mode != HostMode::SpaceDep,
            "dependent Host requires an explicit Client binding source"
        );
        Self::open_source(root, host_id, mode, env, api_base, None).await
    }
    pub async fn open_dependent(
        root: PathBuf,
        host_id: String,
        env: RuntimeEnv,
        api_base: &str,
        binding: PathBuf,
    ) -> Result<Arc<Self>> {
        Self::open_source(
            root,
            host_id,
            HostMode::SpaceDep,
            env,
            api_base,
            Some(crate::device_binding::Reader::new(binding)),
        )
        .await
    }
    async fn open_source(
        root: PathBuf,
        host_id: String,
        mode: HostMode,
        env: RuntimeEnv,
        api_base: &str,
        binding: Option<crate::device_binding::Reader>,
    ) -> Result<Arc<Self>> {
        ensure!(
            !host_id.is_empty() && host_id != "unknown-host",
            "Host identity unavailable"
        );
        ensure!(
            root.join("identity.json").exists()
                || (!root.join(crate::tls::KEY_FILE).exists()
                    && !root.join(crate::tls::CERT_FILE).exists()),
            "Host identity missing; refusing to replace existing TLS identity"
        );
        let store = SecureFile::<Option<AuthorityState>>::new(root.join("identity.json"))?;
        let exists = root.join("identity.json").exists();
        let load = store.clone();
        let id = host_id.clone();
        let state = tokio::task::spawn_blocking(move || {
            load.update(|saved| {
                if saved.is_none() {
                    ensure!(
                        !exists,
                        "Host identity is empty; explicit reset is required"
                    );
                    *saved = Some(AuthorityState {
                        host_id: id.clone(),
                        private_key: encode_key(&new_key())?,
                        authorization: if mode == HostMode::SpaceDep {
                            AuthorizationSource::Client { env }
                        } else {
                            AuthorizationSource::Host {
                                policy: HostPolicy::unclaimed(id.clone(), env, mode, random_id())?,
                            }
                        },
                        administrator_requests: BTreeMap::new(),
                        administrator_profiles: BTreeMap::new(),
                        completed_authorizations: BTreeMap::new(),
                    });
                }
                let state = saved.as_ref().unwrap();
                ensure!(state.host_id == id, "Host security identity mismatch");
                match &state.authorization {
                    AuthorizationSource::Client { env: saved_env } => ensure!(
                        mode == HostMode::SpaceDep && *saved_env == env,
                        "Host product mode or environment changed"
                    ),
                    AuthorizationSource::Host { policy } => {
                        ensure!(mode != HostMode::SpaceDep, "Host product mode changed");
                        policy.validate(mode)?;
                        ensure!(
                            policy
                                .status(key_id(decode_key(&state.private_key)?.verifying_key()))
                                .env
                                == env,
                            "Host environment changed"
                        );
                    }
                }
                Ok(state.clone())
            })
        })
        .await??;
        let export_root = root.clone();
        let key = decode_key(&state.private_key)?;
        let export_key = key.clone();
        tokio::task::spawn_blocking(move || {
            let (cert, key) = crate::tls::certificate(&export_key)?;
            crate::secure_file::write_private_export(
                &export_root.join(crate::tls::KEY_FILE),
                key.as_bytes(),
            )?;
            crate::secure_file::write_private_export(
                &export_root.join(crate::tls::CERT_FILE),
                cert.as_bytes(),
            )
        })
        .await??;
        let api_base = api_base.to_owned();
        let keys = tokio::task::spawn_blocking(move || {
            KeyCache::new(&api_base, root.join("signing-keys.json"))
        })
        .await??;
        let keys = Arc::new(keys);
        Ok(Arc::new(Self {
            host_id,
            key,
            store,
            state: Arc::new(RwLock::new(state)),
            mutation: Arc::new(Mutex::new(())),
            keys,
            binding,
        }))
    }
    pub fn public_key(&self) -> PublicKey {
        public_key(self.key.verifying_key())
    }
    pub async fn request_device_reset(&self) -> Result<()> {
        let state = self
            .state
            .read()
            .map_err(|_| anyhow::anyhow!("Host state unavailable"))?
            .clone();
        let AuthorizationSource::Client { env } = state.authorization else {
            anyhow::bail!("Host is not dependent");
        };
        let reader = self
            .binding
            .as_ref()
            .context("Client binding missing")?
            .clone();
        let host = self.component_identity();
        tokio::task::spawn_blocking(move || reader.request_reset(env, host)).await?
    }
    pub fn is_dependent(&self) -> bool {
        self.binding.is_some()
    }

    pub fn component_identity(&self) -> core_api::device_binding::DeviceComponentIdentity {
        core_api::device_binding::DeviceComponentIdentity {
            id: self.host_id.clone(),
            key_id: key_id(self.key.verifying_key()),
        }
    }
    fn effective_policy(&self) -> Result<HostPolicy> {
        let state = self
            .state
            .read()
            .map_err(|_| anyhow::anyhow!("Host state unavailable"))?;
        match &state.authorization {
            AuthorizationSource::Host { policy } => Ok(policy.clone()),
            AuthorizationSource::Client { env } => {
                let binding = self
                    .binding
                    .as_ref()
                    .context("Client binding source missing")?
                    .for_host(&self.component_identity(), *env)?;
                Ok(HostPolicy::from_device_binding(&binding)?)
            }
        }
    }
    pub fn status(&self) -> Result<HostStatus> {
        Ok(self
            .effective_policy()?
            .status(key_id(self.key.verifying_key())))
    }
    pub fn runtime_policy(&self) -> Result<Option<RuntimeSpacePolicy>> {
        Ok(self.effective_policy()?.runtime_policy()?)
    }
    // Keep the gate inside the blocking task: request cancellation must not leave
    // a durable ACL write unpublished or let a later mutation overtake it.
    async fn mutate<R: Send + 'static>(
        &self,
        change: impl FnOnce(&mut AuthorityState) -> Result<R> + Send + 'static,
    ) -> Result<R> {
        let gate = self.mutation.clone().lock_owned().await;
        let store = self.store.clone();
        let cached = self.state.clone();
        tokio::task::spawn_blocking(move || {
            let _gate = gate;
            let (state, result) = store.update(|saved| {
                let state = saved.as_mut().context("Host security state missing")?;
                let result = change(state)?;
                Ok((state.clone(), result))
            })?;
            *cached
                .write()
                .map_err(|_| anyhow::anyhow!("Host state unavailable"))? = state;
            Ok(result)
        })
        .await?
    }
    pub async fn challenge(&self, host_name: String, request: ChallengeRequest) -> Result<String> {
        ensure!(
            access::valid_secret(&request.probe_id),
            "invalid Host probe"
        );
        ensure!(
            request
                .exchange_key_hash
                .as_deref()
                .is_none_or(access::valid_secret),
            "invalid claim exchange hash"
        );
        let now = unix_seconds();
        sign_jws(
            &self.key,
            CHALLENGE_TYPE,
            &HostChallenge {
                device_binding: self
                    .binding
                    .as_ref()
                    .map(|reader| reader.read())
                    .transpose()?,
                status: self.status()?,
                host_name,
                host_public_key: self.public_key(),
                probe_id: request.probe_id,
                exchange_key_hash: request.exchange_key_hash,
                iat: now,
                exp: now + ACCESS_TTL_SECONDS,
            },
        )
    }
    async fn cloud_claims<T: DeserializeOwned>(&self, token: &str, purpose: &str) -> Result<T> {
        ensure!(token.len() <= 16384, "cloud token too large");
        let header = jsonwebtoken::decode_header(token)?;
        ensure!(
            header.alg == jsonwebtoken::Algorithm::RS256 && header.typ.as_deref() == Some(purpose),
            "incorrect Host token algorithm or type"
        );
        ensure!(
            header.jku.is_none()
                && header.jwk.is_none()
                && header.x5u.is_none()
                && header.x5c.is_none(),
            "token cannot supply a verification key"
        );
        let kid = header.kid.context("Host token has no signing key ID")?;
        ensure!(
            !kid.is_empty() && kid.len() <= 128,
            "invalid signing key ID"
        );
        let snapshot = self.keys.refresh(Some(&kid)).await?;
        let set = snapshot.key_set.context("cloud signing keys unavailable")?;
        let public = set
            .keys
            .iter()
            .find(|k| k.kid == kid && k.status != SigningKeyStatus::Disabled)
            .context("cloud signing key unknown or disabled")?;
        let key = jsonwebtoken::DecodingKey::from_rsa_components(&public.n, &public.e)?;
        let mut validation = jsonwebtoken::Validation::new(jsonwebtoken::Algorithm::RS256);
        validation.set_audience(&[ACCESS_AUDIENCE]);
        validation.set_issuer(&[CLOUD_ISSUER]);
        validation.leeway = 0;
        validation.set_required_spec_claims(&["iss", "exp", "iat", "sub", "aud", "jti"]);
        Ok(jsonwebtoken::decode::<T>(token, &key, &validation)?.claims)
    }
    pub async fn authenticate(&self, token: &str, action: Permission) -> Result<String> {
        let claims: AccessClaims = self.cloud_claims(token, ACCESS_TYPE).await?;
        let policy = self.effective_policy()?;
        let status = policy.status(key_id(self.key.verifying_key()));
        access::validate_access(&claims, &status, &policy, action, unix_seconds())?;
        Ok(claims.sub)
    }
    // Exact retry returns only an already persisted receipt, never another ACL write.
    // This cache lasts only until the original transaction deadline. Fresh
    // attempts confirm the current binding instead of depending on this cache.
    fn completed_receipt(&self, proof: &str, exchange_key: Option<&str>) -> Result<Option<String>> {
        let hash = proof_digest(proof.as_bytes());
        let state = self
            .state
            .read()
            .map_err(|_| anyhow::anyhow!("Host state unavailable"))?;
        let found = state
            .completed_authorizations
            .values()
            .find(|item| item.proof_digest == hash && item.expires_at > unix_seconds());
        if let Some(item) = found {
            ensure!(
                item.exchange_hash == exchange_key.map(access::exchange_digest).transpose()?,
                "claim exchange secret mismatch"
            );
            Ok(Some(item.receipt.clone()))
        } else {
            Ok(None)
        }
    }
    pub async fn complete_claim(&self, request: ClaimRequest) -> Result<String> {
        ensure!(
            self.binding.is_none(),
            "dependent Host is initialized with its Client"
        );
        ensure!(
            request.env == self.status()?.env,
            "Host runtime environment mismatch"
        );
        if let Some(receipt) =
            self.completed_receipt(&request.proof, Some(&request.exchange_key))?
        {
            return Ok(receipt);
        }
        let grant: ClaimClaims = self.cloud_claims(&request.proof, CLAIM_TYPE).await?;
        ensure!(
            request.env == self.status()?.env,
            "Host runtime environment mismatch"
        );
        let challenge: HostChallenge =
            verify_jws(self.key.verifying_key(), CHALLENGE_TYPE, &grant.challenge)?;
        access::validate_claim(
            &grant,
            &self.status()?,
            &challenge,
            &request.exchange_key,
            unix_seconds(),
        )?;
        let key = self.key.clone();
        self.mutate(move |state| {
            let now = unix_seconds();
            let status = state.policy()?.status(key_id(key.verifying_key()));
            access::validate_claim(&grant, &status, &challenge, &request.exchange_key, now)?;
            prune(state, now);
            if let Some(previous) = state.completed_authorizations.get(&grant.jti) {
                ensure!(
                    previous.proof_digest == proof_digest(request.proof.as_bytes()),
                    "claim ID was already used"
                );
                return Ok(previous.receipt.clone());
            }
            state
                .policy_mut()?
                .claim_verified(&grant.sub, grant.binding.space.as_ref())?;
            if state.policy()?.mode() == HostMode::Admin {
                state.administrator_profiles.insert(
                    grant.sub.clone(),
                    AdministratorProfile {
                        display_name: grant.display_name,
                        email: grant.email,
                    },
                );
            }
            receipt(
                state,
                &key,
                grant.jti,
                Some(grant.exchange_key_hash),
                request.proof,
                grant.sub,
                grant.iat + AUTHORIZATION_TTL_SECONDS,
            )
        })
        .await
    }
    /// OS-local coordinator only. This verifies the cloud owner's approval but
    /// never writes the Client-owned canonical binding.
    pub async fn verify_device_claim(
        &self,
        request: &ClaimRequest,
    ) -> Result<core_api::device_binding::DeviceBinding> {
        let reader = self
            .binding
            .as_ref()
            .context("Host is not dependent on a Client")?;
        let current = reader.read()?;
        let status = self.status()?;
        ensure!(
            request.env == status.env,
            "Host runtime environment mismatch"
        );
        let grant: ClaimClaims = self.cloud_claims(&request.proof, CLAIM_TYPE).await?;
        let challenge: HostChallenge =
            verify_jws(self.key.verifying_key(), CHALLENGE_TYPE, &grant.challenge)?;
        access::validate_claim(
            &grant,
            &status,
            &challenge,
            &request.exchange_key,
            unix_seconds(),
        )?;
        let original = challenge
            .device_binding
            .context("Client binding missing from approval")?;
        ensure!(
            original.version == current.version
                && original.env == current.env
                && original.client == current.client
                && original.host == current.host
                && original.generation == current.generation,
            "Client installation changed during authorization"
        );
        let mut bound = current;
        bound.space = Some(grant.binding.space.context("Space approval missing")?);
        Ok(bound)
    }

    /// The Client has already committed its binding. Sign that observed result
    /// for the existing cloud directory transaction without storing a Host claim.
    pub async fn device_claim_receipt(&self, request: &ClaimRequest) -> Result<String> {
        let expected = self.verify_device_claim(request).await?;
        let current = self.binding.as_ref().unwrap().read()?;
        ensure!(current == expected, "Client binding has not been committed");
        let grant: ClaimClaims = self.cloud_claims(&request.proof, CLAIM_TYPE).await?;
        sign_jws(
            &self.key,
            RECEIPT_TYPE,
            &AuthorizationReceipt {
                proof: request.proof.clone(),
                status: self.status()?,
                user_id: grant.sub,
            },
        )
    }

    /// Only the applicant identity is trusted here. No management permission is granted.
    pub async fn administrator_application(
        &self,
        action: &str,
        proof: String,
    ) -> Result<serde_json::Value> {
        ensure!(
            matches!(
                action,
                "administratorRequest"
                    | "administratorRequestStatus"
                    | "administratorRequestCancel"
            ),
            "Invalid applicant operation"
        );
        let claims: AdministratorApplicantClaims =
            self.cloud_claims(&proof, ADMIN_APPLICANT_TYPE).await?;
        access::validate_administrator_applicant(&claims, &self.status()?, unix_seconds())?;
        if action == "administratorRequestStatus" {
            let state = self
                .state
                .read()
                .map_err(|_| anyhow::anyhow!("Host state unavailable"))?;
            return Ok(state
                .administrator_requests
                .get(&claims.sub)
                .filter(|r| r.expires_at > unix_seconds())
                .map(AdministratorRequest::view)
                .unwrap_or_else(|| serde_json::json!({"status":"none"})));
        }
        let action = action.to_owned();
        let key = self.key.clone();
        self.mutate(move |state| {
            let now = unix_seconds();
            access::validate_administrator_applicant(
                &claims,
                &state.policy()?.status(key_id(key.verifying_key())),
                now,
            )?;
            prune(state, now);
            let previous = state.administrator_requests.get(&claims.sub);
            if action == "administratorRequestCancel" {
                let request = state
                    .administrator_requests
                    .get_mut(&claims.sub)
                    .context("No administrator request")?;
                if request.status == "pending" {
                    request.status = "cancelled".into();
                }
                return Ok(request.view());
            }
            if let Some(previous) = previous {
                if previous.status == "pending" || previous.request_id == claims.jti {
                    return Ok(previous.view());
                }
            }
            ensure!(
                state.administrator_requests.len() < 64 || previous.is_some(),
                "Administrator request limit reached"
            );
            let profile = AdministratorProfile {
                display_name: claims.display_name,
                email: claims.email,
            };
            let already_member = is_administrator(state.policy()?, &claims.sub);
            let receipt = if already_member {
                Some(sign_jws(
                    &key,
                    RECEIPT_TYPE,
                    &AuthorizationReceipt {
                        proof: proof.clone(),
                        status: state.policy()?.status(key_id(key.verifying_key())),
                        user_id: claims.sub.clone(),
                    },
                )?)
            } else {
                None
            };
            let request = AdministratorRequest {
                request_id: claims.jti,
                user_id: claims.sub.clone(),
                profile,
                created_at: claims.iat,
                expires_at: claims.exp,
                proof,
                status: if already_member {
                    "approved"
                } else {
                    "pending"
                }
                .into(),
                receipt,
            };
            let result = request.view();
            state.administrator_requests.insert(claims.sub, request);
            Ok(result)
        })
        .await
    }
    /// Shared administrator dispatch for every Host platform and transport.
    pub async fn administrator_operation<C>(
        &self,
        actor: &ManagementActor,
        action: &core_api::host::management::HostRuntimeAction<C>,
    ) -> Option<Result<serde_json::Value>> {
        use core_api::host::management::HostRuntimeAction as Action;
        match action {
            Action::Administrators => Some(self.administrators(actor)),
            Action::AdministratorDecide {
                request_id,
                approve,
            } => Some(
                self.decide_administrator(actor, request_id.clone(), *approve)
                    .await,
            ),
            Action::AdministratorRemove { user_id } => Some(
                self.remove_administrator(actor, user_id)
                    .await
                    .map(|()| serde_json::json!({"removed":true})),
            ),
            _ => None,
        }
    }
    pub fn administrators(&self, actor: &ManagementActor) -> Result<serde_json::Value> {
        let state = self
            .state
            .read()
            .map_err(|_| anyhow::anyhow!("Host state unavailable"))?;
        require_administrator(state.policy()?, actor)?;
        let Some(Claim::Admin { user_ids }) = state.policy()?.claim() else {
            anyhow::bail!("Admin Host required")
        };
        let members = user_ids.iter().map(|user| {
            let profile = state.administrator_profiles.get(user);
            serde_json::json!({"userId":user,"displayName":profile.map(|p| &p.display_name),"email":profile.map(|p| &p.email)})
        }).collect::<Vec<_>>();
        let requests = state
            .administrator_requests
            .values()
            .filter(|request| request.expires_at > unix_seconds())
            .map(AdministratorRequest::view)
            .collect::<Vec<_>>();
        Ok(
            serde_json::json!({"currentUserId":actor.user_id(),"administrators":members,"requests":requests}),
        )
    }
    pub async fn decide_administrator(
        &self,
        actor: &ManagementActor,
        request_id: String,
        approve: bool,
    ) -> Result<serde_json::Value> {
        let actor = actor.clone();
        let key = self.key.clone();
        self.mutate(move |state| {
            require_administrator(state.policy()?, &actor)?;
            let request = state
                .administrator_requests
                .values()
                .find(|r| r.request_id == request_id)
                .context("Administrator request not found")?
                .clone();
            ensure!(
                request.expires_at > unix_seconds(),
                "Administrator request expired"
            );
            let decision = if approve { "approved" } else { "rejected" };
            if request.status == decision {
                return Ok(request.view());
            }
            ensure!(
                request.status == "pending",
                "Administrator request was already decided"
            );
            let mut request = request;
            if approve {
                state
                    .policy_mut()?
                    .add_verified_administrator(request.user_id.clone())?;
                state
                    .administrator_profiles
                    .insert(request.user_id.clone(), request.profile.clone());
                request.receipt = Some(sign_jws(
                    &key,
                    RECEIPT_TYPE,
                    &AuthorizationReceipt {
                        proof: request.proof.clone(),
                        status: state.policy()?.status(key_id(key.verifying_key())),
                        user_id: request.user_id.clone(),
                    },
                )?);
            }
            request.status = decision.into();
            let result = request.view();
            state
                .administrator_requests
                .insert(request.user_id.clone(), request);
            Ok(result)
        })
        .await
    }
    pub async fn remove_administrator(&self, actor: &ManagementActor, user: &str) -> Result<()> {
        let actor = actor.clone();
        let user = user.to_owned();
        self.mutate(move |state| {
            // Recheck the caller under the same lock as the final-admin invariant.
            require_administrator(state.policy()?, &actor)?;
            state.policy_mut()?.remove_verified_administrator(&user)?;
            state.administrator_profiles.remove(&user);
            if let Some(request) = state.administrator_requests.get_mut(&user) {
                request.status = "removed".into();
                request.receipt = None;
            }
            Ok(())
        })
        .await
    }
}
fn prune(state: &mut AuthorityState, now: u64) {
    state
        .administrator_requests
        .retain(|_, request| request.expires_at > now);
    state
        .completed_authorizations
        .retain(|_, completed| completed.expires_at > now);
}
fn receipt(
    state: &mut AuthorityState,
    key: &SigningKey,
    id: String,
    exchange_hash: Option<String>,
    proof: String,
    user: String,
    expires_at: u64,
) -> Result<String> {
    let signed = sign_jws(
        key,
        RECEIPT_TYPE,
        &AuthorizationReceipt {
            proof: proof.clone(),
            status: state.policy()?.status(key_id(key.verifying_key())),
            user_id: user,
        },
    )?;
    state.completed_authorizations.insert(
        id,
        Completed {
            expires_at,
            exchange_hash,
            proof_digest: proof_digest(proof.as_bytes()),
            receipt: signed.clone(),
        },
    );
    Ok(signed)
}

#[cfg(test)]
#[path = "authority_tests.rs"]
mod tests;

fn proof_digest(bytes: &[u8]) -> String {
    use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
    use sha2::{Digest, Sha256};
    URL_SAFE_NO_PAD.encode(Sha256::digest(bytes))
}

fn is_loopback_host(host: &str) -> bool {
    matches!(host, "localhost" | "127.0.0.1" | "::1")
}

/// HTTPS cloud, or development loopback HTTP Lion. Non-loopback HTTP is rejected.
pub fn cloud_api_base(configured: &str) -> anyhow::Result<String> {
    let url = reqwest::Url::parse(configured)?;
    anyhow::ensure!(
        url.username().is_empty()
            && url.password().is_none()
            && url.query().is_none()
            && url.fragment().is_none()
            && url.host_str().is_some(),
        "Host authorization requires a trusted cloud endpoint"
    );
    let https = url.scheme() == "https";
    let loopback_http = url.scheme() == "http" && is_loopback_host(url.host_str().unwrap());
    anyhow::ensure!(
        https || loopback_http,
        "Host authorization requires a trusted HTTPS cloud endpoint"
    );
    Ok(configured.trim_end_matches('/').into())
}

#[cfg(test)]
mod cloud_endpoint_tests {
    use super::*;

    #[test]
    fn configured_cloud_allows_https_and_loopback_http() {
        assert_eq!(
            cloud_api_base("http://127.0.0.1:8080/api/v1").unwrap(),
            "http://127.0.0.1:8080/api/v1"
        );
        assert_eq!(
            cloud_api_base("http://localhost:8080/api/v1").unwrap(),
            "http://localhost:8080/api/v1"
        );
        assert_eq!(
            cloud_api_base("https://prod.s.mhome.ai/api/v1").unwrap(),
            "https://prod.s.mhome.ai/api/v1"
        );
        assert!(cloud_api_base("http://cloud.invalid/api/v1").is_err());
        assert!(cloud_api_base("file:///api/v1").is_err());
    }
}
