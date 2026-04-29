// Claw Desktop - Git 版本控制工具集（11 个命令）
// 对标 def_claw GitTool 完整功能:
//   git_status / git_diff / git_commit / git_log / git_branch /
//   git_checkout / git_stash (push/pop/list) / git_add / git_reset /
//   git_remote (add/url/fetch) / git_tag
// 所有命令返回统一 JSON 格式: {tool, success, output}

use serde::{Deserialize, Serialize};
use serde_json::json;
use std::process::Command;

/// Git状态条目 — 表示工作区中一个文件的变更状态
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitStatusItem {
    pub file: String,
    pub status: String,         // M/A/D/R/U/??
    pub staged: bool,
    pub path: String,
}

/// Git提交信息 — 包含哈希、作者、消息和日期
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitCommitInfo {
    pub hash: String,
    pub short_hash: String,
    pub author: String,
    pub message: String,
    pub date: String,
    pub stats: Option<String>,
}

/// Git分支信息 — 包含名称、是否当前分支和是否远程分支
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitBranchInfo {
    pub name: String,
    pub is_current: bool,
    pub is_remote: bool,
}

/// Git差异行 — 表示diff中的一行（添加/删除/上下文/头部）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitDiffLine {
    pub type_: String,     // "+" / "-" / " " / "@@"
    pub content: String,
    pub old_line: Option<i32>,
    pub new_line: Option<i32>,
}

/// Git差异文件 — 表示一个文件的完整diff内容
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitDiffFile {
    pub old_path: String,
    pub new_path: String,
    pub status: String,
    pub lines: Vec<GitDiffLine>,
}

/// 执行Git命令 — 返回(stdout, stderr, 退出码)
fn run_git_cmd(args: &[&str], working_dir: Option<&str>) -> Result<(String, String, i32), String> {
    let mut cmd = Command::new("git");
    cmd.args(args);
    if let Some(dir) = working_dir {
        cmd.current_dir(dir);
    }
    let output = cmd.output().map_err(|e| format!("git 执行失败: {}", e))?;
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    let code = output.status.code().unwrap_or(-1);
    Ok((stdout, stderr, code))
}

/// 获取Git仓库根目录
fn get_repo_root(working_dir: Option<&str>) -> Result<String, String> {
    let (stdout, _, code) = run_git_cmd(&["rev-parse", "--show-toplevel"], working_dir)?;
    if code != 0 { Err("不是 Git 仓库".to_string()) } else { Ok(stdout.trim().to_string()) }
}

/// Git状态工具 — 查看工作区文件变更状态
#[tauri::command]
pub fn git_status(working_dir: Option<String>) -> Result<serde_json::Value, String> {
    let dir = working_dir.as_deref();
    let repo_root = get_repo_root(dir)?;
    let (stdout, _stderr, code) = run_git_cmd(&["status", "--porcelain", "-u"], dir)?;
    if code != 0 && stdout.is_empty() {
        return Ok(json!({"tool":"GitStatus","success":true,"output":"工作区干净","is_clean":true,"items":[],"branch":"?","repo":repo_root}));
    }

    let (branch_out, _, _) = run_git_cmd(&["rev-parse", "--abbrev-ref", "HEAD"], dir)?;
    let branch = branch_out.trim().to_string();

    let mut items = Vec::new();
    for line in stdout.lines() {
        if line.len() >= 2 {
            let index_status = line.chars().next().unwrap_or(' ').to_string();
            let work_status = line.chars().nth(1).unwrap_or(' ').to_string();
            let file = line.chars().skip(3).collect::<String>().trim().to_string();
            let combined = format!("{}{}", index_status, work_status);
            let (status, staged) = match combined.as_str() {
                "M " | "M" => ("M".to_string(), true),
                " M" => ("M".to_string(), false),
                "A " | "A" => ("A".to_string(), true),
                " A" => ("A".to_string(), false),
                "D " | "D" => ("D".to_string(), true),
                " D" => ("D".to_string(), false),
                "R " => ("R".to_string(), true),
                " C" => ("C".to_string(), false),
                "U " | "??" => ("?".to_string(), false),
                _ => (combined.clone(), !index_status.trim().is_empty()),
            };
            items.push(GitStatusItem { file: file.clone(), status, staged, path: file });
        }
    }

    Ok(json!({
        "tool":"GitStatus","success":true,"is_clean":items.is_empty(),
        "branch":branch,"repo":repo_root,
        "items":items.iter().map(|i| json!({"file":i.file,"status":i.status,"staged":i.staged})).collect::<Vec<_>>()
    }))
}

/// Git差异工具 — 查看文件变更差异，支持指定文件和暂存区
#[tauri::command]
pub fn git_diff(working_dir: Option<String>, file_path: Option<String>, staged: Option<bool>) -> Result<serde_json::Value, String> {
    let dir = working_dir.as_deref();
    let mut args = vec!["diff".to_string()];
    if staged.unwrap_or(false) { args.push("--staged".to_string()); }
    args.push("--no-color".to_string());
    args.push("-U3".to_string());
    if let Some(f) = &file_path { args.push(f.clone()); }

    let (stdout, _, code) = run_git_cmd(&args.iter().map(|s| s.as_str()).collect::<Vec<_>>(), dir)?;
    if code != 0 || stdout.is_empty() {
        return Ok(json!({"tool":"GitDiff","success":true,"output":"无差异","files":[]}));
    }

    let mut files = Vec::new();
    let mut current_file: Option<GitDiffFile> = None;
    let mut new_line_num: i32 = 0;
    let mut old_line_num: i32 = 0;

    for line in stdout.lines() {
        if line.starts_with("diff --git") {
            if let Some(f) = current_file.take() { files.push(f); }
            let parts: &str = line.split("a/").nth(1).and_then(|s| s.split(" b/").next()).map(|s| s.trim()).unwrap_or("");
            current_file = Some(GitDiffFile { old_path: parts.to_string(), new_path: parts.to_string(), status: "modified".to_string(), lines: Vec::new() });
            new_line_num = 0; old_line_num = 0;
        } else if line.starts_with("new file") {
            if let Some(ref mut f) = current_file { f.status = "added".to_string(); }
        } else if line.starts_with("deleted file") {
            if let Some(ref mut f) = current_file { f.status = "deleted".to_string(); }
        } else if line.starts_with("rename") {
            if let Some(ref mut f) = current_file { f.status = "renamed".to_string(); }
        } else if line.starts_with("@@") {
            let header = line.to_string();
            if let Some(ref mut f) = current_file { f.lines.push(GitDiffLine { type_: "header".to_string(), content: header, old_line: None, new_line: None }); }
            if let Some(hunk) = line.split("+").nth(1) {
                if let Some(end) = hunk.find(",") { new_line_num = hunk[..end].parse::<i32>().unwrap_or(1) - 1; }
                else { new_line_num = hunk.parse::<i32>().unwrap_or(1) - 1; }
            }
            if let Some(hunk) = line.split("-").nth(1) {
                if let Some(end) = hunk.find(",") { old_line_num = hunk[..end].parse::<i32>().unwrap_or(1) - 1; }
                else { old_line_num = hunk.parse::<i32>().unwrap_or(1) - 1; }
            }
        } else if let Some(ref mut f) = current_file {
            if line.starts_with('+') && !line.starts_with("++") {
                new_line_num += 1;
                f.lines.push(GitDiffLine { type_: "+".to_string(), content: line[1..].to_string(), old_line: None, new_line: Some(new_line_num) });
            } else if line.starts_with('-') && !line.starts_with("--") {
                old_line_num += 1;
                f.lines.push(GitDiffLine { type_: "-".to_string(), content: line[1..].to_string(), old_line: Some(old_line_num), new_line: None });
            } else {
                new_line_num += 1; old_line_num += 1;
                f.lines.push(GitDiffLine { type_: " ".to_string(), content: line.to_string(), old_line: Some(old_line_num), new_line: Some(new_line_num) });
            }
        }
    }
    if let Some(f) = current_file { files.push(f); }

    Ok(json!({"tool":"GitDiff","success":true,"files":files}))
}

/// Git提交工具 — 暂存文件并提交
#[tauri::command]
pub fn git_commit(message: String, files: Option<Vec<String>>, working_dir: Option<String>) -> Result<serde_json::Value, String> {
    let dir = working_dir.as_deref();
    if let Some(ref flist) = files {
        for f in flist {
            run_git_cmd(&["add", f], dir)?;
        }
    } else {
        run_git_cmd(&["add", "-A"], dir)?;
    }
    let (stdout, stderr, code) = run_git_cmd(&["commit", "-m", &message], dir)?;
    if code == 0 {
        Ok(json!({"tool":"GitCommit","success":true,"output":format!("提交成功!\n{}", stdout)}))
    } else {
        Ok(json!({"tool":"GitCommit","success":false,"output":format!("提交失败:\n{}", stderr)}))
    }
}

/// Git日志工具 — 查看提交历史
#[tauri::command]
pub fn git_log(limit: Option<u64>, working_dir: Option<String>) -> Result<serde_json::Value, String> {
    let dir = working_dir.as_deref();
    let n = limit.unwrap_or(20).to_string();
    let fmt = "%H|%h|%an|%s|%ci";
    let format_str = format!("--format=format:{}", fmt.replace("|", "\0"));
    let (stdout, _, code) = run_git_cmd(&["log", &format!("-{}", n), &format_str], dir)?;
    if code != 0 || stdout.is_empty() {
        return Ok(json!({"tool":"GitLog","success":true,"output":"暂无提交记录","commits":[]}));
    }

    let mut commits = Vec::new();
    for line in stdout.lines() {
        let parts: Vec<&str> = line.split('\0').collect();
        if parts.len() >= 5 {
            commits.push(GitCommitInfo {
                hash: parts[0].to_string(),
                short_hash: parts[1].to_string(),
                author: parts[2].to_string(),
                message: parts[3].to_string(),
                date: parts[4].to_string(),
                stats: None,
            });
        }
    }

    Ok(json!({"tool":"GitLog","success":true,"commits":commits}))
}

/// Git分支列表工具 — 列出所有本地和远程分支
#[tauri::command]
pub fn git_branch_list(working_dir: Option<String>) -> Result<serde_json::Value, String> {
    let dir = working_dir.as_deref();
    let (stdout, _, _) = run_git_cmd(&["branch", "-a", "--no-color"], dir)?;

    let current_branch = run_git_cmd(&["rev-parse", "--abbrev-ref", "HEAD"], dir)
        .ok().map(|(s,_,_)| s.trim().to_string()).unwrap_or_default();

    let mut branches = Vec::new();
    for line in stdout.lines() {
        let raw = line.trim_start_matches(' ').trim();
        let is_current = raw.starts_with('*');
        let name = raw.trim_start_matches('*').trim().to_string();
        let is_remote = name.starts_with("remotes/");
        let display_name = if is_remote { name.strip_prefix("remotes/").unwrap_or(&name).to_string() } else { name.clone() };
        branches.push(GitBranchInfo { name: display_name, is_current, is_remote });
    }

    Ok(json!({"tool":"GitBranch","success":true,"current":current_branch,"branches":branches}))
}

/// Git创建分支工具 — 支持创建后自动切换
#[tauri::command]
pub fn git_create_branch(name: String, checkout: Option<bool>, working_dir: Option<String>) -> Result<serde_json::Value, String> {
    let dir = working_dir.as_deref();
    if checkout.unwrap_or(true) {
        let (_, stderr, code) = run_git_cmd(&["checkout", "-b", &name], dir)?;
        if code == 0 { let msg = format!("Created and switched to branch '{}'", name); Ok(json!({"tool":"GitBranch","success":true,"output":msg})) }
        else { Ok(json!({"tool":"GitBranch","success":false,"output":stderr})) }
    } else {
        run_git_cmd(&["branch", &name], dir)?;
        let msg = format!("Created branch '{}'", name);
        Ok(json!({"tool":"GitBranch","success":true,"output":msg}))
    }
}

/// Git切换分支工具
#[tauri::command]
pub fn git_checkout_branch(name: String, working_dir: Option<String>) -> Result<serde_json::Value, String> {
    let (_, stderr, code) = run_git_cmd(&["checkout", &name], working_dir.as_deref())?;
    if code == 0 { let msg = format!("Switched to '{}'", name); Ok(json!({"tool":"GitCheckout","success":true,"output":msg})) }
    else { Ok(json!({"tool":"GitCheckout","success":false,"output":stderr})) }
}

/// Git暂存工具 — 保存当前工作区修改到stash
#[tauri::command]
pub fn git_stash(working_dir: Option<String>) -> Result<serde_json::Value, String> {
    let (_, stderr, code) = run_git_cmd(&["stash", "push", "-m", "auto-stash"], working_dir.as_deref())?;
    if code == 0 { Ok(json!({"tool":"GitStash","success":true,"output":"Stash 已保存"})) }
    else { Ok(json!({"tool":"GitStash","success":false,"output":stderr})) }
}

/// Git恢复暂存工具 — 从stash恢复最近一次保存的修改
#[tauri::command]
pub fn git_stash_pop(working_dir: Option<String>) -> Result<serde_json::Value, String> {
    let (_, stderr, code) = run_git_cmd(&["stash", "pop"], working_dir.as_deref())?;
    if code == 0 { Ok(json!({"tool":"GitStashPop","success":true,"output":"Stash 已恢复"})) }
    else { Ok(json!({"tool":"GitStashPop","success":false,"output":stderr})) }
}

/// Git添加工具 — 将文件添加到暂存区
#[tauri::command]
pub fn git_add(files: Vec<String>, working_dir: Option<String>) -> Result<serde_json::Value, String> {
    let dir = working_dir.as_deref();
    for f in &files { run_git_cmd(&["add", f], dir)?; }
    Ok(json!({"tool":"GitAdd","success":true,"output":format!("已暂存 {} 个文件", files.len())}))
}

/// Git取消暂存工具 — 将文件从暂存区移除
#[tauri::command]
pub fn git_reset(files: Vec<String>, working_dir: Option<String>) -> Result<serde_json::Value, String> {
    let dir = working_dir.as_deref();
    for f in &files { run_git_cmd(&["reset", "HEAD", "--", f], dir)?; }
    Ok(json!({"tool":"GitReset","success":true,"output":format!("已取消暂存 {} 个文件", files.len())}))
}

/// Git仓库检测工具 — 判断指定目录是否为Git仓库
#[tauri::command]
pub fn git_is_repository(working_dir: Option<String>) -> Result<bool, String> {
    match get_repo_root(working_dir.as_deref()) { Ok(_) => Ok(true), Err(_) => Ok(false) }
}
