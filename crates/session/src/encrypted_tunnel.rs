//! Encrypted tunnel for video and control channels
//!
//! Uses AES-GCM to encrypt all data flowing through the connection.

use aes_gcm::{Aes256Gcm, KeyInit, aead::{Aead, Payload}};
use generic_array::GenericArray;
use std::io::{Read, Write};
use std::net::TcpStream;
use tracing::{info, warn, debug};

/// Nonce counter for AES-GCM (12 bytes)
const NONCE_SIZE: usize = 12;
const TAG_SIZE: usize = 16;

/// Encrypted tunnel wrapping a TCP stream
pub struct EncryptedTunnel {
    stream: TcpStream,
    encrypt_key: Aes256Gcm,
    decrypt_key: Aes256Gcm,
    encrypt_nonce: u64,
    decrypt_nonce: u64,
}

impl EncryptedTunnel {
    /// Create new encrypted tunnel from shared secret
    pub fn new(stream: TcpStream, shared_secret: &[u8; 32]) -> Self {
        use sha2::{Sha256, Digest};
        
        // Derive separate keys for encrypt and decrypt
        let mut enc_hash = Sha256::new();
        enc_hash.update(shared_secret);
        enc_hash.update(b"atlas-encrypt-key");
        let enc_key: [u8; 32] = enc_hash.finalize().into();
        
        let mut dec_hash = Sha256::new();
        dec_hash.update(shared_secret);
        dec_hash.update(b"atlas-decrypt-key");
        let dec_key: [u8; 32] = dec_hash.finalize().into();
        
        Self {
            stream,
            encrypt_key: Aes256Gcm::new_from_slice(&enc_key).expect("AES key invalid"),
            decrypt_key: Aes256Gcm::new_from_slice(&dec_key).expect("AES key invalid"),
            encrypt_nonce: 0,
            decrypt_nonce: 0,
        }
    }
    
    /// Encrypt and send data
    pub fn send(&mut self, data: &[u8]) -> std::io::Result<usize> {
        let nonce = self.make_nonce(self.encrypt_nonce);
        self.encrypt_nonce += 1;
        
        let ciphertext = self.encrypt_key.encrypt(&nonce, data)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
        
        // Prepend nonce to ciphertext
        let mut packet = Vec::with_capacity(NONCE_SIZE + ciphertext.len());
        packet.extend_from_slice(&nonce);
        packet.extend_from_slice(&ciphertext);
        
        self.stream.write_all(&packet)
    }
    
    /// Receive and decrypt data
    pub fn recv(&mut self) -> std::io::Result<Vec<u8>> {
        // Read nonce (12 bytes)
        let mut nonce_buf = [0u8; NONCE_SIZE];
        self.stream.read_exact(&mut nonce_buf)?;
        
        // Read ciphertext (variable length, we need to know the size)
        // For simplicity, read until we have enough
        let mut ciphertext_buf = vec![0u8; 65535];
        let n = self.stream.read(&mut ciphertext_buf)?;
        
        let nonce = GenericArray::from_slice(&nonce_buf);
        let ciphertext = &ciphertext_buf[..n];
        
        let plaintext = self.decrypt_key.decrypt(nonce, ciphertext)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
        
        Ok(plaintext)
    }
    
    fn make_nonce(&mut self, counter: u64) -> [u8; NONCE_SIZE] {
        let mut nonce = [0u8; NONCE_SIZE];
        nonce[0..8].copy_from_slice(&counter.to_be_bytes());
        nonce
    }
}

/// Stream adapter for EncryptedTunnel
pub struct TunnelStream {
    tunnel: EncryptedTunnel,
}

impl TunnelStream {
    pub fn new(stream: TcpStream, shared_secret: &[u8; 32]) -> Self {
        Self {
            tunnel: EncryptedTunnel::new(stream, shared_secret),
        }
    }
}

impl Read for TunnelStream {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        // Read encrypted packet
        let packet = self.tunnel.recv()?;
        let len = std::cmp::min(packet.len(), buf.len());
        buf[..len].copy_from_slice(&packet[..len]);
        Ok(len)
    }
}

impl Write for TunnelStream {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.tunnel.send(buf)
    }
    
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

/// Connection metrics
#[derive(Debug, Clone)]
pub struct TunnelMetrics {
    pub bytes_encrypted: u64,
    pub packets_encrypted: u64,
    pub bytes_decrypted: u64,
    pub packets_decrypted: u64,
    pub errors: u64,
}

impl Default for TunnelMetrics {
    fn default() -> Self {
        Self {
            bytes_encrypted: 0,
            packets_encrypted: 0,
            bytes_decrypted: 0,
            packets_decrypted: 0,
            errors: 0,
        }
    }
}

impl TunnelMetrics {
    pub fn record_encrypt(&mut self, bytes: usize) {
        self.bytes_encrypted += bytes as u64;
        self.packets_encrypted += 1;
    }
    
    pub fn record_decrypt(&mut self, bytes: usize) {
        self.bytes_decrypted += bytes as u64;
        self.packets_decrypted += 1;
    }
    
    pub fn record_error(&mut self) {
        self.errors += 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_nonce_generation() {
        let mut enc = EncryptedTunnel {
            stream: std::net::TcpStream::connect("127.0.0.1:1").unwrap_or_else(|_| {
                // Create a dummy stream for test
                std::net::TcpStream::connect("127.0.0.1:1").unwrap()
            }),
            encrypt_key: Aes256Gcm::new_from_slice(&[0u8; 32]).unwrap(),
            decrypt_key: Aes256Gcm::new_from_slice(&[0u8; 32]).unwrap(),
            encrypt_nonce: 0,
            decrypt_nonce: 0,
        };
        
        let n1 = enc.make_nonce(0);
        let n2 = enc.make_nonce(1);
        assert_ne!(n1, n2);
    }
}
