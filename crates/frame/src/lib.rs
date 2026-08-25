//! Atlas Frame - Frame abstraction layer
//!
//! Unified frame representation for capture, codec, and transport.
//! Decouples capture from transport.

use chrono::Utc;
use serde::{Deserialize, Serialize};

/// Pixel format of the captured frame
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PixelFormat {
    Bgra32,
    Rgba32,
    Rgb24,
    Jpeg,
    H264,
    H265,
}

impl std::fmt::Display for PixelFormat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PixelFormat::Bgra32 => write!(f, "BGRA32"),
            PixelFormat::Rgba32 => write!(f, "RGBA32"),
            PixelFormat::Rgb24 => write!(f, "RGB24"),
            PixelFormat::Jpeg => write!(f, "JPEG"),
            PixelFormat::H264 => write!(f, "H264"),
            PixelFormat::H265 => write!(f, "H265"),
        }
    }
}

/// A single video frame
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Frame {
    pub timestamp: u64,
    pub width: u32,
    pub height: u32,
    pub format: PixelFormat,
    pub data: Vec<u8>,
    pub frame_id: u64,
}

impl Frame {
    pub fn empty(width: u32, height: u32, format: PixelFormat) -> Self {
        Self {
            timestamp: Utc::now().timestamp_micros() as u64,
            width,
            height,
            format,
            data: Vec::new(),
            frame_id: 0,
        }
    }

    pub fn test_frame(width: u32, height: u32) -> Self {
        let size = (width * height * 4) as usize;
        Self {
            timestamp: Utc::now().timestamp_micros() as u64,
            width,
            height,
            format: PixelFormat::Bgra32,
            data: vec![0u8; size],
            frame_id: 0,
        }
    }

    pub fn uncompressed_size(&self) -> usize {
        match self.format {
            PixelFormat::Bgra32 | PixelFormat::Rgba32 => (self.width * self.height * 4) as usize,
            PixelFormat::Rgb24 => (self.width * self.height * 3) as usize,
            _ => self.data.len(),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }
}

/// Trait for frame sources (capture devices, webcams, etc.)
pub trait FrameSource {
    fn next_frame(&mut self) -> Option<Frame>;
    fn name(&self) -> &str;
}

/// Trait for frame consumers (display, codec, etc.)
pub trait FrameConsumer {
    fn consume_frame(&mut self, frame: Frame) -> bool;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_frame_creation() {
        let frame = Frame::test_frame(1920, 1080);
        assert_eq!(frame.width, 1920);
        assert_eq!(frame.height, 1080);
        assert_eq!(frame.format, PixelFormat::Bgra32);
    }

    #[test]
    fn test_empty_frame() {
        let frame = Frame::empty(1920, 1080, PixelFormat::Bgra32);
        assert!(frame.is_empty());
    }
}
