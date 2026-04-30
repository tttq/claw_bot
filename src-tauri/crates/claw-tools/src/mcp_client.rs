// Claw Desktop - MCP客户端 - Model Context Protocol客户端实现
// 通过 stdio JSON-RPC 2.0 与外部 MCP Server 通信
// 架构：同步底层 I/O + 异步公共 API（通过 spawn_blocking 桥接）

use claw_types::common::ToolDefinition;
use std::io::{BufRead, BufReader, Write};
use std::process::{Child, Command, Stdio};
use std::sync::Mutex;

static MCP_CLIENT: Mutex<Option<McpProcess>> = Mutex::new(None);

/// MCP进程 — 管理子进程句柄和请求ID计数器
struct McpProcess {
    child: Child,
    request_id: i64,
}

impl McpProcess {
    /// 创建MCP进程 — 启动子进程并捕获stderr日志
    fn new(mut cmd: Command) -> Result<Self, String> {
        log::info!("[MCP] Starting process: {:?}", cmd);

        let mut child = cmd
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| format!("Failed to spawn MCP process: {}", e))?;

        let stderr = child
            .stderr
            .take()
            .ok_or_else(|| "Failed to capture stderr".to_string())?;
        std::thread::spawn(move || {
            let reader = BufReader::new(stderr);
            for line in reader.lines().flatten() {
                if !line.trim().is_empty() {
                    log::debug!("[MCP:stderr] {}", line);
                }
            }
        });

        Ok(Self {
            child,
            request_id: 0,
        })
    }

    /// 生成下一个请求ID
    fn next_id(&mut self) -> i64 {
        self.request_id += 1;
        self.request_id
    }

    /// 发送JSON-RPC请求 — 写入stdin并从stdout读取响应，带30秒超时
    fn send_request(
        &mut self,
        method: &str,
        params: serde_json::Value,
    ) -> Result<serde_json::Value, String> {
        let id = self.next_id();
        let request = serde_json::json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": method,
            "params": params
        });

        let stdin = self
            .child
            .stdin
            .as_mut()
            .ok_or_else(|| "MCP process stdin not available".to_string())?;

        writeln!(
            stdin,
            "{}",
            serde_json::to_string(&request).map_err(|e| e.to_string())?
        )
        .map_err(|e| format!("Failed to write to MCP stdin: {}", e))?;
        stdin
            .flush()
            .map_err(|e| format!("Failed to flush MCP stdin: {}", e))?;

        let stdout = self
            .child
            .stdout
            .as_mut()
            .ok_or_else(|| "MCP process stdout not available".to_string())?;

        let mut reader = BufReader::new(stdout);
        let mut response_line = String::new();

        let start_time = std::time::Instant::now();
        let timeout_secs: u64 = 30;
        loop {
            if start_time.elapsed().as_secs() >= timeout_secs {
                return Err(format!("MCP read timeout after {}s", timeout_secs));
            }
            let read_result = reader
                .read_line(&mut response_line)
                .map_err(|e| format!("MCP read error: {}", e))?;

            if read_result == 0 {
                return Err("MCP process closed stdout (EOF)".to_string());
            }
            if !response_line.trim().is_empty() {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(10));
        }

        let response: serde_json::Value =
            serde_json::from_str(response_line.trim()).map_err(|e| {
                format!(
                    "Invalid MCP response JSON: {} | raw: {}",
                    e,
                    claw_types::truncate_str_safe(&response_line, 200)
                )
            })?;

        if let Some(error) = response.get("error") {
            let code = error.get("code").and_then(|v| v.as_i64()).unwrap_or(-1);
            let message = error
                .get("message")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown");
            return Err(format!("MCP error {}: {}", code, message));
        }

        Ok(response)
    }
}

/// 启动 MCP Server 进程并建立连接
pub async fn start_mcp_server(command: &str, args: Vec<String>) -> Result<(), String> {
    let parts: Vec<&str> = command.split_whitespace().collect();
    if parts.is_empty() {
        return Err("Empty MCP command".to_string());
    }

    let mut cmd = Command::new(parts[0]);
    for arg in &parts[1..] {
        cmd.arg(arg);
    }
    for arg in args {
        cmd.arg(arg);
    }

    let process = McpProcess::new(cmd)?;
    match MCP_CLIENT.lock() {
        Ok(mut guard) => *guard = Some(process),
        Err(e) => return Err(format!("MCP lock poisoned: {}", e)),
    }

    {
        let mut client = MCP_CLIENT
            .lock()
            .map_err(|e| format!("MCP lock poisoned: {}", e))?;
        if let Some(ref mut p) = *client {
            let init_params = serde_json::json!({
                "protocolVersion": "2024-11-05",
                "capabilities": {},
                "clientInfo": { "name": "claw-desktop", "version": "0.1.0" }
            });

            match p.send_request("initialize", init_params) {
                Ok(resp) => {
                    log::info!(
                        "[MCP] Initialized successfully: server_info={}",
                        resp.get("result")
                            .and_then(|r| r.get("serverInfo"))
                            .map(|s| s.to_string())
                            .unwrap_or_else(|| "unknown".into())
                    );

                    drop(client);
                    send_notification("notifications/initialized", serde_json::json!({}))?;
                }
                Err(e) => {
                    log::warn!("[MCP] Initialize failed: {}, continuing without MCP", e);
                    if let Ok(mut guard) = MCP_CLIENT.lock() {
                        *guard = None;
                    }
                }
            }
        }
    }

    Ok(())
}

/// 发送JSON-RPC通知 — 无需响应的单向消息
fn send_notification(method: &str, params: serde_json::Value) -> Result<(), String> {
    let mut client = MCP_CLIENT
        .lock()
        .map_err(|e| format!("MCP lock poisoned: {}", e))?;
    if let Some(ref mut p) = *client {
        let request = serde_json::json!({
            "jsonrpc": "2.0",
            "method": method,
            "params": params
        });
        let stdin = p
            .child
            .stdin
            .as_mut()
            .ok_or_else(|| "MCP stdin unavailable".to_string())?;
        writeln!(
            stdin,
            "{}",
            serde_json::to_string(&request).map_err(|e| e.to_string())?
        )
        .map_err(|e| format!("Write failed: {}", e))?;
        stdin.flush().map_err(|e| format!("Flush failed: {}", e))?;
    }
    Ok(())
}

/// 发现 MCP Server 提供的所有工具
pub async fn discover_tools() -> Result<Vec<ToolDefinition>, String> {
    tokio::task::spawn_blocking(|| {
        let mut client = MCP_CLIENT
            .lock()
            .map_err(|e| format!("MCP lock poisoned: {}", e))?;
        let process = client
            .as_mut()
            .ok_or_else(|| "MCP not connected".to_string())?;

        let result = process.send_request("tools/list", serde_json::json!({}))?;
        let tools_arr = result
            .get("result")
            .and_then(|r| r.get("tools"))
            .and_then(|t| t.as_array())
            .ok_or_else(|| "No tools in MCP response".to_string())?
            .clone();

        let mut tools = Vec::with_capacity(tools_arr.len());
        for tool_value in &tools_arr {
            let name = tool_value
                .get("name")
                .and_then(|n| n.as_str())
                .unwrap_or("")
                .to_string();
            let desc = tool_value
                .get("description")
                .and_then(|d| d.as_str())
                .unwrap_or("")
                .to_string();
            let schema = tool_value
                .get("inputSchema")
                .cloned()
                .or_else(|| tool_value.get("input_schema").cloned())
                .unwrap_or(serde_json::json!({"type": "object"}));

            tools.push(ToolDefinition {
                name,
                description: desc,
                input_schema: schema,
                category: Some("mcp".to_string()),
                tags: vec!["mcp".to_string()],
            });
        }

        log::info!("[MCP] Discovered {} tools", tools.len());
        Ok(tools)
    })
    .await
    .map_err(|e| format!("MCP discover task failed: {}", e))?
}

/// 调用 MCP 工具
pub async fn call_tool(
    name: &str,
    arguments: serde_json::Value,
) -> Result<serde_json::Value, String> {
    let name_owned = name.to_string();
    tokio::task::spawn_blocking(move || {
        let mut client = MCP_CLIENT
            .lock()
            .map_err(|e| format!("MCP lock poisoned: {}", e))?;
        let process = client
            .as_mut()
            .ok_or_else(|| "MCP not connected".to_string())?;

        let result = process.send_request(
            "tools/call",
            serde_json::json!({
                "name": name_owned,
                "arguments": arguments
            }),
        )?;

        let tool_result = result
            .get("result")
            .or_else(|| result.get("error"))
            .cloned()
            .ok_or_else(|| "Empty MCP tool response".to_string())?;

        if tool_result
            .get("isError")
            .and_then(|b| b.as_bool())
            .unwrap_or(false)
        {
            let content = tool_result
                .get("content")
                .and_then(|c| c.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|i| i.get("text").and_then(|t| t.as_str()))
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default()
                .join("; ");
            return Err(format!("MCP tool '{}' error: {}", name_owned, content));
        }

        if let Some(content) = tool_result.get("content") {
            if let Some(arr) = content.as_array() {
                let texts: Vec<serde_json::Value> = arr
                    .iter()
                    .filter_map(|item| item.get("text").cloned())
                    .collect();
                if !texts.is_empty() {
                    return if texts.len() == 1 {
                        Ok(texts.into_iter().next().unwrap_or(serde_json::Value::Null))
                    } else {
                        Ok(serde_json::json!({ "results": texts }))
                    };
                }
            }
        }

        log::debug!(
            "[MCP] Tool '{}' returned raw: {}",
            name_owned,
            claw_types::truncate_str_safe(&tool_result.to_string(), 200)
        );
        Ok(tool_result)
    })
    .await
    .map_err(|e| format!("MCP call_tool task failed: {}", e))?
}

/// 列出可用资源
pub async fn list_resources() -> Result<Vec<serde_json::Value>, String> {
    tokio::task::spawn_blocking(|| {
        let mut client = MCP_CLIENT
            .lock()
            .map_err(|e| format!("MCP lock poisoned: {}", e))?;
        let process = client
            .as_mut()
            .ok_or_else(|| "MCP not connected".to_string())?;

        let result = process.send_request("resources/list", serde_json::json!({}))?;
        let resources = result
            .get("result")
            .and_then(|r| r.get("resources"))
            .and_then(|a| a.as_array())
            .cloned()
            .unwrap_or_default();

        Ok(resources)
    })
    .await
    .map_err(|e| format!("MCP list_resources task failed: {}", e))?
}

/// 读取资源内容
pub async fn read_resource(uri: &str) -> Result<String, String> {
    let uri_owned = uri.to_string();
    tokio::task::spawn_blocking(move || {
        let mut client = MCP_CLIENT
            .lock()
            .map_err(|e| format!("MCP lock poisoned: {}", e))?;
        let process = client
            .as_mut()
            .ok_or_else(|| "MCP not connected".to_string())?;

        let result = process.send_request(
            "resources/read",
            serde_json::json!({
                "uri": uri_owned
            }),
        )?;

        let contents = result
            .get("result")
            .and_then(|r| r.get("contents"))
            .and_then(|a| a.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|c| c.get("text").and_then(|t| t.as_str()))
                    .collect::<Vec<_>>()
                    .join("\n")
            })
            .unwrap_or_default();

        Ok(contents)
    })
    .await
    .map_err(|e| format!("MCP read_resource task failed: {}", e))?
}

/// 断开 MCP 连接
pub async fn disconnect_mcp() -> Result<(), String> {
    tokio::task::spawn_blocking(|| {
        let mut client = MCP_CLIENT
            .lock()
            .map_err(|e| format!("MCP lock poisoned: {}", e))?;
        if let Some(mut process) = client.take() {
            let _ = process.send_request("shutdown", serde_json::json!({}));

            match process.child.kill() {
                Ok(()) => log::info!("[MCP] Process terminated"),
                Err(e) => log::warn!("[MCP] Process already exited: {}", e),
            }
            let _ = process.child.wait();
        }
        Ok(())
    })
    .await
    .map_err(|e| format!("MCP disconnect task failed: {}", e))?
}
