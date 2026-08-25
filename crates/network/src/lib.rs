//! Atlas Network - Network utilities with STUN/TURN/QUIC support

pub mod discovery;
pub mod stun;
pub mod turn;
pub mod quic;

pub use discovery::{DiscoveredDevice, DiscoveryManager};
pub use stun::{StunClient, NatType};
pub use turn::TurnClient;
pub use quic::{QuicClient, QuicServer, QuicConnection, NetworkMetrics};

use std::net::SocketAddr;

#[derive(Debug, Clone)]
pub struct NetworkConfig {
    pub stun_servers: Vec<String>,
    pub turn_server: Option<String>,
    pub relay_enabled: bool,
    pub preferred_transport: String,
}

impl Default for NetworkConfig {
    fn default() -> Self {
        Self {
            stun_servers: vec![
                "198.199.101.193:19302".to_string(),
                "stun.l.google.com:19302".to_string(),
            ],
            turn_server: None,
            relay_enabled: true,
            preferred_transport: "quic".to_string(),
        }
    }
}

/// Parse a socket address string
pub fn parse_addr(addr: &str) -> Result<SocketAddr, String> {
    addr.parse::<SocketAddr>().map_err(|e| e.to_string())
}

/// Get default STUN server address
pub fn default_stun_addr() -> SocketAddr {
    "198.199.101.193:19302".parse().unwrap()
}

/// Check if address is valid
pub fn is_valid_addr(addr: &SocketAddr) -> bool {
    !addr.ip().is_unspecified() && addr.port() > 0
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_network_config() {
        let config = NetworkConfig::default();
        assert!(!config.stun_servers.is_empty());
        assert_eq!(config.preferred_transport, "quic");
    }
    
    #[test]
    fn test_parse_addr() {
        let addr = parse_addr("127.0.0.1:8080").unwrap();
        assert_eq!(addr.port(), 8080);
    }
    
    #[test]
    fn test_default_stun() {
        let addr = default_stun_addr();
        assert_eq!(addr.port(), 19302);
    }
    
    #[test]
    fn test_is_valid_addr() {
        assert!(is_valid_addr(&"127.0.0.1:8080".parse().unwrap()));
        assert!(!is_valid_addr(&"0.0.0.0:0".parse().unwrap()));
    }
}
