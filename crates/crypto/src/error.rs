//! Cryptography error types

use thiserror::Error;

#[derive(Error, Debug)]
pub enum CryptoError {
    #[error("Invalid key")]
    InvalidKey,
    #[error("Encryption failed: {0}")]
    EncryptionFailed(String),
    #[error("Decryption failed: {0}")]
    DecryptionFailed(String),
    #[error("Signature verification failed")]
    SignatureFailed,
}

pub type Result<T> = std::result::Result<T, CryptoError>;
