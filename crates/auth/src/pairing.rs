//! Pairing hash generation

use sha2::{Sha256, Digest};

/// Generate pairing hash for device verification
pub fn generate_pairing_hash(device_id: &str, pin: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(device_id);
    hasher.update(pin);
    format!("{:x}", hasher.finalize())
}
