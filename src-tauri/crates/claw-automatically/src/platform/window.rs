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
            "Window management not supported on this platform".to_string(),
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
            "Window listing not supported on this platform".to_string(),
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
            "Window focus not supported on this platform".to_string(),
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
            "Screen size query not supported on this platform".to_string(),
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
            "Window minimize not supported on this platform".to_string(),
        ))
    }
}

#[cfg(target_os = "windows")]
use windows::Win32::Foundation::{BOOL, HWND, LPARAM};

/// Windows平台：获取前台窗口信息
#[cfg(target_os = "windows")]
fn get_active_window_windows() -> Result<WindowInfo> {
    use windows::Win32::Foundation::RECT;
    use windows::Win32::UI::WindowsAndMessaging::{
        GetForegroundWindow, GetWindowRect, GetWindowTextW,
    };

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
    use std::sync::Mutex;
    use windows::Win32::UI::WindowsAndMessaging::EnumWindows;

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
    use std::sync::Mutex;
    use windows::Win32::Foundation::RECT;
    use windows::Win32::UI::WindowsAndMessaging::{
        GetWindowRect, GetWindowTextW, GetWindowThreadProcessId, IsWindowVisible,
    };

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
    use std::sync::Mutex;
    use windows::Win32::UI::WindowsAndMessaging::{
        EnumWindows, SW_RESTORE, SetForegroundWindow, ShowWindow,
    };

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
            "No window found containing '{}'",
            title_contains
        )))
    }
}

/// Windows回调：查找窗口 — 按标题关键字匹配窗口
#[cfg(target_os = "windows")]
unsafe extern "system" fn find_window_callback(hwnd: HWND, lparam: LPARAM) -> BOOL {
    use windows::Win32::UI::WindowsAndMessaging::{GetWindowTextW, IsWindowVisible};

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
    use windows::Win32::Graphics::Gdi::{GetDC, GetDeviceCaps, HORZRES, ReleaseDC, VERTRES};
    use windows::Win32::UI::WindowsAndMessaging::GetDesktopWindow;

    unsafe {
        let hwnd = GetDesktopWindow();
        let hdc = GetDC(hwnd);
        if hdc.0.is_null() {
            return Err(AutomaticallyError::Capture(
                "Failed to get screen DC".to_string(),
            ));
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
    use windows::Win32::UI::WindowsAndMessaging::{GetForegroundWindow, SW_MINIMIZE, ShowWindow};

    unsafe {
        let hwnd = GetForegroundWindow();
        let _ = ShowWindow(hwnd, SW_MINIMIZE);
    }
    Ok(())
}

/// Linux平台：获取活动窗口 — 优先使用xdotool，回退到X11
#[cfg(target_os = "linux")]
fn get_active_window_linux() -> Result<WindowInfo> {
    use std::process::Command;

    let xdotool_output = Command::new("xdotool")
        .args(["getactivewindow", "getwindowpid", "getwindowgeometry", "--shell"])
        .output();

    if let Ok(out) = xdotool_output {
        if out.status.success() {
            let stdout = String::from_utf8_lossy(&out.stdout);
            let mut window_id: u64 = 0;
            let mut pid: u32 = 0;
            let mut x: i32 = 0;
            let mut y: i32 = 0;
            let mut width: i32 = 0;
            let mut height: i32 = 0;

            for line in stdout.lines() {
                if let Some(val) = line.strip_prefix("WINDOW=") {
                    window_id = val.parse().unwrap_or(0);
                } else if let Some(val) = line.strip_prefix("PID=") {
                    pid = val.parse().unwrap_or(0);
                } else if let Some(val) = line.strip_prefix("X=") {
                    x = val.parse().unwrap_or(0);
                } else if let Some(val) = line.strip_prefix("Y=") {
                    y = val.parse().unwrap_or(0);
                } else if let Some(val) = line.strip_prefix("WIDTH=") {
                    width = val.parse().unwrap_or(0);
                } else if let Some(val) = line.strip_prefix("HEIGHT=") {
                    height = val.parse().unwrap_or(0);
                }
            }

            let title_output = Command::new("xdotool")
                .args(["getactivewindow", "getwindowname"])
                .output();

            let title = if let Ok(tout) = title_output {
                String::from_utf8_lossy(&tout.stdout).trim().to_string()
            } else {
                String::new()
            };

            return Ok(WindowInfo {
                title,
                process_id: pid,
                window_id,
                rect: Some(WindowRect { x, y, width, height }),
            });
        }
    }

    let fallback_output = Command::new("xprop")
        .args(["-root", "_NET_ACTIVE_WINDOW"])
        .output();

    if let Ok(out) = fallback_output {
        let stdout = String::from_utf8_lossy(&out.stdout);
        if let Some(id_start) = stdout.find("0x") {
            let id_str = &stdout[id_start..].split_whitespace().next().unwrap_or("0x0");
            let window_id = u64::from_str_radix(id_str.trim_start_matches("0x"), 16).unwrap_or(0);

            return Ok(WindowInfo {
                title: String::new(),
                process_id: 0,
                window_id,
                rect: None,
            });
        }
    }

    Err(AutomaticallyError::PlatformNotSupported(
        "Linux active window detection requires xdotool or xprop. Install: sudo apt install xdotool".to_string(),
    ))
}

/// Linux平台：列出窗口 — 使用wmctrl枚举所有可见窗口
#[cfg(target_os = "linux")]
fn list_windows_linux() -> Result<Vec<WindowInfo>> {
    use std::process::Command;

    let output = Command::new("wmctrl")
        .args(["-l", "-G", "-x"])
        .output()
        .map_err(|_| {
            AutomaticallyError::PlatformNotSupported(
                "Linux window listing requires wmctrl. Install: sudo apt install wmctrl"
                    .to_string(),
            )
        })?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut windows = Vec::new();

    for line in stdout.lines() {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() >= 8 {
            let window_id = u64::from_str_radix(parts[0].trim_start_matches("0x"), 16).unwrap_or(0);
            let x = parts[2].parse().unwrap_or(0);
            let y = parts[3].parse().unwrap_or(0);
            let width = parts[4].parse().unwrap_or(0);
            let height = parts[5].parse().unwrap_or(0);
            let title = parts[7..].join(" ");

            windows.push(WindowInfo {
                title,
                process_id: 0,
                window_id,
                rect: Some(WindowRect { x, y, width, height }),
            });
        }
    }

    Ok(windows)
}

/// Linux平台：聚焦窗口 — 使用xdotool按标题关键词查找并激活
#[cfg(target_os = "linux")]
fn focus_window_linux(title_contains: &str) -> Result<()> {
    use std::process::Command;

    let search_output = Command::new("xdotool")
        .args([
            "search",
            "--name",
            title_contains,
        ])
        .output()
        .map_err(|_| {
            AutomaticallyError::PlatformNotSupported(
                "Linux window focus requires xdotool. Install: sudo apt install xdotool"
                    .to_string(),
            )
        })?;

    let stdout = String::from_utf8_lossy(&search_output.stdout).trim().to_string();
    if let Some(window_id) = stdout.lines().next() {
        let _ = Command::new("xdotool")
            .args(["windowactivate", window_id])
            .output();
        Ok(())
    } else {
        Err(AutomaticallyError::Automation(format!(
            "No window found containing '{}'",
            title_contains
        )))
    }
}

/// Linux平台：获取屏幕尺寸 — 使用X11 API
#[cfg(target_os = "linux")]
fn get_screen_size_linux() -> Result<(u32, u32)> {
    use x11::xlib;

    unsafe {
        let display = xlib::XOpenDisplay(std::ptr::null());
        if display.is_null() {
            return Err(AutomaticallyError::Capture(
                "Cannot open X display".to_string(),
            ));
        }
        let screen = xlib::XDefaultScreen(display);
        let width = xlib::XDisplayWidth(display, screen) as u32;
        let height = xlib::XDisplayHeight(display, screen) as u32;
        xlib::XCloseDisplay(display);
        Ok((width, height))
    }
}

/// Linux平台：最小化窗口 — 使用xdotool最小化活动窗口
#[cfg(target_os = "linux")]
fn minimize_window_linux() -> Result<()> {
    use std::process::Command;

    Command::new("xdotool")
        .args(["getactivewindow", "windowminimize"])
        .output()
        .map_err(|_| {
            AutomaticallyError::PlatformNotSupported(
                "Linux window minimize requires xdotool. Install: sudo apt install xdotool"
                    .to_string(),
            )
        })?;

    Ok(())
}

/// macOS平台：获取活动窗口 — 使用AppleScript获取前台应用窗口信息
#[cfg(target_os = "macos")]
fn get_active_window_macos() -> Result<WindowInfo> {
    use std::process::Command;

    let script = r#"tell application "System Events"
        set frontApp to first application process whose frontmost is true
        set appName to name of frontApp
        set winTitle to name of front window of frontApp
        set winPos to position of front window of frontApp
        set winSize to size of front window of frontApp
        return appName & "|" & winTitle & "|" & (item 1 of winPos) & "," & (item 2 of winPos) & "|" & (item 1 of winSize) & "," & (item 2 of winSize)
    end tell"#;

    let output = Command::new("osascript").arg("-e").arg(script).output();

    if let Ok(out) = output {
        if out.status.success() {
            let stdout = String::from_utf8_lossy(&out.stdout).trim().to_string();
            let parts: Vec<&str> = stdout.split('|').collect();
            if parts.len() >= 4 {
                let pos_parts: Vec<&str> = parts[2].split(',').collect();
                let size_parts: Vec<&str> = parts[3].split(',').collect();
                if pos_parts.len() == 2 && size_parts.len() == 2 {
                    let x = pos_parts[0].parse().unwrap_or(0);
                    let y = pos_parts[1].parse().unwrap_or(0);
                    let width = size_parts[0].parse().unwrap_or(0);
                    let height = size_parts[1].parse().unwrap_or(0);

                    return Ok(WindowInfo {
                        title: format!("{} - {}", parts[0], parts[1]),
                        process_id: 0,
                        window_id: 0,
                        rect: Some(WindowRect { x, y, width, height }),
                    });
                }
            }
        }
    }

    let fallback_script = r#"tell application "System Events"
        set frontApp to first application process whose frontmost is true
        return name of frontApp
    end tell"#;

    let fallback = Command::new("osascript")
        .arg("-e")
        .arg(fallback_script)
        .output();

    if let Ok(out) = fallback {
        if out.status.success() {
            let title = String::from_utf8_lossy(&out.stdout).trim().to_string();
            return Ok(WindowInfo {
                title,
                process_id: 0,
                window_id: 0,
                rect: None,
            });
        }
    }

    Err(AutomaticallyError::PlatformNotSupported(
        "macOS active window detection requires Accessibility permissions. Grant in System Preferences > Privacy & Security > Accessibility.".to_string(),
    ))
}

/// macOS平台：列出窗口 — 使用AppleScript枚举所有应用窗口
#[cfg(target_os = "macos")]
fn list_windows_macos() -> Result<Vec<WindowInfo>> {
    use std::process::Command;

    let script = r#"tell application "System Events"
        set windowList to {}
        repeat with proc in (application processes where visible is true)
            try
                repeat with w in (windows of proc)
                    set end of windowList to (name of proc & "|" & name of w)
                end repeat
            end try
        end repeat
        return windowList as string
    end tell"#;

    let output = Command::new("osascript").arg("-e").arg(script).output();

    if let Ok(out) = output {
        let stdout = String::from_utf8_lossy(&out.stdout);
        let mut windows = Vec::new();

        for line in stdout.lines() {
            let line = line.trim();
            if let Some(pos) = line.find('|') {
                let app_name = &line[..pos];
                let win_title = &line[pos + 1..];
                windows.push(WindowInfo {
                    title: format!("{} - {}", app_name, win_title),
                    process_id: 0,
                    window_id: 0,
                    rect: None,
                });
            }
        }

        log::info!(
            "[Window:Linux] Found {} visible windows via AppleScript",
            windows.len()
        );
        return Ok(windows);
    }

    Err(AutomaticallyError::PlatformNotSupported(
        "macOS window listing requires Accessibility permissions.".to_string(),
    ))
}

/// macOS平台：聚焦窗口 — 使用AppleScript按标题关键词查找并激活
#[cfg(target_os = "macos")]
fn focus_window_macos(title_contains: &str) -> Result<()> {
    use std::process::Command;

    let escaped = title_contains.replace('\'', "'\\''");
    let script = format!(
        r#"tell application "System Events"
    repeat with proc in (application processes where visible is true)
        try
            repeat with w in (windows of proc)
                if name of w contains "{}" then
                    set frontmost of proc to true
                    perform action "AXRaise" of w
                    return "OK"
                end if
            end repeat
        end try
    end repeat
    return "NOT_FOUND"
end tell"#,
        escaped
    );

    let output = Command::new("osascript").arg("-e").arg(&script).output()
        .map_err(|e| {
            AutomaticallyError::PlatformNotSupported(format!(
                "macOS window focus failed: {}",
                e
            ))
        })?;

    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if stdout == "OK" {
        Ok(())
    } else {
        Err(AutomaticallyError::Automation(format!(
            "No window found containing '{}'",
            title_contains
        )))
    }
}

/// macOS平台：获取屏幕尺寸 — 使用CoreGraphics API
#[cfg(target_os = "macos")]
fn get_screen_size_macos() -> Result<(u32, u32)> {
    use core_graphics::display::{CGDisplay, CGMainDisplayID};

    let display_id = CGMainDisplayID();
    let display = CGDisplay::new(display_id);
    let width = display.pixels_wide();
    let height = display.pixels_high();
    Ok((width, height))
}

/// macOS平台：最小化窗口 — 使用AppleScript最小化前台窗口
#[cfg(target_os = "macos")]
fn minimize_window_macos() -> Result<()> {
    use std::process::Command;

    let script = r#"tell application "System Events"
        set frontApp to first application process whose frontmost is true
        set winTitle to name of front window of frontApp
        perform action "AXMinimize" of front window of frontApp
    end tell"#;

    Command::new("osascript")
        .arg("-e")
        .arg(script)
        .output()
        .map_err(|e| {
            AutomaticallyError::PlatformNotSupported(format!(
                "macOS window minimize requires Accessibility permissions: {}",
                e
            ))
        })?;

    Ok(())
}
