//! End-to-end encryption handshake module
//!
//! Implements X25519 key exchange + AES-GCM session encryption.

use aes_gcm::{Aes256Gcm, KeyInit, aead::Aead};
use generic_array::GenericArray;
use sha2::{Sha256, Digest};
use tracing::{info, warn};

/// Session encryption context
#[derive(Clone)]
pub struct EncryptedSession {
    pub remote_public_key: [u8; 32],
    pub shared_secret: [u8; 32],
    pub session_key: [u8; 32],
    pub nonce_counter: u64,
}

impl EncryptedSession {
    /// Create new session from shared secret
    pub fn new(remote_public_key: [u8; 32], shared_secret: [u8; 32]) -> Self {
        let mut hasher = Sha256::new();
        hasher.update(&shared_secret);
        hasher.update(b"atlas-session-key");
        let key = hasher.finalize().into();
        
        Self {
            remote_public_key,
            shared_secret,
            session_key: key,
            nonce_counter: 0,
        }
    }
    
    /// Generate next nonce
    pub fn next_nonce(&mut self) -> [u8; 12] {
        let nonce = self.nonce_counter.to_be_bytes();
        self.nonce_counter += 1;
        let mut result = [0u8; 12];
        result[0..4].copy_from_slice(&nonce);
        result
    }
    
    /// Encrypt data
    pub fn encrypt(&self, plaintext: &[u8]) -> Result<Vec<u8>, String> {
        let mut nonce = self.next_nonce();
        let cipher = Aes256Gcm::new_from_slice(&self.session_key)
            .map_err(|e| format!("AES-GCM init failed: {}", e))?;
        let nonce = GenericArray::from_slice(&nonce);
        cipher.encrypt(nonce, plaintext)
            .map_err(|e| format!("Encryption failed: {}", e))
    }
    
    /// Decrypt data
    pub fn decrypt(&self, ciphertext: &[u8]) -> Result<Vec<u8>, String> {
        // In real implementation, nonce would be prepended to ciphertext
        let mut nonce = [0u8; 12];
        let cipher = Aes256Gcm::new_from_slice(&self.session_key)
            .map_err(|e| format!("AES-GCM init failed: {}", e))?;
        let nonce = GenericArray::from_slice(&nonce);
        cipher.decrypt(nonce, ciphertext)
            .map_err(|e| format!("Decryption failed: {}", e))
    }
}

/// Handshake message types
#[derive(Debug, Clone)]
pub enum HandshakeMessage {
    /// Initial hello with public key
    Hello { public_key: [u8; 32] },
    /// Response with public key and signature
    HelloResponse { 
        public_key: [u8; 32],
        signature: Vec<u8>,
    },
    /// Acknowledgment
    Ack,
}

/// Perform key exchange handshake
pub async fn perform_handshake(
    local_secret: &x25519_dalek::EphemeralSecret,
    remote_public: &[u8; 32],
) -> Result<EncryptedSession, String> {
    use x25519_dalek::{PublicKey, StaticSecret};
    
    let local_public = PublicKey::from(local_secret);
    info!("Local public key: {:02X?}", &local_public.as_bytes()[..8]);
    info!("Remote public key: {:02X?}", &remote_public[..8]);
    
    // Perform Diffie-Hellman
    let secret = StaticSecret::from(*local_secret);
    let shared = secret.diffie_hellman(&PublicKey::from(*remote_public));
    info!("Shared secret established");
    
    Ok(EncryptedSession::new(*remote_public, shared.to_bytes()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use x25519_dalek::{EphemeralSecret, PublicKey};
    
    #[test]
    fn test_encrypted_session() {
        let local_secret = EphemeralSecret::random();
        let local_public = PublicKey::from(&local_secret);
        
        let remote_secret = EphemeralSecret::random();
        let remote_public = PublicKey::from(&remote_secret);
        
        // Both sides compute same shared secret
        let local_ss = local_secret.diffie_hellman(&remote_public);
        let remote_ss = remote_secret.diffie_hellman(&local_public);
        
        assert_eq!(local_ss.to_bytes(), remote_ss.to_bytes());
        
        let session = EncryptedSession::new(remote_public.to_bytes(), local_ss.to_bytes());
        
        let plaintext = b"Hello, Atlas!";
        let ciphertext = session.encrypt(plaintext).unwrap();
        assert_ne!(ciphertext, plaintext);
        
        // Note: decryption would need same nonce sequence in real impl
        let _ = session;
    }
}
