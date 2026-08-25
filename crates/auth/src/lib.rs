//! Authentication module - Device pairing and PIN verification
//!
//! Provides device ID generation, PIN-based authentication, and session keys.

use rand::Rng;
use sha2::{Sha256, Digest};
use std::time::{Duration, SystemTime};
use tracing::{info, warn};

/// 8-character device ID
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct DeviceId(pub String);

impl DeviceId {
    /// Generate a random 8-character device ID
    pub fn generate() -> Self {
        const CHARS: &[u8] = b"ABCDEFGHJKLMNPQRSTUVWXYZ23456789"; // Remove ambiguous chars
        let mut rng = rand::thread_rng();
        let id: String = (0..8)
            .map(|_| {
                let idx = rng.gen_range(0..CHARS.len());
                CHARS[idx] as char
            })
            .collect();
        Self(id)
    }
    
    /// Validate device ID format
    pub fn is_valid(&self) -> bool {
        self.0.len() == 8 && self.0.chars().all(|c| {
            matches!(c, 'A'..='Z' | '2'..='9')
        })
    }
}

impl std::fmt::Display for DeviceId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// 4-digit PIN for device pairing
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PinCode(pub String);

impl PinCode {
    /// Generate a random 4-digit PIN
    pub fn generate() -> Self {
        let mut rng = rand::thread_rng();
        let pin: String = (0..4)
            .map(|_| rng.gen_range(0..10).to_string())
            .collect();
        Self(pin)
    }
    
    /// Validate PIN format
    pub fn is_valid(&self) -> bool {
        self.0.len() == 4 && self.0.chars().all(|c| c.is_ascii_digit())
    }
}

impl std::fmt::Display for PinCode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Authentication session
#[derive(Debug, Clone)]
pub struct AuthSession {
    pub device_id: DeviceId,
    pub pin: PinCode,
    pub session_key: Vec<u8>,
    pub created_at: SystemTime,
    pub expires_at: SystemTime,
}

impl AuthSession {
    pub fn is_expired(&self) -> bool {
        SystemTime::now() > self.expires_at
    }
}

/// Access credential for remote control
#[derive(Debug, Clone)]
pub struct AccessCredential {
    pub device_id: DeviceId,
    pub access_pin: PinCode,
    pub allowed: bool,
    pub created_at: SystemTime,
}

impl AccessCredential {
    /// Verify access PIN
    pub fn verify_pin(&self, pin: &PinCode) -> bool {
        self.access_pin == *pin && self.allowed
    }
}

/// Generate pairing hash for device verification
pub fn generate_pairing_hash(device_id: &str, pin: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(device_id);
    hasher.update(pin);
    format!("{:x}", hasher.finalize())
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_device_id_generation() {
        let id = DeviceId::generate();
        assert_eq!(id.0.len(), 8);
        assert!(id.is_valid());
    }
    
    #[test]
    fn test_pin_generation() {
        let pin = PinCode::generate();
        assert_eq!(pin.0.len(), 4);
        assert!(pin.is_valid());
    }
    
    #[test]
    fn test_pairing_hash() {
        let hash = generate_pairing_hash("ABCD1234", "5678");
        assert_eq!(hash.len(), 64); // SHA-256 hex
    }
}
