// Claw Desktop - 应用索引模块
// 维护已安装应用的搜索索引，支持模糊匹配、名称查找、索引刷新
use crate::error::Result;
use crate::types::AppInfo;
use once_cell::sync::Lazy;
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::Mutex;
use std::time::{Instant, SystemTime};

static APP_INDEX: Lazy<Mutex<AppIndexState>> = Lazy::new(|| {
    Mutex::new(AppIndexState {
        apps: Vec::new(),
        name_map: HashMap::new(),
        last_built: None,
        is_building: false,
    })
});

const CACHE_TTL_SECS: u64 = 3600;

/// 应用索引全局状态 — 存储已索引应用列表、名称映射和构建时间
struct AppIndexState {
    apps: Vec<AppInfo>,
    name_map: HashMap<String, usize>,
    last_built: Option<Instant>,
    is_building: bool,
}

/// 获取缓存文件路径 — ~/AppData/Local/claw-desktop/app_index.json
fn cache_file_path() -> PathBuf {
    dirs::data_local_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("claw-desktop")
        .join("app_index.json")
}

/// 构建应用索引 — 按平台扫描所有应用来源，去重并排序
pub fn build_index() -> Result<Vec<AppInfo>> {
    log::info!("[AppIndex:build_index] Starting full app index build");
    let start = Instant::now();

    let mut seen_names: HashSet<String> = HashSet::new();
    let mut all_apps: Vec<AppInfo> = Vec::new();

    #[cfg(target_os = "windows")]
    {
        let registry_apps = scan_windows_registry();
        for app in registry_apps {
            if !seen_names.contains(&app.name.to_lowercase()) {
                seen_names.insert(app.name.to_lowercase());
                all_apps.push(app);
            }
        }

        let start_menu_apps = scan_windows_start_menu();
        for app in start_menu_apps {
            if !seen_names.contains(&app.name.to_lowercase()) {
                seen_names.insert(app.name.to_lowercase());
                all_apps.push(app);
            }
        }

        let desktop_apps = scan_windows_desktop();
        for app in desktop_apps {
            if !seen_names.contains(&app.name.to_lowercase()) {
                seen_names.insert(app.name.to_lowercase());
                all_apps.push(app);
            }
        }

        let uwp_apps = scan_windows_uwp();
        for app in uwp_apps {
            if !seen_names.contains(&app.name.to_lowercase()) {
                seen_names.insert(app.name.to_lowercase());
                all_apps.push(app);
            }
        }

        let path_apps = scan_windows_path();
        for app in path_apps {
            if !seen_names.contains(&app.name.to_lowercase()) {
                seen_names.insert(app.name.to_lowercase());
                all_apps.push(app);
            }
        }
    }

    #[cfg(target_os = "linux")]
    {
        let desktop_apps = scan_linux_desktop_entries();
        for app in desktop_apps {
            if !seen_names.contains(&app.name.to_lowercase()) {
                seen_names.insert(app.name.to_lowercase());
                all_apps.push(app);
            }
        }

        let path_apps = scan_linux_path();
        for app in path_apps {
            if !seen_names.contains(&app.name.to_lowercase()) {
                seen_names.insert(app.name.to_lowercase());
                all_apps.push(app);
            }
        }

        let snap_apps = scan_linux_snap().unwrap_or_default();
        for app in snap_apps {
            if !seen_names.contains(&app.name.to_lowercase()) {
                seen_names.insert(app.name.to_lowercase());
                all_apps.push(app);
            }
        }

        let flatpak_apps = scan_linux_flatpak().unwrap_or_default();
        for app in flatpak_apps {
            if !seen_names.contains(&app.name.to_lowercase()) {
                seen_names.insert(app.name.to_lowercase());
                all_apps.push(app);
            }
        }
    }

    #[cfg(target_os = "macos")]
    {
        let applications = scan_macos_applications();
        for app in applications {
            if !seen_names.contains(&app.name.to_lowercase()) {
                seen_names.insert(app.name.to_lowercase());
                all_apps.push(app);
            }
        }

        let homebrew_apps = scan_macos_homebrew().unwrap_or_default();
        for app in homebrew_apps {
            if !seen_names.contains(&app.name.to_lowercase()) {
                seen_names.insert(app.name.to_lowercase());
                all_apps.push(app);
            }
        }

        let path_apps = scan_macos_path();
        for app in path_apps {
            if !seen_names.contains(&app.name.to_lowercase()) {
                seen_names.insert(app.name.to_lowercase());
                all_apps.push(app);
            }
        }
    }

    all_apps.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));

    let elapsed = start.elapsed();
    log::info!(
        "[AppIndex:build_index] Indexed {} apps in {:.2}ms",
        all_apps.len(),
        elapsed.as_millis()
    );

    save_cache(&all_apps);

    Ok(all_apps)
}

/// 保存索引缓存 — 将应用列表序列化为JSON写入本地文件
fn save_cache(apps: &[AppInfo]) {
    let cache_path = cache_file_path();
    if let Some(parent) = cache_path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    match serde_json::to_string_pretty(apps) {
        Ok(json) => {
            if let Err(e) = std::fs::write(&cache_path, json) {
                log::warn!("[AppIndex:save_cache] Failed to write cache: {}", e);
            } else {
                log::debug!("[AppIndex:save_cache] Cache saved to {:?}", cache_path);
            }
        }
        Err(e) => {
            log::warn!("[AppIndex:save_cache] Failed to serialize: {}", e);
        }
    }
}

/// 加载索引缓存 — 从本地文件读取，超过1小时则视为过期
fn load_cache() -> Option<Vec<AppInfo>> {
    let cache_path = cache_file_path();
    if !cache_path.exists() {
        return None;
    }

    let metadata = match std::fs::metadata(&cache_path) {
        Ok(m) => m,
        Err(_) => return None,
    };

    let modified_time = match metadata.modified() {
        Ok(t) => t,
        Err(_) => return None,
    };

    let age = match SystemTime::now().duration_since(modified_time) {
        Ok(d) => d,
        Err(_) => return None,
    };

    if age.as_secs() > CACHE_TTL_SECS {
        log::info!("[AppIndex:load_cache] Cache expired (age: {:.0}h), rebuilding", age.as_secs() / 3600);
        return None;
    }

    match std::fs::read_to_string(&cache_path) {
        Ok(content) => match serde_json::from_str::<Vec<AppInfo>>(&content) {
            Ok(apps) => {
                log::info!(
                    "[AppIndex:load_cache] Loaded {} apps from cache (age: {:.1}m)",
                    apps.len(),
                    age.as_secs() / 60
                );
                Some(apps)
            }
            Err(e) => {
                log::warn!("[AppIndex:load_cache] Cache parse error: {}", e);
                None
            }
        },
        Err(e) => {
            log::warn!("[AppIndex:load_cache] Cache read error: {}", e);
            None
        }
    }
}

/// 更新全局索引状态 — 写入应用列表和名称映射，更新构建时间
fn update_global_state(apps: Vec<AppInfo>) {
    let mut state = APP_INDEX.lock().unwrap_or_else(|e| e.into_inner());
    let name_pairs: Vec<(String, usize)> = apps.iter()
        .enumerate()
        .map(|(i, app)| (app.name.to_lowercase(), i))
        .collect();
    state.apps = apps;
    state.name_map.clear();
    for (name, idx) in name_pairs {
        state.name_map.insert(name, idx);
    }
    state.last_built = Some(Instant::now());
    state.is_building = false;
}

/// 确保索引已构建 — 优先使用内存缓存，其次磁盘缓存，最后全量构建
pub fn ensure_indexed() -> Vec<AppInfo> {
    {
        let state = APP_INDEX.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(built_at) = state.last_built {
            if built_at.elapsed().as_secs() < CACHE_TTL_SECS && !state.apps.is_empty() {
                log::info!("[AppIndex:ensure_indexed] Using cached index ({} apps, age: {:.0}s)", state.apps.len(), built_at.elapsed().as_secs());
                return state.apps.clone();
            }
        }
        if state.is_building {
            log::info!("[AppIndex:ensure_indexed] Index already building, waiting...");
            drop(state);
            std::thread::sleep(std::time::Duration::from_millis(500));
            let state = APP_INDEX.lock().unwrap_or_else(|e| e.into_inner());
            return state.apps.clone();
        }
    }

    if let Some(cached) = load_cache() {
        update_global_state(cached.clone());
        return cached;
    }

    {
        let mut state = APP_INDEX.lock().unwrap_or_else(|e| e.into_inner());
        state.is_building = true;
    }

    match build_index() {
        Ok(apps) => {
            let result = apps.clone();
            update_global_state(apps);
            result
        }
        Err(e) => {
            log::error!("[AppIndex:ensure_indexed] Build failed: {}", e);
            let mut state = APP_INDEX.lock().unwrap_or_else(|e| e.into_inner());
            state.is_building = false;
            state.apps.clone()
        }
    }
}

/// 搜索应用 — 按关键词模糊匹配并按相关度排序
pub fn search(query: &str) -> Vec<AppInfo> {
    let apps = ensure_indexed();
    if query.is_empty() {
        return apps;
    }

    let query_trimmed = query.trim();
    let mut results: Vec<(f64, AppInfo)> = apps
        .into_iter()
        .filter(|app| app.matches_query(query_trimmed))
        .map(|app| (app.relevance_score(query_trimmed), app))
        .collect();

    results.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
    results.into_iter().map(|(_, app)| app).collect()
}

/// 查找最佳匹配 — 返回搜索结果中相关度最高的应用
pub fn find_best_match(query: &str) -> Option<AppInfo> {
    let results = search(query);
    results.into_iter().next()
}

/// 按名称查找应用 — 精确匹配→包含匹配→模糊搜索
pub fn find_by_name(name: &str) -> Option<AppInfo> {
    let apps = ensure_indexed();
    let name_lower = name.to_lowercase();

    for app in &apps {
        if app.name.to_lowercase() == name_lower {
            return Some(app.clone());
        }
    }

    for app in &apps {
        if app.name.to_lowercase().contains(&name_lower) || name_lower.contains(&app.name.to_lowercase()) {
            return Some(app.clone());
        }
    }

    search(name).into_iter().next()
}

/// 获取所有已索引应用
pub fn get_all_apps() -> Vec<AppInfo> {
    ensure_indexed()
}

/// 刷新索引 — 清除缓存并强制重新扫描
pub fn refresh_index() -> Result<Vec<AppInfo>> {
    log::info!("[AppIndex:refresh_index] Force refreshing app index");
    let cache_path = cache_file_path();
    if cache_path.exists() {
        let _ = std::fs::remove_file(&cache_path);
    }
    {
        let mut state = APP_INDEX.lock().unwrap_or_else(|e| e.into_inner());
        state.last_built = None;
        state.apps.clear();
        state.name_map.clear();
    }
    let apps = build_index()?;
    update_global_state(apps.clone());
    Ok(apps)
}

/// 获取索引统计 — 返回应用总数、索引年龄和来源分布
pub fn get_stats() -> serde_json::Value {
    let state = APP_INDEX.lock().unwrap_or_else(|e| e.into_inner());
    let age_secs = state
        .last_built
        .map(|t| t.elapsed().as_secs())
        .unwrap_or(u64::MAX);

    let source_counts: std::collections::HashMap<&str, i32> =
        state.apps.iter().fold(HashMap::new(), |mut acc, app| {
            *acc.entry(&app.app_source as &str).or_insert(0) += 1;
            acc
        });

    serde_json::json!({
        "total_apps": state.apps.len(),
        "index_age_seconds": age_secs,
        "is_cached": age_secs < CACHE_TTL_SECS,
        "sources": source_counts,
    })
}

/// Windows: 扫描注册表卸载项 — 获取已安装应用信息
#[cfg(target_os = "windows")]
fn scan_windows_registry() -> Vec<AppInfo> {
    use std::process::Command;

    let ps_script = r#"
$results = @()

$hives = @(
    'HKLM:\SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall',
    'HKCU:\SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall',
    'HKLM:\SOFTWARE\WOW6432Node\Microsoft\Windows\CurrentVersion\Uninstall'
)

foreach ($hive in $hives) {
    if (Test-Path $hive) {
        Get-ChildItem $hive -ErrorAction SilentlyContinue | ForEach-Object {
            $props = Get-ItemProperty $_.PSPath -ErrorAction SilentlyContinue
            if ($props) {
                $name = $props.DisplayName
                $publisher = $props.Publisher
                $version = $props.DisplayVersion
                $installLoc = $props.InstallLocation
                $uninstallStr = $props.UninstallString

                if ($name -and $name.Trim() -ne '') {
                    $exePath = ''
                    if ($installLoc -and (Test-Path $installLoc)) {
                        $exes = Get-ChildItem $installLoc -Filter '*.exe' -Recurse -Depth 1 -ErrorAction SilentlyContinue | Select-Object -First 1
                        if ($exes) { $exePath = $exes.FullName }
                    }

                    if (-not $exePath -and $uninstallStr) {
                        if ($uninstallStr -match '"([^"]+\.exe)"') {
                            $candidate = $Matches[1]
                            if (Test-Path $candidate) { $exePath = $candidate }
                        }
                    }

                    $results += [PSCustomObject]@{
                        Name = $name.Trim()
                        Publisher = if ($publisher) { $publisher.Trim() } else { '' }
                        Version = if ($version) { $version.Trim() } else { '' }
                        Path = $exePath
                        Source = 'registry'
                    }
                }
            }
        }
    }
}

$results | ConvertTo-Json -Compress
"#;

    let output = Command::new("powershell")
        .args(["-NoProfile", "-NonInteractive", "-Command", ps_script])
        .output();

    let mut apps = Vec::new();
    if let Ok(out) = output {
        let stdout = String::from_utf8_lossy(&out.stdout);
        if let Ok(json_arr) = serde_json::from_str::<serde_json::Value>(&stdout) {
            if let Some(arr) = json_arr.as_array() {
                for item in arr {
                    let name = item.get("Name").and_then(|v| v.as_str()).unwrap_or("").to_string();
                    if name.is_empty() { continue; }

                    apps.push(AppInfo {
                        name,
                        executable_path: item.get("Path").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                        description: None,
                        publisher: {
                            let p = item.get("Publisher").and_then(|v| v.as_str()).unwrap_or("").to_string();
                            if p.is_empty() { None } else { Some(p) }
                        },
                        version: {
                            let v = item.get("Version").and_then(|v| v.as_str()).unwrap_or("").to_string();
                            if v.is_empty() { None } else { Some(v) }
                        },
                        launch_command: None,
                        app_source: "registry".to_string(),
                        keywords: Vec::new(),
                    });
                }
            }
        }
    }

    log::info!("[AppIndex:scan_windows_registry] Found {} apps from registry", apps.len());
    apps
}

/// Windows: 扫描开始菜单快捷方式 — 获取开始菜单中的应用
#[cfg(target_os = "windows")]
fn scan_windows_start_menu() -> Vec<AppInfo> {
    use std::process::Command;

    let ps_script = r#"
$results = @()
$paths = @(
    'C:\ProgramData\Microsoft\Windows\Start Menu\Programs',
    "$env:APPDATA\Microsoft\Windows\Start Menu\Programs"
)
foreach ($root in $paths) {
    if (Test-Path $root) {
        Get-ChildItem $root -Recurse -Include '*.lnk' -ErrorAction SilentlyContinue | ForEach-Object {
            $name = [System.IO.Path]::GetFileNameWithoutExtension($_.Name)
            if ($name -and $name.Trim() -ne '') {
                $results += [PSCustomObject]@{
                    Name = $name.Trim()
                    Path = $_.FullName
                    Source = 'start_menu'
                }
            }
        }
    }
}
$results | ConvertTo-Json -Compress
"#;

    let output = Command::new("powershell")
        .args(["-NoProfile", "-NonInteractive", "-Command", ps_script])
        .output();

    let mut apps = Vec::new();
    if let Ok(out) = output {
        let stdout = String::from_utf8_lossy(&out.stdout);
        if let Ok(json_arr) = serde_json::from_str::<serde_json::Value>(&stdout) {
            if let Some(arr) = json_arr.as_array() {
                for item in arr {
                    let name = item.get("Name").and_then(|v| v.as_str()).unwrap_or("").to_string();
                    if name.is_empty() { continue; }
                    let path = item.get("Path").and_then(|v| v.as_str()).unwrap_or("").to_string();

                    apps.push(AppInfo {
                        name,
                        executable_path: path.clone(),
                        description: None,
                        publisher: None,
                        version: None,
                        launch_command: Some(path),
                        app_source: "start_menu".to_string(),
                        keywords: vec!["shortcut".to_string(), "start menu".to_string()],
                    });
                }
            }
        }
    }

    log::info!("[AppIndex:scan_windows_start_menu] Found {} apps from Start Menu", apps.len());
    apps
}

/// Windows: 扫描桌面快捷方式 — 获取桌面上的应用和exe
#[cfg(target_os = "windows")]
fn scan_windows_desktop() -> Vec<AppInfo> {
    use std::process::Command;

    let ps_script = r#"
$results = @()
$paths = @([Environment]::GetFolderPath('Desktop'), [Environment]::GetFolderPath('CommonDesktopDirectory'))
foreach ($p in $paths) {
    if (Test-Path $p) {
        Get-ChildItem $p -Filter '*.lnk' -ErrorAction SilentlyContinue | ForEach-Object {
            $name = [System.IO.Path]::GetFileNameWithoutExtension($_.Name)
            if ($name -and $name.Trim() -ne '' -and $name -ne 'Recycle Bin') {
                $results += [PSCustomObject]@{ Name = $name.Trim(); Path = $_.FullName; Source = 'desktop' }
            }
        }
        Get-ChildItem $p -Filter '*.exe' -ErrorAction SilentlyContinue | ForEach-Object {
            $name = [System.IO.Path]::GetFileNameWithoutExtension($_.Name)
            if ($name -and $name.Trim() -ne '') {
                $results += [PSCustomObject]@{ Name = $name.Trim(); Path = $_.FullName; Source = 'desktop' }
            }
        }
    }
}
$results | ConvertTo-Json -Compress
"#;

    let output = Command::new("powershell")
        .args(["-NoProfile", "-NonInteractive", "-Command", ps_script])
        .output();

    let mut apps = Vec::new();
    if let Ok(out) = output {
        let stdout = String::from_utf8_lossy(&out.stdout);
        if let Ok(json_arr) = serde_json::from_str::<serde_json::Value>(&stdout) {
            if let Some(arr) = json_arr.as_array() {
                for item in arr {
                    let name = item.get("Name").and_then(|v| v.as_str()).unwrap_or("").to_string();
                    if name.is_empty() { continue; }
                    let path = item.get("Path").and_then(|v| v.as_str()).unwrap_or("").to_string();

                    apps.push(AppInfo {
                        name,
                        executable_path: path.clone(),
                        description: None,
                        publisher: None,
                        version: None,
                        launch_command: Some(path),
                        app_source: "desktop".to_string(),
                        keywords: vec!["shortcut".to_string(), "desktop".to_string()],
                    });
                }
            }
        }
    }

    log::info!("[AppIndex:scan_windows_desktop] Found {} apps from Desktop", apps.len());
    apps
}

/// Windows: 扫描UWP应用 — 通过Get-StartApps获取Microsoft Store应用
#[cfg(target_os = "windows")]
fn scan_windows_uwp() -> Vec<AppInfo> {
    use std::process::Command;

    let ps_script = r#"
$apps = Get-StartApps | Select-Object -First 200
$results = @()
foreach ($app in $apps) {
    if ($app.Name -and $app.Name.Trim() -ne '') {
        $results += [PSCustomObject]@{
            Name = $app.Name.Trim()
            AppID = $app.AppID
            Source = 'uwp'
        }
    }
}
$results | ConvertTo-Json -Compress
"#;

    let output = Command::new("powershell")
        .args(["-NoProfile", "-NonInteractive", "-Command", ps_script])
        .output();

    let mut apps = Vec::new();
    if let Ok(out) = output {
        let stdout = String::from_utf8_lossy(&out.stdout);
        if let Ok(json_arr) = serde_json::from_str::<serde_json::Value>(&stdout) {
            if let Some(arr) = json_arr.as_array() {
                for item in arr {
                    let name = item.get("Name").and_then(|v| v.as_str()).unwrap_or("").to_string();
                    if name.is_empty() { continue; }
                    let app_id = item.get("AppID").and_then(|v| v.as_str()).unwrap_or("").to_string();

                    apps.push(AppInfo {
                        name,
                        executable_path: String::new(),
                        description: None,
                        publisher: None,
                        version: None,
                        launch_command: if app_id.is_empty() { None } else { Some(app_id) },
                        app_source: "uwp".to_string(),
                        keywords: vec!["uwp".to_string(), "windows store".to_string()],
                    });
                }
            }
        }
    }

    log::info!("[AppIndex:scan_windows_uwp] Found {} UWP apps", apps.len());
    apps
}

/// Windows: 扫描Program Files目录 — 查找可执行文件
#[cfg(target_os = "windows")]
fn scan_windows_path() -> Vec<AppInfo> {
    use std::process::Command;

    let ps_script = r#"
$dirs = @(
    'C:\Program Files',
    "${env:ProgramFiles(x86)}",
    "$env:LOCALAPPDATA\Programs",
    "$env:APPDATA\Microsoft\Windows\Start Menu\Programs"
)
$seen = @{}
$results = @()
foreach ($dir in $dirs) {
    if (Test-Path $dir) {
        Get-ChildItem $dir -Directory -ErrorAction SilentlyContinue | ForEach-Object {
            $subDir = $_.FullName
            $exes = Get-ChildItem $subDir -Filter '*.exe' -Depth 1 -ErrorAction SilentlyContinue | Select-Object -First 3
            foreach ($exe in $exes) {
                $name = [System.IO.Path]::GetFileNameWithoutExtension($exe.Name)
                $lower = $name.ToLower()
                if (-not $seen.ContainsKey($lower) -and $name -and $name.Trim() -ne '') {
                    $seen[$lower] = $true
                    $results += [PSCustomObject]@{
                        Name = $name.Trim()
                        Path = $exe.FullName
                        Source = 'program_files'
                    }
                }
            }
        }
    }
}
$results | ConvertTo-Json -Compress
"#;

    let output = Command::new("powershell")
        .args(["-NoProfile", "-NonInteractive", "-Command", ps_script])
        .output();

    let mut apps = Vec::new();
    if let Ok(out) = output {
        let stdout = String::from_utf8_lossy(&out.stdout);
        if let Ok(json_arr) = serde_json::from_str::<serde_json::Value>(&stdout) {
            if let Some(arr) = json_arr.as_array() {
                for item in arr {
                    let name = item.get("Name").and_then(|v| v.as_str()).unwrap_or("").to_string();
                    if name.is_empty() { continue; }
                    let path = item.get("Path").and_then(|v| v.as_str()).unwrap_or("").to_string();

                    apps.push(AppInfo {
                        name,
                        executable_path: path.clone(),
                        description: None,
                        publisher: None,
                        version: None,
                        launch_command: Some(path),
                        app_source: "program_files".to_string(),
                        keywords: vec!["executable".to_string()],
                    });
                }
            }
        }
    }

    log::info!("[AppIndex:scan_windows_path] Found {} apps from Program Files", apps.len());
    apps
}

/// Linux: 扫描.desktop文件 — 解析系统应用入口
#[cfg(target_os = "linux")]
fn scan_linux_desktop_entries() -> Vec<AppInfo> {
    let desktop_dirs = [
        "/usr/share/applications",
        "/usr/local/share/applications",
        &format!("{}/.local/share/applications",
            std::env::var("HOME").unwrap_or_default()),
        "/var/lib/flatpak/exports/share/applications",
        "/var/lib/snapd/desktop/applications",
    ];

    let mut apps = Vec::new();
    for dir in &desktop_dirs {
        let dir_path = std::path::Path::new(dir);
        if !dir_path.exists() { continue; }
        if let Ok(entries) = std::fs::read_dir(dir_path) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().and_then(|e| e.to_str()) != Some("desktop") { continue; }
                if let Ok(content) = std::fs::read_to_string(&path) {
                    let (name, exec, comment, categories) = parse_desktop_entry(&content);
                    if name.is_empty() { continue; }
                    apps.push(AppInfo {
                        name,
                        executable_path: path.to_string_lossy().to_string(),
                        description: if comment.is_empty() { None } else { Some(comment) },
                        publisher: None,
                        version: None,
                        launch_command: if exec.is_empty() { None } else { Some(exec) },
                        app_source: "desktop_entry".to_string(),
                        keywords: categories.split(';')
                            .filter(|s| !s.is_empty())
                            .map(String::from)
                            .collect(),
                    });
                }
            }
        }
    }

    log::info!("[AppIndex:scan_linux_desktop_entries] Found {} .desktop apps", apps.len());
    apps
}

/// Linux: 解析.desktop文件内容 — 提取Name、Exec、Comment和Categories
#[cfg(target_os = "linux")]
fn parse_desktop_entry(content: &str) -> (String, String, String, String) {
    let mut name = String::new();
    let mut exec = String::new();
    let mut comment = String::new();
    let mut categories = String::new();

    for line in content.lines() {
        let line = line.trim();
        if line.starts_with('[') && line.ends_with(']') { break; }
        if line.starts_with("Name=") {
            name = line[5..].to_string();
        } else if line.starts_with("Exec=") {
            exec = line[5..].split_whitespace().next().unwrap_or(&line[5..]).to_string();
        } else if line.starts_with("Comment=") {
            comment = line[8..].to_string();
        } else if line.starts_with("Categories=") {
            categories = line[11..].to_string();
        }
    }

    (name, exec, comment, categories)
}

/// Linux: 扫描PATH可执行文件 — 查找具有执行权限的命令
#[cfg(target_os = "linux")]
fn scan_linux_path() -> Vec<AppInfo> {
    let mut apps = Vec::new();
    let path_var = std::env::var("PATH").unwrap_or_default();
    let mut seen = HashSet::new();

    for dir in path_var.split(':') {
        let dir_path = std::path::Path::new(dir);
        if !dir_path.exists() { continue; }
        if let Ok(entries) = std::fs::read_dir(dir_path) {
            for entry in entries.flatten() {
                let path = entry.path();
                if let Some(ext) = path.extension() {
                    if ext != "exe" && ext.to_str() != Some("") { continue; }
                }
                if let Some(file_name) = path.file_name() {
                    let name = file_name.to_string_lossy().to_string();
                    let lower = name.to_lowercase();
                    if seen.contains(&lower) { continue; }
                    seen.insert(lower);

                    if path.metadata().map(|m| m.permissions().mode() & 0o111 != 0).unwrap_or(false) {
                        apps.push(AppInfo {
                            name,
                            executable_path: path.to_string_lossy().to_string(),
                            description: None,
                            publisher: None,
                            version: None,
                            launch_command: Some(path.to_string_lossy().to_string()),
                            app_source: "path".to_string(),
                            keywords: vec!["cli".to_string(), "command".to_string()],
                        });
                    }
                }
            }
        }
    }

    log::info!("[AppIndex:scan_linux_path] Found {} executables from PATH", apps.len());
    apps
}

/// Linux: 扫描Snap包 — 列出已安装的snap应用
#[cfg(target_os = "linux")]
fn scan_linux_snap() -> Result<Vec<AppInfo>> {
    use std::process::Command;

    let output = Command::new("snap")
        .args(["list", "--ascii"])
        .output();

    let mut apps = Vec::new();
    if let Ok(out) = output {
        if out.status.success() {
            for line in String::from_utf8_lossy(&out.stdout).lines().skip(1) {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 2 {
                    let name = parts[0].to_string();
                    apps.push(AppInfo {
                        name,
                        executable_path: format!("snap run {}", parts[0]),
                        description: None,
                        publisher: None,
                        version: Some(parts.get(1).map(|s| s.to_string()).unwrap_or_default()),
                        launch_command: Some(format!("snap run {}", parts[0])),
                        app_source: "snap".to_string(),
                        keywords: vec!["snap".to_string(), "ubuntu".to_string()],
                    });
                }
            }
        }
    }

    log::info!("[AppIndex:scan_linux_snap] Found {} snap packages", apps.len());
    Ok(apps)
}

/// Linux: 扫描Flatpak应用 — 列出已安装的flatpak应用
#[cfg(target_os = "linux")]
fn scan_linux_flatpak() -> Result<Vec<AppInfo>> {
    use std::process::Command;

    let output = Command::new("flatpak")
        .args(["list", "--columns=application,name,version"])
        .output();

    let mut apps = Vec::new();
    if let Ok(out) = output {
        if out.status.success() {
            for line in String::from_utf8_lossy(&out.stdout).lines() {
                let parts: Vec<&str> = line.split('\t').collect();
                if parts.len() >= 2 {
                    let name = parts[1].to_string();
                    apps.push(AppInfo {
                        name,
                        executable_path: format!("flatpak run {}", parts[0]),
                        description: None,
                        publisher: None,
                        version: if parts.len() >= 3 { Some(parts[2].to_string()) } else { None },
                        launch_command: Some(format!("flatpak run {}", parts[0])),
                        app_source: "flatpak".to_string(),
                        keywords: vec!["flatpak".to_string()],
                    });
                }
            }
        }
    }

    log::info!("[AppIndex:scan_linux_flatpak] Found {} flatpak apps", apps.len());
    Ok(apps)
}

/// macOS: 扫描.app应用包 — 遍历/Applications目录
#[cfg(target_os = "macos")]
fn scan_macos_applications() -> Vec<AppInfo> {
    let app_dirs = [
        "/Applications",
        &format!("{}/Applications", std::env::var("HOME").unwrap_or_default()),
    ];

    let mut apps = Vec::new();
    for dir in &app_dirs {
        let dir_path = std::path::Path::new(dir);
        if !dir_path.exists() { continue; }
        if let Ok(entries) = std::fs::read_dir(dir_path) {
            for entry in entries.flatten() {
                let path = entry.path();
                let is_app = path.extension()
                    .and_then(|e| e.to_str())
                    .map_or(false, |e| e == "app")
                    || path.to_string_lossy().ends_with(".app");

                if !is_app { continue; }

                let name = path.file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("")
                    .to_string();

                if name.is_empty() { continue; }

                let plist_path = path.join("Contents/Info.plist");
                let (bundle_id, version, min_version) = read_macos_plist(&plist_path);

                apps.push(AppInfo {
                    name,
                    executable_path: path.to_string_lossy().to_string(),
                    description: None,
                    publisher: None,
                    version,
                    launch_command: Some(path.to_string_lossy().to_string()),
                    app_source: "applications".to_string(),
                    keywords: {
                        let mut kw = vec!["macos".to_string()];
                        if let Some(ref bid) = bundle_id { kw.push(bid.clone()); }
                        kw
                    },
                });
            }
        }
    }

    log::info!("[AppIndex:scan_macos_applications] Found {} .app bundles", apps.len());
    apps
}

/// macOS: 读取Info.plist — 提取Bundle ID和版本号
#[cfg(target_os = "macos")]
fn read_macos_plist(plist_path: &std::path::Path) -> (Option<String>, Option<String>, Option<String>) {
    use std::process::Command;

    if !plist_path.exists() { return (None, None, None); }

    let output = Command::new("/usr/libexec/PlistBuddy")
        .args(["-c", "Print :CFBundleIdentifier", &plist_path.to_string_lossy()])
        .output();

    let bundle_id = if let Ok(out) = output {
        if out.status.success() {
            let s = String::from_utf8_lossy(&out.stdout).trim().to_string();
            if s.is_empty() || s.starts_with("Print") { None } else { Some(s) }
        } else { None }
    } else { None };

    let output2 = Command::new("/usr/libexec/PlistBuddy")
        .args(["-c", "Print :CFBundleShortVersionString", &plist_path.to_string_lossy()])
        .output();

    let version = if let Ok(out) = output2 {
        if out.status.success() {
            let s = String::from_utf8_lossy(&out.stdout).trim().to_string();
            if s.is_empty() || s.starts_with("Print") { None } else { Some(s) }
        } else { None }
    } else { None };

    (bundle_id, version, None)
}

/// macOS: 扫描Homebrew应用 — 遍历bin目录和cask列表
#[cfg(target_os = "macos")]
fn scan_macos_homebrew() -> Result<Vec<AppInfo>> {
    use std::process::Command;

    let mut apps = Vec::new();
    let brew_paths = [
        "/opt/homebrew/bin",
        "/usr/local/bin",
    ];

    for brew_dir in &brew_paths {
        let dir = std::path::Path::new(brew_dir);
        if !dir.exists() { continue; }
        if let Ok(entries) = std::fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if let Some(name) = path.file_name() {
                    let name_str = name.to_string_lossy().to_string();
                    apps.push(AppInfo {
                        name: name_str.clone(),
                        executable_path: path.to_string_lossy().to_string(),
                        description: None,
                        publisher: Some("Homebrew".to_string()),
                        version: None,
                        launch_command: Some(path.to_string_lossy().to_string()),
                        app_source: "homebrew".to_string(),
                        keywords: vec!["homebrew".to_string(), "cli".to_string()],
                    });
                }
            }
        }
    }

    let cask_output = Command::new("brew")
        .args(["list", "--cask", "--full-name"])
        .output();

    if let Ok(out) = cask_output {
        if out.status.success() {
            for line in String::from_utf8_lossy(&out.stdout).lines() {
                let name = line.trim().to_string();
                if !name.is_empty() {
                    apps.push(AppInfo {
                        name,
                        executable_path: String::new(),
                        description: None,
                        publisher: Some("Homebrew Cask".to_string()),
                        version: None,
                        launch_command: None,
                        app_source: "homebrew_cask".to_string(),
                        keywords: vec!["homebrew".to_string(), "cask".to_string()],
                    });
                }
            }
        }
    }

    log::info!("[AppIndex:scan_macos_homebrew] Found {} homebrew apps", apps.len());
    Ok(apps)
}

/// macOS: 扫描PATH可执行文件 — 查找具有执行权限的命令
#[cfg(target_os = "macos")]
fn scan_macos_path() -> Vec<AppInfo> {
    let mut apps = Vec::new();
    let path_var = std::env::var("PATH").unwrap_or_default();
    let mut seen = HashSet::new();

    for dir in path_var.split(':') {
        let dir_path = std::path::Path::new(dir);
        if !dir_path.exists() { continue; }
        if let Ok(entries) = std::fs::read_dir(dir_path) {
            for entry in entries.flatten() {
                let path = entry.path();
                if let Some(file_name) = path.file_name() {
                    let name = file_name.to_string_lossy().to_string();
                    let lower = name.to_lowercase();
                    if seen.contains(&lower) { continue; }
                    seen.insert(lower);

                    if path.metadata().map(|m| {
                        let mode = m.permissions().mode();
                        mode & 0o111 != 0 && !name.starts_with('.')
                    }).unwrap_or(false) {
                        apps.push(AppInfo {
                            name,
                            executable_path: path.to_string_lossy().to_string(),
                            description: None,
                            publisher: None,
                            version: None,
                            launch_command: Some(path.to_string_lossy().to_string()),
                            app_source: "path".to_string(),
                            keywords: vec!["cli".to_string(), "command".to_string()],
                        });
                    }
                }
            }
        }
    }

    log::info!("[AppIndex:scan_macos_path] Found {} PATH executables", apps.len());
    apps
}
