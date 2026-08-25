//! Atlas ABR - Adaptive Bitrate Research
//!
//! Dynamically adjusts encoding quality based on network conditions.

use std::time::{Duration, Instant};
use tracing::{info, warn};

/// Network quality metrics
#[derive(Debug, Clone)]
pub struct NetworkMetrics {
    pub rtt: Duration,
    pub packet_loss: f32,
    pub bandwidth_bps: u64,
    pub jitter: Duration,
}

impl Default for NetworkMetrics {
    fn default() -> Self {
        Self {
            rtt: Duration::from_millis(50),
            packet_loss: 0.0,
            bandwidth_bps: 5_000_000,
            jitter: Duration::from_millis(5),
        }
    }
}

/// Encoding quality levels
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QualityLevel {
    UltraLow,    // 360p, 15fps, 500kbps
    Low,         // 720p, 24fps, 1Mbps
    Medium,      // 960p, 30fps, 2Mbps
    High,        // 1080p, 55fps, 4Mbps
    Ultra,       // 1080p, 60fps, 8Mbps
}

impl QualityLevel {
    pub fn resolution(&self) -> (u32, u32) {
        match self {
            Self::UltraLow => (640, 360),
            Self::Low => (1280, 720),
            Self::Medium => (1280, 720),
            Self::High => (1920, 1080),
            Self::Ultra => (1920, 1080),
        }
    }
    
    pub fn target_fps(&self) -> u32 {
        match self {
            Self::UltraLow => 15,
            Self::Low => 24,
            Self::Medium => 30,
            Self::High => 55,
            Self::Ultra => 60,
        }
    }
    
    pub fn target_bitrate(&self) -> u32 {
        match self {
            Self::UltraLow => 500_000,
            Self::Low => 1_000_000,
            Self::Medium => 2_000_000,
            Self::High => 4_000_000,
            Self::Ultra => 8_000_000,
        }
    }
}

impl std::fmt::Display for QualityLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UltraLow => write!(f, "360p/15fps"),
            Self::Low => write!(f, "720p/24fps"),
            Self::Medium => write!(f, "720p/30fps"),
            Self::High => write!(f, "1080p/55fps"),
            Self::Ultra => write!(f, "1080p/60fps"),
        }
    }
}

/// ABR controller
#[derive(Debug)]
pub struct AbcController {
    current_quality: QualityLevel,
    metrics: NetworkMetrics,
    last_adjustment: Instant,
    adjustment_cooldown: Duration,
}

impl AbcController {
    pub fn new() -> Self {
        Self {
            current_quality: QualityLevel::High,
            metrics: NetworkMetrics::default(),
            last_adjustment: Instant::now(),
            adjustment_cooldown: Duration::from_secs(10),
        }
    }
    
    /// Update metrics and potentially adjust quality
    pub fn update_metrics(&mut self, metrics: NetworkMetrics) {
        self.metrics = metrics;
        
        if self.last_adjustment.elapsed() < self.adjustment_cooldown {
            return;
        }
        
        self.adjust_quality();
    }
    
    fn adjust_quality(&mut self) {
        let needs_upgrade = self.metrics.bandwidth_bps > 6_000_000 
            && self.metrics.packet_loss < 0.01
            && self.metrics.rtt < Duration::from_millis(50);
        
        let needs_downgrade = self.metrics.packet_loss > 0.05
            || self.metrics.rtt > Duration::from_millis(200)
            || self.metrics.bandwidth_bps < 1_000_000;
        
        if needs_downgrade && self.current_quality != QualityLevel::UltraLow {
            self.current_quality = self.current_quality.degrade();
            info!("ABR: Downgraded to {}", self.current_quality);
        } else if needs_upgrade && self.current_quality != QualityLevel::Ultra {
            self.current_quality = self.current_quality.upgrade();
            info!("ABR: Upgraded to {}", self.current_quality);
        }
        
        self.last_adjustment = Instant::now();
    }
    
    pub fn current_quality(&self) -> QualityLevel { self.current_quality }
    
    pub fn metrics(&self) -> &NetworkMetrics { &self.metrics }
}

impl QualityLevel {
    fn upgrade(self) -> Self {
        match self {
            Self::UltraLow => Self::Low,
            Self::Low => Self::Medium,
            Self::Medium => Self::High,
            Self::High => Self::Ultra,
            Self::Ultra => Self::Ultra,
        }
    }
    
    fn degrade(self) -> Self {
        match self {
            Self::Ultra => Self::High,
            Self::High => Self::Medium,
            Self::Medium => Self::Low,
            Self::Low => Self::UltraLow,
            Self::UltraLow => Self::UltraLow,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_quality_levels() {
        assert_eq!(QualityLevel::High.resolution(), (1920, 1080));
        assert_eq!(QualityLevel::High.target_fps(), 55);
        assert_eq!(QualityLevel::High.target_bitrate(), 4_000_000);
    }
    
    #[test]
    fn test_upgrade_downgrade() {
        let q = QualityLevel::Medium;
        assert_eq!(q.upgrade(), QualityLevel::High);
        assert_eq!(q.degrade(), QualityLevel::Low);
    }
}
