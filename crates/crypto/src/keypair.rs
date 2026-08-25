//! Key pair management

use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};
use rand::rngs::OsRng;
use serde::{Deserialize, Serialize};
use x25519_dalek::{EphemeralSecret as X25519Secret, PublicKey};

#[derive(Clone, Serialize, Deserialize)]
pub struct DeviceKeys {
    pub signing_key: [u8; 32],
    pub verifying_key: [u8; 32],
    pub exchange_secret: [u8; 32],
}

impl DeviceKeys {
    pub fn generate() -> Self {
        let signing = SigningKey::generate(&mut OsRng);
        let verifying = signing.verifying_key();

        Self {
            signing_key: *signing.as_bytes(),
            verifying_key: *verifying.as_bytes(),
            exchange_secret: [0u8; 32], // Will be generated fresh for each DH
        }
    }

    pub fn signing_key(&self) -> SigningKey {
        SigningKey::from_bytes(&self.signing_key)
    }

    pub fn verifying_key(&self) -> VerifyingKey {
        VerifyingKey::from_bytes(&self.verifying_key).unwrap()
    }

    pub fn exchange_secret(&self) -> X25519Secret {
        X25519Secret::random_from_rng(OsRng)
    }

    pub fn sign(&self, message: &[u8]) -> Signature {
        self.signing_key().sign(message)
    }

    pub fn verify(&self, message: &[u8], sig: &Signature) -> bool {
        self.verifying_key().verify(message, sig).is_ok()
    }

    pub fn diffie_hellman(&self, remote: &PublicKey) -> [u8; 32] {
        let secret = X25519Secret::random_from_rng(OsRng);
        secret.diffie_hellman(remote).to_bytes()
    }
}

/// Key rotation manager
#[derive(Debug, Clone)]
pub struct KeyRotation {
    pub current_key_id: u64,
    pub rotation_interval_seconds: u64,
    pub max_keys: usize,
}

impl Default for KeyRotation {
    fn default() -> Self {
        Self {
            current_key_id: 0,
            rotation_interval_seconds: 3600, // 1 hour default
            max_keys: 10,
        }
    }
}

impl KeyRotation {
    pub fn should_rotate(&self, last_rotation: u64) -> bool {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        now - last_rotation >= self.rotation_interval_seconds
    }

    pub fn next_key_id(&self) -> u64 {
        self.current_key_id + 1
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_key_generation() {
        let keys = DeviceKeys::generate();
        assert_eq!(keys.signing_key.len(), 32);
        assert_eq!(keys.verifying_key.len(), 32);
    }

    #[test]
    fn test_key_rotation() {
        let rotation = KeyRotation::default();

        // Test with recent rotation (should not rotate)
        let recent_time = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        assert!(!rotation.should_rotate(recent_time));

        // Test with old rotation (should rotate)
        let old_time = recent_time - 7200; // 2 hours ago
        assert!(rotation.should_rotate(old_time));

        let next_id = rotation.next_key_id();
        assert_eq!(next_id, 1);
    }
}
