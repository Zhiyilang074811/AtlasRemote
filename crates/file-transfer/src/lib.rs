//! Atlas File Transfer - Bidirectional file transfer over ATLS
//!
//! Supports:
//! - Drag-and-drop from Android to Windows
//! - Bidirectional file transfer
//! - Progress tracking
//! - Chunked transfer for large files

use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::fs::{File, OpenOptions};
use std::sync::Arc;
use std::sync::Mutex;
use std::time::{Duration, Instant};
use tracing::{info, warn, error};

/// File transfer packet types
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TransferCommand {
    /// Request to send a file
    SendFile {
        filename: String,
        size: u64,
        path: PathBuf,
    },
    /// Acknowledge file reception
    AckFile {
        filename: String,
        success: bool,
    },
    /// Send file chunk
    FileChunk {
        filename: String,
        offset: u64,
        data: Vec<u8>,
    },
    /// Cancel transfer
    Cancel {
        filename: String,
        reason: String,
    },
}

/// Transfer status
#[derive(Debug, Clone)]
pub struct TransferStatus {
    pub filename: String,
    pub direction: TransferDirection,
    pub bytes_transferred: u64,
    pub total_bytes: u64,
    pub started_at: Instant,
    pub completed_at: Option<Instant>,
    pub error: Option<String>,
}

impl TransferStatus {
    pub fn progress(&self) -> f32 {
        if self.total_bytes == 0 {
            return 0.0;
        }
        (self.bytes_transferred as f32 / self.total_bytes as f32) * 100.0
    }
    
    pub fn is_complete(&self) -> bool {
        self.bytes_transferred >= self.total_bytes
    }
    
    pub fn speed_bps(&self) -> u64 {
        let elapsed = self.started_at.elapsed();
        if elapsed.as_secs() == 0 {
            return 0;
        }
        (self.bytes_transferred as u64 * 8) / elapsed.as_secs()
    }
}

/// Transfer direction
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TransferDirection {
    Upload,    // Android -> Windows
    Download,  // Windows -> Android
}

/// File transfer manager
#[derive(Debug)]
pub struct TransferManager {
    transfers: Arc<Mutex<Vec<TransferStatus>>>,
    download_dir: PathBuf,
    max_concurrent: usize,
}

impl TransferManager {
    pub fn new(download_dir: &str) -> Self {
        Self {
            transfers: Arc::new(Mutex::new(Vec::new())),
            download_dir: PathBuf::from(download_dir),
            max_concurrent: 3,
        }
    }
    
    /// Start a file transfer
    pub fn start_transfer(&mut self, cmd: TransferCommand) -> Result<(), String> {
        match cmd {
            TransferCommand::SendFile { filename, size, path } => {
                info!("Starting file transfer: {} ({:.2} MB)", filename, size as f64 / 1_000_000.0);
                
                let mut transfers = self.transfers.lock().map_err(|e| e.to_string())?;
                
                if transfers.len() >= self.max_concurrent {
                    return Err("Maximum concurrent transfers reached".to_string());
                }
                
                let status = TransferStatus {
                    filename: filename.clone(),
                    direction: TransferDirection::Upload,
                    bytes_transferred: 0,
                    total_bytes: size,
                    started_at: Instant::now(),
                    completed_at: None,
                    error: None,
                };
                transfers.push(status);
                
                // TODO: Implement actual file reading and chunking
                Ok(())
            }
            TransferCommand::AckFile { filename, success } => {
                info!("File transfer acknowledged: {} (success={})", filename, success);
                Ok(())
            }
            TransferCommand::FileChunk { filename, offset, data } => {
                info!("Receiving chunk: {} offset={} size={}", filename, offset, data.len());
                Ok(())
            }
            TransferCommand::Cancel { filename, reason } => {
                warn!("Transfer cancelled: {} reason={}", filename, reason);
                Ok(())
            }
        }
    }
    
    /// Get transfer status
    pub fn get_status(&self, filename: &str) -> Option<TransferStatus> {
        let transfers = self.transfers.lock().unwrap();
        transfers.iter()
            .find(|t| t.filename == filename)
            .cloned()
    }
    
    /// Get all transfer statuses
    pub fn get_all_statuses(&self) -> Vec<TransferStatus> {
        self.transfers.lock().unwrap().clone()
    }
    
    /// Complete a transfer
    pub fn complete_transfer(&mut self, filename: &str) -> Result<(), String> {
        let mut transfers = self.transfers.lock().map_err(|e| e.to_string())?;
        if let Some(status) = transfers.iter_mut().find(|t| t.filename == filename) {
            status.completed_at = Some(Instant::now());
            info!("Transfer completed: {} ({:.2} MB in {:?})", 
                filename, 
                status.total_bytes as f64 / 1_000_000.0,
                status.started_at.elapsed());
        }
        Ok(())
    }
    
    /// Error a transfer
    pub fn error_transfer(&mut self, filename: &str, error: &str) -> Result<(), String> {
        let mut transfers = self.transfers.lock().map_err(|e| e.to_string())?;
        if let Some(status) = transfers.iter_mut().find(|t| t.filename == filename) {
            status.error = Some(error.to_string());
            status.completed_at = Some(Instant::now());
            error!("Transfer error: {} - {}", filename, error);
        }
        Ok(())
    }
    
    /// Cleanup completed transfers
    pub fn cleanup_completed(&mut self, max_age: Duration) {
        let mut transfers = self.transfers.lock().unwrap();
        transfers.retain(|t| {
            match t.completed_at {
                Some(completed) => completed.elapsed() < max_age,
                None => true,
            }
        });
    }
}

/// Chunk size for file transfer (1MB)
pub const CHUNK_SIZE: usize = 1_048_576;

/// Maximum file size (10GB)
pub const MAX_FILE_SIZE: u64 = 10_737_418_240;

/// Validate file transfer request
pub fn validate_transfer_request(filename: &str, size: u64) -> Result<(), String> {
    // Check filename length
    if filename.len() > 255 {
        return Err("Filename too long (max 255 characters)".to_string());
    }
    
    // Check file size
    if size > MAX_FILE_SIZE {
        return Err(format!("File too large (max {:.2} GB)", MAX_FILE_SIZE as f64 / 1_000_000_000.0));
    }
    
    // Check for invalid characters in filename
    if filename.contains(['<', '>', ':', '"', '|', '?', '*']) {
        return Err("Filename contains invalid characters".to_string());
    }
    
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_transfer_status_progress() {
        let mut status = TransferStatus {
            filename: "test.txt".to_string(),
            direction: TransferDirection::Upload,
            bytes_transferred: 500,
            total_bytes: 1000,
            started_at: Instant::now(),
            completed_at: None,
            error: None,
        };
        
        assert_eq!(status.progress(), 50.0);
        assert!(!status.is_complete());
        
        status.bytes_transferred = 1000;
        assert!(status.is_complete());
        assert_eq!(status.progress(), 100.0);
    }
    
    #[test]
    fn test_validate_filename() {
        assert!(validate_transfer_request("test.txt", 1024).is_ok());
        assert!(validate_transfer_request("test<bad>.txt", 1024).is_err());
    }
    
    #[test]
    fn test_validate_size() {
        assert!(validate_transfer_request("test.txt", 1024).is_ok());
        assert!(validate_transfer_request("test.txt", MAX_FILE_SIZE + 1).is_err());
    }
}
