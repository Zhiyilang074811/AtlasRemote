//! Atlas Multi-Monitor Support
//!
//! Supports multiple displays with selection and switching.

use tracing::{info, warn};

/// Monitor information
#[derive(Debug, Clone)]
pub struct MonitorInfo {
    pub index: u32,
    pub name: String,
    pub width: u32,
    pub height: u32,
    pub is_primary: bool,
}

impl std::fmt::Display for MonitorInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} ({}) {}x{}{}", 
            if self.is_primary { "* " } else { "" },
            self.name,
            self.width, self.height,
            if self.is_primary { " [Primary]" } else { "" }
        )
    }
}

/// Multi-monitor manager
#[derive(Debug)]
pub struct MultiMonitorManager {
    monitors: Vec<MonitorInfo>,
    selected_index: u32,
}

impl MultiMonitorManager {
    pub fn new() -> Self {
        Self {
            monitors: Vec::new(),
            selected_index: 0,
        }
    }
    
    /// Add a monitor
    pub fn add_monitor(&mut self, info: MonitorInfo) {
        info!("Added monitor: {}", info);
        self.monitors.push(info);
    }
    
    /// Get all monitors
    pub fn monitors(&self) -> &[MonitorInfo] {
        &self.monitors
    }
    
    /// Get selected monitor
    pub fn selected(&self) -> Option<&MonitorInfo> {
        self.monitors.get(self.selected_index as usize)
    }
    
    /// Select monitor by index
    pub fn select(&mut self, index: u32) -> Result<(), String> {
        if index >= self.monitors.len() as u32 {
            return Err(format!("Invalid monitor index: {}", index));
        }
        self.selected_index = index;
        info!("Selected monitor: {}", index);
        Ok(())
    }
    
    /// Select primary monitor
    pub fn select_primary(&mut self) -> Result<(), String> {
        if let Some(primary) = self.monitors.iter().position(|m| m.is_primary) {
            self.select(primary as u32)
        } else {
            self.select(0)
        }
    }
    
    /// Cycle to next monitor
    pub fn next(&mut self) {
        if self.monitors.is_empty() {
            return;
        }
        self.selected_index = (self.selected_index + 1) % self.monitors.len() as u32;
        info!("Cycled to monitor: {}", self.selected_index);
    }
    
    /// Get monitor count
    pub fn count(&self) -> usize {
        self.monitors.len()
    }
}

impl Default for MultiMonitorManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Scan for available monitors (placeholder)
pub fn scan_monitors() -> Vec<MonitorInfo> {
    // TODO: Implement real monitor detection using DXGI
    // For now, return a single virtual monitor
    vec![MonitorInfo {
        index: 0,
        name: "Primary Display".to_string(),
        width: 1920,
        height: 1080,
        is_primary: true,
    }]
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_multi_monitor() {
        let mut manager = MultiMonitorManager::new();
        
        manager.add_monitor(MonitorInfo {
            index: 0,
            name: "Monitor 1".to_string(),
            width: 1920,
            height: 1080,
            is_primary: true,
        });
        
        manager.add_monitor(MonitorInfo {
            index: 1,
            name: "Monitor 2".to_string(),
            width: 1280,
            height: 720,
            is_primary: false,
        });
        
        assert_eq!(manager.count(), 2);
        assert!(manager.selected().is_some());
        
        manager.select(1).unwrap();
        assert_eq!(manager.selected().unwrap().index, 1);
    }
    
    #[test]
    fn test_invalid_select() {
        let mut manager = MultiMonitorManager::new();
        assert!(manager.select(99).is_err());
    }
}
