//! ATLS Protocol v1 - FramePacket tests

use atlas_protocol::{FramePacket, PacketType, HEADER_SIZE, ATLS_MAGIC, ATLS_VERSION};
use atlas_frame::PixelFormat;

#[test]
fn test_minimal_frame_serialize() {
    let packet = FramePacket::new_frame(1, 1, 0, PixelFormat::Bgra32, vec![]).unwrap();
    let bytes = packet.serialize();
    assert_eq!(&bytes[0..4], &ATLS_MAGIC);
    assert_eq!(u16::from_be_bytes([bytes[4], bytes[5]]), ATLS_VERSION);
    assert_eq!(u16::from_be_bytes([bytes[6], bytes[7]]), PacketType::Frame as u16);
    assert_eq!(u32::from_be_bytes([bytes[8], bytes[9], bytes[10], bytes[11]]), 0);
    assert_eq!(u32::from_be_bytes([bytes[12], bytes[13], bytes[14], bytes[15]]), 1);
    assert_eq!(u32::from_be_bytes([bytes[16], bytes[17], bytes[18], bytes[19]]), 1);
    assert_eq!(u32::from_be_bytes([bytes[20], bytes[21], bytes[22], bytes[23]]), 0);
    assert_eq!(bytes.len(), atlas_protocol::FULL_HEADER_SIZE);
}

#[test]
fn test_full_frame_roundtrip() {
    let data = vec![0u8; 1920 * 1080 * 4];
    let packet = FramePacket::new_frame(1920, 1080, 1700000000, PixelFormat::Bgra32, data.clone()).unwrap();
    let bytes = packet.serialize();
    let (decoded, consumed) = FramePacket::deserialize(&bytes).unwrap();
    assert_eq!(consumed, bytes.len());
    assert_eq!(decoded.width, 1920);
    assert_eq!(decoded.height, 1080);
    assert_eq!(decoded.timestamp, 1700000000);
    assert_eq!(decoded.payload, data);
}

#[test]
fn test_h264_payload() {
    let h264_data = vec![
        0x00, 0x00, 0x00, 0x01, 0x67, 0x42, 0x00, 0x28,
        0x00, 0x00, 0x00, 0x01, 0x68, 0xce, 0x3c, 0x80,
        0x00, 0x00, 0x00, 0x01, 0x61,
    ];
    let packet = FramePacket::new_frame(1920, 1080, 42, PixelFormat::H264, h264_data.clone()).unwrap();
    let bytes = packet.serialize();
    let (decoded, _) = FramePacket::deserialize(&bytes).unwrap();
    assert_eq!(decoded.payload, h264_data);
    assert_eq!(decoded.packet_type, PacketType::Frame);
}

#[test]
fn test_all_packet_types_roundtrip() {
    let types = [
        PacketType::Frame,
        PacketType::Input,
        PacketType::Control,
        PacketType::PairRequest,
        PacketType::PairResponse,
        PacketType::Ping,
        PacketType::Pong,
    ];
    for t in types {
        let payload = match t {
            PacketType::Ping => vec![],
            PacketType::Pong => 12345u32.to_be_bytes().to_vec(),
            _ => vec![0x01, 0x02, 0x03],
        };
        let wh = if matches!(t, PacketType::Frame) { (1920u32, 1080u32) } else { (0, 0) };
        let packet = FramePacket {
            version: ATLS_VERSION,
            packet_type: t,
            payload_length: payload.len() as u32,
            width: wh.0,
            height: wh.1,
            timestamp: 0,
            codec: 0,
            payload: payload.clone(),
            crc: 0,
        };
        // Compute CRC since we constructed the packet directly
        let crc = packet.compute_crc();
        let bytes = FramePacket { crc, ..packet }.serialize();
        let (decoded, _) = FramePacket::deserialize(&bytes).unwrap();
        assert_eq!(decoded.packet_type, t, "type {:?}", t);
    }
}

#[test]
fn test_crc_validates_correctly() {
    let packet = FramePacket::new_frame(640, 480, 0, PixelFormat::Bgra32, vec![1, 2, 3]).unwrap();
    assert!(packet.validate_crc());
    let mut bytes = packet.serialize();
    bytes[26] ^= 0x01;
    let result = FramePacket::deserialize(&bytes);
    assert!(result.is_err());
}

#[test]
fn test_crc_alone_matches() {
    let payload = vec![0xAB, 0xCD, 0xEF];
    let packet = FramePacket::new_frame(1, 1, 0, PixelFormat::Bgra32, payload).unwrap();
    assert_eq!(packet.crc, packet.compute_crc());
}

#[test]
fn test_too_short_header() {
    assert!(FramePacket::deserialize(&[]).is_err());
    assert!(FramePacket::deserialize(&[0x41, 0x54, 0x4C]).is_err());
    assert!(FramePacket::deserialize(&[0x41, 0x54, 0x4C, 0x53]).is_err());
}

#[test]
fn test_magic_wrong() {
    let packet = FramePacket::new_frame(1, 1, 0, PixelFormat::Bgra32, vec![]).unwrap();
    let mut bytes = packet.serialize();
    bytes[0] = 0x00;
    assert!(FramePacket::deserialize(&bytes).is_err());
}

#[test]
fn test_payload_truncated() {
    let packet = FramePacket::new_frame(1, 1, 0, PixelFormat::Bgra32, vec![1, 2, 3, 4, 5]).unwrap();
    let bytes = packet.serialize();
    let truncated = &bytes[..bytes.len() - 3];
    assert!(FramePacket::deserialize(truncated).is_err());
}

#[test]
fn test_payload_too_large() {
    let huge = vec![0u8; 50_000_001];
    assert!(FramePacket::new_frame(1, 1, 0, PixelFormat::Bgra32, huge).is_err());
}

#[test]
fn test_version_mismatch() {
    let packet = FramePacket::new_frame(1, 1, 0, PixelFormat::Bgra32, vec![1]).unwrap();
    let mut bytes = packet.serialize();
    bytes[4] = 99;
    assert!(FramePacket::deserialize(&bytes).is_err());
}

#[test]
fn test_stream_read_write() {
    use std::io::Cursor;
    let packet = FramePacket::new_frame(320, 240, 555, PixelFormat::Bgra32, vec![0x1, 0x2]).unwrap();
    let mut buf = Vec::new();
    packet.write_to(&mut buf).unwrap();
    let mut cursor = Cursor::new(&buf);
    let decoded = FramePacket::read_from(&mut cursor).unwrap();
    assert_eq!(decoded.width, 320);
    assert_eq!(decoded.height, 240);
    assert_eq!(decoded.payload, vec![0x1, 0x2]);
}

#[test]
fn test_peek_payload_length() {
    let packet = FramePacket::new_frame(1920, 1080, 0, PixelFormat::H264, vec![0u8; 1000]).unwrap();
    let bytes = packet.serialize();
    assert_eq!(atlas_protocol::peek_payload_length(&bytes), Some(1000));
    assert_eq!(atlas_protocol::peek_payload_length(&bytes[..20]), None);
}

#[test]
fn test_total_packet_size() {
    assert_eq!(atlas_protocol::total_packet_size(0), atlas_protocol::FULL_HEADER_SIZE);
    assert_eq!(atlas_protocol::total_packet_size(500), atlas_protocol::FULL_HEADER_SIZE + 500);
    assert_eq!(atlas_protocol::total_packet_size(1_000_000), atlas_protocol::FULL_HEADER_SIZE + 1_000_000);
}

#[test]
fn test_has_enough_data() {
    assert!(!atlas_protocol::has_enough_data_for_header(&[1, 2, 3]));
    assert!(!atlas_protocol::has_enough_data_for_header(&[1; 24]));
    assert!(atlas_protocol::has_enough_data_for_header(&[1; 30]));
    assert!(atlas_protocol::has_enough_data_for_header(&[1; 100]));
}
