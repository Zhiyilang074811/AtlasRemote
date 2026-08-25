//! Atlas Telemetry - Connection quality metrics and logging

use std::time::{Duration, Instant};
use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};

/// Session log entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionLog {
    pub session_id: String,
    pub device_id: String,
    pub start_time: DateTime<Utc>,
    pub end_time: Option<DateTime<Utc>>,
    pub duration: Duration,
    pub bytes_transferred: u64,
    pub avg_fps: f32,
    pub avg_bandwidth_bps: u64,
    pub avg_latency_ms: f32,
    pub packet_loss: f32,
}

impl SessionLog {
    pub fn new(session_id: &str, device_id: &str) -> Self {
        Self {
            session_id: session_id.to_string(),
            device_id: device_id.to_string(),
            start_time: Utc::now(),
            end_time: None,
            duration: Duration::ZERO,
            bytes_transferred: 0,
            avg_fps: 0.0,
            avg_bandwidth_bps: 0,
            avg_latency_ms: 0.0,
            packet_loss: 0.0,
        }
    }
    
    pub fn end(&mut self) {
        self.end_time = Some(Utc::now());
        self.duration = self.end_time.unwrap().signed_duration_since(self.start_time).to_std().unwrap_or(Duration::ZERO);
    }
}

/// Connection metrics collector
#[derive(Debug, Default)]
pub struct MetricsCollector {
    frame_count: u64,
    byte_count: u64,
    start_time: Option<Instant>,
    latency_samples: Vec<f32>,
    fps_samples: Vec<f32>,
}

impl MetricsCollector {
    pub fn new() -> Self { Self::default() }
    
    pub fn record_frame(&mut self, bytes: usize, latency_ms: f32, fps: f32) {
        self.frame_count += 1;
        self.byte_count += bytes as u64;
        self.latency_samples.push(latency_ms);
        self.fps_samples.push(fps);
        
        // Keep only last 100 samples
        if self.latency_samples.len() > 100 {
            self.latency_samples.remove(0);
        }
        if self.fps_samples.len() > 100 {
            self.fps_samples.remove(0);
        }
        
        if self.start_time.is_none() {
            self.start_time = Some(Instant::now());
        }
    }
    
    pub fn avg_latency(&self) -> f32 {
        if self.latency_samples.is_empty() { return 0.0; }
        self.latency_samples.iter().sum::<f32>() / self.latency_samples.len() as f32
    }
    
    pub fn avg_fps(&self) -> f32 {
        if self.fps_samples.is_empty() { return 0.0; }
        self.fps_samples.iter().sum::<f32>() / self.fps_samples.len() as f32
    }
    
    pub fn avg_bandwidth_bps(&self) -> u64 {
        if let Some(start) = self.start_time {
            let elapsed = start.elapsed().as_secs_f64().max(1.0);
            return (self.byte_count as f64 * 8.0 / elapsed) as u64;
        }
        0
    }
    
    pub fn total_bytes(&self) -> u64 { self.byte_count }
}

/// Connection quality status
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConnectionStatus {
    Disconnected,
    Connecting,
    Connected,
    Degraded,      // High latency or packet loss
    Unstable,      // Frequent quality changes
}

impl std::fmt::Display for ConnectionStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Disconnected => write!(f, "Disconnected"),
            Self::Connecting => write!(f, "Connecting..."),
            Self::Connected => write!(f, "Connected"),
            Self::Degraded => write!(f, "Degraded"),
            Self::Unstable => write!(f, "Unstable"),
        }
    }
}
