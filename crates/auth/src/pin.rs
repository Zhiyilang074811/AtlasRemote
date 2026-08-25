//! PIN code module

use rand::Rng;

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

impl Default for PinCode {
    fn default() -> Self {
        Self::generate()
    }
}

impl std::fmt::Display for PinCode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}
