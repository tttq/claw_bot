// Claw Desktop - 文件搜索工具（Glob / Grep）
//
// Glob: 文件名模式匹配（支持通配符 *?[]）
//   - 基础目录指定（默认当前目录）
//   - 排除模式支持
//   - 结果上限 200 条
//
// Grep: 正则表达式内容搜索
//   - 递归目录遍历（最大深度 10 层）
//   - 文件名 include/exclude 过滤
//   - 大文件跳过 (>10MB)
//   - 结果上限 100 条，含行号
use glob::glob;
use regex::Regex;
use std::path::PathBuf;

/// 文件名模式匹配工具 — 支持通配符*?[]，排除模式和结果上限200条
#[tauri::command]
pub fn tool_glob(
    pattern: String,
    path: Option<String>,
    exclude_patterns: Option<Vec<String>>,
) -> Result<serde_json::Value, String> {
    let base_dir = PathBuf::from(path.unwrap_or_else(|| ".".to_string()));
    let full_pattern = base_dir.join(&pattern).to_string_lossy().to_string();
    let excludes = exclude_patterns.unwrap_or_default();

    let entries: Vec<String> = glob(&full_pattern)
        .map_err(|e| e.to_string())?
        .filter_map(|entry: Result<PathBuf, glob::GlobError>| entry.ok())
        .filter(|entry: &PathBuf| {
            let p = entry.to_string_lossy().to_string();
            !excludes
                .iter()
                .any(|excl: &String| p.contains(excl.as_str()))
        })
        .map(|entry: PathBuf| entry.to_string_lossy().to_string())
        .take(200)
        .collect();

    if entries.is_empty() {
        let out = format!("No match '{}' in {}", pattern, base_dir.display());
        Ok(serde_json::json!({"tool":"Glob","success":true,"output":out}))
    } else {
        let out = format!(
            "Found {} matches for '{}':\n{}",
            entries.len(),
            pattern,
            entries.join("\n")
        );
        Ok(serde_json::json!({"tool":"Glob","success":true,"output":out}))
    }
}

/// 正则表达式内容搜索工具 — 递归遍历目录，支持文件名过滤和结果上限100条
#[tauri::command]
pub fn tool_grep(
    pattern: String,
    path: Option<String>,
    include_pattern: Option<String>,
    exclude_pattern: Option<String>,
) -> Result<serde_json::Value, String> {
    let search_dir = PathBuf::from(path.unwrap_or_else(|| ".".to_string()));
    let re = Regex::new(&pattern).map_err(|e| format!("Invalid regex '{}': {}", pattern, e))?;
    let include_re = include_pattern.map(|p| Regex::new(&p).ok()).flatten();
    let exclude_re = exclude_pattern.map(|p| Regex::new(&p).ok()).flatten();
    let mut results = Vec::new();

    fn find_files(dir: &PathBuf, depth: usize) -> Vec<PathBuf> {
        if depth > 10 {
            return vec![];
        }
        let mut files = Vec::new();
        if let Ok(entries) = std::fs::read_dir(dir) {
            for entry in entries.flatten() {
                let p = entry.path();
                if p.is_file() {
                    files.push(p);
                } else if p.is_dir() {
                    files.extend(find_files(&p, depth + 1));
                }
            }
        }
        files
    }

    for file_path in find_files(&search_dir, 0) {
        let fname = file_path.file_name().unwrap_or_default().to_string_lossy();
        if let Some(ref inc) = include_re {
            if !inc.is_match(&fname) {
                continue;
            }
        }
        if let Some(ref exc) = exclude_re {
            if exc.is_match(&fname) {
                continue;
            }
        }
        if let Ok(meta) = std::fs::metadata(&file_path) {
            if meta.len() > 10 * 1024 * 1024 {
                continue;
            }
        }
        if let Ok(content) = std::fs::read_to_string(&file_path) {
            for (idx, line) in content.lines().enumerate() {
                if re.is_match(line) {
                    results.push(format!(
                        "{}:{}: {}",
                        file_path.display(),
                        idx + 1,
                        line.trim()
                    ));
                    if results.len() >= 100 {
                        break;
                    }
                }
            }
        }
        if results.len() >= 100 {
            break;
        }
    }

    if results.is_empty() {
        let out = format!("No match for '{}'", pattern);
        Ok(serde_json::json!({"tool":"Grep","success":true,"output":out}))
    } else {
        let display_results: String = results[..results.len().min(100)].join("\n");
        let out = format!(
            "Found {} matches '{}':\n{}",
            results.len(),
            pattern,
            display_results
        );
        Ok(serde_json::json!({"tool":"Grep","success":true,"output":out}))
    }
}
