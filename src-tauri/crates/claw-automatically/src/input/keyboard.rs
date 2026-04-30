// Claw Desktop - 键盘输入模块
// 提供文本输入、按键按下/释放、快捷键组合等操作（跨平台实现）
use crate::error::{AutomaticallyError, Result};

/// 输入文本
pub async fn type_text(text: &str) -> Result<()> {
    for ch in text.chars() {
        let key = ch.to_string();
        press_key(&key).await?;
        tokio::time::sleep(tokio::time::Duration::from_millis(20)).await;
    }
    Ok(())
}

/// 按下并释放按键
pub async fn press_key(key: &str) -> Result<()> {
    key_down(key).await?;
    tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
    key_up(key).await
}

/// 按下按键（不释放）
pub async fn key_down(key: &str) -> Result<()> {
    #[cfg(target_os = "windows")]
    {
        key_down_windows(key)
    }

    #[cfg(target_os = "linux")]
    {
        key_down_linux(key)
    }

    #[cfg(target_os = "macos")]
    {
        key_down_macos(key)
    }

    #[cfg(not(any(target_os = "windows", target_os = "linux", target_os = "macos")))]
    {
        Err(AutomaticallyError::PlatformNotSupported(
            "Keyboard input not supported on this platform".to_string(),
        ))
    }
}

/// 释放按键
pub async fn key_up(key: &str) -> Result<()> {
    #[cfg(target_os = "windows")]
    {
        key_up_windows(key)
    }

    #[cfg(target_os = "linux")]
    {
        key_up_linux(key)
    }

    #[cfg(target_os = "macos")]
    {
        key_up_macos(key)
    }

    #[cfg(not(any(target_os = "windows", target_os = "linux", target_os = "macos")))]
    {
        Err(AutomaticallyError::PlatformNotSupported(
            "Keyboard input not supported on this platform".to_string(),
        ))
    }
}

/// Windows平台：按下按键 — 使用Win32 keybd_event API
#[cfg(target_os = "windows")]
fn key_down_windows(key: &str) -> Result<()> {
    use windows::Win32::UI::Input::KeyboardAndMouse::{
        KEYBD_EVENT_FLAGS, VK_0, VK_1, VK_2, VK_3, VK_4, VK_5, VK_6, VK_7, VK_8, VK_9, VK_A, VK_B,
        VK_BACK, VK_C, VK_CONTROL, VK_D, VK_DELETE, VK_DOWN, VK_E, VK_ESCAPE, VK_F, VK_F1, VK_F2,
        VK_F3, VK_F4, VK_F5, VK_F6, VK_F7, VK_F8, VK_F9, VK_F10, VK_F11, VK_F12, VK_G, VK_H, VK_I,
        VK_J, VK_K, VK_L, VK_LEFT, VK_LWIN, VK_M, VK_MENU, VK_N, VK_O, VK_P, VK_Q, VK_R, VK_RETURN,
        VK_RIGHT, VK_S, VK_SHIFT, VK_SPACE, VK_T, VK_TAB, VK_U, VK_UP, VK_V, VK_W, VK_X, VK_Y,
        VK_Z, keybd_event,
    };

    let vk = match key.to_lowercase().as_str() {
        "a" => VK_A.0,
        "b" => VK_B.0,
        "c" => VK_C.0,
        "d" => VK_D.0,
        "e" => VK_E.0,
        "f" => VK_F.0,
        "g" => VK_G.0,
        "h" => VK_H.0,
        "i" => VK_I.0,
        "j" => VK_J.0,
        "k" => VK_K.0,
        "l" => VK_L.0,
        "m" => VK_M.0,
        "n" => VK_N.0,
        "o" => VK_O.0,
        "p" => VK_P.0,
        "q" => VK_Q.0,
        "r" => VK_R.0,
        "s" => VK_S.0,
        "t" => VK_T.0,
        "u" => VK_U.0,
        "v" => VK_V.0,
        "w" => VK_W.0,
        "x" => VK_X.0,
        "y" => VK_Y.0,
        "z" => VK_Z.0,
        "0" => VK_0.0,
        "1" => VK_1.0,
        "2" => VK_2.0,
        "3" => VK_3.0,
        "4" => VK_4.0,
        "5" => VK_5.0,
        "6" => VK_6.0,
        "7" => VK_7.0,
        "8" => VK_8.0,
        "9" => VK_9.0,
        "space" | " " => VK_SPACE.0,
        "return" | "enter" => VK_RETURN.0,
        "tab" => VK_TAB.0,
        "escape" | "esc" => VK_ESCAPE.0,
        "backspace" | "back" => VK_BACK.0,
        "delete" | "del" => VK_DELETE.0,
        "shift" => VK_SHIFT.0,
        "ctrl" | "control" => VK_CONTROL.0,
        "alt" => VK_MENU.0,
        "win" | "command" | "cmd" => VK_LWIN.0,
        "left" => VK_LEFT.0,
        "right" => VK_RIGHT.0,
        "up" => VK_UP.0,
        "down" => VK_DOWN.0,
        "f1" => VK_F1.0,
        "f2" => VK_F2.0,
        "f3" => VK_F3.0,
        "f4" => VK_F4.0,
        "f5" => VK_F5.0,
        "f6" => VK_F6.0,
        "f7" => VK_F7.0,
        "f8" => VK_F8.0,
        "f9" => VK_F9.0,
        "f10" => VK_F10.0,
        "f11" => VK_F11.0,
        "f12" => VK_F12.0,
        _ => {
            // 尝试解析单个字符
            if key.len() == 1 {
                let ch = key.chars().next().ok_or_else(|| {
                    AutomaticallyError::Input(format!("Unsupported key: {}", key))
                })?;
                match ch {
                    'a'..='z' => VK_A.0 + (ch as u16 - 'a' as u16),
                    'A'..='Z' => VK_A.0 + (ch as u16 - 'A' as u16),
                    '0'..='9' => VK_0.0 + (ch as u16 - '0' as u16),
                    ' ' => VK_SPACE.0,
                    _ => {
                        return Err(AutomaticallyError::Input(format!(
                            "Unsupported key: {}",
                            key
                        )));
                    }
                }
            } else {
                return Err(AutomaticallyError::Input(format!(
                    "Unsupported key: {}",
                    key
                )));
            }
        }
    };

    unsafe {
        keybd_event(vk as u8, 0, KEYBD_EVENT_FLAGS(0), 0);
    }

    Ok(())
}

/// Windows平台：释放按键 — 使用Win32 keybd_event API
#[cfg(target_os = "windows")]
fn key_up_windows(key: &str) -> Result<()> {
    use windows::Win32::UI::Input::KeyboardAndMouse::{
        KEYEVENTF_KEYUP, VK_0, VK_1, VK_2, VK_3, VK_4, VK_5, VK_6, VK_7, VK_8, VK_9, VK_A, VK_B,
        VK_BACK, VK_C, VK_CONTROL, VK_D, VK_DELETE, VK_DOWN, VK_E, VK_ESCAPE, VK_F, VK_F1, VK_F2,
        VK_F3, VK_F4, VK_F5, VK_F6, VK_F7, VK_F8, VK_F9, VK_F10, VK_F11, VK_F12, VK_G, VK_H, VK_I,
        VK_J, VK_K, VK_L, VK_LEFT, VK_LWIN, VK_M, VK_MENU, VK_N, VK_O, VK_P, VK_Q, VK_R, VK_RETURN,
        VK_RIGHT, VK_S, VK_SHIFT, VK_SPACE, VK_T, VK_TAB, VK_U, VK_UP, VK_V, VK_W, VK_X, VK_Y,
        VK_Z, keybd_event,
    };

    let vk = match key.to_lowercase().as_str() {
        "a" => VK_A.0,
        "b" => VK_B.0,
        "c" => VK_C.0,
        "d" => VK_D.0,
        "e" => VK_E.0,
        "f" => VK_F.0,
        "g" => VK_G.0,
        "h" => VK_H.0,
        "i" => VK_I.0,
        "j" => VK_J.0,
        "k" => VK_K.0,
        "l" => VK_L.0,
        "m" => VK_M.0,
        "n" => VK_N.0,
        "o" => VK_O.0,
        "p" => VK_P.0,
        "q" => VK_Q.0,
        "r" => VK_R.0,
        "s" => VK_S.0,
        "t" => VK_T.0,
        "u" => VK_U.0,
        "v" => VK_V.0,
        "w" => VK_W.0,
        "x" => VK_X.0,
        "y" => VK_Y.0,
        "z" => VK_Z.0,
        "0" => VK_0.0,
        "1" => VK_1.0,
        "2" => VK_2.0,
        "3" => VK_3.0,
        "4" => VK_4.0,
        "5" => VK_5.0,
        "6" => VK_6.0,
        "7" => VK_7.0,
        "8" => VK_8.0,
        "9" => VK_9.0,
        "space" | " " => VK_SPACE.0,
        "return" | "enter" => VK_RETURN.0,
        "tab" => VK_TAB.0,
        "escape" | "esc" => VK_ESCAPE.0,
        "backspace" | "back" => VK_BACK.0,
        "delete" | "del" => VK_DELETE.0,
        "shift" => VK_SHIFT.0,
        "ctrl" | "control" => VK_CONTROL.0,
        "alt" => VK_MENU.0,
        "win" | "command" | "cmd" => VK_LWIN.0,
        "left" => VK_LEFT.0,
        "right" => VK_RIGHT.0,
        "up" => VK_UP.0,
        "down" => VK_DOWN.0,
        "f1" => VK_F1.0,
        "f2" => VK_F2.0,
        "f3" => VK_F3.0,
        "f4" => VK_F4.0,
        "f5" => VK_F5.0,
        "f6" => VK_F6.0,
        "f7" => VK_F7.0,
        "f8" => VK_F8.0,
        "f9" => VK_F9.0,
        "f10" => VK_F10.0,
        "f11" => VK_F11.0,
        "f12" => VK_F12.0,
        _ => {
            if key.len() == 1 {
                let ch = key.chars().next().ok_or_else(|| {
                    AutomaticallyError::Input(format!("Unsupported key: {}", key))
                })?;
                match ch {
                    'a'..='z' => VK_A.0 + (ch as u16 - 'a' as u16),
                    'A'..='Z' => VK_A.0 + (ch as u16 - 'A' as u16),
                    '0'..='9' => VK_0.0 + (ch as u16 - '0' as u16),
                    ' ' => VK_SPACE.0,
                    _ => {
                        return Err(AutomaticallyError::Input(format!(
                            "Unsupported key: {}",
                            key
                        )));
                    }
                }
            } else {
                return Err(AutomaticallyError::Input(format!(
                    "Unsupported key: {}",
                    key
                )));
            }
        }
    };

    unsafe {
        keybd_event(vk as u8, 0, KEYEVENTF_KEYUP, 0);
    }

    Ok(())
}

/// Linux平台：按下按键 — 使用X11 XTest扩展
#[cfg(target_os = "linux")]
fn key_down_linux(key: &str) -> Result<()> {
    use x11::xlib;
    use x11::xtest;

    let keysym = match key.to_lowercase().as_str() {
        "a" => x11::keysym::XK_a,
        "b" => x11::keysym::XK_b,
        "c" => x11::keysym::XK_c,
        "d" => x11::keysym::XK_d,
        "e" => x11::keysym::XK_e,
        "f" => x11::keysym::XK_f,
        "g" => x11::keysym::XK_g,
        "h" => x11::keysym::XK_h,
        "i" => x11::keysym::XK_i,
        "j" => x11::keysym::XK_j,
        "k" => x11::keysym::XK_k,
        "l" => x11::keysym::XK_l,
        "m" => x11::keysym::XK_m,
        "n" => x11::keysym::XK_n,
        "o" => x11::keysym::XK_o,
        "p" => x11::keysym::XK_p,
        "q" => x11::keysym::XK_q,
        "r" => x11::keysym::XK_r,
        "s" => x11::keysym::XK_s,
        "t" => x11::keysym::XK_t,
        "u" => x11::keysym::XK_u,
        "v" => x11::keysym::XK_v,
        "w" => x11::keysym::XK_w,
        "x" => x11::keysym::XK_x,
        "y" => x11::keysym::XK_y,
        "z" => x11::keysym::XK_z,
        "0" => x11::keysym::XK_0,
        "1" => x11::keysym::XK_1,
        "2" => x11::keysym::XK_2,
        "3" => x11::keysym::XK_3,
        "4" => x11::keysym::XK_4,
        "5" => x11::keysym::XK_5,
        "6" => x11::keysym::XK_6,
        "7" => x11::keysym::XK_7,
        "8" => x11::keysym::XK_8,
        "9" => x11::keysym::XK_9,
        "space" | " " => x11::keysym::XK_space,
        "return" | "enter" => x11::keysym::XK_Return,
        "tab" => x11::keysym::XK_Tab,
        "escape" | "esc" => x11::keysym::XK_Escape,
        "backspace" | "back" => x11::keysym::XK_BackSpace,
        "delete" | "del" => x11::keysym::XK_Delete,
        "shift" => x11::keysym::XK_Shift_L,
        "ctrl" | "control" => x11::keysym::XK_Control_L,
        "alt" => x11::keysym::XK_Alt_L,
        "win" | "command" | "cmd" => x11::keysym::XK_Super_L,
        "left" => x11::keysym::XK_Left,
        "right" => x11::keysym::XK_Right,
        "up" => x11::keysym::XK_Up,
        "down" => x11::keysym::XK_Down,
        "f1" => x11::keysym::XK_F1,
        "f2" => x11::keysym::XK_F2,
        "f3" => x11::keysym::XK_F3,
        "f4" => x11::keysym::XK_F4,
        "f5" => x11::keysym::XK_F5,
        "f6" => x11::keysym::XK_F6,
        "f7" => x11::keysym::XK_F7,
        "f8" => x11::keysym::XK_F8,
        "f9" => x11::keysym::XK_F9,
        "f10" => x11::keysym::XK_F10,
        "f11" => x11::keysym::XK_F11,
        "f12" => x11::keysym::XK_F12,
        _ => {
            if key.len() == 1 {
                let ch = key.chars().next().ok_or_else(|| {
                    AutomaticallyError::Input(format!("Unsupported key: {}", key))
                })?;
                match ch {
                    'a'..='z' => x11::keysym::XK_a + (ch as u64 - 'a' as u64),
                    'A'..='Z' => x11::keysym::XK_A + (ch as u64 - 'A' as u64),
                    '0'..='9' => x11::keysym::XK_0 + (ch as u64 - '0' as u64),
                    ' ' => x11::keysym::XK_space,
                    _ => {
                        return Err(AutomaticallyError::Input(format!(
                            "Unsupported key: {}",
                            key
                        )));
                    }
                }
            } else {
                return Err(AutomaticallyError::Input(format!(
                    "Unsupported key: {}",
                    key
                )));
            }
        }
    };

    unsafe {
        let display = xlib::XOpenDisplay(std::ptr::null());
        if display.is_null() {
            return Err(AutomaticallyError::Input(
                "Cannot open X display".to_string(),
            ));
        }

        let keycode = xlib::XKeysymToKeycode(display, keysym);
        xtest::XTestFakeKeyEvent(display, keycode as u32, 1, 0);
        xlib::XFlush(display);
        xlib::XCloseDisplay(display);
    }

    Ok(())
}

/// Linux平台：释放按键 — 使用X11 XTest扩展
#[cfg(target_os = "linux")]
fn key_up_linux(key: &str) -> Result<()> {
    use x11::xlib;
    use x11::xtest;

    let keysym = match key.to_lowercase().as_str() {
        "a" => x11::keysym::XK_a,
        "b" => x11::keysym::XK_b,
        "c" => x11::keysym::XK_c,
        "d" => x11::keysym::XK_d,
        "e" => x11::keysym::XK_e,
        "f" => x11::keysym::XK_f,
        "g" => x11::keysym::XK_g,
        "h" => x11::keysym::XK_h,
        "i" => x11::keysym::XK_i,
        "j" => x11::keysym::XK_j,
        "k" => x11::keysym::XK_k,
        "l" => x11::keysym::XK_l,
        "m" => x11::keysym::XK_m,
        "n" => x11::keysym::XK_n,
        "o" => x11::keysym::XK_o,
        "p" => x11::keysym::XK_p,
        "q" => x11::keysym::XK_q,
        "r" => x11::keysym::XK_r,
        "s" => x11::keysym::XK_s,
        "t" => x11::keysym::XK_t,
        "u" => x11::keysym::XK_u,
        "v" => x11::keysym::XK_v,
        "w" => x11::keysym::XK_w,
        "x" => x11::keysym::XK_x,
        "y" => x11::keysym::XK_y,
        "z" => x11::keysym::XK_z,
        "0" => x11::keysym::XK_0,
        "1" => x11::keysym::XK_1,
        "2" => x11::keysym::XK_2,
        "3" => x11::keysym::XK_3,
        "4" => x11::keysym::XK_4,
        "5" => x11::keysym::XK_5,
        "6" => x11::keysym::XK_6,
        "7" => x11::keysym::XK_7,
        "8" => x11::keysym::XK_8,
        "9" => x11::keysym::XK_9,
        "space" | " " => x11::keysym::XK_space,
        "return" | "enter" => x11::keysym::XK_Return,
        "tab" => x11::keysym::XK_Tab,
        "escape" | "esc" => x11::keysym::XK_Escape,
        "backspace" | "back" => x11::keysym::XK_BackSpace,
        "delete" | "del" => x11::keysym::XK_Delete,
        "shift" => x11::keysym::XK_Shift_L,
        "ctrl" | "control" => x11::keysym::XK_Control_L,
        "alt" => x11::keysym::XK_Alt_L,
        "win" | "command" | "cmd" => x11::keysym::XK_Super_L,
        "left" => x11::keysym::XK_Left,
        "right" => x11::keysym::XK_Right,
        "up" => x11::keysym::XK_Up,
        "down" => x11::keysym::XK_Down,
        "f1" => x11::keysym::XK_F1,
        "f2" => x11::keysym::XK_F2,
        "f3" => x11::keysym::XK_F3,
        "f4" => x11::keysym::XK_F4,
        "f5" => x11::keysym::XK_F5,
        "f6" => x11::keysym::XK_F6,
        "f7" => x11::keysym::XK_F7,
        "f8" => x11::keysym::XK_F8,
        "f9" => x11::keysym::XK_F9,
        "f10" => x11::keysym::XK_F10,
        "f11" => x11::keysym::XK_F11,
        "f12" => x11::keysym::XK_F12,
        _ => {
            if key.len() == 1 {
                let ch = key.chars().next().ok_or_else(|| {
                    AutomaticallyError::Input(format!("Unsupported key: {}", key))
                })?;
                match ch {
                    'a'..='z' => x11::keysym::XK_a + (ch as u64 - 'a' as u64),
                    'A'..='Z' => x11::keysym::XK_A + (ch as u64 - 'A' as u64),
                    '0'..='9' => x11::keysym::XK_0 + (ch as u64 - '0' as u64),
                    ' ' => x11::keysym::XK_space,
                    _ => {
                        return Err(AutomaticallyError::Input(format!(
                            "Unsupported key: {}",
                            key
                        )));
                    }
                }
            } else {
                return Err(AutomaticallyError::Input(format!(
                    "Unsupported key: {}",
                    key
                )));
            }
        }
    };

    unsafe {
        let display = xlib::XOpenDisplay(std::ptr::null());
        if display.is_null() {
            return Err(AutomaticallyError::Input(
                "Cannot open X display".to_string(),
            ));
        }

        let keycode = xlib::XKeysymToKeycode(display, keysym);
        xtest::XTestFakeKeyEvent(display, keycode as u32, 0, 0);
        xlib::XFlush(display);
        xlib::XCloseDisplay(display);
    }

    Ok(())
}

/// macOS平台：按下按键 — 使用CoreGraphics CGEvent API
#[cfg(target_os = "macos")]
fn key_down_macos(key: &str) -> Result<()> {
    use core_graphics::event::{CGEvent, CGEventFlags, CGKeyCode};
    use core_graphics::event_source::{CGEventSource, CGEventSourceStateID};

    let keycode = match key.to_lowercase().as_str() {
        "a" => 0,
        "s" => 1,
        "d" => 2,
        "f" => 3,
        "h" => 4,
        "g" => 5,
        "z" => 6,
        "x" => 7,
        "c" => 8,
        "v" => 9,
        "b" => 11,
        "q" => 12,
        "w" => 13,
        "e" => 14,
        "r" => 15,
        "y" => 16,
        "t" => 17,
        "1" => 18,
        "2" => 19,
        "3" => 20,
        "4" => 21,
        "6" => 22,
        "5" => 23,
        "=" => 24,
        "9" => 25,
        "7" => 26,
        "-" => 27,
        "8" => 28,
        "0" => 29,
        "]" => 30,
        "o" => 31,
        "u" => 32,
        "[" => 33,
        "i" => 34,
        "p" => 35,
        "return" | "enter" => 36,
        "l" => 37,
        "j" => 38,
        "'" => 39,
        "k" => 40,
        ";" => 41,
        "\\" => 42,
        "," => 43,
        "/" => 44,
        "n" => 45,
        "m" => 46,
        "." => 47,
        "tab" => 48,
        "space" | " " => 49,
        "`" => 50,
        "delete" | "backspace" | "back" => 51,
        "escape" | "esc" => 53,
        "command" | "cmd" | "win" => 55,
        "shift" => 56,
        "caps" => 57,
        "option" | "alt" => 58,
        "control" | "ctrl" => 59,
        "right" => 124,
        "left" => 123,
        "down" => 125,
        "up" => 126,
        _ => {
            return Err(AutomaticallyError::Input(format!(
                "Unsupported key: {}",
                key
            )));
        }
    };

    let source = CGEventSource::new(CGEventSourceStateID::HIDSystemState)
        .map_err(|_| AutomaticallyError::Input("Failed to create event source".to_string()))?;

    let event = CGEvent::new_keyboard_event(source, keycode as CGKeyCode, true)
        .map_err(|_| AutomaticallyError::Input("Failed to create key event".to_string()))?;

    event.post(core_graphics::event::CGEventTapLocation::HID);

    Ok(())
}

/// macOS平台：释放按键 — 使用CoreGraphics CGEvent API
#[cfg(target_os = "macos")]
fn key_up_macos(key: &str) -> Result<()> {
    use core_graphics::event::{CGEvent, CGEventFlags, CGKeyCode};
    use core_graphics::event_source::{CGEventSource, CGEventSourceStateID};

    let keycode = match key.to_lowercase().as_str() {
        "a" => 0,
        "s" => 1,
        "d" => 2,
        "f" => 3,
        "h" => 4,
        "g" => 5,
        "z" => 6,
        "x" => 7,
        "c" => 8,
        "v" => 9,
        "b" => 11,
        "q" => 12,
        "w" => 13,
        "e" => 14,
        "r" => 15,
        "y" => 16,
        "t" => 17,
        "1" => 18,
        "2" => 19,
        "3" => 20,
        "4" => 21,
        "6" => 22,
        "5" => 23,
        "=" => 24,
        "9" => 25,
        "7" => 26,
        "-" => 27,
        "8" => 28,
        "0" => 29,
        "]" => 30,
        "o" => 31,
        "u" => 32,
        "[" => 33,
        "i" => 34,
        "p" => 35,
        "return" | "enter" => 36,
        "l" => 37,
        "j" => 38,
        "'" => 39,
        "k" => 40,
        ";" => 41,
        "\\" => 42,
        "," => 43,
        "/" => 44,
        "n" => 45,
        "m" => 46,
        "." => 47,
        "tab" => 48,
        "space" | " " => 49,
        "`" => 50,
        "delete" | "backspace" | "back" => 51,
        "escape" | "esc" => 53,
        "command" | "cmd" | "win" => 55,
        "shift" => 56,
        "caps" => 57,
        "option" | "alt" => 58,
        "control" | "ctrl" => 59,
        "right" => 124,
        "left" => 123,
        "down" => 125,
        "up" => 126,
        _ => {
            return Err(AutomaticallyError::Input(format!(
                "Unsupported key: {}",
                key
            )));
        }
    };

    let source = CGEventSource::new(CGEventSourceStateID::HIDSystemState)
        .map_err(|_| AutomaticallyError::Input("Failed to create event source".to_string()))?;

    let event = CGEvent::new_keyboard_event(source, keycode as CGKeyCode, false)
        .map_err(|_| AutomaticallyError::Input("Failed to create key event".to_string()))?;

    event.post(core_graphics::event::CGEventTapLocation::HID);

    Ok(())
}
