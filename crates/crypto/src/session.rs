//! Session key management

use aes_gcm::aead::Aead;
use aes_gcm::{Aes256Gcm, KeyInit};
use generic_array::GenericArray;
use sha2::{Digest, Sha256};

#[derive(Clone)]
pub struct SessionKey {
    pub key: [u8; 32],
}

impl SessionKey {
    pub fn from_shared_secret(secret: &[u8; 32]) -> Self {
        let mut hasher = Sha256::new();
        hasher.update(secret);
        let result = hasher.finalize();
        Self { key: result.into() }
    }

    pub fn encrypt(&self, nonce: &[u8; 12], plaintext: &[u8]) -> Result<Vec<u8>, String> {
        let cipher = Aes256Gcm::new_from_slice(&self.key).map_err(|e| e.to_string())?;
        let nonce = GenericArray::from_slice(nonce);
        cipher.encrypt(nonce, plaintext).map_err(|e| e.to_string())
    }

    pub fn decrypt(&self, nonce: &[u8; 12], ciphertext: &[u8]) -> Result<Vec<u8>, String> {
        let cipher = Aes256Gcm::new_from_slice(&self.key).map_err(|e| e.to_string())?;
        let nonce = GenericArray::from_slice(nonce);
        cipher.decrypt(nonce, ciphertext).map_err(|e| e.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_session_key() {
        let secret = [42u8; 32];
        let key = SessionKey::from_shared_secret(&secret);
        let nonce: [u8; 12] = [0; 12];
        let plain = b"test data";
        let cipher = key.encrypt(&nonce, plain).unwrap();
        let dec = key.decrypt(&nonce, &cipher).unwrap();
        assert_eq!(dec, plain);
    }
}
