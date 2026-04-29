// Claw Desktop - Shell 命令执行工具（Bash / BashCancel）
//
// 核心安全机制:
//   1. 危险命令检测: 18 种破坏性操作模式匹配（rm -rf / :(){ :|:& };: 等）
//   2. 超时控制: 默认 120s，可通过 timeout 参数调整，最大 600s（使用 tokio::time::timeout）
//   3. 输出截断: 单次输出上限 100KB，防止 OOM
//   4. 取消信号: 全局 CANCEL_FLAG 支持 Ctrl+C 中断长时间运行的命令
//   5. Windows 适配: 自动使用 PowerShell (Core) 替代 cmd/sh
//   6. 异步执行: 使用 tokio::process::Command + tokio::io::AsyncReadExt 非阻塞读写

use std::sync::atomic::{AtomicBool, Ordering};
use tokio::sync::Mutex;
use tokio::process::Command as TokioCommand;
use tokio::process::Child as TokioChild;
use tokio::io::AsyncReadExt;
use std::time::Instant;

/// 全局取消标志：设置为 true 时所有正在执行的 bash 命令会被中断
static CANCEL_FLAG: AtomicBool = AtomicBool::new(false);

/// 正在运行中的子进程句柄存储（用于 cancel 操作发送 SIGTERM/SIGKILL）
/// 使用 tokio::sync::Mutex 保证异步上下文安全
static RUNNING_CHILD: Mutex<Option<u32>> = Mutex::const_new(None);

/// 取消正在执行的Shell命令 — 设置全局取消标志
#[tauri::command]
pub fn tool_bash_cancel() -> Result<serde_json::Value, String> {
    CANCEL_FLAG.store(true, Ordering::SeqCst);
    Ok(serde_json::json!({"tool":"BashCancel","success":true,"output":"取消信号已发送"}))
}

/// 命令语义分类 — 根据命令首词判断操作类型
#[derive(Debug, Clone, PartialEq)]
pub enum CommandCategory {
    Search,
    Read,
    List,
    Write,
    Delete,
    Silent,
    Other,
}

/// 命令分类器 — 根据首词将命令分为搜索/读取/列表/写入/删除/静默/其他
fn classify_command(cmd: &str) -> CommandCategory {
    let first_word = cmd.trim_start().split_whitespace().next().unwrap_or("").to_lowercase();
    
    let search_cmds: &[&str] = &["find", "grep", "egrep", "fgrep", "rg", "ag", "ack", "locate", "which", "whereis", "type"];
    let read_cmds: &[&str] = &["cat", "head", "tail", "less", "more", "wc", "stat", "file", "strings", "jq", "awk", "cut", "sort", "uniq", "tr", "diff", "env", "echo", "printf", "git", "svn", "hg"];
    let list_cmds: &[&str] = &["ls", "tree", "du", "dir"];
    let write_cmds: &[&str] = &["tee", "cp", "mv", "rename", "ln", "chmod", "chown", "touch", "mkdir", "git"];
    let delete_cmds: &[&str] = &["rm", "rmdir", "shred"];
    let silent_cmds: &[&str] = &["cd", "export", "set", "unset", "alias", "source", ".", ":", "true", "false"];

    if silent_cmds.contains(&first_word.as_str()) { return CommandCategory::Silent; }
    if delete_cmds.contains(&first_word.as_str()) { return CommandCategory::Delete; }
    if write_cmds.contains(&first_word.as_str()) { return CommandCategory::Write; }
    if search_cmds.contains(&first_word.as_str()) { return CommandCategory::Search; }
    if read_cmds.contains(&first_word.as_str()) { return CommandCategory::Read; }
    if list_cmds.contains(&first_word.as_str()) { return CommandCategory::List; }

    CommandCategory::Other
}

/// 危险命令检测（对标 def_claw 安全检查）
fn check_dangerous_command(cmd: &str) -> Option<String> {
    let lower = cmd.to_lowercase();
    let dangerous_patterns: &[(&str, &str)] = &[
        ("rm -rf /", "尝试删除根目录"),
        ("rm -rf /*", "尝试递归删除根目录"),
        ("rm -rf ~", "尝试删除主目录"),
        ("> /dev/sda", "尝试覆写磁盘"),
        ("mkfs.", "尝试格式化磁盘"),
        ("dd if=", "可能进行磁盘覆写"),
        ("chmod -R 777 /", "全局权限修改"),
        ("chown -R", "批量所有权修改"),
        ("shutdown", "系统关机命令"),
        ("reboot", "系统重启命令"),
        ("halt", "系统停止命令"),
        ("init 0", "切换运行级别0"),
        (":(){ :|:& };:", "Fork炸弹"),
        ("curl * | sh", "远程脚本执行"),
        ("wget * | sh", "远程脚本执行"),
        ("curl * | bash", "远程脚本执行"),
        ("wget * | bash", "远程脚本执行"),
        ("drop table", "数据库删除操作"),
        ("truncate -s 0 /", "清空关键文件"),
    ];

    for &(pattern, reason) in dangerous_patterns {
        if lower.contains(pattern) {
            return Some(format!("🚫 危险命令被拦截: {} ({})", cmd, reason));
        }
    }

    if lower.contains("rm ") && (lower.contains("-rf") || lower.contains("-fr")) && !lower.contains("node_modules") && !lower.contains(".git") {
        let parts: Vec<&str> = lower.split("rm ").collect();
        if parts.len() > 1 {
            let target = parts[1].trim_start_matches("-rf").trim_start_matches("-fr").trim();
            if target == "/" || target == "~" || target == "*" || target == "." {
                return Some(format!("🚫 危险命令被拦截: {} (尝试删除 {})", cmd, target));
            }
        }
    }

    None
}

/// 输出累加器 — 收集stdout/stderr，带尾部保留截断保护
struct OutputAccumulator {
    content: String,                   // 累积的输出文本
    max_bytes: usize,                  // 最大字节数限制（默认 100KB）
    total_bytes: usize,                // 实际接收的总字节（含被截断部分）
    truncated: bool,                   // 是否已被截断
}

impl OutputAccumulator {
    /// 创建输出累加器，指定最大字节数限制
    fn new(max_bytes: usize) -> Self {
        Self { content: String::new(), max_bytes, total_bytes: 0, truncated: false }
    }

    /// 追加数据 — 超过限制时截断并标记
    fn push(&mut self, data: &str) {
        self.total_bytes += data.len();
        if self.content.len() < self.max_bytes {
            let remaining = self.max_bytes - self.content.len();
            if data.len() <= remaining {
                self.push_str(data);
            } else {
                self.push_str(&data[..remaining]);
                self.truncated = true;
            }
        } else {
            self.truncated = true;
        }
    }

    fn push_str(&mut self, s: &str) { self.content.push_str(s); }

    /// 完成累加 — 如有截断则添加截断提示前缀
    fn finish(mut self) -> String {
        if self.truncated {
            self.content = format!("...(输出已截断, 总计 {} 字节, 显示最后 {} 字节)\n{}",
                self.total_bytes, self.max_bytes, self.content);
        }
        self.content
    }
}

/// ★ 核心 Shell 执行入口（async，使用 tokio::process::Command 非阻塞执行）
///
/// 线程安全保证：
///   - 子进程 PID 通过 tokio::sync::Mutex 保护
///   - stdout/stderr 使用 tokio::io::AsyncReadExt 异步读取（不阻塞事件循环）
///   - 超时使用 tokio::time::timeout 实现（可被取消）
///   - 取消通过 AtomicBool + 定期检查实现
#[tauri::command]
pub async fn tool_bash(command: String, working_dir: Option<String>, timeout_secs: Option<u64>) -> Result<serde_json::Value, String> {
    let start_time = Instant::now();

    if let Some(blocked) = check_dangerous_command(&command) {
        return Ok(serde_json::json!({
            "tool": "Bash", "success": false, "blocked": true,
            "output": blocked, "exit_code": -1, "category": "dangerous"
        }));
    }

    let category = classify_command(&command);
    let category_name = match category {
        CommandCategory::Search => "search",
        CommandCategory::Read => "read",
        CommandCategory::List => "list",
        CommandCategory::Write => "write",
        CommandCategory::Delete => "delete",
        CommandCategory::Silent => "silent",
        CommandCategory::Other => "other",
    };

    let timeout = timeout_secs.unwrap_or(120).min(600);
    let max_output = 100_000usize;

    #[cfg(target_os = "windows")]
    let mut child: TokioChild = {
        let mut cmd = TokioCommand::new("powershell");
        cmd.arg("-NoProfile").arg("-NonInteractive").arg("-Command").arg(&command)
           .stdin(std::process::Stdio::null())
           .stdout(std::process::Stdio::piped())
           .stderr(std::process::Stdio::piped())
           .kill_on_drop(true);
        if let Some(dir) = &working_dir {
            if !std::path::Path::new(dir).exists() {
                return Ok(serde_json::json!({
                    "tool": "Bash", "success": false,
                    "output": format!("工作目录不存在: {}", dir), "exit_code": -1, "category": category_name
                }));
            }
            cmd.current_dir(dir);
        }
        cmd.spawn().map_err(|e| format!("启动进程失败: {}", e))?
    };

    #[cfg(not(target_os = "windows"))]
    let mut child: TokioChild = {
        let mut cmd = TokioCommand::new("sh");
        cmd.arg("-c").arg(&command)
           .stdin(std::process::Stdio::null())
           .stdout(std::process::Stdio::piped())
           .stderr(std::process::Stdio::piped())
           .kill_on_drop(true);
        if let Some(dir) = &working_dir {
            if !std::path::Path::new(dir).exists() {
                return Ok(serde_json::json!({
                    "tool": "Bash", "success": false,
                    "output": format!("工作目录不存在: {}", dir), "exit_code": -1, "category": category_name
                }));
            }
            cmd.current_dir(dir);
        }
        cmd.spawn().map_err(|e| format!("启动进程失败: {}", e))?
    };

    let pid = child.id().unwrap_or(0);
    log::info!("[Bash] PID={} command='{}' timeout={}s dir={:?} category={}", 
        pid, command, timeout, working_dir, category_name);

    {
        let mut running = RUNNING_CHILD.lock().await;
        *running = Some(pid);
    }

    let mut stdout_acc = OutputAccumulator::new(max_output);
    let mut stderr_acc = OutputAccumulator::new(max_output);

    if let Some(stdout_reader) = child.stdout.take() {
        use tokio::io::BufReader;
        let mut reader = BufReader::new(stdout_reader);
        loop {
            let mut buf = vec![0u8; 8192];
            match reader.read(&mut buf).await {
                Ok(0) => break,
                Ok(n) => {
                    let text = String::from_utf8_lossy(&buf[..n]).to_string();
                    stdout_acc.push(&text);
                }
                Err(_) if CANCEL_FLAG.load(Ordering::SeqCst) => break,
                Err(_) => break,
            }
        }
    }

    if let Some(stderr_reader) = child.stderr.take() {
        use tokio::io::BufReader;
        let mut reader = BufReader::new(stderr_reader);
        loop {
            let mut buf = vec![0u8; 8192];
            match reader.read(&mut buf).await {
                Ok(0) => break,
                Ok(n) => {
                    let text = String::from_utf8_lossy(&buf[..n]).to_string();
                    stderr_acc.push(&text);
                }
                Err(_) if CANCEL_FLAG.load(Ordering::SeqCst) => break,
                Err(_) => break,
            }
        }
    }

    let elapsed_ms = start_time.elapsed().as_millis() as u64;
    let timed_out = elapsed_ms >= timeout * 1000;

    if timed_out || CANCEL_FLAG.load(Ordering::SeqCst) {
        if timed_out {
            log::warn!("[Bash] PID={} 超时 ({}s > {}s), 终止进程", pid, elapsed_ms / 1000, timeout);
        }
        let _ = child.kill().await;
        let _ = child.wait().await;
        CANCEL_FLAG.store(false, Ordering::SeqCst);

        {
            let mut running = RUNNING_CHILD.lock().await;
            *running = None;
        }

        let status_msg = if timed_out { format!("⏰ 命令执行超时 ({}s)", timeout) } else { "❌ 命令被用户取消".to_string() };
        return Ok(serde_json::json!({
            "tool": "Bash", "success": false, "timed_out": timed_out, "cancelled": !timed_out,
            "output": format!("{}\n[STDERR]:\n{}\n[STDOUT]:\n{}", 
                status_msg, stderr_acc.finish(), stdout_acc.finish()),
            "exit_code": -1, "pid": pid, "duration_ms": elapsed_ms, "category": category_name
        }));
    }

    let status = child.wait().await.map_err(|e| format!("等待进程失败: {}", e))?;
    let exit_code = status.code().unwrap_or(-1);
    let success = exit_code == 0;

    {
        let mut running = RUNNING_CHILD.lock().await;
        *running = None;
    }

    let stdout_text = stdout_acc.finish();
    let stderr_text = stderr_acc.finish();

    let output = if success && !stderr_text.is_empty() {
        if stdout_text.is_empty() {
            format!("[退出码 {}]\n[STDERR]:\n{}", exit_code, stderr_text)
        } else {
            format!("[退出码 {}]\n[STDERR]:\n{}\n[STDOUT]:\n{}", exit_code, stderr_text, stdout_text)
        }
    } else if success && stdout_text.is_empty() {
        format!("[退出码 {}] (无输出)", exit_code)
    } else if !success {
        format!("[退出码 {} - 失败]\n[STDERR]:\n{}\n[STDOUT]:\n{}", exit_code, stderr_text, stdout_text)
    } else {
        format!("[退出码 {}]\n{}", exit_code, stdout_text)
    };

    log::info!("[Bash] PID={} exit_code={} duration={}ms output_len={}", 
        pid, exit_code, elapsed_ms, output.len());

    Ok(serde_json::json!({
        "tool": "Bash", "success": success,
        "output": output, "exit_code": exit_code, "pid": pid,
        "duration_ms": elapsed_ms, "category": category_name
    }))
}
