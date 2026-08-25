//! Device ID module

use rand::Rng;
use sha2::{Sha256, Digest};

/// 8-character device ID
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct DeviceId(pub String);

impl DeviceId {
    /// Generate a random 8-character device ID
    pub fn generate() -> Self {
        const CHARS: &[u8] = b"ABCDEFGHJKLMNPQRSTUVWXYZ23456789";
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

impl Default for DeviceId {
    fn default() -> Self {
        Self::generate()
    }
}

impl std::fmt::Display for DeviceId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}
