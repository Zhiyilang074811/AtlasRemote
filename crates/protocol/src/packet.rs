//! ATLS Protocol v1 - PacketType enum
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u16)]
pub enum PacketType {
    Frame = 1,
    Input = 2,
    Control = 3,
    PairRequest = 4,
    PairResponse = 5,
    Ping = 6,
    Pong = 7,
}
impl PacketType {
    pub fn from_u16(v: u16) -> Option<Self> {
        match v {
            1 => Some(PacketType::Frame),
            2 => Some(PacketType::Input),
            3 => Some(PacketType::Control),
            4 => Some(PacketType::PairRequest),
            5 => Some(PacketType::PairResponse),
            6 => Some(PacketType::Ping),
            7 => Some(PacketType::Pong),
            _ => None,
        }
    }
    pub fn to_u16(self) -> u16 { self as u16 }
}

pub const ATLS_MAGIC: [u8; 4] = [0x41, 0x54, 0x4C, 0x53];
pub const ATLS_VERSION: u16 = 1;
pub const HEADER_SIZE: usize = 30;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u16)]
pub enum FrameType {
    Bgra = 0,
    H264 = 2,
    H265 = 3,
}
impl FrameType {
    pub fn from_u16(v: u16) -> Option<Self> {
        match v {
            1 => Some(FrameType::Bgra),
            2 => Some(FrameType::H264),
            3 => Some(FrameType::H265),
            _ => None,
        }
    }
    pub fn to_u16(self) -> u16 { self as u16 }
}
