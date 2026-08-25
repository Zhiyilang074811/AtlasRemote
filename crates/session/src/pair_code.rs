//! 6-digit pairing code generation and validation

use rand::Rng;
use std::fmt;

/// 6-digit numeric pairing code
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct PairCode(pub String);

impl PairCode {
    /// Generate a random 6-digit pairing code
    pub fn generate() -> Self {
        let mut rng = rand::thread_rng();
        let code: String = (0..6)
            .map(|_| rng.gen_range(0..10).to_string())
            .collect();
        Self(code)
    }
    
    /// Validate pairing code format (exactly 6 digits)
    pub fn is_valid(&self) -> bool {
        self.0.len() == 6 && self.0.chars().all(|c| c.is_ascii_digit())
    }
    
    /// Check if code has expired (5-minute TTL)
    pub fn is_expired(&self, created_at: chrono::DateTime<chrono::Utc>) -> bool {
        chrono::Utc::now().signed_duration_since(created_at).num_minutes() >= 5
    }
}

impl Default for PairCode {
    fn default() -> Self { Self::generate() }
}

impl fmt::Display for PairCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Format code with dashes for readability: 123-456
impl PairCode {
    pub fn display(&self) -> String {
        format!("{}-{}", &self.0[..3], &self.0[3..])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_generate() {
        let code = PairCode::generate();
        assert_eq!(code.0.len(), 6);
        assert!(code.is_valid());
    }
    
    #[test]
    fn test_format() {
        let code = PairCode("123456".to_string());
        assert_eq!(code.display(), "123-456");
    }
    
    #[test]
    fn test_invalid() {
        assert!(!PairCode("12345".to_string()).is_valid());
        assert!(!PairCode("1234567".to_string()).is_valid());
        assert!(!PairCode("1234ab".to_string()).is_valid());
    }
}
