use thiserror::Error;

#[derive(Error, Debug)]
pub enum AtlasError {
    #[error("Crypto error: {0}")]
    Crypto(String),
    #[error("Network error: {0}")]
    Network(String),
    #[error("Device error: {0}")]
    Device(String),
    #[error("Capture error: {0}")]
    Capture(String),
    #[error("Codec error: {0}")]
    Codec(String),
    #[error("Input error: {0}")]
    Input(String),
    #[error("Permission denied: {0}")]
    Permission(String),
    #[error("Internal error: {0}")]
    Internal(String),
    #[error("Protocol error: {0}")]
    Protocol(String),
    #[error("Configuration error: {0}")]
    Config(String),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
}

pub type Result<T> = std::result::Result<T, AtlasError>;

pub trait IntoAtlasError {
    fn into_atlas_error(self) -> AtlasError;
}

impl<T: std::fmt::Debug> IntoAtlasError for std::result::Result<T, AtlasError> {
    fn into_atlas_error(self) -> AtlasError {
        self.unwrap_err()
    }
}
