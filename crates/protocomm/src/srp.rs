//! SRP6a over the RFC 5054 3072-bit group with SHA-512, byte-compatible with
//! the ESP-IDF `esp_srp` device side.
//!
//! - `x = H(salt as an integer | H(I ":" P))`: leading zero bytes of the salt are
//!   dropped, as `esp_srp_gen_salt_verifier` does.
//! - `u = H(PAD(A) | PAD(B))`, `k = H(N | PAD(g))`.
//! - `M = H(H(N) xor H(PAD(g)) | H(I) | salt | A | B | K)` and
//!   `H(AMK) = H(A | M | K)` use the salt, `A` and `B` bytes as exchanged.
//! - `A` travels as exactly 384 bytes; `B` without leading zeros.
use std::sync::OnceLock;

use num_bigint::BigUint;
use rand::{rngs::OsRng, RngCore};
use sha2::{Digest, Sha512};

const N_HEX: &str = concat!(
    "FFFFFFFFFFFFFFFFC90FDAA22168C234C4C6628B80DC1CD129024E088A67CC74",
    "020BBEA63B139B22514A08798E3404DDEF9519B3CD3A431B302B0A6DF25F1437",
    "4FE1356D6D51C245E485B576625E7EC6F44C42E9A637ED6B0BFF5CB6F406B7ED",
    "EE386BFB5A899FA5AE9F24117C4B1FE649286651ECE45B3DC2007CB8A163BF05",
    "98DA48361C55D39A69163FA8FD24CF5F83655D23DCA3AD961C62F356208552BB",
    "9ED529077096966D670C354E4ABC9804F1746C08CA18217C32905E462E36CE3B",
    "E39E772C180E86039B2783A2EC07A28FB5C55DF06F4C52C9DE2BCBF695581718",
    "3995497CEA956AE515D2261898FA051015728E5A8AAAC42DAD33170D04507A33",
    "A85521ABDF1CBA64ECFB850458DBEF0A8AEA71575D060C7DB3970F85A6E1E4C7",
    "ABF5AE8CDB0933D71E8C94E04A25619DCEE3D2261AD2EE6BF12FFA06D98A0864",
    "D87602733EC86A64521F2B18177B200CBBE117577A615D6C770988C0BAD946E2",
    "08E24FA074E5AB3143DB5BFCE0FD108E4B82D120A93AD2CAFFFFFFFFFFFFFFFF",
);
const GENERATOR: u32 = 5;
/// Size of the group, of `A` on the wire and of padded hash inputs.
pub const GROUP_BYTES: usize = 384;
pub const SALT_BYTES: usize = 16;
pub const PROOF_BYTES: usize = 64;
const EPHEMERAL_BYTES: usize = 32;

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum SrpError {
    #[error("peer public key is invalid")]
    InvalidPublicKey,
    #[error("peer proof does not match")]
    ProofMismatch,
}

struct Group {
    n: BigUint,
    g: BigUint,
    k: BigUint,
    hn_xor_hg: [u8; 64],
}

fn group() -> &'static Group {
    static GROUP: OnceLock<Group> = OnceLock::new();
    GROUP.get_or_init(|| {
        let n = BigUint::parse_bytes(N_HEX.as_bytes(), 16).expect("valid SRP group");
        let g = BigUint::from(GENERATOR);
        let n_bytes = n.to_bytes_be();
        let padded_g = pad(&g.to_bytes_be());
        let k = BigUint::from_bytes_be(&hash(&[&n_bytes, &padded_g]));
        let hn = hash(&[&n_bytes]);
        let hg = hash(&[&padded_g]);
        let mut hn_xor_hg = [0u8; 64];
        for (i, byte) in hn_xor_hg.iter_mut().enumerate() {
            *byte = hn[i] ^ hg[i];
        }
        Group { n, g, k, hn_xor_hg }
    })
}

fn hash(parts: &[&[u8]]) -> [u8; 64] {
    let mut hasher = Sha512::new();
    for part in parts {
        hasher.update(part);
    }
    hasher.finalize().into()
}

fn pad(bytes: &[u8]) -> Vec<u8> {
    let bytes = strip(bytes);
    let mut out = vec![0u8; GROUP_BYTES.saturating_sub(bytes.len())];
    out.extend_from_slice(bytes);
    out
}

fn strip(bytes: &[u8]) -> &[u8] {
    let first = bytes
        .iter()
        .position(|byte| *byte != 0)
        .unwrap_or(bytes.len());
    &bytes[first..]
}

fn random_ephemeral() -> BigUint {
    let mut bytes = [0u8; EPHEMERAL_BYTES];
    OsRng.fill_bytes(&mut bytes);
    bytes[0] |= 0x80;
    BigUint::from_bytes_be(&bytes)
}

fn compute_x(salt: &[u8], username: &str, password: &str) -> BigUint {
    let inner = hash(&[username.as_bytes(), b":", password.as_bytes()]);
    BigUint::from_bytes_be(&hash(&[strip(salt), &inner]))
}

fn compute_u(a_pub: &[u8], b_pub: &[u8]) -> BigUint {
    BigUint::from_bytes_be(&hash(&[&pad(a_pub), &pad(b_pub)]))
}

fn compute_m(username: &str, salt: &[u8], a_pub: &[u8], b_pub: &[u8], key: &[u8]) -> [u8; 64] {
    let hu = hash(&[username.as_bytes()]);
    hash(&[&group().hn_xor_hg, &hu, salt, a_pub, b_pub, key])
}

fn compute_h_amk(a_pub: &[u8], m: &[u8], key: &[u8]) -> [u8; 64] {
    hash(&[a_pub, m, key])
}

/// A device-side salt and verifier, as `esp_srp_gen_salt_verifier` produces
/// them. The salt never starts with a zero byte, so clients that hash it as an
/// integer everywhere agree with ESP-IDF.
pub fn generate_salt_verifier(username: &str, password: &str) -> (Vec<u8>, Vec<u8>) {
    let mut salt = vec![0u8; SALT_BYTES];
    while salt[0] == 0 {
        OsRng.fill_bytes(&mut salt);
    }
    let verifier = verifier_for(&salt, username, password);
    (salt, verifier)
}

pub fn verifier_for(salt: &[u8], username: &str, password: &str) -> Vec<u8> {
    let group = group();
    group
        .g
        .modpow(&compute_x(salt, username, password), &group.n)
        .to_bytes_be()
}

/// The commissioner side.
pub struct SrpClient {
    username: String,
    password: String,
    a: BigUint,
    a_pub: Vec<u8>,
}

pub struct ClientProof {
    pub proof: Vec<u8>,
    expected_device_proof: [u8; 64],
    key: [u8; 64],
}

impl ClientProof {
    /// Checks the device proof and returns the 64-byte session key.
    pub fn verify(self, device_proof: &[u8]) -> Result<[u8; 64], SrpError> {
        if device_proof != self.expected_device_proof {
            return Err(SrpError::ProofMismatch);
        }
        Ok(self.key)
    }
}

impl SrpClient {
    /// Picks a new ephemeral until `A` fills all 384 bytes; ESP-IDF rejects any
    /// other length and a padded `A` would differ from what other clients hash.
    pub fn new(username: &str, password: &str) -> Self {
        loop {
            if let Some(client) = Self::with_ephemeral(username, password, random_ephemeral()) {
                return client;
            }
        }
    }

    /// `None` when `g^a mod N` has a leading zero byte.
    pub fn with_ephemeral(username: &str, password: &str, a: BigUint) -> Option<Self> {
        let group = group();
        let a_pub = group.g.modpow(&a, &group.n).to_bytes_be();
        (a_pub.len() == GROUP_BYTES).then(|| Self {
            username: username.to_string(),
            password: password.to_string(),
            a,
            a_pub,
        })
    }

    pub fn public_key(&self) -> Vec<u8> {
        self.a_pub.clone()
    }

    pub fn process_challenge(
        &self,
        salt: &[u8],
        device_pubkey: &[u8],
    ) -> Result<ClientProof, SrpError> {
        let group = group();
        if device_pubkey.len() > GROUP_BYTES {
            return Err(SrpError::InvalidPublicKey);
        }
        let b_pub = BigUint::from_bytes_be(device_pubkey);
        if (&b_pub % &group.n) == BigUint::ZERO {
            return Err(SrpError::InvalidPublicKey);
        }
        let u = compute_u(&self.a_pub, device_pubkey);
        if u == BigUint::ZERO {
            return Err(SrpError::InvalidPublicKey);
        }
        let x = compute_x(salt, &self.username, &self.password);
        let v = group.g.modpow(&x, &group.n);
        let kv = (&group.k * &v) % &group.n;
        let base = ((&b_pub % &group.n) + &group.n - kv) % &group.n;
        let s = base.modpow(&(&self.a + &u * &x), &group.n);
        let key = hash(&[&s.to_bytes_be()]);
        let m = compute_m(&self.username, salt, &self.a_pub, device_pubkey, &key);
        let expected_device_proof = compute_h_amk(&self.a_pub, &m, &key);
        Ok(ClientProof {
            proof: m.to_vec(),
            expected_device_proof,
            key,
        })
    }
}

/// The device side, holding only the salt and verifier.
pub struct SrpServer {
    username: String,
    salt: Vec<u8>,
    v: BigUint,
    b: BigUint,
    b_pub: Vec<u8>,
}

pub struct ServerSession {
    pub device_proof: Vec<u8>,
    pub key: [u8; 64],
}

impl SrpServer {
    pub fn new(username: &str, salt: &[u8], verifier: &[u8]) -> Self {
        Self::with_ephemeral(username, salt, verifier, random_ephemeral())
    }

    pub fn with_ephemeral(username: &str, salt: &[u8], verifier: &[u8], b: BigUint) -> Self {
        let group = group();
        let v = BigUint::from_bytes_be(verifier);
        let b_pub = ((&group.k * &v) + group.g.modpow(&b, &group.n)) % &group.n;
        Self {
            username: username.to_string(),
            salt: salt.to_vec(),
            v,
            b,
            b_pub: b_pub.to_bytes_be(),
        }
    }

    /// `B` without leading zero bytes, as ESP-IDF sends it.
    pub fn public_key(&self) -> Vec<u8> {
        self.b_pub.clone()
    }

    pub fn salt(&self) -> &[u8] {
        &self.salt
    }

    /// `client_pubkey` must be exactly 384 bytes.
    pub fn verify_client(
        &self,
        client_pubkey: &[u8],
        client_proof: &[u8],
    ) -> Result<ServerSession, SrpError> {
        let group = group();
        if client_pubkey.len() != GROUP_BYTES {
            return Err(SrpError::InvalidPublicKey);
        }
        let a_pub = BigUint::from_bytes_be(client_pubkey);
        if (&a_pub % &group.n) == BigUint::ZERO {
            return Err(SrpError::InvalidPublicKey);
        }
        let u = compute_u(client_pubkey, &self.b_pub);
        let s = ((&a_pub * self.v.modpow(&u, &group.n)) % &group.n).modpow(&self.b, &group.n);
        let key = hash(&[&s.to_bytes_be()]);
        let m = compute_m(&self.username, &self.salt, client_pubkey, &self.b_pub, &key);
        if client_proof != m {
            return Err(SrpError::ProofMismatch);
        }
        Ok(ServerSession {
            device_proof: compute_h_amk(client_pubkey, &m, &key).to_vec(),
            key,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn client_and_server_agree_and_reject_a_wrong_code() {
        let (salt, verifier) = generate_salt_verifier("meow", "042137");
        assert_eq!(salt.len(), SALT_BYTES);
        assert_ne!(salt[0], 0);
        let server = SrpServer::new("meow", &salt, &verifier);
        let client = SrpClient::new("meow", "042137");
        assert_eq!(client.public_key().len(), GROUP_BYTES);
        let proof = client
            .process_challenge(server.salt(), &server.public_key())
            .unwrap();
        let session = server
            .verify_client(&client.public_key(), &proof.proof)
            .unwrap();
        assert_eq!(proof.verify(&session.device_proof).unwrap(), session.key);

        let wrong = SrpClient::new("meow", "042138");
        let wrong_proof = wrong
            .process_challenge(server.salt(), &server.public_key())
            .unwrap();
        assert_eq!(
            server
                .verify_client(&wrong.public_key(), &wrong_proof.proof)
                .err(),
            Some(SrpError::ProofMismatch)
        );
    }

    #[test]
    fn rejects_degenerate_public_keys() {
        let (salt, verifier) = generate_salt_verifier("meow", "042137");
        let server = SrpServer::new("meow", &salt, &verifier);
        let client = SrpClient::new("meow", "042137");
        assert!(client.process_challenge(&salt, &[0u8; 384]).is_err());
        assert!(client.process_challenge(&salt, &[1u8; 385]).is_err());
        let n = group().n.to_bytes_be();
        assert_eq!(
            server.verify_client(&n, &[0u8; 64]).err(),
            Some(SrpError::InvalidPublicKey)
        );
    }
}
