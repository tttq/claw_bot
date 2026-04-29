// Claw Desktop - 扩展管理器 - 管理外部扩展的安装和加载
// 对标 def_claw utils/plugins/pluginLoader.ts 的简化版
// 支持从 extensions/ 目录发现、加载、管理扩展

use std::fs;
use std::path::{Path, PathBuf};
use serde::{Deserialize, Serialize};
use serde_json::json;

/// 扩展清单 — 描述扩展的元数据信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtensionManifest {
    pub name: String,
    #[serde(default)]
    pub version: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub author: Option<String>,
    #[serde(default)]
    pub tools: Option<Vec<ExtensionToolDef>>,
    #[serde(default)]
    pub skills: Option<Vec<String>>,
    #[serde(default)]
    pub enabled: Option<bool>,
}

/// 扩展工具定义 — 描述扩展提供的工具
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtensionToolDef {
    pub name: String,
    pub description: String,
    #[serde(default)]
    pub input_schema: Option<serde_json::Value>,
    #[serde(default)]
    pub handler: Option<String>,
}

/// 已加载的扩展 — 包含清单、路径和加载状态
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoadedExtension {
    pub manifest: ExtensionManifest,
    pub path: String,
    pub enabled: bool,
    pub loaded_at: String,
}

/// 从目录加载扩展清单 — 读取manifest.json并解析
fn load_manifest(dir: &Path) -> Option<ExtensionManifest> {
    let manifest_path = dir.join("manifest.json");
    if !manifest_path.exists() { return None; }
    
    let content = fs::read_to_string(&manifest_path).ok()?;
    let manifest: ExtensionManifest = serde_json::from_str(&content).ok()?;
    
    Some(manifest)
}

/// 加载单个扩展 — 解析清单并注册扩展提供的工具到工具注册表
pub async fn load_extension(dir: &Path) -> Option<LoadedExtension> {
    if !dir.is_dir() { return None; }
    
    let manifest = load_manifest(dir)?;
    let name = manifest.name.clone();
    
    log::info!("[ExtensionManager] 发现扩展: {} v{} ({})", 
        name, manifest.version.as_deref().unwrap_or("?"), dir.display());
    
    if let Some(ref tools) = manifest.tools {
        use crate::tool_registry::{register_tool, ToolSource};
        use claw_types::common::ToolDefinition;
        for tool_def in tools {
            let def = ToolDefinition {
                name: tool_def.name.clone(),
                description: tool_def.description.clone(),
                input_schema: tool_def.input_schema.clone()
                    .unwrap_or_else(|| json!({"type":"object","properties":{}})),
                category: None,
                tags: Vec::new(),
            };
            register_tool(def, ToolSource::Extension, tool_def.handler.clone()).await;
        }
    }
    
    Some(LoadedExtension {
        path: dir.to_string_lossy().to_string(),
        enabled: manifest.enabled.unwrap_or(true),
        loaded_at: chrono::Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string(),
        manifest,
    })
}

/// 扫描扩展目录 — 遍历目录下所有子目录并加载扩展
pub async fn scan_extensions_dir(dir: &Path) -> Vec<LoadedExtension> {
    let mut extensions = Vec::new();
    
    if !dir.exists() || !dir.is_dir() { return extensions; }
    
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let ft = match entry.file_type() { Ok(ft) => ft, Err(_) => continue };
            if ft.is_dir() || ft.is_symlink() {
                if let Some(ext) = load_extension(&entry.path()).await {
                    extensions.push(ext);
                }
            }
        }
    }
    
    extensions
}

/// 获取默认扩展目录路径 — 优先使用应用数据目录，回退到用户主目录
pub fn default_extensions_dir() -> Option<PathBuf> {
    let app_ext = claw_config::path_resolver::extensions_dir();
    if app_ext.exists() || cfg!(debug_assertions) { return Some(app_ext); }
    dirs::home_dir().map(|h| h.join(".claw-desktop").join("extensions"))
}

/// 发现并加载所有扩展 — 从默认扩展目录扫描
pub async fn discover_and_load_extensions() -> Vec<LoadedExtension> {
    if let Some(dir) = default_extensions_dir() {
        scan_extensions_dir(&dir).await
    } else {
        Vec::new()
    }
}

/// Tauri命令：扫描并列出所有已安装的扩展
#[tauri::command]
pub async fn cmd_scan_extensions() -> Result<serde_json::Value, String> {
    let extensions = discover_and_load_extensions().await;
    
    Ok(json!({
        "total": extensions.len(),
        "extensions": extensions.iter().map(|e| json!({
            "name": e.manifest.name,
            "version": e.manifest.version,
            "description": e.manifest.description,
            "author": e.manifest.author,
            "enabled": e.enabled,
            "tools_count": e.manifest.tools.as_ref().map(|t| t.len()).unwrap_or(0),
            "skills_count": e.manifest.skills.as_ref().map(|s| s.len()).unwrap_or(0),
            "path": e.path,
        })).collect::<Vec<_>>()
    }))
}

/// Tauri命令：安装扩展 — 从URL创建扩展目录和占位清单
#[tauri::command]
pub fn cmd_install_extension(url: String, name: Option<String>) -> Result<serde_json::Value, String> {
    let ext_name = name.unwrap_or_else(|| url.split('/').last().unwrap_or("unknown").to_string());
    let ext_dir = default_extensions_dir()
        .ok_or("Cannot determine user directory".to_string())?
        .join(&ext_name);

    if ext_dir.exists() {
        return Err(format!("Extension '{}' already exists", ext_name));
    }

    fs::create_dir_all(&ext_dir).map_err(|e| format!("Failed to create directory: {}", e))?;

    let manifest = json!({
        "name": ext_name,
        "version": "1.0.0",
        "description": format!("Extension installed from {}", url),
        "enabled": true
    });

    let manifest_str = serde_json::to_string_pretty(&manifest).map_err(|e| format!("Serialization failed: {}", e))?;
    fs::write(ext_dir.join("manifest.json"), manifest_str)
        .map_err(|e| format!("Failed to write manifest: {}", e))?;

    Ok(json!({
        "success": true,
        "name": ext_name,
        "path": ext_dir.to_string_lossy(),
        "message": format!("Extension '{}' installed (placeholder). Please add tool definitions to manifest.json manually.", ext_name)
    }))
}

/// Tauri命令：卸载扩展 — 删除扩展目录及所有文件
#[tauri::command]
pub fn cmd_uninstall_extension(name: String) -> Result<serde_json::Value, String> {
    let ext_dir = default_extensions_dir()
        .ok_or("Cannot determine user directory".to_string())?
        .join(&name);

    if !ext_dir.exists() {
        return Err(format!("Extension '{}' not found", name));
    }

    fs::remove_dir_all(&ext_dir).map_err(|e| format!("Failed to remove: {}", e))?;

    Ok(json!({"success": true, "message": format!("Extension '{}' uninstalled", name)}))
}
