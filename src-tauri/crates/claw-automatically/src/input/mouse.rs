// Claw Desktop - 鼠标输入模块
// 提供鼠标移动、点击、双击、右键、滚动、拖拽等操作（跨平台实现）
use crate::error::{AutomaticallyError, Result};

/// 移动鼠标到指定位置
pub async fn move_to(x: f64, y: f64) -> Result<()> {
    #[cfg(target_os = "windows")]
    {
        move_to_windows(x, y)
    }

    #[cfg(target_os = "linux")]
    {
        move_to_linux(x, y)
    }

    #[cfg(target_os = "macos")]
    {
        move_to_macos(x, y)
    }

    #[cfg(not(any(target_os = "windows", target_os = "linux", target_os = "macos")))]
    {
        Err(AutomaticallyError::PlatformNotSupported(
            "Mouse control not supported on this platform".to_string(),
        ))
    }
}

/// 获取当前鼠标位置
pub async fn get_position() -> Result<(f64, f64)> {
    #[cfg(target_os = "windows")]
    {
        get_position_windows()
    }

    #[cfg(target_os = "linux")]
    {
        get_position_linux()
    }

    #[cfg(target_os = "macos")]
    {
        get_position_macos()
    }

    #[cfg(not(any(target_os = "windows", target_os = "linux", target_os = "macos")))]
    {
        Err(AutomaticallyError::PlatformNotSupported(
            "Mouse position query not supported on this platform".to_string(),
        ))
    }
}

/// 点击鼠标左键
pub async fn click(x: f64, y: f64) -> Result<()> {
    move_to(x, y).await?;
    mouse_down("left").await?;
    tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
    mouse_up("left").await?;
    Ok(())
}

/// 双击鼠标左键
pub async fn double_click(x: f64, y: f64) -> Result<()> {
    move_to(x, y).await?;
    click(x, y).await?;
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
    click(x, y).await
}

/// 点击鼠标右键
pub async fn right_click(x: f64, y: f64) -> Result<()> {
    move_to(x, y).await?;
    mouse_down("right").await?;
    tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
    mouse_up("right").await?;
    Ok(())
}

/// 按下鼠标按钮
pub async fn mouse_down(button: &str) -> Result<()> {
    #[cfg(target_os = "windows")]
    {
        mouse_down_windows(button)
    }

    #[cfg(target_os = "linux")]
    {
        mouse_down_linux(button)
    }

    #[cfg(target_os = "macos")]
    {
        mouse_down_macos(button)
    }

    #[cfg(not(any(target_os = "windows", target_os = "linux", target_os = "macos")))]
    {
        Err(AutomaticallyError::PlatformNotSupported(
            "Mouse down not supported on this platform".to_string(),
        ))
    }
}

/// 释放鼠标按钮
pub async fn mouse_up(button: &str) -> Result<()> {
    #[cfg(target_os = "windows")]
    {
        mouse_up_windows(button)
    }

    #[cfg(target_os = "linux")]
    {
        mouse_up_linux(button)
    }

    #[cfg(target_os = "macos")]
    {
        mouse_up_macos(button)
    }

    #[cfg(not(any(target_os = "windows", target_os = "linux", target_os = "macos")))]
    {
        Err(AutomaticallyError::PlatformNotSupported(
            "Mouse up not supported on this platform".to_string(),
        ))
    }
}

/// 在指定位置滚动鼠标滚轮
pub async fn scroll_at(x: f64, y: f64, amount: i32) -> Result<()> {
    move_to(x, y).await?;
    scroll(amount).await
}

/// 滚动鼠标滚轮
pub async fn scroll(amount: i32) -> Result<()> {
    #[cfg(target_os = "windows")]
    {
        scroll_windows(amount)
    }

    #[cfg(target_os = "linux")]
    {
        scroll_linux(amount)
    }

    #[cfg(target_os = "macos")]
    {
        scroll_macos(amount)
    }

    #[cfg(not(any(target_os = "windows", target_os = "linux", target_os = "macos")))]
    {
        Err(AutomaticallyError::PlatformNotSupported(
            "Mouse scroll not supported on this platform".to_string(),
        ))
    }
}

/// 拖拽操作
pub async fn drag(from_x: f64, from_y: f64, to_x: f64, to_y: f64) -> Result<()> {
    move_to(from_x, from_y).await?;
    mouse_down("left").await?;
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
    move_to(to_x, to_y).await?;
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
    mouse_up("left").await
}

/// Windows平台：获取屏幕尺寸
#[cfg(target_os = "windows")]
fn get_screen_size_windows() -> (i32, i32) {
    use windows::Win32::Graphics::Gdi::{GetDC, GetDeviceCaps, HORZRES, ReleaseDC, VERTRES};
    use windows::Win32::UI::WindowsAndMessaging::GetDesktopWindow;

    unsafe {
        let hwnd = GetDesktopWindow();
        let hdc = GetDC(hwnd);
        if hdc.0.is_null() {
            return (1920, 1080);
        }
        let width = GetDeviceCaps(hdc, HORZRES);
        let height = GetDeviceCaps(hdc, VERTRES);
        ReleaseDC(hwnd, hdc);
        if width <= 0 || height <= 0 {
            (1920, 1080)
        } else {
            (width, height)
        }
    }
}

/// Windows平台：移动鼠标 — 使用绝对坐标映射
#[cfg(target_os = "windows")]
fn move_to_windows(x: f64, y: f64) -> Result<()> {
    use windows::Win32::UI::Input::KeyboardAndMouse::{
        MOUSEEVENTF_ABSOLUTE, MOUSEEVENTF_MOVE, mouse_event,
    };

    let (screen_w, screen_h) = get_screen_size_windows();
    let abs_x = (x * 65535.0 / screen_w as f64) as i32;
    let abs_y = (y * 65535.0 / screen_h as f64) as i32;

    unsafe {
        mouse_event(MOUSEEVENTF_ABSOLUTE | MOUSEEVENTF_MOVE, abs_x, abs_y, 0, 0);
    }

    Ok(())
}

/// Windows平台：获取鼠标位置
#[cfg(target_os = "windows")]
fn get_position_windows() -> Result<(f64, f64)> {
    use windows::Win32::Foundation::POINT;
    use windows::Win32::UI::WindowsAndMessaging::GetCursorPos;

    unsafe {
        let mut point = POINT { x: 0, y: 0 };
        GetCursorPos(&mut point).map_err(|e| {
            AutomaticallyError::Input(format!("Failed to get cursor position: {}", e))
        })?;
        Ok((point.x as f64, point.y as f64))
    }
}

/// Windows平台：按下鼠标按钮
#[cfg(target_os = "windows")]
fn mouse_down_windows(button: &str) -> Result<()> {
    use windows::Win32::UI::Input::KeyboardAndMouse::{
        MOUSEEVENTF_LEFTDOWN, MOUSEEVENTF_MIDDLEDOWN, MOUSEEVENTF_RIGHTDOWN, mouse_event,
    };

    let flags = match button {
        "left" => MOUSEEVENTF_LEFTDOWN,
        "right" => MOUSEEVENTF_RIGHTDOWN,
        "middle" => MOUSEEVENTF_MIDDLEDOWN,
        _ => {
            return Err(AutomaticallyError::Input(format!(
                "Unknown mouse button: {}",
                button
            )));
        }
    };

    unsafe {
        mouse_event(flags, 0, 0, 0, 0);
    }

    Ok(())
}

/// Windows平台：释放鼠标按钮
#[cfg(target_os = "windows")]
fn mouse_up_windows(button: &str) -> Result<()> {
    use windows::Win32::UI::Input::KeyboardAndMouse::{
        MOUSEEVENTF_LEFTUP, MOUSEEVENTF_MIDDLEUP, MOUSEEVENTF_RIGHTUP, mouse_event,
    };

    let flags = match button {
        "left" => MOUSEEVENTF_LEFTUP,
        "right" => MOUSEEVENTF_RIGHTUP,
        "middle" => MOUSEEVENTF_MIDDLEUP,
        _ => {
            return Err(AutomaticallyError::Input(format!(
                "Unknown mouse button: {}",
                button
            )));
        }
    };

    unsafe {
        mouse_event(flags, 0, 0, 0, 0);
    }

    Ok(())
}

/// Windows平台：滚动鼠标滚轮
#[cfg(target_os = "windows")]
fn scroll_windows(amount: i32) -> Result<()> {
    use windows::Win32::UI::Input::KeyboardAndMouse::{MOUSEEVENTF_WHEEL, mouse_event};

    // WHEEL_DELTA = 120
    let wheel_delta = amount * 120;

    unsafe {
        mouse_event(MOUSEEVENTF_WHEEL, 0, 0, wheel_delta, 0);
    }

    Ok(())
}

/// Linux平台：移动鼠标 — 使用X11 XTest扩展
#[cfg(target_os = "linux")]
fn move_to_linux(x: f64, y: f64) -> Result<()> {
    use x11::xlib;
    use x11::xtest;

    unsafe {
        let display = xlib::XOpenDisplay(std::ptr::null());
        if display.is_null() {
            return Err(AutomaticallyError::Input(
                "Cannot open X display".to_string(),
            ));
        }

        let screen = xlib::XDefaultScreen(display);
        xtest::XTestFakeMotionEvent(display, screen, x as i32, y as i32, 0);
        xlib::XFlush(display);
        xlib::XCloseDisplay(display);
    }

    Ok(())
}

/// Linux平台：获取鼠标位置 — 使用X11 XQueryPointer
#[cfg(target_os = "linux")]
fn get_position_linux() -> Result<(f64, f64)> {
    use x11::xlib;

    unsafe {
        let display = xlib::XOpenDisplay(std::ptr::null());
        if display.is_null() {
            return Err(AutomaticallyError::Input(
                "Cannot open X display".to_string(),
            ));
        }

        let mut root_x = 0;
        let mut root_y = 0;
        let mut win_x = 0;
        let mut win_y = 0;
        let mut mask = 0;
        let mut root_return = 0;
        let mut child_return = 0;

        let root = xlib::XRootWindow(display, xlib::XDefaultScreen(display));

        xlib::XQueryPointer(
            display,
            root,
            &mut root_return,
            &mut child_return,
            &mut root_x,
            &mut root_y,
            &mut win_x,
            &mut win_y,
            &mut mask,
        );

        xlib::XCloseDisplay(display);

        Ok((root_x as f64, root_y as f64))
    }
}

/// Linux平台：按下鼠标按钮 — 使用X11 XTest扩展
#[cfg(target_os = "linux")]
fn mouse_down_linux(button: &str) -> Result<()> {
    use x11::xlib;
    use x11::xtest;

    let button_num = match button {
        "left" => 1,
        "middle" => 2,
        "right" => 3,
        _ => {
            return Err(AutomaticallyError::Input(format!(
                "Unknown mouse button: {}",
                button
            )));
        }
    };

    unsafe {
        let display = xlib::XOpenDisplay(std::ptr::null());
        if display.is_null() {
            return Err(AutomaticallyError::Input(
                "Cannot open X display".to_string(),
            ));
        }

        xtest::XTestFakeButtonEvent(display, button_num, 1, 0);
        xlib::XFlush(display);
        xlib::XCloseDisplay(display);
    }

    Ok(())
}

/// Linux平台：释放鼠标按钮 — 使用X11 XTest扩展
#[cfg(target_os = "linux")]
fn mouse_up_linux(button: &str) -> Result<()> {
    use x11::xlib;
    use x11::xtest;

    let button_num = match button {
        "left" => 1,
        "middle" => 2,
        "right" => 3,
        _ => {
            return Err(AutomaticallyError::Input(format!(
                "Unknown mouse button: {}",
                button
            )));
        }
    };

    unsafe {
        let display = xlib::XOpenDisplay(std::ptr::null());
        if display.is_null() {
            return Err(AutomaticallyError::Input(
                "Cannot open X display".to_string(),
            ));
        }

        xtest::XTestFakeButtonEvent(display, button_num, 0, 0);
        xlib::XFlush(display);
        xlib::XCloseDisplay(display);
    }

    Ok(())
}

/// Linux平台：滚动鼠标滚轮 — 使用X11 XTest按钮4/5模拟
#[cfg(target_os = "linux")]
fn scroll_linux(amount: i32) -> Result<()> {
    use x11::xlib;
    use x11::xtest;

    unsafe {
        let display = xlib::XOpenDisplay(std::ptr::null());
        if display.is_null() {
            return Err(AutomaticallyError::Input(
                "Cannot open X display".to_string(),
            ));
        }

        // 按钮 4 和 5 分别对应滚轮向上和向下
        let button = if amount > 0 { 4 } else { 5 };
        let clicks = amount.abs();

        for _ in 0..clicks {
            xtest::XTestFakeButtonEvent(display, button, 1, 0);
            xtest::XTestFakeButtonEvent(display, button, 0, 0);
        }

        xlib::XFlush(display);
        xlib::XCloseDisplay(display);
    }

    Ok(())
}

/// macOS平台：移动鼠标 — 使用CoreGraphics CGEvent API
#[cfg(target_os = "macos")]
fn move_to_macos(x: f64, y: f64) -> Result<()> {
    use core_graphics::display::CGDisplay;
    use core_graphics::event::{CGEvent, CGEventType, CGMouseButton};
    use core_graphics::event_source::{CGEventSource, CGEventSourceStateID};

    let source = CGEventSource::new(CGEventSourceStateID::HIDSystemState)
        .map_err(|_| AutomaticallyError::Input("Failed to create event source".to_string()))?;

    let event = CGEvent::new_mouse_event(
        source,
        CGEventType::MouseMoved,
        core_graphics::geometry::CGPoint::new(x, y),
        CGMouseButton::Left,
    )
    .map_err(|_| AutomaticallyError::Input("Failed to create mouse event".to_string()))?;

    event.post(core_graphics::event::CGEventTapLocation::HID);

    Ok(())
}

/// macOS平台：获取鼠标位置 — 使用CoreGraphics CGEvent
#[cfg(target_os = "macos")]
fn get_position_macos() -> Result<(f64, f64)> {
    use core_graphics::event::CGEvent;
    use core_graphics::event_source::{CGEventSource, CGEventSourceStateID};

    let source = CGEventSource::new(CGEventSourceStateID::HIDSystemState)
        .map_err(|_| AutomaticallyError::Input("Failed to create event source".to_string()))?;

    let event = CGEvent::new(source)
        .map_err(|_| AutomaticallyError::Input("Failed to create event".to_string()))?;

    let location = event.location();
    Ok((location.x, location.y))
}

/// macOS平台：按下鼠标按钮 — 使用CoreGraphics CGEvent API
#[cfg(target_os = "macos")]
fn mouse_down_macos(button: &str) -> Result<()> {
    use core_graphics::event::{CGEvent, CGEventType, CGMouseButton};
    use core_graphics::event_source::{CGEventSource, CGEventSourceStateID};

    let source = CGEventSource::new(CGEventSourceStateID::HIDSystemState)
        .map_err(|_| AutomaticallyError::Input("Failed to create event source".to_string()))?;

    let button_type = match button {
        "left" => CGMouseButton::Left,
        "right" => CGMouseButton::Right,
        "middle" => CGMouseButton::Center,
        _ => {
            return Err(AutomaticallyError::Input(format!(
                "Unknown mouse button: {}",
                button
            )));
        }
    };

    let event_type = match button {
        "left" => CGEventType::LeftMouseDown,
        "right" => CGEventType::RightMouseDown,
        _ => CGEventType::OtherMouseDown,
    };

    let event = CGEvent::new_mouse_event(
        source,
        event_type,
        core_graphics::geometry::CGPoint::new(0.0, 0.0),
        button_type,
    )
    .map_err(|_| AutomaticallyError::Input("Failed to create mouse event".to_string()))?;

    event.post(core_graphics::event::CGEventTapLocation::HID);

    Ok(())
}

/// macOS平台：释放鼠标按钮 — 使用CoreGraphics CGEvent API
#[cfg(target_os = "macos")]
fn mouse_up_macos(button: &str) -> Result<()> {
    use core_graphics::event::{CGEvent, CGEventType, CGMouseButton};
    use core_graphics::event_source::{CGEventSource, CGEventSourceStateID};

    let source = CGEventSource::new(CGEventSourceStateID::HIDSystemState)
        .map_err(|_| AutomaticallyError::Input("Failed to create event source".to_string()))?;

    let button_type = match button {
        "left" => CGMouseButton::Left,
        "right" => CGMouseButton::Right,
        "middle" => CGMouseButton::Center,
        _ => {
            return Err(AutomaticallyError::Input(format!(
                "Unknown mouse button: {}",
                button
            )));
        }
    };

    let event_type = match button {
        "left" => CGEventType::LeftMouseUp,
        "right" => CGEventType::RightMouseUp,
        _ => CGEventType::OtherMouseUp,
    };

    let event = CGEvent::new_mouse_event(
        source,
        event_type,
        core_graphics::geometry::CGPoint::new(0.0, 0.0),
        button_type,
    )
    .map_err(|_| AutomaticallyError::Input("Failed to create mouse event".to_string()))?;

    event.post(core_graphics::event::CGEventTapLocation::HID);

    Ok(())
}

/// macOS平台：滚动鼠标滚轮 — 使用CoreGraphics CGScrollEvent
#[cfg(target_os = "macos")]
fn scroll_macos(amount: i32) -> Result<()> {
    use core_graphics::event::{CGEvent, CGScrollEventUnit};
    use core_graphics::event_source::{CGEventSource, CGEventSourceStateID};

    let source = CGEventSource::new(CGEventSourceStateID::HIDSystemState)
        .map_err(|_| AutomaticallyError::Input("Failed to create event source".to_string()))?;

    let event = CGEvent::new_scroll_event(source, CGScrollEventUnit::Pixel, 1, amount, 0, 0)
        .map_err(|_| AutomaticallyError::Input("Failed to create scroll event".to_string()))?;

    event.post(core_graphics::event::CGEventTapLocation::HID);

    Ok(())
}
