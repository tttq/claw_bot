// Claw Desktop - 屏幕捕获模块
// 提供跨平台屏幕截图和OCR文字识别功能（Windows/macOS/Linux）
use crate::error::{AutomaticallyError, Result};
use crate::types::ImageFrame;

/// 截取全屏 — 根据平台自动选择实现
pub fn capture_screen() -> Result<ImageFrame> {
    #[cfg(target_os = "windows")]
    {
        capture_screen_windows()
    }

    #[cfg(target_os = "linux")]
    {
        capture_screen_linux()
    }

    #[cfg(target_os = "macos")]
    {
        capture_screen_macos()
    }

    #[cfg(not(any(target_os = "windows", target_os = "linux", target_os = "macos")))]
    {
        Err(AutomaticallyError::PlatformNotSupported(
            "Screen capture not supported on this platform".to_string()
        ))
    }
}

/// 截取屏幕指定区域
pub fn capture_region(x: u32, y: u32, width: u32, height: u32) -> Result<ImageFrame> {
    let full_frame = capture_screen()?;
    crop_frame(&full_frame, x, y, width, height)
}

/// 裁剪图像帧 — 从完整帧中提取指定区域
fn crop_frame(frame: &ImageFrame, x: u32, y: u32, width: u32, height: u32) -> Result<ImageFrame> {
    if x + width > frame.width || y + height > frame.height {
        return Err(AutomaticallyError::InvalidCoordinates(
            x as f64, y as f64, frame.width, frame.height,
        ));
    }

    let mut cropped_data = Vec::with_capacity((width * height * 3) as usize);
    for row in y..y + height {
        let start = ((row * frame.width + x) * 3) as usize;
        let end = start + (width * 3) as usize;
        cropped_data.extend_from_slice(&frame.data[start..end]);
    }

    Ok(ImageFrame::new(width, height, cropped_data))
}

/// OCR屏幕文字识别 — 根据平台自动选择实现
pub fn ocr_screen_text() -> Result<String> {
    #[cfg(target_os = "windows")]
    {
        ocr_screen_text_windows()
    }

    #[cfg(target_os = "linux")]
    {
        ocr_screen_text_linux()
    }

    #[cfg(target_os = "macos")]
    {
        ocr_screen_text_macos()
    }

    #[cfg(not(any(target_os = "windows", target_os = "linux", target_os = "macos")))]
    {
        Err(AutomaticallyError::PlatformNotSupported(
            "OCR not supported on this platform".to_string()
        ))
    }
}

/// Windows平台OCR — 使用PowerShell调用Windows.Media.Ocr引擎
#[cfg(target_os = "windows")]
fn ocr_screen_text_windows() -> Result<String> {
    let frame = capture_screen_windows()?;
    let png_data = frame.to_png().map_err(|e| AutomaticallyError::Capture(format!("PNG encode failed: {}", e)))?;

    let temp_dir = std::env::temp_dir();
    let png_path = temp_dir.join(format!("claw_ocr_{}.png", uuid::Uuid::new_v4()));
    std::fs::write(&png_path, &png_data)
        .map_err(|e| AutomaticallyError::Automation(format!("Failed to write temp PNG: {}", e)))?;

    let png_path_str = png_path.to_string_lossy().replace('\'', "''");

    let ps_script = format!(
        r#"
try {{
    Add-Type -AssemblyName System.Runtime.WindowsRuntime
    [Windows.Storage.StorageFile,Windows.Storage,ContentType=Windows] | Out-Null
    [Windows.Media.Ocr.OcrEngine,Windows.Media.Ocr,ContentType=Windows] | Out-Null

    $file = [Windows.Storage.StorageFile]::GetFileFromPathAsync('{}').GetAwaiter().GetResult()
    $stream = $file.OpenAsync([Windows.Storage.FileAccessMode]::Read).GetAwaiter().GetResult()
    $decoder = [Windows.Graphics.Imaging.BitmapDecoder]::CreateAsync($stream).GetAwaiter().GetResult()
    $bmp = $decoder.GetSoftwareBitmapAsync().GetAwaiter().GetResult()
    $stream.Dispose()

    $engine = [Windows.Media.Ocr.OcrEngine]::TryCreateFromUserProfileLanguages()
    if ($null -eq $engine) {{
        $engine = [Windows.Media.Ocr.OcrEngine]::TryCreateFromLanguage([Windows.Globalization.Language]::new('en'))
    }}
    if ($null -eq $engine) {{
        Write-Output 'ERROR: OCR engine not available'
        exit 1
    }}
    $result = $engine.RecognizeAsync($bmp).GetAwaiter().GetResult()
    Write-Output $result.Text
}} catch {{
    Write-Output "ERROR: $($_.Exception.Message)"
}} finally {{
    Remove-Item -Path '{}' -Force -ErrorAction SilentlyContinue
}}
"#,
        png_path_str, png_path_str
    );

    let output = std::process::Command::new("powershell")
        .args(["-NoProfile", "-NonInteractive", "-Command", &ps_script])
        .output()
        .map_err(|e| AutomaticallyError::Automation(format!("PowerShell OCR failed: {}", e)))?;

    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if stdout.starts_with("ERROR:") {
        return Err(AutomaticallyError::Automation(stdout));
    }
    if stdout.is_empty() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        if !stderr.is_empty() {
            log::warn!("[OCR:Windows] PowerShell stderr: {}", stderr);
        }
        return Ok("[Screen captured but no text detected]".to_string());
    }
    Ok(stdout)
}

/// Linux平台OCR — 使用tesseract命令行工具
#[cfg(target_os = "linux")]
fn ocr_screen_text_linux() -> Result<String> {
    let which = std::process::Command::new("which")
        .arg("tesseract")
        .output();
    match which {
        Ok(o) if o.status.success() => {
            let frame = capture_screen_linux()?;
            let png_data = frame.to_png().map_err(|e| AutomaticallyError::Capture(format!("PNG encode failed: {}", e)))?;
            let mut child = std::process::Command::new("tesseract")
                .args(["stdin", "stdout", "-l", "eng"])
                .stdin(std::process::Stdio::piped())
                .stdout(std::process::Stdio::piped())
                .stderr(std::process::Stdio::null())
                .spawn()
                .map_err(|e| AutomaticallyError::Automation(format!("tesseract spawn failed: {}", e)))?;
            if let Some(mut stdin) = child.stdin.take() {
                use std::io::Write;
                let _ = stdin.write_all(&png_data);
            }
            let output = child.wait_with_output()
                .map_err(|e| AutomaticallyError::Automation(format!("tesseract wait failed: {}", e)))?;
            let text = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if text.is_empty() {
                Ok("[Screen captured but no text detected]".to_string())
            } else {
                Ok(text)
            }
        }
        _ => Ok("[OCR not available: tesseract not installed. Install with: sudo apt install tesseract-ocr]".to_string())
    }
}

/// macOS平台OCR — 提示使用CUA Agent进行视觉理解
#[cfg(target_os = "macos")]
fn ocr_screen_text_macos() -> Result<String> {
    Ok("[OCR on macOS: use ExecuteAutomation for visual understanding]".to_string())
}

/// Windows平台屏幕截图 — 使用GDI API截取桌面
#[cfg(target_os = "windows")]
fn capture_screen_windows() -> Result<ImageFrame> {
    use windows::Win32::Graphics::Gdi::{
        BitBlt, CreateCompatibleDC, CreateDIBSection,
        DeleteDC, DeleteObject, GetDC, ReleaseDC, SelectObject,
        BITMAPINFO, BITMAPINFOHEADER, BI_RGB, DIB_RGB_COLORS, SRCCOPY,
        GetDeviceCaps, HORZRES, VERTRES,
    };
    use windows::Win32::UI::WindowsAndMessaging::GetDesktopWindow;

    unsafe {
        let hwnd = GetDesktopWindow();
        let hdc_screen = GetDC(hwnd);
        if hdc_screen.0.is_null() {
            return Err(AutomaticallyError::Capture("Failed to get screen DC".to_string()));
        }

        let width = GetDeviceCaps(hdc_screen, HORZRES) as i32;
        let height = GetDeviceCaps(hdc_screen, VERTRES) as i32;

        let hdc_mem = CreateCompatibleDC(hdc_screen);
        if hdc_mem.0.is_null() {
            ReleaseDC(hwnd, hdc_screen);
            return Err(AutomaticallyError::Capture("Failed to create compatible DC".to_string()));
        }

        let bmi = BITMAPINFO {
            bmiHeader: BITMAPINFOHEADER {
                biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                biWidth: width,
                biHeight: -height,
                biPlanes: 1,
                biBitCount: 24,
                biCompression: BI_RGB.0,
                ..Default::default()
            },
            ..Default::default()
        };

        let mut ppv_bits: *mut u8 = std::ptr::null_mut();
        let h_bitmap = CreateDIBSection(
            hdc_mem,
            &bmi,
            DIB_RGB_COLORS,
            &mut ppv_bits as *mut _ as *mut _,
            None,
            0,
        ).map_err(|e| AutomaticallyError::Capture(format!("CreateDIBSection failed: {}", e)))?;

        if h_bitmap.is_invalid() {
            let _ = DeleteDC(hdc_mem);
            ReleaseDC(hwnd, hdc_screen);
            return Err(AutomaticallyError::Capture("Failed to create DIB section".to_string()));
        }

        SelectObject(hdc_mem, h_bitmap);
        let _ = BitBlt(hdc_mem, 0, 0, width, height, hdc_screen, 0, 0, SRCCOPY);

        let row_size = ((width * 3 + 3) & !3) as usize;
        let data_size = row_size * height as usize;
        let mut rgb_data = Vec::with_capacity(width as usize * height as usize * 3);

        if !ppv_bits.is_null() {
            let src = std::slice::from_raw_parts(ppv_bits, data_size);
            for row in 0..height as usize {
                let row_start = row * row_size;
                for col in 0..width as usize {
                    let pixel_start = row_start + col * 3;
                    if pixel_start + 2 < src.len() {
                        rgb_data.push(src[pixel_start + 2]);
                        rgb_data.push(src[pixel_start + 1]);
                        rgb_data.push(src[pixel_start]);
                    }
                }
            }
        }

        let _ = DeleteObject(h_bitmap);
        let _ = DeleteDC(hdc_mem);
        ReleaseDC(hwnd, hdc_screen);

        if rgb_data.is_empty() {
            return Err(AutomaticallyError::Capture("Captured empty screen data".to_string()));
        }

        Ok(ImageFrame::new(width as u32, height as u32, rgb_data))
    }
}

/// Linux平台屏幕截图 — 使用X11 API截取桌面
#[cfg(target_os = "linux")]
fn capture_screen_linux() -> Result<ImageFrame> {
    use x11::xlib;

    unsafe {
        let display = x11::xlib::XOpenDisplay(std::ptr::null());
        if display.is_null() {
            return Err(AutomaticallyError::Capture("Cannot open X display".to_string()));
        }

        let screen = x11::xlib::XDefaultScreen(display);
        let root = x11::xlib::XRootWindow(display, screen);
        let width = x11::xlib::XDisplayWidth(display, screen) as u32;
        let height = x11::xlib::XDisplayHeight(display, screen) as u32;

        let ximage = x11::xlib::XGetImage(
            display, root,
            0, 0, width, height,
            x11::xlib::XAllPlanes(),
            x11::xlib::ZPixmap,
        );

        if ximage.is_null() {
            x11::xlib::XCloseDisplay(display);
            return Err(AutomaticallyError::Capture("XGetImage failed".to_string()));
        }

        let image = &*ximage;
        let data = std::slice::from_raw_parts(
            image.data as *const u8,
            (width * height * 4) as usize,
        );

        let mut rgb_data = Vec::with_capacity((width * height * 3) as usize);
        for i in 0..(width * height) as usize {
            let offset = i * 4;
            rgb_data.push(data[offset + 2]);
            rgb_data.push(data[offset + 1]);
            rgb_data.push(data[offset]);
        }

        x11::xlib::XDestroyImage(ximage);
        x11::xlib::XCloseDisplay(display);

        Ok(ImageFrame::new(width, height, rgb_data))
    }
}

/// macOS平台屏幕截图 — 使用CoreGraphics API截取桌面
#[cfg(target_os = "macos")]
fn capture_screen_macos() -> Result<ImageFrame> {
    use core_graphics::display::{CGDisplay, CGMainDisplayID};

    let display_id = CGMainDisplayID();
    let display = CGDisplay::new(display_id);

    let image = display.image()
        .ok_or_else(|| AutomaticallyError::Capture("Failed to capture screen on macOS".to_string()))?;

    let width = image.width() as u32;
    let height = image.height() as u32;

    let data_provider = image.data_provider()
        .ok_or_else(|| AutomaticallyError::Capture("No data provider".to_string()))?;
    let data = data_provider.data();
    let bytes = data.bytes();

    let mut rgb_data = Vec::with_capacity((width * height * 3) as usize);
    for i in 0..(width * height) as usize {
        let offset = i * 4;
        if offset + 2 < bytes.len() {
            rgb_data.push(bytes[offset + 2]);
            rgb_data.push(bytes[offset + 1]);
            rgb_data.push(bytes[offset]);
        }
    }

    Ok(ImageFrame::new(width, height, rgb_data))
}
