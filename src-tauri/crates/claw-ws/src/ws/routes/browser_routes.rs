// Claw Desktop - 浏览器路由 - 处理CDP浏览器控制的WS请求
use axum::{
    Json, Router,
    extract::{Extension, Path, Query},
    routing::{get, post},
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::ws::app_state::AppState;
use crate::ws::response::ApiResponse;
use crate::ws::router_trait::ClawRouter;
use base64::Engine;
use claw_tools::browser_manager;
use claw_tools::chrome_cdp::ChromeCdpClient;

/// 浏览器路由 — 处理浏览器自动化操作的WS请求
pub struct BrowserRoutes;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
/// 启动浏览器查询参数
pub struct LaunchBrowserQuery {
    browser_path: Option<String>,
    #[serde(default = "default_port")]
    port: u16,
}

/// 默认CDP端口 — 9222
fn default_port() -> u16 {
    9222
}

/// 检测已安装浏览器
pub async fn browser_detect(
    Extension(_state): Extension<Arc<AppState>>,
) -> Json<ApiResponse<serde_json::Value>> {
    let browsers = browser_manager::detect_chrome_installations();
    Json(ApiResponse::ok(serde_json::json!({
        "browsers": browsers,
        "count": browsers.len(),
        "platform": std::env::consts::OS
    })))
}

/// 启动浏览器实例
pub async fn browser_launch(
    Extension(_state): Extension<Arc<AppState>>,
    Query(query): Query<LaunchBrowserQuery>,
) -> Json<ApiResponse<serde_json::Value>> {
    let browsers = browser_manager::detect_chrome_installations();
    let path = if let Some(p) = query.browser_path {
        p
    } else if !browsers.is_empty() {
        browsers[0].path.clone()
    } else {
        return Json(ApiResponse::err("No browser found"));
    };

    let config = browser_manager::ChromeLaunchConfig {
        remote_debugging_port: query.port,
        ..Default::default()
    };

    match browser_manager::launch_chrome_with_debugging(&path, &config) {
        Ok(port) => Json(ApiResponse::ok(
            serde_json::json!({ "success": true, "port": port }),
        )),
        Err(e) => Json(ApiResponse::err(&e)),
    }
}

/// 检查CDP端口是否可用
pub async fn browser_check_port(
    Extension(_state): Extension<Arc<AppState>>,
    Path(port): Path<u16>,
) -> Json<ApiResponse<serde_json::Value>> {
    match browser_manager::check_debug_port(port) {
        Ok(is_open) => Json(ApiResponse::ok(
            serde_json::json!({ "available": is_open, "port": port }),
        )),
        Err(e) => Json(ApiResponse::err(&e)),
    }
}

/// 列出浏览器标签页
pub async fn browser_list_tabs(
    Extension(_state): Extension<Arc<AppState>>,
    Path(port): Path<u16>,
) -> Json<ApiResponse<serde_json::Value>> {
    match browser_manager::list_browser_tabs(port).await {
        Ok(tabs) => Json(ApiResponse::ok(
            serde_json::json!({ "tabs": tabs, "count": tabs.len() }),
        )),
        Err(e) => Json(ApiResponse::err(&e)),
    }
}

/// 获取标签页CDP客户端
async fn get_tab_client(port: u16, tab_id: &str) -> Result<(String, String), String> {
    let tabs = browser_manager::list_browser_tabs(port)
        .await
        .map_err(|e| e.to_string())?;

    let tab = tabs
        .iter()
        .find(|t| t.id == tab_id)
        .ok_or_else(|| format!("Tab {} not found", tab_id))?;

    Ok((tab.id.clone(), tab.web_socket_url.clone()))
}

/// 连接到指定标签页
async fn connect_tab(port: u16, tab_id: &str) -> Result<ChromeCdpClient, String> {
    let (_id, ws_url) = get_tab_client(port, tab_id).await?;
    ChromeCdpClient::connect(&ws_url)
        .await
        .map_err(|e| e.to_string())
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
/// 填充输入请求 — CSS选择器和值
pub struct FillInputRequest {
    selector: String,
    value: String,
}

/// 填充输入框
pub async fn browser_fill_input(
    Extension(_state): Extension<Arc<AppState>>,
    Path((port, tab_id)): Path<(u16, String)>,
    Json(req): Json<FillInputRequest>,
) -> Json<ApiResponse<serde_json::Value>> {
    match connect_tab(port, &tab_id).await {
        Ok(client) => match client.fill_input(&req.selector, &req.value).await {
            Ok(data) => Json(ApiResponse::ok(data)),
            Err(e) => Json(ApiResponse::err(&e)),
        },
        Err(e) => Json(ApiResponse::err(&e)),
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
/// 执行JS请求 — JavaScript代码
pub struct ExecuteJsRequest {
    script: String,
}

/// 执行JavaScript代码
pub async fn browser_execute_js(
    Extension(_state): Extension<Arc<AppState>>,
    Path((port, tab_id)): Path<(u16, String)>,
    Json(req): Json<ExecuteJsRequest>,
) -> Json<ApiResponse<serde_json::Value>> {
    match connect_tab(port, &tab_id).await {
        Ok(client) => match client.execute_javascript(&req.script).await {
            Ok(data) => Json(ApiResponse::ok(data)),
            Err(e) => Json(ApiResponse::err(&e)),
        },
        Err(e) => Json(ApiResponse::err(&e)),
    }
}

/// 获取浏览器信息
pub async fn browser_get_info(
    Extension(_state): Extension<Arc<AppState>>,
    Path((port, tab_id)): Path<(u16, String)>,
) -> Json<ApiResponse<serde_json::Value>> {
    match connect_tab(port, &tab_id).await {
        Ok(client) => match client.get_page_info().await {
            Ok(info) => Json(ApiResponse::ok(serde_json::json!(info))),
            Err(e) => Json(ApiResponse::err(&e)),
        },
        Err(e) => Json(ApiResponse::err(&e)),
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
/// 刷新页面查询参数
pub struct ReloadQuery {
    #[serde(default)]
    ignore_cache: bool,
}

pub async fn browser_reload(
    Extension(_state): Extension<Arc<AppState>>,
    Path((port, tab_id)): Path<(u16, String)>,
    Query(query): Query<ReloadQuery>,
) -> Json<ApiResponse<serde_json::Value>> {
    match connect_tab(port, &tab_id).await {
        Ok(client) => match client.reload(query.ignore_cache).await {
            Ok(_) => Json(ApiResponse::ok(serde_json::json!({ "success": true }))),
            Err(e) => Json(ApiResponse::err(&e)),
        },
        Err(e) => Json(ApiResponse::err(&e)),
    }
}

/// 关闭标签页
pub async fn browser_close_tab(
    Extension(_state): Extension<Arc<AppState>>,
    Path((port, tab_id)): Path<(u16, String)>,
) -> Json<ApiResponse<serde_json::Value>> {
    match connect_tab(port, &tab_id).await {
        Ok(client) => match client.close_tab().await {
            Ok(_) => Json(ApiResponse::ok(serde_json::json!({ "success": true }))),
            Err(e) => Json(ApiResponse::err(&e)),
        },
        Err(e) => Json(ApiResponse::err(&e)),
    }
}

/// 导航到URL — 通过CDP客户端导航到指定URL
pub async fn browser_navigate(
    Extension(_state): Extension<Arc<AppState>>,
    Path((port, tab_id)): Path<(u16, String)>,
    Json(body): Json<serde_json::Value>,
) -> Json<ApiResponse<serde_json::Value>> {
    let url = body
        .get("url")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    if url.is_empty() {
        return Json(ApiResponse::err("Missing url"));
    }

    match connect_tab(port, &tab_id).await {
        Ok(client) => match client.navigate(&url).await {
            Ok(_) => Json(ApiResponse::ok(
                serde_json::json!({ "success": true, "url": url }),
            )),
            Err(e) => Json(ApiResponse::err(&e)),
        },
        Err(e) => Json(ApiResponse::err(&e)),
    }
}

/// 获取页面内容 — 通过CDP客户端获取页面文本
pub async fn browser_get_content(
    Extension(_state): Extension<Arc<AppState>>,
    Path((port, tab_id)): Path<(u16, String)>,
) -> Json<ApiResponse<serde_json::Value>> {
    match connect_tab(port, &tab_id).await {
        Ok(client) => match client.get_page_content().await {
            Ok(content) => Json(ApiResponse::ok(serde_json::json!({ "content": content }))),
            Err(e) => Json(ApiResponse::err(&e)),
        },
        Err(e) => Json(ApiResponse::err(&e)),
    }
}

/// 截取页面截图 — 通过CDP客户端获取Base64编码的截图
pub async fn browser_screenshot(
    Extension(_state): Extension<Arc<AppState>>,
    Path((port, tab_id)): Path<(u16, String)>,
    Query(query): Query<std::collections::HashMap<String, String>>,
) -> Json<ApiResponse<serde_json::Value>> {
    let format = query
        .get("format")
        .cloned()
        .unwrap_or_else(|| "png".to_string());

    match connect_tab(port, &tab_id).await {
        Ok(client) => match client.screenshot(&format).await {
            Ok(bytes) => {
                let data = base64::engine::general_purpose::STANDARD.encode(&bytes);
                Json(ApiResponse::ok(serde_json::json!({
                    "data": data,
                    "format": format,
                    "size": bytes.len()
                })))
            }
            Err(e) => Json(ApiResponse::err(&e)),
        },
        Err(e) => Json(ApiResponse::err(&e)),
    }
}

/// 点击页面元素 — 通过CDP客户端执行CSS选择器点击
pub async fn browser_click(
    Extension(_state): Extension<Arc<AppState>>,
    Path((port, tab_id)): Path<(u16, String)>,
    Json(body): Json<serde_json::Value>,
) -> Json<ApiResponse<serde_json::Value>> {
    let selector = body
        .get("selector")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    if selector.is_empty() {
        return Json(ApiResponse::err("Missing selector"));
    }

    match connect_tab(port, &tab_id).await {
        Ok(client) => {
            let script = format!(
                "document.querySelector('{}')?.click()",
                selector.replace('\'', "\\'")
            );
            match client.execute_javascript(&script).await {
                Ok(_) => Json(ApiResponse::ok(
                    serde_json::json!({ "success": true, "selector": selector }),
                )),
                Err(e) => Json(ApiResponse::err(&e)),
            }
        }
        Err(e) => Json(ApiResponse::err(&e)),
    }
}

impl ClawRouter for BrowserRoutes {
    fn router() -> Router {
        Router::new()
            .route("/api/browser/detect", get(browser_detect))
            .route("/api/browser/launch", get(browser_launch))
            .route("/api/browser/check-port/:port", get(browser_check_port))
            .route("/api/browser/tabs/:port", get(browser_list_tabs))
            .route(
                "/api/browser/navigate/:port/:tab_id",
                post(browser_navigate),
            )
            .route(
                "/api/browser/content/:port/:tab_id",
                get(browser_get_content),
            )
            .route(
                "/api/browser/screenshot/:port/:tab_id",
                get(browser_screenshot),
            )
            .route("/api/browser/click/:port/:tab_id", post(browser_click))
            .route(
                "/api/browser/fill-input/:port/:tab_id",
                post(browser_fill_input),
            )
            .route(
                "/api/browser/execute-js/:port/:tab_id",
                post(browser_execute_js),
            )
            .route("/api/browser/info/:port/:tab_id", get(browser_get_info))
            .route("/api/browser/reload/:port/:tab_id", get(browser_reload))
            .route(
                "/api/browser/close-tab/:port/:tab_id",
                post(browser_close_tab),
            )
    }
}
