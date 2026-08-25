# ATLS Protocol Specification v1.0 (Frozen)

> **Status**: FROZEN - Do not change without version bump
> **Version**: 1.0
> **Date**: 2026-08-21

---

## Overview

ATLS (Atlas Transport Layer Security) is the protocol used by AtlasRemote for:
- Video frame transmission
- Input/control commands
- Session management

---

## Frame Packet Format

```
Offset  Size    Field          Description
------  ----    -----          -----------
0       4       Magic          0x41544C53 ("ATLS")
4       2       Version        0x0001
6       2       PacketType     Frame=0x0001, Input=0x0002
8       4       PayloadLen     Length of payload in bytes
12      4       Width          Frame width (pixels)
16      4       Height         Frame height (pixels)
20      4       Timestamp      Milliseconds since epoch
24      2       Codec          0=BGRA, 2=H264, 3=H265, 4=JPEG
26      4       CRC32          CRC32 of header+codec+payload
30      N       Payload        Frame data or input data
```

**Total Header**: 30 bytes (FULL_HEADER_SIZE)

---

## Packet Types

| Type     | Value  | Description              |
|----------|--------|--------------------------|
| Frame    | 0x0001 | Video frame              |
| Input    | 0x0002 | Mouse/keyboard input     |
| Ping     | 0x0003 | Keepalive ping           |
| Pong     | 0x0004 | Keepalive response       |
| PairReq  | 0x0005 | Pairing request          |
| PairRes  | 0x0006 | Pairing response         |

---

## Codec Types

| Codec    | Value  | Description              |
|----------|--------|--------------------------|
| BGRA     | 0x0000 | Raw 32-bit BGRA          |
| H264     | 0x0002 | H.264 NAL units          |
| H265     | 0x0003 | H.265/HEVC NAL units     |
| JPEG     | 0x0004 | JPEG compressed          |

---

## Input Packet Format

```
Offset  Size    Field          Description
------  ----    -----          -----------
0       4       Magic          0x494E5054 ("INPT")
4       1       Version        0x01
5       1       InputType      MouseMove=1, MouseButton=2, etc.
6       2       PayloadLen     Length of payload
8       8       Timestamp      Microseconds since epoch
16      N       Payload        Input data
```

**Total Header**: 13 bytes (INPUT_HEADER_SIZE)

---

## Input Types

| Type           | Value | Payload Format                  |
|----------------|-------|----------------------------------|
| MouseMove      | 1     | x(f32le) + y(f32le) = 8 bytes   |
| MouseButton    | 2     | button(u8) + down(u8) = 2 bytes |
| MouseWheel     | 3     | delta(i32le) = 4 bytes          |
| KeyPress       | 4     | hid_code(u16le) + down(u8) = 3 bytes |
| Clipboard      | 5     | UTF-8 text                      |

---

## CRC32 Algorithm

IEEE 802.3 CRC32 polynomial: 0xEDB88320

```rust
fn crc32(data: &[u8]) -> u32 {
    let mut crc = 0xFFFFFFFFu32;
    for &b in data {
        crc ^= b as u32;
        for _ in 0..8 {
            crc = if crc & 1 != 0 { (crc >> 1) ^ 0xEDB88320 } else { crc >> 1 };
        }
    }
    crc ^ 0xFFFFFFFF
}
```

**CRC covers**: header (24 bytes) + codec (2 bytes) + payload (N bytes)

---

## Quality Levels (ABR)

| Level    | Resolution | FPS  | Target Bitrate |
|----------|------------|------|----------------|
| UltraLow | 640x360    | 15   | 500 kbps       |
| Low      | 1280x720   | 24   | 1 Mbps         |
| Medium   | 1280x720   | 30   | 2 Mbps         |
| High     | 1920x1080  | 55   | 4 Mbps         |
| Ultra    | 1920x1080  | 60   | 8 Mbps         |

---

## Security Notes

- Pairing codes are 6-digit numeric, 5-minute TTL
- Ed25519 signatures for device authentication
- X25519 key exchange for session key establishment
- AES-256-GCM for encrypted tunnel
- Session keys expire after 60 minutes

---

## Version History

| Version | Date       | Changes                    |
|---------|------------|----------------------------|
| 1.0     | 2026-08-21 | Initial frozen spec        |

---

## Future Extensions

- [ ] Multi-monitor support
- [ ] File transfer protocol
- [ ] Privacy black screen mode
- [ ] QUIC transport option
