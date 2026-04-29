// Claw Desktop - 应用启动器
// 提供应用列表扫描（开始菜单/注册表/AppPaths）和应用启动功能
use crate::error::{AutomaticallyError, Result};
use crate::types::AppInfo;

/// 启动应用程序 — 根据平台自动选择实现
pub fn launch_application(name: &str) -> Result<()> {
    log::info!("[AppLauncher:launch_application] Launching: {}", name);

    #[cfg(target_os = "windows")]
    {
        launch_application_windows(name)
    }

    #[cfg(target_os = "linux")]
    {
        launch_application_linux(name)
    }

    #[cfg(target_os = "macos")]
    {
        launch_application_macos(name)
    }

    #[cfg(not(any(target_os = "windows", target_os = "linux", target_os = "macos")))]
    {
        Err(AutomaticallyError::PlatformNotSupported(
            "Application launching not supported on this platform".to_string()
        ))
    }
}

/// 列出已安装应用 — 支持按名称过滤
pub fn list_installed_apps(filter: Option<&str>) -> Result<Vec<AppInfo>> {
    log::info!("[AppLauncher:list_installed_apps] filter={:?}", filter);

    #[cfg(target_os = "windows")]
    {
        list_installed_apps_windows(filter)
    }

    #[cfg(target_os = "linux")]
    {
        list_installed_apps_linux(filter)
    }

    #[cfg(target_os = "macos")]
    {
        list_installed_apps_macos(filter)
    }

    #[cfg(not(any(target_os = "windows", target_os = "linux", target_os = "macos")))]
    {
        Err(AutomaticallyError::PlatformNotSupported(
            "Application listing not supported on this platform".to_string()
        ))
    }
}

/// Windows平台：启动应用 — 按扩展名、AppID、可执行路径和开始菜单回退策略
#[cfg(target_os = "windows")]
fn launch_application_windows(name: &str) -> Result<()> {
    use std::process::Command;

    let lower = name.to_lowercase();

    if lower.ends_with(".exe") || lower.ends_with(".msi") || lower.ends_with(".bat") || lower.ends_with(".lnk") {
        let target_path = name;
        if !std::path::Path::new(target_path).exists() {
            return Err(AutomaticallyError::Automation(format!(
                "File not found: '{}'", target_path
            )));
        }
        return launch_via_powershell(target_path);
    }

    if lower.contains("://") || lower.starts_with("http") {
        return launch_via_powershell(name);
    }

    let apps = list_installed_apps_windows(Some(name)).unwrap_or_default();
    if let Some(app) = apps.first() {
        if let Some(ref app_id) = app.launch_command {
            if !app_id.is_empty() {
                log::info!("[AppLauncher:launch_application] Found '{}' with AppID: {}", app.name, app_id);
                if app_id.contains('\\') || app_id.contains('/') || app_id.ends_with(".lnk") {
                    if std::path::Path::new(app_id).exists() {
                        match launch_via_powershell(app_id) {
                            Ok(()) => return Ok(()),
                            Err(e) => {
                                log::warn!("[AppLauncher:launch_application] Path launch failed: {}, trying other methods", e);
                            }
                        }
                    } else {
                        log::warn!("[AppLauncher:launch_application] Path does not exist: {}", app_id);
                    }
                }

                let ps_script = r#"
$appId = '{0}'
$launched = $false

try {
    Add-Type -AssemblyName System.Runtime.WindowsRuntime
    [Windows.ApplicationModel.Package,Windows.ApplicationModel,ContentType=Windows] | Out-Null
    [Windows.Management.Deployment.PackageManager,Windows.Management.Deployment,ContentType=Windows] | Out-Null
    $mgr = [Windows.Management.Deployment.PackageManager]::new()
    $pkg = $mgr.FindPackagesForUser('', $appId) | Select-Object -First 1
    if ($pkg) {
        $entries = $pkg.GetAppListEntries()
        $entry = $entries | Select-Object -First 1
        if ($entry) {
            $entry.LaunchAsync().GetAwaiter().GetResult()
            $launched = $true
        }
    }
} catch {}

if (-not $launched) {
    try {
        Start-Process -FilePath 'explorer.exe' -ArgumentList "shell:AppsFolder\$appId" -ErrorAction Stop
        Start-Sleep -Milliseconds 500
        $launched = $true
    } catch {}
}

if ($launched) {
    Write-Output 'OK'
} else {
    Write-Output 'FALLBACK'
}
"#;
                let ps_script_filled = ps_script.replace("{0}", app_id);
                let output = Command::new("powershell")
                    .args(["-NoProfile", "-NonInteractive", "-Command", &ps_script_filled])
                    .output();
                if let Ok(out) = output {
                    let stdout = String::from_utf8_lossy(&out.stdout).trim().to_string();
                    if stdout == "OK" { return Ok(()); }
                }
            }
        }
        if !app.executable_path.is_empty() && std::path::Path::new(&app.executable_path).exists() {
            log::info!("[AppLauncher:launch_application] Launching via executable path: {}", app.executable_path);
            match launch_via_powershell(&app.executable_path) {
                Ok(()) => return Ok(()),
                Err(e) => log::warn!("[AppLauncher:launch_application] Executable path launch failed: {}", e),
            }
        }
    }

    let ps_fallback_template = r#"
$searchPaths = @(
    'C:\ProgramData\Microsoft\Windows\Start Menu\Programs',
    [Environment]::GetFolderPath('Desktop'),
    [Environment]::GetFolderPath('StartMenu'),
    "$env:APPDATA\Microsoft\Windows\Start Menu\Programs"
)
$found = $null
$searchName = '{0}'
foreach ($p in $searchPaths) {
    if ((-not $found) -and (Test-Path $p)) {
        $items = Get-ChildItem $p -Filter ("*" + $searchName + "*") -ErrorAction SilentlyContinue
        foreach ($item in $items) {
            $found = $item.FullName
            break
        }
        if (-not $found) {
            $items = Get-ChildItem $p -Recurse -Filter ("*" + $searchName + "*") -ErrorAction SilentlyContinue | Select-Object -First 3
            foreach ($item in $items) { $found = $item.FullName; break }
        }
    }
}
if ($found) {
    try {
        Start-Process $found -ErrorAction Stop
        Write-Output ("LAUNCHED:" + $found)
    } catch {
        Write-Output ("LAUNCH_FAILED:" + $_.Exception.Message)
    }
} else {
    Write-Output "NOT_FOUND"
}
"#;
    let ps_fallback = ps_fallback_template.replace("{0}", name);
    let output = Command::new("powershell")
        .args(["-NoProfile", "-NonInteractive", "-Command", &ps_fallback])
        .output()
        .map_err(|e| AutomaticallyError::Automation(format!("Launch fallback failed: {}", e)))?;
    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if stdout.starts_with("LAUNCHED:") {
        return Ok(());
    }
    if stdout.starts_with("LAUNCH_FAILED:") {
        return Err(AutomaticallyError::Automation(format!(
            "Found '{}' but failed to launch: {}", name, stdout.strip_prefix("LAUNCH_FAILED:").unwrap_or("unknown error")
        )));
    }

    Err(AutomaticallyError::Automation(format!(
        "Application '{}' not found. Searched Start Menu, Desktop, PATH, and registry. Available similar apps: {}",
        name,
        apps.iter().map(|a| a.name.as_str()).collect::<Vec<_>>().join(", ")
    )))
}

/// Windows平台：通过PowerShell Start-Process启动应用（可靠检测启动失败）
#[cfg(target_os = "windows")]
fn launch_via_powershell(target: &str) -> Result<()> {
    use std::process::Command;

    let escaped = target.replace("'", "''");
    let ps_script = format!(
        "Start-Process -FilePath '{}' -ErrorAction Stop",
        escaped
    );

    let output = Command::new("powershell")
        .args(["-NoProfile", "-NonInteractive", "-Command", &ps_script])
        .output()
        .map_err(|e| AutomaticallyError::Automation(format!("Failed to execute PowerShell launch: {}", e)))?;

    if output.status.success() {
        Ok(())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        Err(AutomaticallyError::Automation(format!(
            "Failed to launch '{}': {}", target, stderr
        )))
    }
}

/// Windows平台：列出已安装应用 — 扫描开始菜单、桌面快捷方式
#[cfg(target_os = "windows")]
fn list_installed_apps_windows(filter: Option<&str>) -> Result<Vec<AppInfo>> {
    use std::process::Command;

    let mut apps = Vec::new();

    let output = Command::new("powershell")
        .args([
            "-NoProfile",
            "-Command",
            r#"
$results = @()

$apps = Get-StartApps | Select-Object -First 150
foreach ($app in $apps) {
    $results += [PSCustomObject]@{
        Name = $app.Name
        AppID = $app.AppID
        Type = 'StartMenu'
        Path = ''
    }
}

$startMenuPaths = @(
    'C:\ProgramData\Microsoft\Windows\Start Menu\Programs',
    "$env:APPDATA\Microsoft\Windows\Start Menu\Programs"
)
foreach ($smPath in $startMenuPaths) {
    if (Test-Path $smPath) {
        Get-ChildItem $smPath -Recurse -Include '*.lnk' -ErrorAction SilentlyContinue | ForEach-Object {
            $lnkName = [System.IO.Path]::GetFileNameWithoutExtension($_.Name)
            $already = $results | Where-Object { $_.Name -eq $lnkName } | Measure-Object
            if ($already.Count -eq 0) {
                $results += [PSCustomObject]@{
                    Name = $lnkName
                    AppID = ''
                    Type = 'Shortcut'
                    Path = $_.FullName
                }
            }
        }
    }
}

$desktopPaths = @(
    [Environment]::GetFolderPath('Desktop'),
    [Environment]::GetFolderPath('CommonDesktopDirectory')
)
foreach ($deskPath in $desktopPaths) {
    if (Test-Path $deskPath) {
        Get-ChildItem $deskPath -Filter '*.lnk' -ErrorAction SilentlyContinue | ForEach-Object {
            $lnkName = [System.IO.Path]::GetFileNameWithoutExtension($_.Name)
            $already = $results | Where-Object { $_.Name -eq $lnkName } | Measure-Object
            if ($already.Count -eq 0) {
                $results += [PSCustomObject]@{
                    Name = $lnkName
                    AppID = ''
                    Type = 'DesktopShortcut'
                    Path = $_.FullName
                }
            }
        }
    }
}

$results | ConvertTo-Json -Compress
"#,
        ])
        .output()
        .map_err(|e| AutomaticallyError::Automation(format!("Failed to list apps: {}", e)))?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    if let Ok(json_array) = serde_json::from_str::<serde_json::Value>(&stdout) {
        if let Some(arr) = json_array.as_array() {
            for item in arr {
                let app_name = item.get("Name").and_then(|v| v.as_str()).unwrap_or("").to_string();
                let app_id = item.get("AppID").and_then(|v| v.as_str()).unwrap_or("").to_string();
                let _app_type = item.get("Type").and_then(|v| v.as_str()).unwrap_or("").to_string();
                let app_path = item.get("Path").and_then(|v| v.as_str()).unwrap_or("").to_string();

                if app_name.is_empty() { continue; }
                if let Some(f) = filter {
                    if !app_name.to_lowercase().contains(&f.to_lowercase()) {
                        continue;
                    }
                }

                apps.push(AppInfo {
                    name: app_name,
                    executable_path: app_path.clone(),
                    description: None,
                    publisher: None,
                    version: None,
                    launch_command: if app_id.is_empty() { Some(app_path.clone()) } else { Some(app_id) },
                    app_source: "start_menu".to_string(),
                    keywords: Vec::new(),
                });
            }
        }
    }

    Ok(apps)
}

/// Linux平台：启动应用 — 使用nohup后台执行
#[cfg(target_os = "linux")]
fn launch_application_linux(name: &str) -> Result<()> {
    use std::process::Command;

    Command::new("sh")
        .arg("-c")
        .arg(format!("nohup {} &>/dev/null &", name))
        .spawn()
        .map_err(|e| AutomaticallyError::Automation(format!("Failed to launch '{}': {}", name, e)))?;

    Ok(())
}

/// Linux平台：列出已安装应用 — 扫描.desktop文件
#[cfg(target_os = "linux")]
fn list_installed_apps_linux(filter: Option<&str>) -> Result<Vec<AppInfo>> {
    let desktop_dirs = [
        "/usr/share/applications",
        &format!("{}/.local/share/applications", std::env::var("HOME").unwrap_or_default()),
    ];

    let mut apps = Vec::new();

    for dir in &desktop_dirs {
        let dir_path = std::path::Path::new(dir);
        if !dir_path.exists() {
            continue;
        }
        if let Ok(entries) = std::fs::read_dir(dir_path) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().and_then(|e| e.to_str()) != Some("desktop") {
                    continue;
                }
                if let Ok(content) = std::fs::read_to_string(&path) {
                    let mut app_name = String::new();
                    let mut app_exec = String::new();
                    let mut app_comment = String::new();

                    for line in content.lines() {
                        let line = line.trim();
                        if line.starts_with("Name=") {
                            app_name = line[5..].to_string();
                        } else if line.starts_with("Exec=") {
                            app_exec = line[5..].to_string();
                        } else if line.starts_with("Comment=") {
                            app_comment = line[8..].to_string();
                        }
                    }

                    if app_name.is_empty() {
                        continue;
                    }

                    if let Some(f) = filter {
                        if !app_name.to_lowercase().contains(&f.to_lowercase()) {
                            continue;
                        }
                    }

                    apps.push(AppInfo {
                        name: app_name,
                        executable_path: path.to_string_lossy().to_string(),
                        description: if app_comment.is_empty() { None } else { Some(app_comment) },
                        publisher: None,
                        version: None,
                        launch_command: Some(app_exec),
                        app_source: "desktop_entry".to_string(),
                        keywords: Vec::new(),
                    });
                }
            }
        }
    }

    Ok(apps)
}

/// macOS平台：启动应用 — 使用open命令
#[cfg(target_os = "macos")]
fn launch_application_macos(name: &str) -> Result<()> {
    use std::process::Command;

    Command::new("open")
        .arg("-a")
        .arg(name)
        .spawn()
        .map_err(|e| AutomaticallyError::Automation(format!("Failed to launch '{}': {}", name, e)))?;

    Ok(())
}

/// macOS平台：列出已安装应用 — 扫描/Applications目录
#[cfg(target_os = "macos")]
fn list_installed_apps_macos(filter: Option<&str>) -> Result<Vec<AppInfo>> {
    let apps_dir = std::path::Path::new("/Applications");
    let mut apps = Vec::new();

    if let Ok(entries) = std::fs::read_dir(apps_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            let name = path.file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("")
                .to_string();

            let is_app = path.extension()
                .and_then(|e| e.to_str())
                .map_or(false, |e| e == "app")
                || path.to_string_lossy().ends_with(".app");

            if name.is_empty() || !is_app {
                continue;
            }

            if let Some(f) = filter {
                if !name.to_lowercase().contains(&f.to_lowercase()) {
                    continue;
                }
            }

            apps.push(AppInfo {
                name,
                executable_path: path.to_string_lossy().to_string(),
                description: None,
                publisher: None,
                version: None,
                launch_command: None,
                app_source: "applications".to_string(),
                keywords: Vec::new(),
            });
        }
    }

    Ok(apps)
}
