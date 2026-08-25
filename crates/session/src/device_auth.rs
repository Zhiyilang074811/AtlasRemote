//! Device authentication with Ed25519 signatures

use atlas_crypto::{DeviceKeys, SessionKey};
use ed25519_dalek::{Signature, Verifier};
use x25519_dalek::{PublicKey, EphemeralSecret};
use tracing::{info, warn};

/// Device authentication result
#[derive(Debug, Clone)]
pub enum AuthResult {
    Success { 
        device_id: String, 
        session_key: SessionKey,
        remote_public_key: [u8; 32],
    },
    InvalidSignature,
    Expired,
    UnknownDevice,
}

/// Perform device authentication
pub async fn verify_device(
    local_keys: &DeviceKeys,
    remote_pub_key: &[u8; 32],
    device_id: &str,
    signature: &Signature,
    challenge: &[u8],
) -> AuthResult {
    // Verify device signature
    let remote_verifying_key = match ed25519_dalek::VerifyingKey::from_bytes(remote_pub_key) {
        Ok(key) => key,
        Err(_) => return AuthResult::InvalidSignature,
    };
    
    if remote_verifying_key.verify(challenge, signature).is_err() {
        warn!("Invalid device signature for {}", device_id);
        return AuthResult::InvalidSignature;
    }
    
    info!("Device {} authenticated successfully", device_id);
    
    // Perform key exchange
    let local_secret = EphemeralSecret::random_from_rng(rand::thread_rng());
    let local_public = PublicKey::from(&local_secret);
    let shared_secret = local_secret.diffie_hellman(&remote_verifying_key);
    let session_key = SessionKey::from_shared_secret(&shared_secret.to_bytes());
    
    AuthResult::Success {
        device_id: device_id.to_string(),
        session_key,
        remote_public_key: *remote_pub_key,
    }
}

/// Generate authentication challenge
pub fn generate_challenge(device_id: &str, timestamp: u64) -> Vec<u8> {
    use sha2::{Sha256, Digest};
    let mut hasher = Sha256::new();
    hasher.update(device_id);
    hasher.update(&timestamp.to_le_bytes());
    hasher.update(b"atlas-challenge");
    hasher.finalize().to_vec()
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_challenge_generation() {
        let c1 = generate_challenge("device-1", 1000);
        let c2 = generate_challenge("device-1", 1000);
        let c3 = generate_challenge("device-2", 1000);
        assert_eq!(c1, c2);
        assert_ne!(c1, c3);
    }
}
