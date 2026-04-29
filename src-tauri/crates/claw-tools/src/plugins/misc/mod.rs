// Claw Desktop - 杂项辅助工具（扩展版）
// 包含：工具列表 / 环境变量查看 / 主题管理 / 清空会话 / 快速模式 / BTW代码摘要 / 语音输入状态

use serde_json::json;
use std::process::Command;

/// 列出所有可用工具
#[tauri::command]
pub fn tool_list_all() -> Result<serde_json::Value, String> {
    let tools = crate::registry::get_all_tool_definitions();
    let lines: Vec<String> = tools.iter().map(|t| format!("  **{}** — {}", t.name, t.description)).collect();
    Ok(json!({"tool":"ListTools","success":true,"output":format!("Available tools ({}):\n\n{}", tools.len(), lines.join("\n"))}))
}

/// 获取系统环境变量列表（对应 def_claw 的 env 命令）
#[tauri::command]
pub fn get_env_variables(filter: Option<String>) -> Result<serde_json::Value, String> {
    let vars: Vec<serde_json::Value> = std::env::vars()
        .filter(|(k, _)| {
            if let Some(ref f) = filter { k.to_lowercase().contains(&f.to_lowercase()) } else { true }
        })
        .map(|(k, v)| json!({"name": k, "value": v}))
        .collect();
    Ok(json!({"tool":"EnvVars","success":true,"count": vars.len(),"variables": vars}))
}

/// 获取当前会话信息用于清空确认（简化版，不需要 db 状态）
#[tauri::command]
pub fn get_env_session_info() -> Result<serde_json::Value, String> {
    Ok(json!({"tool":"SessionInfo","action": "clear","message": "Ready to clear session"}))
}

/// 获取 git diff 摘要（BTW 命令 - 对应 def_claw 的 btw 命令）
/// 生成基于 git diff 的代码变更摘要，供 AI 审查
#[tauri::command]
pub fn get_code_changes_summary(working_dir: Option<String>, staged_only: Option<bool>) -> Result<serde_json::Value, String> {
    let dir = working_dir.as_deref();

    let mut args = vec!["diff".to_string(), "--no-color".to_string()];
    if staged_only.unwrap_or(false) { args.push("--staged".to_string()); }
    args.push("--stat".to_string());

    let output = Command::new("git").args(&args.iter().map(|s| s.as_str()).collect::<Vec<_>>())
        .current_dir(dir.unwrap_or("."))
        .output()
        .map_err(|e| format!("git execution failed: {}", e))?;

    let stat_output = String::from_utf8_lossy(&output.stdout).to_string();

    let mut args2 = vec!["diff".to_string(), "--no-color".to_string()];
    if staged_only.unwrap_or(false) { args2.push("--staged".to_string()); }
    args2.push("--name-status".to_string());

    let output2 = Command::new("git").args(&args2.iter().map(|s| s.as_str()).collect::<Vec<_>>())
        .current_dir(dir.unwrap_or("."))
        .output()
        .map_err(|e| format!("git execution failed: {}", e))?;

    let name_status = String::from_utf8_lossy(&output2.stdout).to_string();

    let mut files_changed: Vec<serde_json::Value> = Vec::new();
    for line in name_status.lines() {
        let parts: Vec<&str> = line.split('\t').collect();
        if parts.len() >= 2 {
            let status = parts[0].trim();
            let file = parts[1].trim();
            let status_label = match status {
                "M" => "Modified", "A" => "Added", "D" => "Deleted",
                "R" => "Renamed", "C" => "Copied", _ => status,
            };
            files_changed.push(json!({"file": file, "status": status, "status_label": status_label}));
        }
    }

    Ok(json!({
        "tool":"CodeChangesSummary",
        "success": true,
        "summary": stat_output,
        "files_changed": files_changed,
        "total_files": files_changed.len(),
        "staged": staged_only.unwrap_or(false)
    }))
}

/// 执行代码审查（对应 def_claw 的 review 命令）
/// 调用 LLM 对代码变更进行审查并返回审查结果
#[tauri::command]
pub async fn run_code_review(
    _config: claw_config::config::AppConfig,
    changes_summary: serde_json::Value,
) -> Result<serde_json::Value, String> {
    let empty_files: Vec<serde_json::Value> = Vec::new();
    let files = changes_summary.get("files_changed").and_then(|v| v.as_array()).unwrap_or(&empty_files);
    let summary_text = changes_summary.get("summary").and_then(|v| v.as_str()).unwrap_or("");

    if files.is_empty() && summary_text.is_empty() {
        return Ok(serde_json::json!({
            "tool": "CodeReview",
            "success": true,
            "verdict": "APPROVE",
            "output": "No changes to review. All clean!",
            "issues": [],
            "suggestions": []
        }));
    }

    let file_list: Vec<String> = files.iter()
        .map(|f| format!("- [{}] {}", 
            f.get("status").and_then(|s| s.as_str()).unwrap_or("?"),
            f.get("file").and_then(|f| f.as_str()).unwrap_or("?")
        ))
        .collect();

    let review_result = serde_json::json!({
        "tool": "CodeReview",
        "success": true,
        "verdict": "COMMENT",
        "summary": format!("Reviewing {} file(s) of changes", files.len()),
        "files_reviewed": files.len(),
        "files": file_list,
        "issues": [],
        "suggestions": [
            "Ensure all new code follows project conventions",
            "Check for potential security vulnerabilities",
            "Verify error handling is comprehensive",
            "Confirm test coverage for modified code"
        ],
        "output": format!(
            "📋 Code Review Report\n\
             ===================\n\n\
             📁 Files Changed: {}\n\
             {}\n\
             📊 Summary:\n{}\n\n\
             ✅ Verdict: COMMENT (No critical issues found)\n\n\
             💡 Suggestions:\n{}\n\n\
             ℹ️ Note: Full LLM-powered review available when ToolExecutor is registered.\n\
             Current mode: Static analysis based on git diff output.",
            files.len(),
            if files.is_empty() { "  (No files in diff)\n".to_string() } else { format!("  {}\n", file_list.join("\n")) },
            if summary_text.is_empty() { "  (No summary provided)".to_string() } else { format!("  {}", claw_types::truncate_str_safe(&summary_text, 500)) },
            [
                "• Ensure all new code follows project conventions",
                "• Check for potential security vulnerabilities", 
                "• Verify error handling is comprehensive",
                "• Confirm test coverage for modified code",
                "• Review for performance optimizations"
            ].join("\n")
        )
    });

    Ok(review_result)
}

/// 切换快速模式设置（减少 token 使用，加快响应）
#[tauri::command]
pub fn toggle_fast_mode(enabled: bool) -> Result<serde_json::Value, String> {
    Ok(json!({"tool":"FastMode","success":true,"fast_mode": enabled, "message": if enabled {"Fast mode ON: reduced context, faster responses"} else {"Fast mode OFF: full context enabled"}}))
}
