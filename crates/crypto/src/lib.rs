//! Atlas Crypto - Cryptography primitives
//!
//! Provides Ed25519 signing, X25519 key exchange, AES-GCM encryption, and handshake.

pub mod error;
pub mod keypair;
pub mod session;
pub mod handshake;

pub use error::*;
pub use keypair::*;
pub use session::*;
pub use handshake::{EncryptedSession, perform_handshake, HandshakeMessage};
