//! Atlas Capture - Screen capture module
//!
//! Provides screen capture functionality using Windows GDI.

pub mod dxgi;

// Re-export for convenience
pub use dxgi::DxgiCapture;
pub use dxgi::ScreenCapture;

// Import trait for method resolution
use atlas_frame::FrameSource;

/// Main capture device
pub struct CaptureDevice {
    capture: DxgiCapture,
}

impl CaptureDevice {
    pub fn new(monitor_index: u32) -> Result<Self, anyhow::Error> {
        let capture = DxgiCapture::new(monitor_index)?;
        Ok(Self { capture })
    }

    pub fn capture_frame(&mut self) -> Option<atlas_frame::Frame> {
        self.capture.next_frame()
    }

    pub fn name(&self) -> &str {
        self.capture.name()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_device_creation() {
        let device = CaptureDevice::new(0);
        assert!(device.is_ok());
    }
}
