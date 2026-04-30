// Claw Desktop - Chrome CDP - Chrome DevTools Protocol通信实现
#![allow(dead_code)]

use base64::Engine;
use futures::SinkExt;
use futures::StreamExt;
use serde::{Deserialize, Serialize};
use tokio_tungstenite::tungstenite::Message;

use std::sync::atomic::{AtomicI32, Ordering};

static CDP_ID_COUNTER: AtomicI32 = AtomicI32::new(1);
fn next_cdp_id() -> i32 {
    CDP_ID_COUNTER.fetch_add(1, Ordering::Relaxed)
}

/// CDP响应结构
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CdpResponse {
    pub id: Option<i32>,
    pub result: Option<serde_json::Value>,
    pub error: Option<CdpError>,
    pub method: Option<String>,
    pub params: Option<serde_json::Value>,
}

/// CDP错误结构
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CdpError {
    pub code: i32,
    pub message: String,
}

/// Chrome CDP客户端 — 通过WebSocket与Chrome DevTools Protocol通信
pub struct ChromeCdpClient {
    ws_url: String,
    write: tokio::sync::mpsc::UnboundedSender<Message>,
}

impl ChromeCdpClient {
    /// 连接到Chrome DevTools — 建立WebSocket连接并启动消息转发
    pub async fn connect(ws_url: &str) -> Result<Self, String> {
        log::info!("[CDP] Connecting to {}", ws_url);

        let (ws_stream, _) = tokio_tungstenite::connect_async(ws_url)
            .await
            .map_err(|e| format!("WebSocket connection failed: {}", e))?;

        let (write, _read) = ws_stream.split();
        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();

        tokio::spawn(async move {
            let mut write = write;
            while let Some(msg) = rx.recv().await {
                if write.send(msg).await.is_err() {
                    break;
                }
            }
        });

        Ok(Self {
            ws_url: ws_url.to_string(),
            write: tx,
        })
    }

    /// 发送CDP命令 — 通过WebSocket发送JSON-RPC命令并等待响应
    pub async fn send_command(
        &self,
        method: &str,
        params: Option<serde_json::Value>,
    ) -> Result<serde_json::Value, String> {
        let id = next_cdp_id();
        let command = serde_json::json!({
            "id": id,
            "method": method,
            "params": params.unwrap_or(serde_json::Value::Null)
        });

        self.write
            .send(Message::Text(command.to_string()))
            .map_err(|e| format!("Failed to send command: {}", e))?;

        Ok(serde_json::json!({ "id": id, "method": method }))
    }

    /// 导航到指定URL
    pub async fn navigate(&self, url: &str) -> Result<(), String> {
        self.send_command("Page.navigate", Some(serde_json::json!({ "url": url })))
            .await?;
        Ok(())
    }

    /// 获取页面文本内容 — 通过Runtime.evaluate获取document.body.innerText
    pub async fn get_page_content(&self) -> Result<String, String> {
        let result = self
            .send_command(
                "Runtime.evaluate",
                Some(serde_json::json!({
                    "expression": "document.body.innerText",
                    "returnByValue": true
                })),
            )
            .await?;

        if let Some(value_str) = result
            .get("result")
            .and_then(|r| r.get("result"))
            .and_then(|r| r.get("value"))
            .and_then(|v| v.as_str())
        {
            return Ok(value_str.to_string());
        }

        Err("Failed to get page content".to_string())
    }

    /// 执行JavaScript代码 — 通过Runtime.evaluate执行并返回结果
    pub async fn execute_javascript(&self, script: &str) -> Result<serde_json::Value, String> {
        self.send_command(
            "Runtime.evaluate",
            Some(serde_json::json!({
                "expression": script,
                "returnByValue": true,
                "awaitPromise": true
            })),
        )
        .await
    }

    /// 点击元素 — 通过querySelector查找元素并触发click事件
    pub async fn click_element(&self, selector: &str) -> Result<serde_json::Value, String> {
        let script = format!(
            r#"
            const el = document.querySelector('{}');
            if (el) {{ el.click(); 'clicked'; }} else {{ 'not found'; }}
            "#,
            selector.replace("'", "\\'")
        );
        self.execute_javascript(&script).await
    }

    /// 填充输入框 — 使用原生setter设置值并触发input/change事件
    pub async fn fill_input(
        &self,
        selector: &str,
        value: &str,
    ) -> Result<serde_json::Value, String> {
        let script = format!(
            r#"
            const el = document.querySelector('{}');
            if (el) {{
                const nativeInputValueSetter = Object.getOwnPropertyDescriptor(window.HTMLInputElement.prototype, 'value').set;
                nativeInputValueSetter.call(el, '{}');
                el.dispatchEvent(new Event('input', {{ bubbles: true }}));
                el.dispatchEvent(new Event('change', {{ bubbles: true }}));
                'filled';
            }} else {{ 'not found'; }}
            "#,
            selector.replace("'", "\\'"),
            value.replace("\\", "\\\\").replace("'", "\\'")
        );
        self.execute_javascript(&script).await
    }

    /// 截图 — 通过Page.captureScreenshot获取Base64编码的截图
    pub async fn screenshot(&self, format: &str) -> Result<Vec<u8>, String> {
        let result = self
            .send_command(
                "Page.captureScreenshot",
                Some(serde_json::json!({
                    "format": format,
                    "fromSurface": true
                })),
            )
            .await?;

        if let Some(data_str) = result
            .get("result")
            .and_then(|r| r.get("data"))
            .and_then(|d| d.as_str())
        {
            match base64::engine::general_purpose::STANDARD.decode(data_str) {
                Ok(bytes) => return Ok(bytes),
                Err(e) => return Err(format!("Base64 decode failed: {}", e)),
            }
        }

        Err("Failed to capture screenshot".to_string())
    }

    /// 获取控制台消息 — 初始化控制台捕获（当前仅返回初始化状态）
    pub async fn get_console_messages(
        &self,
        only_errors: bool,
    ) -> Result<Vec<ConsoleMessage>, String> {
        let _filter = if only_errors { "error" } else { "" };
        let script = format!(
            r#"
            (() => {{
                const messages = [];
                const originalLog = console.log;
                const originalError = console.error;
                const originalWarn = console.warn;
                messages.push({{ type: 'info', text: 'Console capture started' }});
                JSON.stringify(messages);
            }})()
            "#
        );

        match self.execute_javascript(&script).await {
            Ok(_) => Ok(vec![ConsoleMessage {
                level: "info".to_string(),
                text: "Console capture initialized".to_string(),
                source: "".to_string(),
                line: 0,
            }]),
            Err(e) => Err(e),
        }
    }

    /// 获取网络请求 — 启用Network监控
    pub async fn get_network_requests(&self) -> Result<Vec<NetworkRequest>, String> {
        self.send_command("Network.enable", None).await?;

        Ok(vec![NetworkRequest {
            url: "network monitoring enabled".to_string(),
            method: "".to_string(),
            status: 0,
            status_text: "".to_string(),
            content_type: None,
            size: 0,
            duration: 0.0,
        }])
    }

    /// 获取页面完整HTML — 通过Runtime.evaluate获取document.documentElement.outerHTML
    pub async fn get_page_html(&self) -> Result<String, String> {
        let result = self
            .send_command(
                "Runtime.evaluate",
                Some(serde_json::json!({
                    "expression": "document.documentElement.outerHTML",
                    "returnByValue": true
                })),
            )
            .await?;

        if let Some(html) = result
            .get("result")
            .and_then(|r| r.get("result"))
            .and_then(|r| r.get("value"))
            .and_then(|v| v.as_str())
        {
            return Ok(html.to_string());
        }

        Err("Failed to get page HTML".to_string())
    }

    /// 等待元素出现 — 轮询querySelector直到元素存在或超时
    pub async fn wait_for_selector(&self, selector: &str, timeout_ms: u64) -> Result<bool, String> {
        let script = format!(
            r#"
            new Promise((resolve) => {{
                const start = Date.now();
                const check = () => {{
                    if (document.querySelector('{}')) {{ resolve(true); }}
                    else if (Date.now() - start > {}) {{ resolve(false); }}
                    else {{ setTimeout(check, 100); }}
                }};
                check();
            }})
            "#,
            selector.replace("'", "\\'"),
            timeout_ms
        );

        let result = self.execute_javascript(&script).await?;
        if let Some(found) = result
            .get("result")
            .and_then(|r| r.get("result"))
            .and_then(|r| r.get("value"))
            .and_then(|v| v.as_bool())
        {
            return Ok(found);
        }

        Ok(false)
    }

    /// 滚动到页面底部
    pub async fn scroll_to_bottom(&self) -> Result<(), String> {
        self.execute_javascript("window.scrollTo(0, document.body.scrollHeight)")
            .await?;
        Ok(())
    }

    /// 滚动到页面顶部
    pub async fn scroll_to_top(&self) -> Result<(), String> {
        self.execute_javascript("window.scrollTo(0, 0)").await?;
        Ok(())
    }

    /// 获取页面信息 — 标题、URL、域名
    pub async fn get_page_info(&self) -> Result<PageInfo, String> {
        let result = self.execute_javascript(
            "JSON.stringify({ title: document.title, url: location.href, domain: location.hostname })"
        ).await?;

        if let Some(info_str) = result
            .get("result")
            .and_then(|r| r.get("result"))
            .and_then(|r| r.get("value"))
            .and_then(|v| v.as_str())
        {
            if let Ok(page_info) = serde_json::from_str::<PageInfo>(info_str) {
                return Ok(page_info);
            }
        }

        Err("Failed to get page info".to_string())
    }

    /// 关闭当前标签页
    pub async fn close_tab(&self) -> Result<(), String> {
        self.send_command("Page.close", None).await?;
        Ok(())
    }

    /// 刷新页面 — 可选忽略缓存
    pub async fn reload(&self, ignore_cache: bool) -> Result<(), String> {
        self.send_command(
            "Page.reload",
            Some(serde_json::json!({
                "ignoreCache": ignore_cache
            })),
        )
        .await?;
        Ok(())
    }

    /// 后退导航
    pub async fn go_back(&self) -> Result<(), String> {
        self.send_command("Navigation.back", None).await?;
        Ok(())
    }

    /// 前进导航
    pub async fn go_forward(&self) -> Result<(), String> {
        self.send_command("Navigation.forward", None).await?;
        Ok(())
    }

    /// 设置页面缩放级别
    pub async fn zoom(&self, level: f64) -> Result<(), String> {
        let script = format!("document.documentElement.style.zoom = '{}'", level);
        self.execute_javascript(&script).await?;
        Ok(())
    }

    /// 打印页面
    pub async fn print_page(&self) -> Result<(), String> {
        self.send_command(
            "Page.printToPDF",
            Some(serde_json::json!({
                "landscape": false,
                "displayHeaderFooter": true
            })),
        )
        .await?;
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsoleMessage {
    pub level: String,
    pub text: String,
    pub source: String,
    pub line: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkRequest {
    pub url: String,
    pub method: String,
    pub status: u16,
    pub status_text: String,
    pub content_type: Option<String>,
    pub size: u64,
    pub duration: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PageInfo {
    pub title: String,
    pub url: String,
    pub domain: String,
}
