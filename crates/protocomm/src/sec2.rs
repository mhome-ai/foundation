//! Protocomm security 2 record layer: AES-256-GCM keyed with the first half of
//! the SRP session key, 12-byte device nonce, 16-byte tag appended.
use aes_gcm::aead::{Aead, KeyInit};
use aes_gcm::{Aes256Gcm, Nonce};
use rand::{rngs::OsRng, RngCore};

/// Highest security 2 patch version this crate speaks.
pub const MAX_PATCH_VERSION: u32 = 1;

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum CipherError {
    #[error("device nonce must be 12 bytes with a non-zero counter")]
    InvalidNonce,
    #[error("security 2 patch version {0} is not supported")]
    UnsupportedPatch(u32),
    #[error("record authentication failed")]
    Authentication,
    #[error("nonce counter exhausted")]
    Exhausted,
}

/// A device nonce as ESP-IDF builds it: 8 random session bytes, then a
/// big-endian 32-bit counter starting at 1.
pub fn new_device_nonce() -> [u8; 12] {
    let mut nonce = [0u8; 12];
    OsRng.fill_bytes(&mut nonce[..8]);
    nonce[8..].copy_from_slice(&1u32.to_be_bytes());
    nonce
}

/// From patch version 1 on each side advances the nonce counter after every
/// encrypt and decrypt; patch 0 keeps the nonce fixed. A failed decrypt does
/// not advance it.
pub struct Sec2Cipher {
    cipher: Aes256Gcm,
    nonce: [u8; 12],
    counter: bool,
}

impl Sec2Cipher {
    pub fn new(
        session_key: &[u8; 64],
        device_nonce: &[u8],
        patch_version: u32,
    ) -> Result<Self, CipherError> {
        if patch_version > MAX_PATCH_VERSION {
            return Err(CipherError::UnsupportedPatch(patch_version));
        }
        let nonce: [u8; 12] = device_nonce
            .try_into()
            .map_err(|_| CipherError::InvalidNonce)?;
        let counter = patch_version >= 1;
        if counter && nonce[8..] == [0, 0, 0, 0] {
            return Err(CipherError::InvalidNonce);
        }
        let cipher = Aes256Gcm::new_from_slice(&session_key[..32]).expect("32-byte AES key");
        Ok(Self {
            cipher,
            nonce,
            counter,
        })
    }

    pub fn encrypt(&mut self, plaintext: &[u8]) -> Result<Vec<u8>, CipherError> {
        self.check_counter()?;
        let out = self
            .cipher
            .encrypt(Nonce::from_slice(&self.nonce), plaintext)
            .map_err(|_| CipherError::Authentication)?;
        self.advance();
        Ok(out)
    }

    pub fn decrypt(&mut self, ciphertext: &[u8]) -> Result<Vec<u8>, CipherError> {
        self.check_counter()?;
        let out = self
            .cipher
            .decrypt(Nonce::from_slice(&self.nonce), ciphertext)
            .map_err(|_| CipherError::Authentication)?;
        self.advance();
        Ok(out)
    }

    fn counter_value(&self) -> u32 {
        u32::from_be_bytes(self.nonce[8..].try_into().expect("4 bytes"))
    }

    /// The counter wraps to 0 after `u32::MAX`, which ESP-IDF treats as invalid.
    fn check_counter(&self) -> Result<(), CipherError> {
        if self.counter && self.counter_value() == 0 {
            return Err(CipherError::Exhausted);
        }
        Ok(())
    }

    fn advance(&mut self) {
        if self.counter {
            let next = self.counter_value().wrapping_add(1);
            self.nonce[8..].copy_from_slice(&next.to_be_bytes());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn both_sides_stay_in_step() {
        let key = [7u8; 64];
        let nonce = [0, 1, 2, 3, 4, 5, 6, 7, 0xff, 0xff, 0xff, 0xfd];
        let mut client = Sec2Cipher::new(&key, &nonce, 1).unwrap();
        let mut device = Sec2Cipher::new(&key, &nonce, 1).unwrap();
        let request = client.encrypt(b"{\"op\":\"status\"}").unwrap();
        assert_eq!(device.decrypt(&request).unwrap(), b"{\"op\":\"status\"}");
        let response = device.encrypt(b"{}").unwrap();
        assert_eq!(client.decrypt(&response).unwrap(), b"{}");
        assert!(client.encrypt(b"last").is_ok());
        assert_eq!(client.encrypt(b"x").err(), Some(CipherError::Exhausted));
        let mut fixed = Sec2Cipher::new(&key, &nonce, 0).unwrap();
        let first = fixed.encrypt(b"same").unwrap();
        assert_eq!(fixed.encrypt(b"same").unwrap(), first);
    }

    #[test]
    fn a_failed_decrypt_keeps_the_counter() {
        let key = [9u8; 64];
        let nonce = new_device_nonce();
        assert_eq!(nonce[8..], [0, 0, 0, 1]);
        let mut client = Sec2Cipher::new(&key, &nonce, 1).unwrap();
        let mut device = Sec2Cipher::new(&key, &nonce, 1).unwrap();
        let record = client.encrypt(b"hello").unwrap();
        let mut tampered = record.clone();
        tampered[0] ^= 1;
        assert_eq!(
            device.decrypt(&tampered).err(),
            Some(CipherError::Authentication)
        );
        assert_eq!(device.decrypt(&record).unwrap(), b"hello");
    }

    #[test]
    fn rejects_unknown_patches_and_zero_counters() {
        let key = [1u8; 64];
        assert_eq!(
            Sec2Cipher::new(&key, &[0u8; 12], 1).err(),
            Some(CipherError::InvalidNonce)
        );
        assert!(Sec2Cipher::new(&key, &[0u8; 12], 0).is_ok());
        assert_eq!(
            Sec2Cipher::new(&key, &new_device_nonce(), 2).err(),
            Some(CipherError::UnsupportedPatch(2))
        );
        assert_eq!(
            Sec2Cipher::new(&key, &[0u8; 11], 1).err(),
            Some(CipherError::InvalidNonce)
        );
    }
}
