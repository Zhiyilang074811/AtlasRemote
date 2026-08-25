//! Atlas Device - Device identity management with pairing support

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceInfo {
    pub id: String,
    pub name: String,
    pub platform: String,
    pub public_key: Vec<u8>,
    pub paired_with: Vec<String>,
    pub last_connected: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceStore {
    pub devices: HashMap<String, DeviceInfo>,
    pub trusted_devices: Vec<String>,
}

impl DeviceStore {
    pub fn new() -> Self {
        Self {
            devices: HashMap::new(),
            trusted_devices: Vec::new(),
        }
    }

    pub fn register(&mut self, device: DeviceInfo) {
        self.devices.insert(device.id.clone(), device);
    }

    pub fn get(&self, id: &str) -> Option<&DeviceInfo> {
        self.devices.get(id)
    }

    pub fn is_trusted(&self, device_id: &str) -> bool {
        self.trusted_devices.contains(&device_id.to_string())
    }

    pub fn add_trusted(&mut self, device_id: &str) {
        if !self.trusted_devices.contains(&device_id.to_string()) {
            self.trusted_devices.push(device_id.to_string());
        }
    }
}

impl Default for DeviceStore {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_device_store() {
        let mut store = DeviceStore::new();
        let device = DeviceInfo {
            id: "test-1".to_string(),
            name: "Test Device".to_string(),
            platform: "windows".to_string(),
            public_key: vec![1, 2, 3],
            paired_with: vec![],
            last_connected: None,
        };
        store.register(device);
        assert!(store.get("test-1").is_some());
        assert!(!store.is_trusted("test-1"));
        store.add_trusted("test-1");
        assert!(store.is_trusted("test-1"));
    }
}
