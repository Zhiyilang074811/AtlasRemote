//! mDNS Device Discovery

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tokio::sync::mpsc;
use tracing::{info, warn};

#[derive(Debug, Clone)]
pub struct DiscoveredDevice {
    pub name: String,
    pub ip: String,
    pub port: u16,
    pub device_id: String,
    pub public_key: Vec<u8>,
}

impl DiscoveredDevice {
    pub fn new(name: &str, ip: &str, port: u16, device_id: &str, public_key: &[u8]) -> Self {
        Self {
            name: name.to_string(),
            ip: ip.to_string(),
            port,
            device_id: device_id.to_string(),
            public_key: public_key.to_vec(),
        }
    }
}

pub struct DiscoveryManager {
    services: Arc<Mutex<HashMap<String, DiscoveredDevice>>>,
}

impl DiscoveryManager {
    pub fn new() -> Self {
        Self {
            services: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub fn add_device(&self, device: DiscoveredDevice) {
        let mut services = self.services.lock().unwrap();
        services.insert(device.device_id.clone(), device);
    }

    pub fn remove_device(&self, device_id: &str) {
        let mut services = self.services.lock().unwrap();
        services.remove(device_id);
    }

    pub fn get_devices(&self) -> Vec<DiscoveredDevice> {
        let services = self.services.lock().unwrap();
        services.values().cloned().collect()
    }

    pub fn is_trusted(&self, device_id: &str) -> bool {
        // Check trusted_devices.json
        let path = format!(
            "{}/trusted_devices.json",
            std::env::var("ATLAS_DATA_DIR").unwrap_or_else(|_| ".".to_string())
        );
        if std::path::Path::new(&path).exists() {
            if let Ok(content) = std::fs::read_to_string(&path) {
                if let Ok(devices) = serde_json::from_str::<Vec<serde_json::Value>>(&content) {
                    devices.iter().any(|d| {
                        d.get("device_id")
                            .and_then(|v| v.as_str())
                            .map(|id| id == device_id)
                            .unwrap_or(false)
                    })
                } else {
                    false
                }
            } else {
                false
            }
        } else {
            false
        }
    }
}

pub async fn start_discovery(
    tx: mpsc::Sender<DiscoveredDevice>,
) -> Result<(), Box<dyn std::error::Error>> {
    info!("Starting mDNS discovery for _atlasremote._tcp");

    // Note: Full mDNS implementation requires native dependencies.
    // For now, we implement a simple LAN scan as fallback.
    spawn_lan_scan(tx).await;

    Ok(())
}

async fn spawn_lan_scan(
    tx: mpsc::Sender<DiscoveredDevice>,
) {
    // Simplified LAN scan - in production, use mdns crate
    info!("LAN scan not yet implemented - use manual pairing");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_discovery_manager() {
        let manager = DiscoveryManager::new();
        assert!(manager.get_devices().is_empty());

        let device = DiscoveredDevice::new(
            "Y700P-RTX3050",
            "192.168.1.100",
            8080,
            "device-1",
            &[1, 2, 3],
        );
        manager.add_device(device.clone());
        assert_eq!(manager.get_devices().len(), 1);
        assert_eq!(manager.get_devices()[0].name, "Y700P-RTX3050");
    }

    #[test]
    fn test_remove_device() {
        let manager = DiscoveryManager::new();
        let device = DiscoveredDevice::new("test", "127.0.0.1", 8080, "dev-1", &[1]);
        manager.add_device(device);
        manager.remove_device("dev-1");
        assert!(manager.get_devices().is_empty());
    }
}
