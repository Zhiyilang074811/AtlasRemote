//! ATLS Protocol v2 - Unified protocol layer
//!
//! Single source of truth: all modules (Host / Android FFI / Transport) must reference this crate.

pub mod packet;
pub mod frame;

/// Compute CRC32 (IEEE 802.3) checksum
pub fn crc32(data: &[u8]) -> u32 {
    let mut crc = 0xFFFFFFFFu32;
    for &b in data {
        crc ^= b as u32;
        for _ in 0..8 {
            crc = if crc & 1 != 0 { (crc >> 1) ^ 0xEDB88320 } else { crc >> 1 };
        }
    }
    crc ^ 0xFFFFFFFF
}

pub use packet::{ATLS_MAGIC, ATLS_VERSION, HEADER_SIZE, PacketType, FrameType};
pub use frame::{FramePacket, InputPacket, InputType, ProtocolError, Result, has_enough_data_for_header, peek_payload_length, total_packet_size, FULL_HEADER_SIZE, INPUT_HEADER_SIZE, INPUT_MAGIC, INPUT_VERSION, hid};