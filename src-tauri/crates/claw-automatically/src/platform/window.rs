// Claw Desktop - 窗口管理模块
// 提供获取活动窗口、窗口标题、窗口列表、窗口聚焦、屏幕尺寸等平台功能
use crate::error::{AutomaticallyError, Result};
use crate::types::WindowInfo;
use serde::{Deserialize, Serialize};

/// 窗口矩形区域
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WindowRect {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
}

/// 获取当前活动窗口信息 — 根据平台自动选择实现
pub fn get_active_window() -> Result<WindowInfo> {
    #[cfg(target_os = "windows")]
    {
        get_active_window_windows()
    }

    #[cfg(target_os = "linux")]
    {
        get_active_window_linux()
    }

    #[cfg(target_os = "macos")]
    {
        get_active_window_macos()
    }

    #[cfg(not(any(target_os = "windows", target_os = "linux", target_os = "macos")))]
    {
        Err(AutomaticallyError::PlatformNotSupported(
            "Window management not supported on this platform".to_string()
        ))
    }
}

/// 获取当前活动窗口标题
pub fn get_window_title() -> Result<String> {
    let info = get_active_window()?;
    Ok(info.title)
}

/// 列出所有可见窗口 — 根据平台自动选择实现
pub fn list_windows() -> Result<Vec<WindowInfo>> {
    #[cfg(target_os = "windows")]
    {
        list_windows_windows()
    }

    #[cfg(target_os = "linux")]
    {
        list_windows_linux()
    }

    #[cfg(target_os = "macos")]
    {
        list_windows_macos()
    }

    #[cfg(not(any(target_os = "windows", target_os = "linux", target_os = "macos")))]
    {
        Err(AutomaticallyError::PlatformNotSupported(
            "Window listing not supported on this platform".to_string()
        ))
    }
}

/// 聚焦窗口 — 按标题关键字查找并激活窗口
pub fn focus_window(title_contains: &str) -> Result<()> {
    #[cfg(target_os = "windows")]
    {
        focus_window_windows(title_contains)
    }

    #[cfg(target_os = "linux")]
    {
        focus_window_linux(title_contains)
    }

    #[cfg(target_os = "macos")]
    {
        focus_window_macos(title_contains)
    }

    #[cfg(not(any(target_os = "windows", target_os = "linux", target_os = "macos")))]
    {
        Err(AutomaticallyError::PlatformNotSupported(
            "Window focus not supported on this platform".to_string()
        ))
    }
}

/// 获取屏幕尺寸 — 返回(宽度, 高度)像素值
pub fn get_screen_size() -> Result<(u32, u32)> {
    #[cfg(target_os = "windows")]
    {
        get_screen_size_windows()
    }

    #[cfg(target_os = "linux")]
    {
        get_screen_size_linux()
    }

    #[cfg(target_os = "macos")]
    {
        get_screen_size_macos()
    }

    #[cfg(not(any(target_os = "windows", target_os = "linux", target_os = "macos")))]
    {
        Err(AutomaticallyError::PlatformNotSupported(
            "Screen size query not supported on this platform".to_string()
        ))
    }
}

/// 最小化当前活动窗口
pub fn minimize_window() -> Result<()> {
    #[cfg(target_os = "windows")]
    {
        minimize_window_windows()
    }

    #[cfg(target_os = "linux")]
    {
        minimize_window_linux()
    }

    #[cfg(target_os = "macos")]
    {
        minimize_window_macos()
    }

    #[cfg(not(any(target_os = "windows", target_os = "linux", target_os = "macos")))]
    {
        Err(AutomaticallyError::PlatformNotSupported(
            "Window minimize not supported on this platform".to_string()
        ))
    }
}

#[cfg(target_os = "windows")]
use windows::Win32::Foundation::{BOOL, HWND, LPARAM};

/// Windows平台：获取前台窗口信息
#[cfg(target_os = "windows")]
fn get_active_window_windows() -> Result<WindowInfo> {
    use windows::Win32::UI::WindowsAndMessaging::{GetForegroundWindow, GetWindowTextW, GetWindowRect};
    use windows::Win32::Foundation::RECT;

    unsafe {
        let hwnd = GetForegroundWindow();
        let mut title_buf = [0u16; 512];
        let title_len = GetWindowTextW(hwnd, &mut title_buf);
        let title = String::from_utf16_lossy(&title_buf[..title_len as usize]);

        let mut rect = RECT::default();
        let _ = GetWindowRect(hwnd, &mut rect);

        Ok(WindowInfo {
            title,
            process_id: 0,
            window_id: hwnd.0 as u64,
            rect: Some(WindowRect {
                x: rect.left,
                y: rect.top,
                width: rect.right - rect.left,
                height: rect.bottom - rect.top,
            }),
        })
    }
}

/// Windows平台：枚举所有可见窗口
#[cfg(target_os = "windows")]
fn list_windows_windows() -> Result<Vec<WindowInfo>> {
    use windows::Win32::UI::WindowsAndMessaging::EnumWindows;
    use std::sync::Mutex;

    let windows: Mutex<Vec<WindowInfo>> = Mutex::new(Vec::new());

    unsafe {
        let lparam = LPARAM(&windows as *const Mutex<Vec<WindowInfo>> as isize);
        let _ = EnumWindows(Some(enum_windows_callback), lparam);
    }

    Ok(windows.into_inner().unwrap_or_default())
}

/// Windows回调：枚举窗口 — 收集可见窗口的标题、PID和位置
#[cfg(target_os = "windows")]
unsafe extern "system" fn enum_windows_callback(hwnd: HWND, lparam: LPARAM) -> BOOL {
    use windows::Win32::UI::WindowsAndMessaging::{IsWindowVisible, GetWindowTextW, GetWindowThreadProcessId, GetWindowRect};
    use windows::Win32::Foundation::RECT;
    use std::sync::Mutex;

    unsafe {
        if !IsWindowVisible(hwnd).as_bool() {
            return BOOL(1);
        }

        let mut title_buf = [0u16; 512];
        let title_len = GetWindowTextW(hwnd, &mut title_buf);
        let title = String::from_utf16_lossy(&title_buf[..title_len as usize]);

        if title.is_empty() {
            return BOOL(1);
        }

        let mut pid: u32 = 0;
        let _ = GetWindowThreadProcessId(hwnd, Some(&mut pid));

        let mut rect = RECT::default();
        let _ = GetWindowRect(hwnd, &mut rect);

        let info = WindowInfo {
            title,
            process_id: pid,
            window_id: hwnd.0 as u64,
            rect: Some(WindowRect {
                x: rect.left,
                y: rect.top,
                width: rect.right - rect.left,
                height: rect.bottom - rect.top,
            }),
        };

        let windows = &*(lparam.0 as *const Mutex<Vec<WindowInfo>>);
        if let Ok(mut guard) = windows.lock() {
            guard.push(info);
        }

        BOOL(1)
    }
}

/// Windows平台：按标题关键字查找并聚焦窗口
#[cfg(target_os = "windows")]
fn focus_window_windows(title_contains: &str) -> Result<()> {
    use windows::Win32::UI::WindowsAndMessaging::{EnumWindows, SetForegroundWindow, ShowWindow, SW_RESTORE};
    use std::sync::Mutex;

    let target = title_contains.to_lowercase();
    let found: Mutex<Option<(HWND, String)>> = Mutex::new(None);

    #[allow(dead_code)]
    struct CallbackData {
        target: String,
        found: Mutex<Option<(HWND, String)>>,
    }

    let data = CallbackData { target, found };

    unsafe {
        let lparam = LPARAM(&data as *const CallbackData as isize);
        let _ = EnumWindows(Some(find_window_callback), lparam);
    }

    if let Some((hwnd, _title)) = data.found.into_inner().unwrap_or_default() {
        unsafe {
            let _ = ShowWindow(hwnd, SW_RESTORE);
            let _ = SetForegroundWindow(hwnd);
        }
        Ok(())
    } else {
        Err(AutomaticallyError::Automation(format!(
            "No window found containing '{}'", title_contains
        )))
    }
}

/// Windows回调：查找窗口 — 按标题关键字匹配窗口
#[cfg(target_os = "windows")]
unsafe extern "system" fn find_window_callback(hwnd: HWND, lparam: LPARAM) -> BOOL {
    use windows::Win32::UI::WindowsAndMessaging::{IsWindowVisible, GetWindowTextW};

    struct CallbackData {
        target: String,
        found: std::sync::Mutex<Option<(HWND, String)>>,
    }

    unsafe {
        if !IsWindowVisible(hwnd).as_bool() {
            return BOOL(1);
        }

        let mut title_buf = [0u16; 512];
        let title_len = GetWindowTextW(hwnd, &mut title_buf);
        let title = String::from_utf16_lossy(&title_buf[..title_len as usize]);

        if title.is_empty() {
            return BOOL(1);
        }

        let data = &*(lparam.0 as *const CallbackData);
        if title.to_lowercase().contains(&data.target) {
            if let Ok(mut guard) = data.found.lock() {
                *guard = Some((hwnd, title));
            }
            return BOOL(0);
        }

        BOOL(1)
    }
}

/// Windows平台：获取屏幕尺寸
#[cfg(target_os = "windows")]
fn get_screen_size_windows() -> Result<(u32, u32)> {
    use windows::Win32::Graphics::Gdi::{GetDC, GetDeviceCaps, HORZRES, VERTRES, ReleaseDC};
    use windows::Win32::UI::WindowsAndMessaging::GetDesktopWindow;

    unsafe {
        let hwnd = GetDesktopWindow();
        let hdc = GetDC(hwnd);
        if hdc.0.is_null() {
            return Err(AutomaticallyError::Capture("Failed to get screen DC".to_string()));
        }
        let width = GetDeviceCaps(hdc, HORZRES) as u32;
        let height = GetDeviceCaps(hdc, VERTRES) as u32;
        ReleaseDC(hwnd, hdc);
        Ok((width, height))
    }
}

/// Windows平台：最小化前台窗口
#[cfg(target_os = "windows")]
fn minimize_window_windows() -> Result<()> {
    use windows::Win32::UI::WindowsAndMessaging::{GetForegroundWindow, ShowWindow, SW_MINIMIZE};

    unsafe {
        let hwnd = GetForegroundWindow();
        let _ = ShowWindow(hwnd, SW_MINIMIZE);
    }
    Ok(())
}

/// Linux平台：获取活动窗口（需要xdotool）
#[cfg(target_os = "linux")]
fn get_active_window_linux() -> Result<WindowInfo> {
    Err(AutomaticallyError::PlatformNotSupported(
        "Linux active window detection requires xdotool".to_string()
    ))
}

/// Linux平台：列出窗口（需要wmctrl）
#[cfg(target_os = "linux")]
fn list_windows_linux() -> Result<Vec<WindowInfo>> {
    Err(AutomaticallyError::PlatformNotSupported(
        "Linux window listing requires wmctrl".to_string()
    ))
}

/// Linux平台：聚焦窗口（需要xdotool）
#[cfg(target_os = "linux")]
fn focus_window_linux(_title_contains: &str) -> Result<()> {
    Err(AutomaticallyError::PlatformNotSupported(
        "Linux window focus requires xdotool".to_string()
    ))
}

/// Linux平台：获取屏幕尺寸 — 使用X11 API
#[cfg(target_os = "linux")]
fn get_screen_size_linux() -> Result<(u32, u32)> {
    use x11::xlib;

    unsafe {
        let display = xlib::XOpenDisplay(std::ptr::null());
        if display.is_null() {
            return Err(AutomaticallyError::Capture("Cannot open X display".to_string()));
        }
        let screen = xlib::XDefaultScreen(display);
        let width = xlib::XDisplayWidth(display, screen) as u32;
        let height = xlib::XDisplayHeight(display, screen) as u32;
        xlib::XCloseDisplay(display);
        Ok((width, height))
    }
}

/// Linux平台：最小化窗口（需要xdotool）
#[cfg(target_os = "linux")]
fn minimize_window_linux() -> Result<()> {
    Err(AutomaticallyError::PlatformNotSupported(
        "Linux window minimize requires xdotool".to_string()
    ))
}

/// macOS平台：获取活动窗口（需要Accessibility API）
#[cfg(target_os = "macos")]
fn get_active_window_macos() -> Result<WindowInfo> {
    Err(AutomaticallyError::PlatformNotSupported(
        "macOS active window detection requires Accessibility API".to_string()
    ))
}

/// macOS平台：列出窗口（需要Accessibility API）
#[cfg(target_os = "macos")]
fn list_windows_macos() -> Result<Vec<WindowInfo>> {
    Err(AutomaticallyError::PlatformNotSupported(
        "macOS window listing requires Accessibility API".to_string()
    ))
}

/// macOS平台：聚焦窗口（需要Accessibility API）
#[cfg(target_os = "macos")]
fn focus_window_macos(_title_contains: &str) -> Result<()> {
    Err(AutomaticallyError::PlatformNotSupported(
        "macOS window focus requires Accessibility API".to_string()
    ))
}

/// macOS平台：获取屏幕尺寸 — 使用CoreGraphics API
#[cfg(target_os = "macos")]
fn get_screen_size_macos() -> Result<(u32, u32)> {
    use core_graphics::display::{CGMainDisplayID, CGDisplay};

    let display_id = CGMainDisplayID();
    let display = CGDisplay::new(display_id);
    let width = display.pixels_wide();
    let height = display.pixels_high();
    Ok((width, height))
}

/// macOS平台：最小化窗口（需要Accessibility API）
#[cfg(target_os = "macos")]
fn minimize_window_macos() -> Result<()> {
    Err(AutomaticallyError::PlatformNotSupported(
        "macOS window minimize requires Accessibility API".to_string()
    ))
}
