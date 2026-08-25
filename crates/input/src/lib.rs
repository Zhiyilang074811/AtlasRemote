//! Atlas Input - HID-based input injection for Windows
//!
//! Uses Windows SendInput API with proper HID key codes
//! Supports mouse, keyboard, and clipboard

use std::time::{SystemTime, UNIX_EPOCH};

use windows::Win32::UI::Input::KeyboardAndMouse::*;

/// Scroll wheel delta per click (Windows constant)
const WHEEL_DELTA: i32 = 120;

/// Get current timestamp in milliseconds
pub fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

/// Send mouse move to absolute position (0.0-1.0)
pub fn send_mouse_move(x: f32, y: f32) -> Result<(), String> {
    unsafe {
        let mx = (x * 65535.0) as i32;
        let my = (y * 65535.0) as i32;
        let inputs = [INPUT {
            r#type: INPUT_MOUSE,
            Anonymous: INPUT_0 { mi: MOUSEINPUT {
                dx: mx, dy: my, mouseData: 0,
                dwFlags: MOUSEEVENTF_MOVE | MOUSEEVENTF_ABSOLUTE,
                time: 0, dwExtraInfo: 0,
            } },
        }];
        let count = SendInput(&inputs, std::mem::size_of::<INPUT>() as i32);
        if count == 0 { Err("SendInput failed".to_string()) } else { Ok(()) }
    }
}

/// Send mouse click
pub fn send_mouse_click(button: u8, down: bool) -> Result<(), String> {
    unsafe {
        let (down_flag, up_flag) = match button {
            1 => (MOUSEEVENTF_LEFTDOWN, MOUSEEVENTF_LEFTUP),
            2 => (MOUSEEVENTF_RIGHTDOWN, MOUSEEVENTF_RIGHTUP),
            3 => (MOUSEEVENTF_MIDDLEDOWN, MOUSEEVENTF_MIDDLEUP),
            _ => (MOUSEEVENTF_LEFTDOWN, MOUSEEVENTF_LEFTUP),
        };
        let flags = if down { down_flag } else { up_flag };
        let inputs = [INPUT {
            r#type: INPUT_MOUSE,
            Anonymous: INPUT_0 { mi: MOUSEINPUT {
                dx: 0, dy: 0, mouseData: 0,
                dwFlags: flags, time: 0, dwExtraInfo: 0,
            } },
        }];
        let count = SendInput(&inputs, std::mem::size_of::<INPUT>() as i32);
        if count == 0 { Err("SendInput failed".to_string()) } else { Ok(()) }
    }
}

/// Send mouse wheel scroll
pub fn send_mouse_wheel(delta: i32) -> Result<(), String> {
    unsafe {
        let inputs = [INPUT {
            r#type: INPUT_MOUSE,
            Anonymous: INPUT_0 { mi: MOUSEINPUT {
                dx: 0, dy: 0, mouseData: (delta * WHEEL_DELTA) as u32,
                dwFlags: MOUSEEVENTF_WHEEL, time: 0, dwExtraInfo: 0,
            } },
        }];
        let count = SendInput(&inputs, std::mem::size_of::<INPUT>() as i32);
        if count == 0 { Err("SendInput failed".to_string()) } else { Ok(()) }
    }
}

/// Send keyboard key press/release using HID code
pub fn send_key(hid_code: u16, down: bool) -> Result<(), String> {
    unsafe {
        let flags = if down {
            KEYBD_EVENT_FLAGS(0)
        } else {
            KEYEVENTF_KEYUP
        };
        let inputs = [INPUT {
            r#type: INPUT_KEYBOARD,
            Anonymous: INPUT_0 { ki: KEYBDINPUT {
                wVk: VIRTUAL_KEY(hid_code), wScan: 0,
                dwFlags: flags, time: 0, dwExtraInfo: 0,
            } },
        }];
        let count = SendInput(&inputs, std::mem::size_of::<INPUT>() as i32);
        if count == 0 { Err("SendInput failed".to_string()) } else { Ok(()) }
    }
}

/// Convert ASCII char to HID code
pub fn char_to_hid(c: char) -> Option<u16> {
    match c {
        'a'..='z' => Some(0x04 + (c as u16 - b'a' as u16)),
        'A'..='Z' => Some(0x1E + (c as u16 - b'A' as u16)),
        '0'..='9' => Some(0x22 - (c as u8 - b'0') as u16),
        ' ' => Some(0x39),
        '\n' | '\r' => Some(0x28),
        '\t' => Some(0x0F),
        '\x08' => Some(0x0E),
        '\x1B' => Some(0x29),
        _ => None,
    }
}

/// Convert common key name to HID code
pub fn key_name_to_hid(name: &str) -> Option<u16> {
    match name.to_lowercase().as_str() {
        "enter" | "return" => Some(0x28),
        "escape" => Some(0x29),
        "backspace" | "bksp" => Some(0x0E),
        "tab" => Some(0x0F),
        "space" => Some(0x39),
        "delete" | "del" => Some(0x4C),
        "home" => Some(0x47),
        "end" => Some(0x4F),
        "pageup" | "pgup" => Some(0x49),
        "pagedown" | "pgdn" => Some(0x51),
        "up" => Some(0x52),
        "down" => Some(0x50),
        "left" => Some(0x4B),
        "right" => Some(0x4D),
        "insert" | "ins" => Some(0x52),
        "f1" => Some(0x3A),
        "f2" => Some(0x3B),
        "f3" => Some(0x3C),
        "f4" => Some(0x3D),
        "f5" => Some(0x3E),
        "f6" => Some(0x3F),
        "f7" => Some(0x40),
        "f8" => Some(0x41),
        "f9" => Some(0x42),
        "f10" => Some(0x43),
        "f11" => Some(0x44),
        "f12" => Some(0x45),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_char_to_hid() {
        assert_eq!(char_to_hid('a'), Some(0x1E));
        assert_eq!(char_to_hid('A'), Some(0x1E));
        assert_eq!(char_to_hid('0'), Some(0x22));
        assert_eq!(char_to_hid(' '), Some(0x39));
        assert_eq!(char_to_hid('z'), Some(0x1D));
    }

    #[test]
    fn test_key_name_to_hid() {
        assert_eq!(key_name_to_hid("enter"), Some(0x28));
        assert_eq!(key_name_to_hid("escape"), Some(0x29));
        assert_eq!(key_name_to_hid("backspace"), Some(0x0E));
        assert_eq!(key_name_to_hid("up"), Some(0x52));
        assert_eq!(key_name_to_hid("f1"), Some(0x3A));
        assert!(key_name_to_hid("unknownkey").is_none());
    }

    #[test]
    fn test_now_ms() {
        let ms = now_ms();
        assert!(ms > 0);
    }
}
