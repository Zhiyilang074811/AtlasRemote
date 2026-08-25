//! Atlas Config - Configuration management
//!
//! Supports TOML config files with hot-reload capability.

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub network: NetworkConfig,
    pub video: VideoConfig,
    pub security: SecurityConfig,
    pub logging: LoggingConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkConfig {
    pub preferred_transport: String,
    pub allow_relay: bool,
    pub stun_servers: Vec<String>,
    pub relay_enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VideoConfig {
    pub max_fps: u32,
    pub codec: String,
    pub resolution_width: u32,
    pub resolution_height: u32,
    pub max_bitrate: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityConfig {
    pub require_pair: bool,
    pub key_rotation_hours: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoggingConfig {
    pub level: String,
    pub filter_sensitive: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            network: NetworkConfig {
                preferred_transport: "quic".to_string(),
                allow_relay: true,
                stun_servers: vec!["stun.l.google.com:19302".to_string()],
                relay_enabled: true,
            },
            video: VideoConfig {
                max_fps: 60,
                codec: "auto".to_string(),
                resolution_width: 1920,
                resolution_height: 1080,
                max_bitrate: 8_000_000,
            },
            security: SecurityConfig {
                require_pair: true,
                key_rotation_hours: 24,
            },
            logging: LoggingConfig {
                level: "info".to_string(),
                filter_sensitive: true,
            },
        }
    }
}

impl Config {
    pub fn load(path: &Path) -> Result<Self, Box<dyn std::error::Error>> {
        if path.exists() {
            let content = std::fs::read_to_string(path)?;
            Ok(toml::from_str(&content)?)
        } else {
            Ok(Self::default())
        }
    }

    pub fn load_or_default() -> Self {
        let config_path = PathBuf::from("config/default.toml");
        Self::load(&config_path).unwrap_or_default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = Config::default();
        assert_eq!(config.network.preferred_transport, "quic");
        assert_eq!(config.video.max_fps, 60);
        assert!(config.network.allow_relay);
    }
}
