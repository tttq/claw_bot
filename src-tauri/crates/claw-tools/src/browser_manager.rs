// Claw Desktop - 浏览器管理器 - 管理CDP浏览器实例
use serde::{Deserialize, Serialize};
use std::process::Command;

/// 浏览器信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrowserInfo {
    pub name: String,
    pub path: String,
    pub version: Option<String>,
    pub is_installed: bool,
}

/// Chrome启动配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChromeLaunchConfig {
    pub remote_debugging_port: u16,
    pub user_data_dir: Option<String>,
    pub headless: bool,
    pub disable_gpu: bool,
    pub no_sandbox: bool,
    pub window_size: Option<(u32, u32)>,
    pub additional_args: Vec<String>,
}

impl Default for ChromeLaunchConfig {
    fn default() -> Self {
        Self {
            remote_debugging_port: 9222,
            user_data_dir: None,
            headless: false,
            disable_gpu: true,
            no_sandbox: false,
            window_size: Some((1280, 800)),
            additional_args: vec![],
        }
    }
}

/// 检测系统安装的Chrome/Edge浏览器 — 扫描Windows常见安装路径
pub fn detect_chrome_installations() -> Vec<BrowserInfo> {
    let mut browsers = Vec::new();

    let chrome_paths = get_windows_chrome_paths();
    for path in chrome_paths {
        if std::path::Path::new(&path).exists() {
            let version = get_chrome_version(&path);
            browsers.push(BrowserInfo {
                name: "Google Chrome".to_string(),
                path: path.clone(),
                version,
                is_installed: true,
            });
        }
    }

    let edge_paths = get_windows_edge_paths();
    for path in edge_paths {
        if std::path::Path::new(&path).exists() {
            let version = get_chrome_version(&path);
            browsers.push(BrowserInfo {
                name: "Microsoft Edge".to_string(),
                path: path.clone(),
                version,
                is_installed: true,
            });
        }
    }

    browsers
}

#[cfg(target_os = "windows")]
/// 获取Windows Chrome安装路径
fn get_windows_chrome_paths() -> Vec<String> {
    let mut paths = Vec::new();

    let program_files = [
        r"C:\Program Files\Google\Chrome\Application\chrome.exe",
        r"C:\Program Files (x86)\Google\Chrome\Application\chrome.exe",
    ];

    for path in &program_files {
        paths.push(path.to_string());
    }

    if let Ok(output) = Command::new("reg")
        .args([
            "query",
            "HKLM\\SOFTWARE\\Microsoft\\Windows\\CurrentVersion\\App Paths\\chrome.exe",
        ])
        .output()
    {
        let stdout = String::from_utf8_lossy(&output.stdout);
        for line in stdout.lines() {
            let line = line.trim();
            if line.ends_with(".exe") && !line.contains("REG") {
                paths.push(line.to_string());
            }
        }
    }

    if let Ok(output) = Command::new("where").arg("chrome.exe").output() {
        let stdout = String::from_utf8_lossy(&output.stdout);
        for line in stdout.lines() {
            let line = line.trim();
            if !line.is_empty() && line.ends_with("chrome.exe") {
                paths.push(line.to_string());
            }
        }
    }

    paths.sort();
    paths.dedup();
    paths
}

#[cfg(target_os = "windows")]
/// 获取Windows Edge安装路径
fn get_windows_edge_paths() -> Vec<String> {
    let mut paths = vec![
        r"C:\Program Files (x86)\Microsoft\Edge\Application\msedge.exe".to_string(),
        r"C:\Program Files\Microsoft\Edge\Application\msedge.exe".to_string(),
    ];

    if let Ok(output) = Command::new("where").arg("msedge.exe").output() {
        let stdout = String::from_utf8_lossy(&output.stdout);
        for line in stdout.lines() {
            let line = line.trim();
            if !line.is_empty() && line.ends_with("msedge.exe") {
                paths.push(line.to_string());
            }
        }
    }

    paths.sort();
    paths.dedup();
    paths
}

/// 获取Chrome版本号 — 通过执行chrome --version命令
fn get_chrome_version(path: &str) -> Option<String> {
    #[cfg(target_os = "windows")]
    {
        if let Ok(output) = Command::new("powershell")
            .args([
                "-Command",
                &format!(
                    "(Get-Item '{}').VersionInfo.FileVersion",
                    path.replace("'", "''")
                ),
            ])
            .output()
        {
            let version = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if !version.is_empty() {
                return Some(version);
            }
        }
    }

    None
}

/// 启动Chrome并开启远程调试端口 — 返回实际绑定的调试端口
pub fn launch_chrome_with_debugging(
    browser_path: &str,
    config: &ChromeLaunchConfig,
) -> Result<u16, String> {
    log::info!(
        "[Browser] Launching Chrome with debugging on port {}",
        config.remote_debugging_port
    );

    let mut args: Vec<String> = Vec::new();

    args.push(format!(
        "--remote-debugging-port={}",
        config.remote_debugging_port
    ));

    if config.headless {
        args.push("--headless=new".to_string());
        args.push("--disable-gpu".to_string());
    } else if config.disable_gpu {
        args.push("--disable-gpu".to_string());
    }

    if config.no_sandbox {
        args.push("--no-sandbox".to_string());
    }

    if let Some(user_data_dir) = &config.user_data_dir {
        args.push(format!("--user-data-dir={}", user_data_dir));
    } else {
        let temp_dir = std::env::temp_dir().join("claw-chrome-debug-profile");
        args.push(format!("--user-data-dir={}", temp_dir.display()));
    }

    if let Some((width, height)) = config.window_size {
        args.push(format!("--window-size={},{}", width, height));
    }

    args.extend(config.additional_args.iter().cloned());

    args.push("--first-run=no".to_string());
    args.push("--no-default-browser-check".to_string());

    match Command::new(browser_path).args(&args).spawn() {
        Ok(_) => {
            log::info!("[Browser] Chrome launched successfully");
            Ok(config.remote_debugging_port)
        }
        Err(e) => Err(format!("Failed to launch Chrome: {}", e)),
    }
}

/// 检查调试端口是否可用 — 尝试TCP连接
pub fn check_debug_port(port: u16) -> Result<bool, String> {
    use std::net::TcpStream;
    let addr = format!("127.0.0.1:{}", port)
        .parse()
        .map_err(|e| format!("Failed to parse address: {}", e))?;
    match TcpStream::connect_timeout(&addr, std::time::Duration::from_millis(500)) {
        Ok(_) => Ok(true),
        Err(_) => Ok(false),
    }
}

/// 获取CDP版本信息 — 请求/json/version端点
pub async fn fetch_cdp_version(port: u16) -> Result<serde_json::Value, String> {
    let url = format!("http://127.0.0.1:{}/json/version", port);

    match reqwest::get(&url).await {
        Ok(response) => match response.json::<serde_json::Value>().await {
            Ok(json) => Ok(json),
            Err(e) => Err(format!("Failed to parse CDP response: {}", e)),
        },
        Err(e) => Err(format!("Failed to connect to CDP endpoint: {}", e)),
    }
}

/// 列出浏览器标签页 — 请求/json端点获取所有标签的WebSocket URL
pub async fn list_browser_tabs(port: u16) -> Result<Vec<BrowserTab>, String> {
    let url = format!("http://127.0.0.1:{}/json/list", port);

    match reqwest::get(&url).await {
        Ok(response) => match response.json::<Vec<serde_json::Value>>().await {
            Ok(tabs) => {
                let result: Vec<BrowserTab> = tabs
                    .iter()
                    .filter_map(|tab| {
                        Some(BrowserTab {
                            id: tab.get("id")?.as_str()?.to_string(),
                            url: tab.get("url")?.as_str().unwrap_or("").to_string(),
                            title: tab.get("title")?.as_str().unwrap_or("Untitled").to_string(),
                            web_socket_url: tab.get("webSocketDebuggerUrl")?.as_str()?.to_string(),
                            devtools_url: tab
                                .get("devtoolsFrontendUrl")?
                                .as_str()
                                .map(|s| s.to_string()),
                        })
                    })
                    .collect();
                Ok(result)
            }
            Err(e) => Err(format!("Failed to parse tabs: {}", e)),
        },
        Err(e) => Err(format!("Failed to list tabs: {}", e)),
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrowserTab {
    pub id: String,
    pub url: String,
    pub title: String,
    pub web_socket_url: String,
    pub devtools_url: Option<String>,
}
