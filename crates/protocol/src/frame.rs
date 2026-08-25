//! ATLS Protocol v2 - FramePacket + InputPacket
//! 
//! Packet types:
//! - FramePacket (0x0001): Video frames (H264/BGRA)
//! - InputPacket (0x0002): Mouse/keyboard/scroll events
//! - Ping/Pong: Keepalive

use std::io::{Read, Write};

use atlas_frame::PixelFormat;
use thiserror::Error;

use crate::packet::{ATLS_MAGIC, ATLS_VERSION, HEADER_SIZE, PacketType};
use crate::crc32;

pub const CODEC_SIZE: usize = 2;
pub const CRC_SIZE: usize = 4;
pub const FULL_HEADER_SIZE: usize = HEADER_SIZE + CODEC_SIZE + CRC_SIZE;

// Input packet magic
pub const INPUT_MAGIC: [u8; 4] = [0x49, 0x4E, 0x50, 0x54]; // "INPT"
pub const INPUT_VERSION: u8 = 1;
pub const INPUT_HEADER_SIZE: usize = 13; // magic(4) + version(1) + type(1) + length(2) + timestamp(8)

#[derive(Debug, Error)]
pub enum ProtocolError {
    #[error("Header too short: expected {expected}, got {got}")]
    HeaderTooShort { expected: usize, got: usize },
    #[error("Magic mismatch: expected ATLS, got {:?}", .0)]
    MagicMismatch([u8; 4]),
    #[error("Unsupported version: {0}")]
    UnsupportedVersion(u16),
    #[error("Invalid packet type: {0}")]
    InvalidPacketType(u16),
    #[error("CRC mismatch: expected {expected:#010x}, got {actual:#010x}")]
    CrcMismatch { expected: u32, actual: u32 },
    #[error("Payload too large: {0} bytes (max {1})")]
    PayloadTooLarge(usize, usize),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Truncated payload: expected {expected}, got {got}")]
    TruncatedPayload { expected: usize, got: usize },
}

pub type Result<T> = std::result::Result<T, ProtocolError>;

#[derive(Debug, Clone)]
pub struct FramePacket {
    pub version: u16,
    pub packet_type: PacketType,
    pub payload_length: u32,
    pub width: u32,
    pub height: u32,
    pub timestamp: u32,
    pub codec: u16,
    pub payload: Vec<u8>,
    pub crc: u32,
}

impl FramePacket {
    pub fn total_size(&self) -> usize {
        FULL_HEADER_SIZE + self.payload.len()
    }

    pub fn compute_crc(&self) -> u32 {
        let mut buf = Vec::with_capacity(HEADER_SIZE + CODEC_SIZE + self.payload.len());
        buf.extend_from_slice(&ATLS_MAGIC);
        buf.extend_from_slice(&self.version.to_be_bytes());
        buf.extend_from_slice(&self.packet_type.to_u16().to_be_bytes());
        buf.extend_from_slice(&self.payload_length.to_be_bytes());
        buf.extend_from_slice(&self.width.to_be_bytes());
        buf.extend_from_slice(&self.height.to_be_bytes());
        buf.extend_from_slice(&self.timestamp.to_be_bytes());
        buf.extend_from_slice(&self.codec.to_be_bytes());
        buf.extend_from_slice(&self.payload); crc32(&buf)
    }

    pub fn validate_crc(&self) -> bool {
        self.compute_crc() == self.crc
    }

    pub fn new_frame(width: u32, height: u32, timestamp_ms: u32, pixel_format: PixelFormat, payload: Vec<u8>) -> Result<Self> {
        let payload_length = payload.len() as u32;
        if payload_length > 50_000_000 {
            return Err(ProtocolError::PayloadTooLarge(payload_length as usize, 50_000_000));
        }
        let codec = match pixel_format {
            PixelFormat::Bgra32 | PixelFormat::Rgba32 | PixelFormat::Rgb24 => 0u16,
            PixelFormat::H264 => 2,
            PixelFormat::H265 => 3,
            PixelFormat::Jpeg => 4,
        };
        let tmp = FramePacket {
            version: ATLS_VERSION, packet_type: PacketType::Frame, payload_length,
            width, height, timestamp: timestamp_ms, codec, payload: payload.clone(), crc: 0,
        };
        let crc = tmp.compute_crc();
        Ok(FramePacket {
            version: ATLS_VERSION, packet_type: PacketType::Frame, payload_length,
            width, height, timestamp: timestamp_ms, codec, payload, crc,
        })
    }

    pub fn new_ping() -> Self {
        let p = FramePacket { version: ATLS_VERSION, packet_type: PacketType::Ping, payload_length: 0, width: 0, height: 0, timestamp: 0, codec: 0, payload: Vec::new(), crc: 0 };
        let crc = p.compute_crc();
        FramePacket { crc, ..p }
    }

    pub fn new_pong(timestamp_ms: u32) -> Self {
        let p = FramePacket { version: ATLS_VERSION, packet_type: PacketType::Pong, payload_length: 4, width: 0, height: 0, timestamp: timestamp_ms, codec: 0, payload: timestamp_ms.to_be_bytes().to_vec(), crc: 0 };
        let crc = p.compute_crc();
        FramePacket { crc, ..p }
    }

    pub fn serialize(&self) -> Vec<u8> {
        let total = FULL_HEADER_SIZE + self.payload.len();
        let mut buf = Vec::with_capacity(total);
        buf.extend_from_slice(&ATLS_MAGIC);
        buf.extend_from_slice(&self.version.to_be_bytes());
        buf.extend_from_slice(&self.packet_type.to_u16().to_be_bytes());
        buf.extend_from_slice(&self.payload_length.to_be_bytes());
        buf.extend_from_slice(&self.width.to_be_bytes());
        buf.extend_from_slice(&self.height.to_be_bytes());
        buf.extend_from_slice(&self.timestamp.to_be_bytes());
        buf.extend_from_slice(&self.codec.to_be_bytes());
        buf.extend_from_slice(&self.crc.to_be_bytes());
        buf.extend_from_slice(&self.payload);
        buf
    }

    pub fn write_to<W: Write>(&self, writer: &mut W) -> std::io::Result<()> {
        writer.write_all(&self.serialize())
    }
}

impl FramePacket {
    pub fn deserialize(bytes: &[u8]) -> Result<(FramePacket, usize)> {
        if bytes.len() < FULL_HEADER_SIZE {
            return Err(ProtocolError::HeaderTooShort { expected: FULL_HEADER_SIZE, got: bytes.len() });
        }
        let magic: [u8; 4] = [bytes[0], bytes[1], bytes[2], bytes[3]];
        if magic != ATLS_MAGIC {
            return Err(ProtocolError::MagicMismatch(magic));
        }
        let version = u16::from_be_bytes([bytes[4], bytes[5]]);
        if version != ATLS_VERSION {
            return Err(ProtocolError::UnsupportedVersion(version));
        }
        let packet_type = u16::from_be_bytes([bytes[6], bytes[7]]);
        let packet_type = PacketType::from_u16(packet_type).ok_or_else(|| ProtocolError::InvalidPacketType(packet_type))?;
        let payload_length = u32::from_be_bytes([bytes[8], bytes[9], bytes[10], bytes[11]]) as usize;
        let width = u32::from_be_bytes([bytes[12], bytes[13], bytes[14], bytes[15]]);
        let height = u32::from_be_bytes([bytes[16], bytes[17], bytes[18], bytes[19]]);
        let timestamp = u32::from_be_bytes([bytes[20], bytes[21], bytes[22], bytes[23]]);
        let codec = u16::from_be_bytes([bytes[24], bytes[25]]);
        let crc = u32::from_be_bytes([bytes[26], bytes[27], bytes[28], bytes[29]]);
        let total = FULL_HEADER_SIZE + payload_length;
        if bytes.len() < total {
            return Err(ProtocolError::TruncatedPayload { expected: total, got: bytes.len() });
        }
        let payload = bytes[FULL_HEADER_SIZE..total].to_vec();
        let tmp = FramePacket { version, packet_type, payload_length: payload_length as u32, width, height, timestamp, codec, payload: payload.clone(), crc: 0 };
        let expected_crc = tmp.compute_crc();
        if expected_crc != crc {
            return Err(ProtocolError::CrcMismatch { expected: expected_crc, actual: crc });
        }
        Ok((FramePacket { payload, crc, ..tmp }, total))
    }

    pub fn read_from<R: Read>(reader: &mut R) -> Result<Self> {
        let mut header = [0u8; FULL_HEADER_SIZE];
        reader.read_exact(&mut header[..])?;
        let payload_length = u32::from_be_bytes([header[8], header[9], header[10], header[11]]) as usize;
        let mut payload = Vec::with_capacity(payload_length);
        if payload_length > 0 {
            reader.read_to_end(&mut payload)?;
        }
        let tmp = FramePacket {
            version: u16::from_be_bytes([header[4], header[5]]),
            packet_type: PacketType::from_u16(u16::from_be_bytes([header[6], header[7]])).ok_or_else(|| ProtocolError::InvalidPacketType(u16::from_be_bytes([header[6], header[7]])))?,
            payload_length: payload_length as u32,
            width: u32::from_be_bytes([header[12], header[13], header[14], header[15]]),
            height: u32::from_be_bytes([header[16], header[17], header[18], header[19]]),
            timestamp: u32::from_be_bytes([header[20], header[21], header[22], header[23]]),
            codec: u16::from_be_bytes([header[24], header[25]]),
            payload: payload.clone(),
            crc: u32::from_be_bytes([header[26], header[27], header[28], header[29]]),
        };
        let expected_crc = tmp.compute_crc();
        if expected_crc != tmp.crc {
            return Err(ProtocolError::CrcMismatch { expected: expected_crc, actual: tmp.crc });
        }
        Ok(tmp)
    }
}

// ========== Input Packet (S-006) ==========

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum InputType {
    MouseMove = 0x01,
    MouseButton = 0x02,
    MouseWheel = 0x03,
    KeyPress = 0x04,
    Clipboard = 0x05,
}

impl InputType {
    pub fn from_u8(v: u8) -> Option<Self> {
        match v {
            0x01 => Some(InputType::MouseMove),
            0x02 => Some(InputType::MouseButton),
            0x03 => Some(InputType::MouseWheel),
            0x04 => Some(InputType::KeyPress),
            0x05 => Some(InputType::Clipboard),
            _ => None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct InputPacket {
    pub version: u8,
    pub input_type: InputType,
    pub timestamp_ms: u64,
    pub payload: Vec<u8>,
}

impl InputPacket {
    pub fn mouse_move(x: f32, y: f32, timestamp_ms: u64) -> Self {
        let mut payload = Vec::with_capacity(8);
        payload.extend_from_slice(&x.to_le_bytes());
        payload.extend_from_slice(&y.to_le_bytes());
        Self { version: INPUT_VERSION, input_type: InputType::MouseMove, timestamp_ms, payload }
    }

    pub fn mouse_button(button: u8, down: bool, timestamp_ms: u64) -> Self {
        let mut payload = Vec::with_capacity(2);
        payload.push(button);
        payload.push(if down { 1 } else { 0 });
        Self { version: INPUT_VERSION, input_type: InputType::MouseButton, timestamp_ms, payload }
    }

    pub fn mouse_wheel(delta: i32, timestamp_ms: u64) -> Self {
        let mut payload = Vec::with_capacity(4);
        payload.extend_from_slice(&delta.to_le_bytes());
        Self { version: INPUT_VERSION, input_type: InputType::MouseWheel, timestamp_ms, payload }
    }

    pub fn key_press(hid_code: u16, down: bool, timestamp_ms: u64) -> Self {
        let mut payload = Vec::with_capacity(3);
        payload.extend_from_slice(&hid_code.to_le_bytes());
        payload.push(if down { 1 } else { 0 });
        Self { version: INPUT_VERSION, input_type: InputType::KeyPress, timestamp_ms, payload }
    }

    pub fn clipboard(text: &str, timestamp_ms: u64) -> Self {
        let payload = text.as_bytes().to_vec();
        Self { version: INPUT_VERSION, input_type: InputType::Clipboard, timestamp_ms, payload }
    }

    pub fn serialize(&self) -> Vec<u8> {
        let mut buf = Vec::with_capacity(INPUT_HEADER_SIZE + self.payload.len());
        buf.extend_from_slice(&INPUT_MAGIC);
        buf.push(self.version);
        buf.push(self.input_type as u8);
        buf.extend_from_slice(&(self.payload.len() as u16).to_le_bytes());
        buf.extend_from_slice(&self.timestamp_ms.to_le_bytes());
        buf.extend_from_slice(&self.payload);
        buf
    }

    pub fn deserialize(bytes: &[u8]) -> Option<(InputPacket, usize)> {
        if bytes.len() < INPUT_HEADER_SIZE { return None; }
        let magic: [u8; 4] = [bytes[0], bytes[1], bytes[2], bytes[3]];
        if magic != INPUT_MAGIC { return None; }
        let version = bytes[4];
        let input_type = InputType::from_u8(bytes[5])?;
        let length = u16::from_le_bytes([bytes[6], bytes[7]]) as usize;
        let timestamp_ms = u64::from_le_bytes([bytes[8], bytes[9], bytes[10], bytes[11], bytes[12], bytes[13], bytes[14], bytes[15]]);
        if bytes.len() < INPUT_HEADER_SIZE + length { return None; }
        let payload = bytes[INPUT_HEADER_SIZE..INPUT_HEADER_SIZE + length].to_vec();
        Some((InputPacket { version, input_type, timestamp_ms, payload }, INPUT_HEADER_SIZE + length))
    }
}

// HID keyboard codes (same as Windows VK codes for ASCII)
pub mod hid {
    pub const KEY_A: u16 = 0x04;
    pub const KEY_B: u16 = 0x05;
    pub const KEY_C: u16 = 0x06;
    pub const KEY_D: u16 = 0x07;
    pub const KEY_E: u16 = 0x08;
    pub const KEY_F: u16 = 0x09;
    pub const KEY_G: u16 = 0x0A;
    pub const KEY_H: u16 = 0x0B;
    pub const KEY_I: u16 = 0x0C;
    pub const KEY_J: u16 = 0x0D;
    pub const KEY_K: u16 = 0x0E;
    pub const KEY_L: u16 = 0x0F;
    pub const KEY_M: u16 = 0x10;
    pub const KEY_N: u16 = 0x11;
    pub const KEY_O: u16 = 0x12;
    pub const KEY_P: u16 = 0x13;
    pub const KEY_Q: u16 = 0x14;
    pub const KEY_R: u16 = 0x15;
    pub const KEY_S: u16 = 0x16;
    pub const KEY_T: u16 = 0x17;
    pub const KEY_U: u16 = 0x18;
    pub const KEY_V: u16 = 0x19;
    pub const KEY_W: u16 = 0x1A;
    pub const KEY_X: u16 = 0x1B;
    pub const KEY_Y: u16 = 0x1C;
    pub const KEY_Z: u16 = 0x1D;
    pub const KEY_0: u16 = 0x22;
    pub const KEY_1: u16 = 0x02;
    pub const KEY_2: u16 = 0x03;
    pub const KEY_3: u16 = 0x04;
    pub const KEY_4: u16 = 0x05;
    pub const KEY_5: u16 = 0x06;
    pub const KEY_6: u16 = 0x07;
    pub const KEY_7: u16 = 0x08;
    pub const KEY_8: u16 = 0x09;
    pub const KEY_9: u16 = 0x0A;
    pub const KEY_ENTER: u16 = 0x28;
    pub const KEY_ESCAPE: u16 = 0x29;
    pub const KEY_BACKSPACE: u16 = 0x0E;
    pub const KEY_TAB: u16 = 0x0F;
    pub const KEY_SPACE: u16 = 0x39;
    pub const KEY_DELETE: u16 = 0x4C;
    pub const KEY_HOME: u16 = 0x47;
    pub const KEY_END: u16 = 0x4F;
    pub const KEY_PAGE_UP: u16 = 0x49;
    pub const KEY_PAGE_DOWN: u16 = 0x51;
    pub const KEY_UP: u16 = 0x52;
    pub const KEY_DOWN: u16 = 0x50;
    pub const KEY_LEFT: u16 = 0x4B;
    pub const KEY_RIGHT: u16 = 0x4D;
    pub const KEY_F1: u16 = 0x3A;
    pub const KEY_F2: u16 = 0x3B;
    pub const KEY_F3: u16 = 0x3C;
    pub const KEY_F4: u16 = 0x3D;
    pub const KEY_F5: u16 = 0x3E;
    pub const KEY_F6: u16 = 0x3F;
    pub const KEY_F7: u16 = 0x40;
    pub const KEY_F8: u16 = 0x41;
    pub const KEY_F9: u16 = 0x42;
    pub const KEY_F10: u16 = 0x43;
    pub const KEY_F11: u16 = 0x44;
    pub const KEY_F12: u16 = 0x45;
    
    // Mouse buttons
    pub const MOUSE_LEFT: u8 = 1;
    pub const MOUSE_RIGHT: u8 = 2;
    pub const MOUSE_MIDDLE: u8 = 3;

    /// Map key name string to HID/VK code
    pub fn key_name_to_hid(name: &str) -> Option<u16> {
        match name.to_lowercase().as_str() {
            "enter" | "return" => Some(0x0D),
            "escape" => Some(0x1B),
            "backspace" | "bksp" => Some(0x08),
            "tab" => Some(0x09),
            "space" => Some(0x20),
            "delete" | "del" => Some(0x2E),
            "home" => Some(0x24),
            "end" => Some(0x23),
            "pageup" | "pgup" => Some(0x21),
            "pagedown" | "pgdn" => Some(0x22),
            "up" => Some(0x26),
            "down" => Some(0x28),
            "left" => Some(0x25),
            "right" => Some(0x27),
            "insert" | "ins" => Some(0x2D),
            "f1" => Some(0x70),
            "f2" => Some(0x71),
            "f3" => Some(0x72),
            "f4" => Some(0x73),
            "f5" => Some(0x74),
            "f6" => Some(0x75),
            "f7" => Some(0x76),
            "f8" => Some(0x77),
            "f9" => Some(0x78),
            "f10" => Some(0x79),
            "f11" => Some(0x7A),
            "f12" => Some(0x7B),
            "lctrl" => Some(0x1D),
            "rctrl" => Some(0x1D),
            "lshift" => Some(0x2A),
            "rshift" => Some(0x36),
            "lalt" => Some(0x38),
            "ralt" => Some(0xB8),
            "lwin" => Some(0x5B),
            "rwin" => Some(0x5C),
            "menu" => Some(0x5D),
            "capslock" => Some(0x14),
            "numlock" => Some(0x90),
            "scrolllock" => Some(0x91),
            _ => {
                if name.len() == 1 {
                    let c = name.chars().next().unwrap();
                    if c >= 'a' && c <= 'z' { return Some(0x1E + (c as u16 - b'a' as u16)); }
                    if c >= 'A' && c <= 'Z' { return Some(0x1E + (c as u16 - b'A' as u16)); }
                    if c >= '0' && c <= '9' { return Some(0x32 - (c as u16 - b'0' as u16)); }
                }
                None
            }
        }
    }
}


/// Get current timestamp in milliseconds
pub fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

pub fn has_enough_data_for_header(buf: &[u8]) -> bool { buf.len() >= FULL_HEADER_SIZE }
pub fn peek_payload_length(buf: &[u8]) -> Option<usize> {
    if buf.len() < FULL_HEADER_SIZE { return None; }
    Some(u32::from_be_bytes([buf[8], buf[9], buf[10], buf[11]]) as usize)
}
pub fn total_packet_size(payload_len: usize) -> usize { FULL_HEADER_SIZE + payload_len }

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_frame_roundtrip() {
        let packet = FramePacket::new_frame(640, 480, 12345, atlas_frame::PixelFormat::Bgra32, vec![1, 2, 3]).unwrap();
        let bytes = packet.serialize();
        let (decoded, _) = FramePacket::deserialize(&bytes).unwrap();
        assert_eq!(decoded.width, 640);
        assert_eq!(decoded.height, 480);
        assert_eq!(decoded.payload, vec![1, 2, 3]);
    }

    #[test]
    fn test_input_packet_mouse_move() {
        let p = InputPacket::mouse_move(0.5, 0.3, 12345678);
        let bytes = p.serialize();
        assert_eq!(bytes.len(), INPUT_HEADER_SIZE + 8);
        assert_eq!(&bytes[0..4], &INPUT_MAGIC);
        assert_eq!(bytes[4], INPUT_VERSION);
        assert_eq!(bytes[5], InputType::MouseMove as u8);
        let (decoded, _) = InputPacket::deserialize(&bytes).unwrap();
        assert_eq!(decoded.input_type, InputType::MouseMove);
    }

    #[test]
    fn test_input_packet_key() {
        let p = InputPacket::key_press(hid::KEY_A, true, 100);
        assert_eq!(p.serialize().len(), INPUT_HEADER_SIZE + 3);
        let (decoded, _) = InputPacket::deserialize(&p.serialize()).unwrap();
        assert_eq!(decoded.input_type, InputType::KeyPress);
        assert_eq!(u16::from_le_bytes([decoded.payload[0], decoded.payload[1]]), hid::KEY_A);
    }

    #[test]
    fn test_input_deserialize_missing() {
        assert!(InputPacket::deserialize(&[0x49, 0x4E]).is_none());
        assert!(InputPacket::deserialize(&[0x41, 0x54, 0x4C, 0x53]).is_none());
    }
}






