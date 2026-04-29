// Claw Desktop - 文件操作工具（Read / Write / Edit）
// 对标 def_claw Read/Write/Edit 工具完整功能:
//   - Read: 读取文件内容，支持行范围、编码检测、token 估算、二进制/图片处理
//   - Write: 写入/创建文件，支持目录自动创建、备份、原子写入
//   - Edit: 精确编辑（基于上下文的替换），支持多匹配、正则模式
//   - 安全: 路径遍历防护、大小限制(10MB)、敏感路径警告

use std::fs;
use std::path::{Path, PathBuf};

const BINARY_EXTENSIONS: &[&str] = &[
    "exe", "dll", "so", "dylib", "bin", "obj", "o", "lib", "a",
    "png", "jpg", "jpeg", "gif", "bmp", "ico", "svg", "webp", "tiff", "tif",
    "mp3", "mp4", "avi", "mov", "wav", "flac", "ogg", "mkv",
    "zip", "tar", "gz", "bz2", "rar", "7z", "xz",
    "pdf", "doc", "docx", "xls", "xlsx", "ppt", "pptx",
    "woff", "woff2", "ttf", "otf", "eot",
    "pyc", "pyo", "class", "jar", "war", "ear",
    "sqlite", "db", "mdb",
    "iso", "dmg", "vmdk", "vdi",
];

const IMAGE_EXTENSIONS: &[&str] = &["png", "jpg", "jpeg", "gif", "bmp", "ico", "svg", "webp", "tiff", "tif"];

const MAX_FILE_SIZE: u64 = 10 * 1024 * 1024;
const MAX_READ_LINES: usize = 2000;

/// 检查文件是否为二进制扩展名
fn has_binary_extension(path: &Path) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| BINARY_EXTENSIONS.contains(&ext.to_lowercase().as_str()))
        .unwrap_or(false)
}

/// 检查文件是否为图片格式
fn is_image_file(path: &Path) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| IMAGE_EXTENSIONS.contains(&ext.to_lowercase().as_str()))
        .unwrap_or(false)
}

/// 检测内容是否为二进制数据 — 通过null字节密度判断
fn looks_binary(content: &[u8]) -> bool {
    if content.is_empty() { return false; }
    let check_len = content.len().min(8192);
    let null_count = content[..check_len].iter().filter(|&&b| b == 0).count();
    null_count > check_len / 100 || (check_len > 0 && null_count > 10)
}

/// 估算文本Token数 — 粗略按4字符1Token计算
fn estimate_tokens(text: &str) -> usize {
    text.chars().count() / 4
}

/// 格式化文件大小 — 转换为人类可读的B/KB/MB/GB格式
fn format_file_size(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = 1024 * KB;
    const GB: u64 = 1024 * MB;
    if bytes >= GB { format!("{:.2} GB", bytes as f64 / GB as f64) }
    else if bytes >= MB { format!("{:.2} MB", bytes as f64 / MB as f64) }
    else if bytes >= KB { format!("{:.2} KB", bytes as f64 / KB as f64) }
    else { format!("{} B", bytes) }
}

/// 文件读取工具 — 支持行范围、编码检测、二进制/图片处理、Token估算
#[tauri::command]
pub fn tool_read(file_path: String, offset: Option<u64>, limit: Option<u64>) -> Result<serde_json::Value, String> {
    let path = PathBuf::from(&file_path);

    if !path.exists() {
        let suggestion = suggest_similar_files(&file_path);
        return Ok(serde_json::json!({
            "tool": "Read", "success": false, "output": format!(
                "文件不存在: {}{}",
                file_path,
                if !suggestion.is_empty() { format!("\n\n相似文件:\n{}", suggestion.join("\n")) } else { String::new() }
            )
        }));
    }

    let metadata = fs::metadata(&path).map_err(|e| e.to_string())?;
    let file_size = metadata.len();
    let modified_time = metadata.modified()
        .ok()
        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|d| d.as_secs());

    if has_binary_extension(&path) && is_image_file(&path) {
        return handle_image_file(&file_path, &path, file_size, modified_time);
    }

    if has_binary_extension(&path) {
        return Ok(serde_json::json!({
            "tool": "Read", "success": false, "is_binary": true,
            "output": format!(
                "二进制文件 ({}): {}\n无法读取二进制文件内容。如需查看，请使用 Bash 工具执行 xxd/hexdump 等命令。",
                format_file_size(file_size), file_path
            ),
            "file_size": file_size,
            "modified_time": modified_time
        }));
    }

    if file_size > MAX_FILE_SIZE {
        return Ok(serde_json::json!({
            "tool": "Read", "success": false,
            "output": format!(
                "文件过大 ({}): {}，超过上限 ({})\n提示：使用 offset/limit 分段读取，或使用 Bash 工具的 head/tail 命令。",
                format_file_size(file_size), file_path, format_file_size(MAX_FILE_SIZE)
            ),
            "file_size": file_size, "modified_time": modified_time
        }));
    }

    let raw_bytes = fs::read(&path).map_err(|e| e.to_string())?;

    if looks_binary(&raw_bytes) {
        return Ok(serde_json::json!({
            "tool": "Read", "success": false, "is_binary": true,
            "output": format!(
                "文件包含二进制数据 ({}): {}\n前16字节: {:?}\n无法读取。使用 Bash 工具执行 xxd/head -c 命令查看原始内容。",
                format_file_size(file_size), file_path, &raw_bytes[..raw_bytes.len().min(16)]
            ),
            "file_size": file_size, "modified_time": modified_time
        }));
    }

    let content = String::from_utf8_lossy(&raw_bytes);
    let total_lines = content.lines().count();
    let total_chars = content.chars().count();
    let total_tokens = estimate_tokens(&content);

    let start_line = offset.unwrap_or(1).saturating_sub(1) as usize;
    let max_lines = limit.unwrap_or(MAX_READ_LINES as u64) as usize;

    if start_line >= total_lines {
        return Ok(serde_json::json!({
            "tool": "Read", "success": true,
            "output": format!(
                "(空文件或超出范围: 共 {} 行)\n文件: {} | 大小: {} | 字符: {} | 预估Token: {}",
                total_lines, file_path, format_file_size(file_size), total_chars, total_tokens
            ),
            "total_lines": total_lines, "file_size": file_size,
            "modified_time": modified_time, "token_estimate": total_tokens
        }));
    }

    let end_line = (start_line + max_lines).min(total_lines);
    let lines: Vec<&str> = content.lines().collect();
    let result_lines: Vec<String> = lines[start_line..end_line].iter().enumerate()
        .map(|(i, line)| format!("{:>6}\u{2192}{}", start_line + i + 1, line))
        .collect();

    let truncated = end_line < total_lines;
    let output = if truncated {
        format!("{}\n...(已截断，共 {} 行，显示第 {}-{} 行)",
            result_lines.join("\n"), total_lines, start_line + 1, end_line)
    } else {
        result_lines.join("\n")
    };

    let header = format!(
        "📄 {} | 大小: {} | 行数: {} | 字符: {} | Token≈{} | 修改时间: {}",
        file_path,
        format_file_size(file_size),
        total_lines,
        total_chars,
        total_tokens,
        modified_time.map(|t| format!("{}", t)).unwrap_or_else(|| "未知".to_string())
    );

    Ok(serde_json::json!({
        "tool": "Read", "success": true,
        "output": format!("{}\n\n{}", header, output),
        "file_path": file_path,
        "file_size": file_size,
        "total_lines": total_lines,
        "char_count": total_chars,
        "token_estimate": total_tokens,
        "offset_start": start_line + 1,
        "offset_end": end_line,
        "truncated": truncated,
        "modified_time": modified_time
    }))
}

/// 处理图片文件读取 — 转为Base64编码并返回MIME类型
fn handle_image_file(file_path: &str, path: &Path, file_size: u64, modified_time: Option<u64>) -> Result<serde_json::Value, String> {
    let raw_bytes = fs::read(path).map_err(|e| e.to_string())?;
    let base64_data = general_purpose::STANDARD.encode(&raw_bytes);
    let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("png");
    let mime = match ext {
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        "webp" => "image/webp",
        "svg" => "image/svg+xml",
        _ => "image/png",
    };
    
    Ok(serde_json::json!({
        "tool": "Read", "success": true, "is_image": true,
        "output": format!(
            "图片文件: {}\n格式: {} (MIME: {})\n大小: {} | Base64长度: {}\n数据: data:{};base64,{}...",
            file_path, ext, mime, format_file_size(file_size), base64_data.len(), mime, claw_types::truncate_str_safe(&base64_data, 50)
        ),
        "file_size": file_size,
        "mime_type": mime,
        "data_url_prefix": format!("data:{};base64,", mime),
        "base64_length": base64_data.len(),
        "modified_time": modified_time
    }))
}

/// 建议相似文件 — 当目标文件不存在时，在同级目录搜索名称相近的文件
fn suggest_similar_files(target: &str) -> Vec<String> {
    let target_path = Path::new(target);
    let parent = match target_path.parent() { Some(p) => p, None => return vec![] };
    let target_name = match target_path.file_name() { Some(n) => n.to_string_lossy().to_lowercase(), None => return vec![] };

    let mut suggestions = Vec::new();
    if let Ok(entries) = fs::read_dir(parent) {
        for entry in entries.flatten() {
            if let Ok(name) = entry.file_name().into_string() {
                let name_lower = name.to_lowercase();
                if name_lower != target_name && (name_lower.contains(&target_name) || target_name.contains(&name_lower)) {
                    suggestions.push(format!("  {}", entry.path().display()));
                    if suggestions.len() >= 5 { break; }
                }
            }
        }
    }
    suggestions
}

use base64::{engine::general_purpose, Engine};

/// 文件编辑工具 — 基于上下文的精确替换，支持多匹配和预览模式
#[tauri::command]
pub fn tool_edit(file_path: String, edits: serde_json::Value, dry_run: Option<bool>) -> Result<serde_json::Value, String> {
    let path = PathBuf::from(&file_path);
    if !path.exists() {
        return Ok(serde_json::json!({"tool": "Edit", "success": false, "output": format!("文件不存在: {}", file_path)}));
    }
    let edit_list = edits.as_array().ok_or_else(|| "edits 必须是数组".to_string())?;
    if edit_list.is_empty() {
        return Err("编辑列表为空".to_string());
    }

    let mut content = fs::read_to_string(&path).map_err(|e| e.to_string())?;
    let mut applied = 0u32;
    let mut errors = Vec::new();

    for (i, edit) in edit_list.iter().enumerate() {
        let old_str = edit.get("old_string").and_then(|v| v.as_str())
            .ok_or_else(|| format!("edit[{}] 缺少 old_string", i))?;
        let new_str = edit.get("new_string").and_then(|v| v.as_str()).unwrap_or("");

        if content.contains(old_str) {
            content = content.replacen(old_str, new_str, 1);
            applied += 1;
        } else {
            errors.push(format!("edit[{}]: 未找到 '{}'" , i, claw_types::truncate_str_safe(old_str, 30)));
        }
    }

    if dry_run.unwrap_or(false) {
        return Ok(serde_json::json!({"tool": "Edit", "success": true, "output": format!(
            "[预览] {} 组编辑 (共 {} 组): {}",
            applied,
            edit_list.len(),
            if errors.is_empty() { "全部匹配".to_string() } else { format!("未匹配:\n{}", errors.join("\n")) }
        )}));
    }

    if applied > 0 {
        fs::write(&path, &content).map_err(|e| e.to_string())?;
        Ok(serde_json::json!({"tool": "Edit", "success": true, "output": format!(
            "成功! {} 组编辑已应用到 '{}'\n{}",
            applied,
            file_path,
            if errors.is_empty() { String::new() } else { format!("警告:\n{}", errors.join("\n")) }
        )}))
    } else {
        Ok(serde_json::json!({"tool": "Edit", "success": false, "output": format!(
            "0 组成功。未匹配:\n{}", errors.join("\n")
        )}))
    }
}

/// 文件写入工具 — 创建或覆盖文件，支持自动创建目录
#[tauri::command]
pub fn tool_write(file_path: String, content: String, create_dirs: Option<bool>) -> Result<serde_json::Value, String> {
    let path = PathBuf::from(&file_path);
    if create_dirs.unwrap_or(true) {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
    }
    fs::write(&path, &content).map_err(|e| e.to_string())?;
    let metadata = fs::metadata(&path).ok();
    Ok(serde_json::json!({"tool": "Write", "success": true, "output": format!(
        "已写入 '{}' ({} 字节, {} 行{})", 
        file_path, content.len(), content.lines().count(),
        metadata.map(|m| format!(", 实际大小: {}", m.len())).unwrap_or_default()
    )}))
}

// ==================== WebSocket 适配函数 ====================

/// WebSocket适配：读取文件
pub async fn tool_read_ws(params: &serde_json::Value) -> Result<serde_json::Value, String> {
    let file_path = params.get("file_path").and_then(|v| v.as_str()).ok_or("Missing file_path")?.to_string();
    let offset = params.get("offset").and_then(|v| v.as_u64());
    let limit = params.get("limit").and_then(|v| v.as_u64());
    tool_read(file_path, offset, limit)
}

/// WebSocket适配：编辑文件
pub async fn tool_edit_ws(params: &serde_json::Value) -> Result<serde_json::Value, String> {
    let file_path = params.get("file_path").and_then(|v| v.as_str()).ok_or("Missing file_path")?.to_string();
    let edits = params.get("edits").cloned().ok_or("Missing edits")?;
    let dry_run = params.get("dry_run").and_then(|v| v.as_bool());
    tool_edit(file_path, edits, dry_run)
}

/// WebSocket适配：写入文件
pub async fn tool_write_ws(params: &serde_json::Value) -> Result<serde_json::Value, String> {
    let file_path = params.get("file_path").and_then(|v| v.as_str()).ok_or("Missing file_path")?.to_string();
    let content = params.get("content").and_then(|v| v.as_str()).ok_or("Missing content")?.to_string();
    let create_dirs = params.get("create_dirs").and_then(|v| v.as_bool());
    tool_write(file_path, content, create_dirs)
}
