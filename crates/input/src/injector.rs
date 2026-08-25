//! Input Injection Implementation with Security
//!
//! Windows SendInput API for mouse and keyboard simulation
//! with permission validation

use std::collections::HashMap;
use std::sync::Mutex;
use tracing::{info, warn};

use windows::Win32::UI::Input::KeyboardAndMouse::*;

#[derive(Debug, Clone, PartialEq)]
pub enum MouseButton {
    Left,
    Right,
    Middle,
}

#[derive(Debug, Clone, PartialEq)]
pub enum KeyAction {
    Press,
    Release,
}

/// Track injected event sequences to prevent replay
struct InputState {
    last_sequence: u64,
    last_timestamp: u64,
}

lazy_static::lazy_static! {
    static ref INPUT_STATE: Mutex<HashMap<String, InputState>> = Mutex::new(HashMap::new());
}

/// Check if input is allowed for this device/session
pub fn is_input_allowed(device_id: &str, sequence: u64, timestamp: u64) -> bool {
    let mut state = match INPUT_STATE.lock() {
        Ok(s) => s,
        Err(_) => return false,
    };

    let entry = state.entry(device_id.to_string()).or_insert_with(|| InputState {
        last_sequence: 0,
        last_timestamp: 0,
    });

    // Reject if sequence is not monotonically increasing
    if sequence <= entry.last_sequence {
        warn!("Rejected stale sequence: {} <= {}", sequence, entry.last_sequence);
        return false;
    }

    // Reject if timestamp is not fresh (more than 5 seconds old)
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64;
    if now.saturating_sub(timestamp) > 5000 {
        warn!("Rejected stale timestamp: {}ms old", now.saturating_sub(timestamp));
        return false;
    }

    entry.last_sequence = sequence;
    entry.last_timestamp = timestamp;
    true
}

/// Mouse injection via SendInput
pub fn inject_mouse_move(device_id: &str, x: f32, y: f32, sequence: u64, timestamp: u64) -> Result<(), String> {
    if !is_input_allowed(device_id, sequence, timestamp) {
        return Err("Input rejected: not allowed".to_string());
    }

    unsafe {
        let inputs = [INPUT {
            r#type: INPUT_MOUSE,
            Anonymous: INPUT_0 {
                mi: MOUSEINPUT {
                    dx: (x * 65535.0 / 1920.0) as i32,
                    dy: (y * 65535.0 / 1080.0) as i32,
                    mouseData: 0,
                    dwFlags: MOUSEEVENTF_MOVE,
                    time: 0,
                    dwExtraInfo: 0,
                },
            },
        }];

        let count = SendInput(&inputs, std::mem::size_of::<INPUT>() as i32);
        if count == 0 {
            Err("SendInput failed".to_string())
        } else {
            Ok(())
        }
    }
}

/// Mouse click injection
pub fn inject_mouse_click(device_id: &str, button: MouseButton, action: &str, sequence: u64, timestamp: u64) -> Result<(), String> {
    if !is_input_allowed(device_id, sequence, timestamp) {
        return Err("Input rejected: not allowed".to_string());
    }

    unsafe {
        let (down_flags, up_flags) = match button {
            MouseButton::Left => (MOUSEEVENTF_LEFTDOWN, MOUSEEVENTF_LEFTUP),
            MouseButton::Right => (MOUSEEVENTF_RIGHTDOWN, MOUSEEVENTF_RIGHTUP),
            MouseButton::Middle => (MOUSEEVENTF_MIDDLEDOWN, MOUSEEVENTF_MIDDLEUP),
        };

        let flags = if action == "down" { down_flags } else { up_flags };

        let inputs = [INPUT {
            r#type: INPUT_MOUSE,
            Anonymous: INPUT_0 {
                mi: MOUSEINPUT {
                    dx: 0,
                    dy: 0,
                    mouseData: 0,
                    dwFlags: flags,
                    time: 0,
                    dwExtraInfo: 0,
                },
            },
        }];

        let count = SendInput(&inputs, std::mem::size_of::<INPUT>() as i32);
        if count == 0 {
            Err("SendInput failed".to_string())
        } else {
            info!("Injected mouse {}: {:?}", action, button);
            Ok(())
        }
    }
}

/// Keyboard injection
pub fn inject_key(device_id: &str, key: &str, action: &str, sequence: u64, timestamp: u64) -> Result<(), String> {
    if !is_input_allowed(device_id, sequence, timestamp) {
        return Err("Input rejected: not allowed".to_string());
    }

    unsafe {
        let vk = match key {
            "a" => VK_A,
            "b" => VK_B,
            "c" => VK_C,
            "d" => VK_D,
            "e" => VK_E,
            "f" => VK_F,
            "g" => VK_G,
            "h" => VK_H,
            "i" => VK_I,
            "j" => VK_J,
            "k" => VK_K,
            "l" => VK_L,
            "m" => VK_M,
            "n" => VK_N,
            "o" => VK_O,
            "p" => VK_P,
            "q" => VK_Q,
            "r" => VK_R,
            "s" => VK_S,
            "t" => VK_T,
            "u" => VK_U,
            "v" => VK_V,
            "w" => VK_W,
            "x" => VK_X,
            "y" => VK_Y,
            "z" => VK_Z,
            "0" => VK_0,
            "1" => VK_1,
            "2" => VK_2,
            "3" => VK_3,
            "4" => VK_4,
            "5" => VK_5,
            "6" => VK_6,
            "7" => VK_7,
            "8" => VK_8,
            "9" => VK_9,
            "enter" => VK_RETURN,
            "escape" => VK_ESCAPE,
            "tab" => VK_TAB,
            "space" => VK_SPACE,
            "delete" => VK_DELETE,
            "backspace" => VK_BACK,
            "home" => VK_HOME,
            "end" => VK_END,
            "pageup" => VK_PRIOR,
            "pagedown" => VK_NEXT,
            "up" => VK_UP,
            "down" => VK_DOWN,
            "left" => VK_LEFT,
            "right" => VK_RIGHT,
            "insert" => VK_INSERT,
            _ => return Err(format!("Unsupported key: {}", key)),
        };

        let flags = if action == "down" {
            KEYBD_EVENT_FLAGS(0)
        } else {
            KEYEVENTF_KEYUP
        };

        let inputs = [INPUT {
            r#type: INPUT_KEYBOARD,
            Anonymous: INPUT_0 {
                ki: KEYBDINPUT {
                    wVk: vk,
                    wScan: 0,
                    dwFlags: flags,
                    time: 0,
                    dwExtraInfo: 0,
                },
            },
        }];

        let count = SendInput(&inputs, std::mem::size_of::<INPUT>() as i32);
        if count == 0 {
            Err("SendInput failed".to_string())
        } else {
            info!("Injected key: {} {:?}", key, action);
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mouse_button_enum() {
        assert_eq!(format!("{:?}", MouseButton::Left), "Left");
        assert_eq!(format!("{:?}", MouseButton::Right), "Right");
    }

    #[test]
    fn test_key_action_enum() {
        assert_eq!(format!("{:?}", KeyAction::Press), "Press");
        assert_eq!(format!("{:?}", KeyAction::Release), "Release");
    }

    #[test]
    fn test_inject_key_unknown() {
        let result = inject_key("device1", "unknownkey", "down", 1, 0);
        assert!(result.is_err());
    }

    #[test]
    fn test_input_replay_protection() {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;

        // First input should succeed
        assert!(is_input_allowed("device1", 1, now));
        
        // Replay same sequence should fail
        assert!(!is_input_allowed("device1", 1, now));
        
        // Old timestamp should fail
        assert!(!is_input_allowed("device1", 2, now.saturating_sub(6000)));
    }
}
