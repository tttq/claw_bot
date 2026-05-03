use crate::error::{AutomaticallyError, Result};
use crate::input::keyboard;

pub async fn copy_to_clipboard(text: &str) -> Result<()> {
    #[cfg(target_os = "windows")]
    {
        copy_to_clipboard_windows(text)
    }

    #[cfg(target_os = "linux")]
    {
        copy_to_clipboard_linux(text)
    }

    #[cfg(target_os = "macos")]
    {
        copy_to_clipboard_macos(text)
    }

    #[cfg(not(any(target_os = "windows", target_os = "linux", target_os = "macos")))]
    {
        Err(AutomaticallyError::PlatformNotSupported(
            "Clipboard not supported on this platform".to_string(),
        ))
    }
}

pub async fn paste_from_clipboard() -> Result<String> {
    #[cfg(target_os = "windows")]
    {
        paste_from_clipboard_windows()
    }

    #[cfg(target_os = "linux")]
    {
        paste_from_clipboard_linux()
    }

    #[cfg(target_os = "macos")]
    {
        paste_from_clipboard_macos()
    }

    #[cfg(not(any(target_os = "windows", target_os = "linux", target_os = "macos")))]
    {
        Err(AutomaticallyError::PlatformNotSupported(
            "Clipboard not supported on this platform".to_string(),
        ))
    }
}

pub async fn paste() -> Result<()> {
    keyboard::press_key("Ctrl+V").await
}

pub async fn copy() -> Result<()> {
    keyboard::press_key("Ctrl+C").await
}

pub async fn cut() -> Result<()> {
    keyboard::press_key("Ctrl+X").await
}

pub async fn select_all() -> Result<()> {
    keyboard::press_key("Ctrl+A").await
}

#[cfg(target_os = "windows")]
fn copy_to_clipboard_windows(text: &str) -> Result<()> {
    use std::process::Command;

    let ps_script = format!(
        r#"Add-Type -AssemblyName System.Windows.Forms
[System.Windows.Forms.Clipboard]::SetText('{}')
Write-Output 'OK'"#,
        text.replace('\'', "''")
    );

    let output = Command::new("powershell")
        .args(["-NoProfile", "-NonInteractive", "-Command", &ps_script])
        .output()
        .map_err(|e| AutomaticallyError::Automation(format!("Clipboard set failed: {}", e)))?;

    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if stdout == "OK" {
        Ok(())
    } else {
        Err(AutomaticallyError::Automation("Failed to set clipboard".to_string()))
    }
}

#[cfg(target_os = "windows")]
fn paste_from_clipboard_windows() -> Result<String> {
    use std::process::Command;

    let ps_script = r#"
Add-Type -AssemblyName System.Windows.Forms
try {
    $text = [System.Windows.Forms.Clipboard]::GetText()
    if ($text) {
        Write-Output $text
    } else {
        Write-Output 'EMPTY'
    }
} catch {
    Write-Output 'ERROR'
}
"#;

    let output = Command::new("powershell")
        .args(["-NoProfile", "-NonInteractive", "-Command", ps_script])
        .output()
        .map_err(|e| {
            AutomaticallyError::Automation(format!("Clipboard get failed: {}", e))
        })?;

    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if stdout == "ERROR" {
        Err(AutomaticallyError::Automation("Failed to read clipboard".to_string()))
    } else if stdout == "EMPTY" {
        Ok(String::new())
    } else {
        Ok(stdout)
    }
}

#[cfg(target_os = "linux")]
fn copy_to_clipboard_linux(text: &str) -> Result<()> {
    use std::process::Command;
    use std::io::Write;

    let mut child = Command::new("xclip")
        .args(["-selection", "clipboard"])
        .stdin(std::process::Stdio::piped())
        .spawn()
        .map_err(|e| {
            AutomaticallyError::Automation(format!("xclip not available: {}", e))
        })?;

    if let Some(mut stdin) = child.stdin.take() {
        let _ = stdin.write_all(text.as_bytes());
    }

    child.wait().map_err(|e| {
        AutomaticallyError::Automation(format!("xclip failed: {}", e))
    })?;

    Ok(())
}

#[cfg(target_os = "linux")]
fn paste_from_clipboard_linux() -> Result<String> {
    use std::process::Command;

    let output = Command::new("xclip")
        .args(["-selection", "clipboard", "-o"])
        .output()
        .map_err(|e| {
            AutomaticallyError::Automation(format!("xclip not available: {}", e))
        })?;

    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

#[cfg(target_os = "macos")]
fn copy_to_clipboard_macos(text: &str) -> Result<()> {
    use std::process::Command;
    use std::io::Write;

    let mut child = Command::new("pbcopy")
        .stdin(std::process::Stdio::piped())
        .spawn()
        .map_err(|e| {
            AutomaticallyError::Automation(format!("pbcopy not available: {}", e))
        })?;

    if let Some(mut stdin) = child.stdin.take() {
        let _ = stdin.write_all(text.as_bytes());
    }

    child.wait().map_err(|e| {
        AutomaticallyError::Automation(format!("pbcopy failed: {}", e))
    })?;

    Ok(())
}

#[cfg(target_os = "macos")]
fn paste_from_clipboard_macos() -> Result<String> {
    use std::process::Command;

    let output = Command::new("pbpaste")
        .output()
        .map_err(|e| {
            AutomaticallyError::Automation(format!("pbpaste not available: {}", e))
        })?;

    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}
