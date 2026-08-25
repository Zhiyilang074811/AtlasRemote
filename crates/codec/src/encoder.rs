//! Encoder Trait - Pluggable video encoder abstraction
//!
//! All encoder implementations (NVENC, QSV, AMF, software) must implement this trait.
//! Host capture pipeline uses this trait to remain encoder-agnostic.

use atlas_frame::Frame;
use anyhow::Result;
use windows::Win32::Graphics::Direct3D11::{ID3D11Device, ID3D11DeviceContext};

/// Pluggable video encoder interface
pub trait Encoder {
    /// Initialize the encoder with a D3D11 device and target resolution
    fn init(&mut self, device: &ID3D11Device, context: &ID3D11DeviceContext, width: u32, height: u32, fps: u32, bitrate: u32) -> Result<()>;

    /// Encode a BGRA frame to compressed video bytes (H264/H265 NAL units)
    ///
    /// Returns Err if encoding fails - caller should drop the frame or fall back.
    fn encode(&mut self, frame: &Frame) -> Result<Vec<u8>>;

    /// Check if this encoder is fully initialized and ready
    fn is_ready(&self) -> bool;

    /// Returns the encoder name (e.g., "NVENC", "QSV", "Software")
    fn name(&self) -> &str;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_trait_exists() {
        // Compile-time check: Encoder trait is properly defined
        fn _assert_encoder<T: Encoder + Send>() {}
        // Cannot instantiate without real implementation, but trait compiles
        assert!(true);
    }
}
