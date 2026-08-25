//! Atlas Capture - Screen capture module
//!
//! Provides screen capture functionality using Windows DXGI Desktop Duplication.
//! Capture runs on a dedicated thread; frames are queued for clients.

pub mod dxgi;

pub use dxgi::DxgiCapture;
pub type ScreenCapture = DxgiCapture;

use atlas_frame::{Frame, FrameSource};
use std::sync::Arc;
use std::sync::Mutex;
use std::thread;

const MAX_QUEUED_FRAMES: usize = 3;

/// Frame queue - bounded, drops oldest when full
pub struct FrameQueue {
    frames: Mutex<Vec<Frame>>,
}

impl FrameQueue {
    pub fn new() -> Self {
        Self {
            frames: Mutex::new(Vec::new()),
        }
    }

    pub fn push(&self, frame: Frame) {
        let mut q = self.frames.lock().unwrap();
        if q.len() >= MAX_QUEUED_FRAMES {
            q.remove(0);
        }
        q.push(frame);
    }

    pub fn peek_frame(&self) -> Option<Frame> {
        let mut q = self.frames.lock().unwrap();
        q.last().cloned()
    }

    pub fn recv(&self) -> Option<Frame> {
        let mut q = self.frames.lock().unwrap();
        if q.is_empty() {
            None
        } else {
            Some(q.remove(0))
        }
    }
}

/// Main capture device with bounded frame queue
pub struct CaptureDevice {
    queue: Arc<FrameQueue>,
}

impl CaptureDevice {
    pub fn new(monitor_index: u32) -> Result<Self, anyhow::Error> {
        let queue = Arc::new(FrameQueue::new());
        let q = queue.clone();
        thread::spawn(move || {
            run_capture_thread(monitor_index, q);
        });
        Ok(Self { queue })
    }

    pub fn capture_frame(&mut self) -> Option<Frame> {
        self.queue.recv()
    }

    pub fn name(&self) -> &str {
        "AtlasCapture"
    }
}

fn run_capture_thread(monitor_index: u32, q: Arc<FrameQueue>) {
    eprintln!("[CAPTURE-THREAD] Starting for monitor {}", monitor_index);
    match DxgiCapture::new(monitor_index) {
        Ok(mut capture) => {
            eprintln!("[CAPTURE-THREAD] Initialized: {}", capture.name());
            let mut frame_id = 0u64;
            loop {
                match capture.next_frame() {
                    Some(mut frame) => {
                        frame_id += 1;
                        if let Some(ref mut enc) = capture.encoder {
                            if let Ok(encoded) = enc.encode(&frame) {
                                frame.data = encoded;
                                frame.format = atlas_frame::PixelFormat::H264;
                            }
                        }
                        q.push(frame);
                        if frame_id % 60 == 0 {
                            let queued = q.frames.lock().unwrap().len();
                            eprintln!("[CAPTURE-THREAD] {} frames captured, {} queued", frame_id, queued);
                        }
                    }
                    None => {
                        std::thread::sleep(std::time::Duration::from_millis(66));
                    }
                }
            }
        }
        Err(e) => {
            eprintln!("[CAPTURE-THREAD] Failed: {}", e);
        }
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
