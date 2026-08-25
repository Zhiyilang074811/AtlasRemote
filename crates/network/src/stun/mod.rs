//! STUN Client - NAT discovery and hole punching
//!
//! Implements RFC 5389 STUN protocol for discovering public IP and port mappings.
//! Supports both REQUEST and BINDING methods.

use std::net::SocketAddr;
use std::time::{Duration, Instant};
use tracing::{info, warn, debug};

/// STUN message types
const STUN_CLASS_REQUEST: u16 = 0x0001;
const STUN_METHOD_BINDING: u16 = 0x0001;
const STUN_METHOD_SHARED_EXT: u16 = 0x0002;

/// STUN message class
const STUN_CLASS_RESPONSE: u16 = 0x0100;
const STUN_CLASS_ERROR: u16 = 0x0200;

/// Attribute types
const ATTR_MAPPED_ADDRESS: u16 = 0x0001;
const ATTR_XOR_MAPPED_ADDRESS: u16 = 0x0020;

/// STUN transaction ID length
const STUN_TRANSACTION_ID_LEN: usize = 12;

/// Build a STUN Binding Request
pub fn build_binding_request(transaction_id: &[u8; STUN_TRANSACTION_ID_LEN]) -> Vec<u8> {
    let mut msg = Vec::with_capacity(28);
    // Type: Binding Request
    msg.extend_from_slice(&[(STUN_CLASS_REQUEST >> 8) as u8, STUN_CLASS_REQUEST as u8]);
    // Method: Binding
    msg.extend_from_slice(&[(STUN_METHOD_BINDING >> 8) as u8, STUN_METHOD_BINDING as u8]);
    // Length: 0 (no attributes)
    msg.extend_from_slice(&[0u8, 0u8]);
    // Transaction ID
    msg.extend_from_slice(transaction_id);
    msg
}

/// Parse STUN response to extract mapped address
pub fn parse_mapped_address(response: &[u8]) -> Option<SocketAddr> {
    if response.len() < 20 {
        return None;
    }
    
    // Parse message type
    let msg_type = u16::from_be_bytes([response[0], response[1]]);
    let msg_class = msg_type & 0x01F0;
    let msg_method = msg_type & 0x000F;
    
    // Must be Binding Response
    if msg_class != STUN_CLASS_RESPONSE || msg_method != STUN_METHOD_BINDING {
        return None;
    }
    
    // Parse length
    let attr_len = u16::from_be_bytes([response[2], response[3]]) as usize;
    if response.len() < 20 + attr_len {
        return None;
    }
    
    // Parse transaction ID
    let tid = &response[4..16];
    debug!("STUN response TID: {:02X?}", tid);
    
    // Parse attributes starting at offset 20
    let mut offset = 20;
    while offset + 4 <= response.len() {
        let attr_type = u16::from_be_bytes([response[offset], response[offset + 1]]);
        let attr_len = u16::from_be_bytes([response[offset + 2], response[offset + 3]]) as usize;
        
        if offset + 4 + attr_len > response.len() {
            break;
        }
        
        let attr_value = &response[offset + 4..offset + 4 + attr_len];
        
        if attr_type == ATTR_MAPPED_ADDRESS && attr_len >= 8 {
            // Parse IPv4 mapped address
            // Format: 0x00 | family(1) | port(2) | ip(4)
            let family = attr_value[0];
            if family == 0x01 { // IPv4
                let port = u16::from_be_bytes([attr_value[1], attr_value[2]]);
                let ip = std::net::Ipv4Addr::new(
                    attr_value[4], attr_value[5], attr_value[6], attr_value[7]
                );
                return Some(SocketAddr::from((ip, port)));
            }
        }
        
        offset += 4 + attr_len;
        // Align to 4-byte boundary
        offset = (offset + 3) & !3;
    }
    
    None
}

/// Parse XOR-mapped address (RFC 5389)
pub fn parse_xor_mapped_address(response: &[u8], transaction_id: &[u8; STUN_TRANSACTION_ID_LEN]) -> Option<SocketAddr> {
    if response.len() < 20 {
        return None;
    }
    
    let mut offset = 20;
    while offset + 4 <= response.len() {
        let attr_type = u16::from_be_bytes([response[offset], response[offset + 1]]);
        let attr_len = u16::from_be_bytes([response[offset + 2], response[offset + 3]]) as usize;
        
        if offset + 4 + attr_len > response.len() {
            break;
        }
        
        let attr_value = &response[offset + 4..offset + 4 + attr_len];
        
        if attr_type == ATTR_XOR_MAPPED_ADDRESS && attr_len >= 8 {
            // XOR-mapped address: port and IP are XORed with transaction ID prefix
            let xor_port = u16::from_be_bytes([attr_value[1], attr_value[2]]) ^ u16::from_be_bytes([transaction_id[0], transaction_id[1]]);
            let xor_ip = [
                attr_value[4] ^ transaction_id[2],
                attr_value[5] ^ transaction_id[3],
                attr_value[6] ^ transaction_id[4],
                attr_value[7] ^ transaction_id[5],
            ];
            let ip = std::net::Ipv4Addr::new(xor_ip[0], xor_ip[1], xor_ip[2], xor_ip[3]);
            return Some(SocketAddr::from((ip, xor_port)));
        }
        
        offset += 4 + attr_len;
        offset = (offset + 3) & !3;
    }
    
    None
}

/// Generate random transaction ID
pub fn generate_transaction_id() -> [u8; STUN_TRANSACTION_ID_LEN] {
    use rand::Rng;
    let mut id = [0u8; STUN_TRANSACTION_ID_LEN];
    rand::thread_rng().fill(&mut id);
    id
}

/// STUN client for NAT discovery
#[derive(Debug, Clone)]
pub struct StunClient {
    server_addr: SocketAddr,
    timeout: Duration,
    retries: u32,
}

impl StunClient {
    pub fn new(server_addr: SocketAddr) -> Self {
        Self {
            server_addr,
            timeout: Duration::from_secs(3),
            retries: 3,
        }
    }
    
    /// Discover public address using STUN
    pub async fn discover(&self) -> Option<SocketAddr> {
        for attempt in 1..=self.retries {
            info!("STUN discovery attempt {}/{}", attempt, self.retries);
            
            let tid = generate_transaction_id();
            let request = build_binding_request(&tid);
            
            // Send request
            match tokio::time::timeout(self.timeout, self.send_request(&request)).await {
                Ok(Ok(response)) => {
                    // Try standard mapping first
                    if let Some(addr) = parse_mapped_address(&response) {
                        info!("STUN discovered: {} (standard)", addr);
                        return Some(addr);
                    }
                    // Try XOR mapping
                    if let Some(addr) = parse_xor_mapped_address(&response, &tid) {
                        info!("STUN discovered: {} (XOR)", addr);
                        return Some(addr);
                    }
                }
                Ok(Err(e)) => {
                    warn!("STUN request failed: {}", e);
                }
                Err(_) => {
                    warn!("STUN request timeout");
                }
            }
            
            if attempt < self.retries {
                tokio::time::sleep(Duration::from_millis(500)).await;
            }
        }
        
        warn!("STUN discovery failed after {} attempts", self.retries);
        None
    }
    
    async fn send_request(&self, request: &[u8]) -> Result<Vec<u8>, std::io::Error> {
        let (mut socket, mut response) = tokio::net::UdpSocket::bind("0.0.0.0:0").await?;
        socket.send_to(request, self.server_addr).await?;
        
        let mut buf = vec![0u8; 65535];
        let n = socket.recv(&mut buf).await?;
        Ok(buf[..n].to_vec())
    }
}

/// NAT type detection
#[derive(Debug, Clone, PartialEq)]
pub enum NatType {
    OpenInternet,
    FullCone,
    RestrictedCone,
    PortRestrictedCone,
    Symmetric,
}

impl NatType {
    /// Determine NAT type based on address responses
    pub fn detect(cases: &[NatTestCase]) -> Self {
        // Simplified NAT detection
        // In practice, need multiple test cases
        if cases.len() < 2 {
            return Self::Unknown;
        }
        
        let addr1 = cases[0].response_addr;
        let addr2 = cases[1].response_addr;
        
        if addr1 == addr2 {
            Self::FullCone
        } else {
            Self::Symmetric
        }
    }
}

#[derive(Debug, Clone)]
pub struct NatTestCase {
    pub response_addr: Option<SocketAddr>,
    pub expected_same: bool,
}

impl NatType {
    pub const Unknown: Self = Self::OpenInternet; // Placeholder
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_transaction_id_length() {
        let id = generate_transaction_id();
        assert_eq!(id.len(), STUN_TRANSACTION_ID_LEN);
    }
    
    #[test]
    fn test_build_request() {
        let tid = [0u8; STUN_TRANSACTION_ID_LEN];
        let request = build_binding_request(&tid);
        assert_eq!(request.len(), 28);
        assert_eq!(u16::from_be_bytes([request[0], request[1]]), STUN_CLASS_REQUEST);
    }
}
