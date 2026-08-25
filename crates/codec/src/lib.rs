//! Atlas Codec - Video encoding/decoding module
//!
//! Pluggable Encoder trait + NVENC hardware implementation.
//! All capture pipelines must use the Encoder trait, never NvencEncoder directly.

pub mod nvenc;
pub mod encoder;

pub use encoder::Encoder;
pub use nvenc::NvencEncoder;

use atlas_frame::Frame;
use anyhow::Result;
use tracing::info;

/// High-level codec facade - wraps a concrete Encoder implementation
pub struct Codec {
    encoder: Option<Box<dyn Encoder>>,
}

impl Codec {
    pub fn new() -> Self {
        Self { encoder: None }
    }

    /// Initialize NVENC hardware encoder
    pub fn init_nvenc(&mut self, device: &windows::Win32::Graphics::Direct3D11::ID3D11Device, context: &windows::Win32::Graphics::Direct3D11::ID3D11DeviceContext, width: u32, height: u32) -> Result<(), String> {
        let mut enc = NvencEncoder::new(width, height, 30, 4_000_000)
            .map_err(|e| e.to_string())?;
        enc.init(device, context, width, height, 60, 6_000_000)
            .map_err(|e| e.to_string())?;
        self.encoder = Some(Box::new(enc));
        info!("Codec initialized: NVENC {}x{}@30fps 4Mbps", width, height);
        Ok(())
    }

    /// Encode a frame using the active encoder.
    /// Returns raw BGRA bytes if no encoder is initialized.
    pub fn encode(&mut self, frame: &Frame) -> Result<Vec<u8>, String> {
        if let Some(ref mut enc) = self.encoder {
            if enc.is_ready() {
                match enc.encode(frame) {
                    Ok(encoded) => {
                        info!("Encoded: {} -> {} bytes ({:.1}x compression)",
                            frame.data.len(), encoded.len(),
                            frame.data.len() as f64 / encoded.len().max(1) as f64);
                        return Ok(encoded);
                    }
                    Err(e) => {
                        info!("Encoder failed: {}, dropping frame", e);
                        return Err(e.to_string());
                    }
                }
            }
        }
        // No encoder ready - return raw frame bytes
        Ok(frame.data.clone())
    }

    /// Check if encoder is ready
    pub fn is_ready(&self) -> bool {
        self.encoder.as_ref().map(|e| e.is_ready()).unwrap_or(false)
    }
}

impl Default for Codec {
    fn default() -> Self { Self::new() }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_codec_creation() {
        let codec = Codec::new();
        assert!(!codec.is_ready());
    }
}
