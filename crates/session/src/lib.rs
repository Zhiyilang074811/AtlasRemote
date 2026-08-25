//! Atlas Session - Secure pairing and encrypted tunnel
//!
//! Provides:
//! - 6-digit pairing code generation
//! - Device authentication with Ed25519 signatures
//! - X25519 key exchange
//! - AES-GCM encrypted tunnel for video + control channels

pub mod pair_code;
pub mod device_auth;
pub mod encrypted_tunnel;

pub use pair_code::PairCode;
pub use device_auth::{DeviceAuth, DeviceKeys, verify_device};
pub use encrypted_tunnel::{EncryptedTunnel, TunnelStream, TunnelMetrics};

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::{info, warn};

/// Global pairing code store
#[derive(Debug, Default)]
pub struct PairingStore {
    codes: Arc<Mutex<HashMap<String, PairingSession>>>,
}

impl PairingStore {
    pub fn new() -> Self { Self::default() }
    
    /// Generate a new 6-digit pairing code
    pub async fn generate_code(&self, device_id: &str) -> String {
        let code = PairCode::generate();
        let session = PairingSession {
            code: code.clone(),
            device_id: device_id.to_string(),
            created_at: chrono::Utc::now(),
            expires_at: chrono::Utc::now() + chrono::Duration::minutes(5),
            verified: false,
        };
        let mut codes = self.codes.lock().await;
        codes.insert(code.0.clone(), session);
        info!("Generated pairing code: {} for device: {}", code, device_id);
        code.0
    }
    
    /// Verify a pairing code
    pub async fn verify_code(&self, code: &str, device_id: &str) -> Option<String> {
        let mut codes = self.codes.lock().await;
        if let Some(session) = codes.get_mut(code) {
            if session.expires_at < chrono::Utc::now() {
                warn!("Pairing code expired: {}", code);
                codes.remove(code);
                return None;
            }
            if session.device_id == device_id && !session.verified {
                session.verified = true;
                info!("Pairing code verified: {} for device: {}", code, device_id);
                return Some(session.device_id.clone());
            }
        }
        warn!("Invalid pairing code: {}", code);
        None
    }
    
    /// Check if pairing code exists and is valid
    pub async fn is_valid(&self, code: &str) -> bool {
        let codes = self.codes.lock().await;
        codes.get(code).map(|s| !s.expires_at.lt(&chrono::Utc::now()) && !s.verified).unwrap_or(false)
    }
    
    /// Clean up expired codes
    pub async fn cleanup(&self) {
        let mut codes = self.codes.lock().await;
        codes.retain(|_, s| s.expires_at >= chrono::Utc::now());
    }
}

#[derive(Debug, Clone)]
struct PairingSession {
    code: PairCode,
    device_id: String,
    created_at: chrono::DateTime<chrono::Utc>,
    expires_at: chrono::DateTime<chrono::Utc>,
    verified: bool,
}

/// Connection state after pairing
#[derive(Debug, Clone)]
pub struct PairedConnection {
    pub device_id: String,
    pub session_key: Vec<u8>,
    pub remote_public_key: [u8; 32],
    pub paired_at: chrono::DateTime<chrono::Utc>,
}

impl PairedConnection {
    pub fn is_expired(&self) -> bool {
        chrono::Utc::now().signed_duration_since(self.paired_at).num_minutes() > 60
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_pairing_code_generation() {
        let store = PairingStore::new();
        let code = store.generate_code("test-device").await;
        assert_eq!(code.len(), 6);
        assert!(code.chars().all(|c| c.is_ascii_digit()));
    }
    
    #[tokio::test]
    async fn test_pairing_code_verification() {
        let store = PairingStore::new();
        let code = store.generate_code("device-1").await;
        
        // Valid verification
        let result = store.verify_code(&code, "device-1").await;
        assert!(result.is_some());
        assert_eq!(result.unwrap(), "device-1");
        
        // Wrong device should fail
        let result = store.verify_code(&code, "device-2").await;
        assert!(result.is_none());
    }
}
