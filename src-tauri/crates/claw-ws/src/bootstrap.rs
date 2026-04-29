// Claw Desktop - WS引导 - 启动WebSocket服务器
use std::sync::{OnceLock};
use tokio::sync::OnceCell;

static WS_STATE: OnceCell<WsState> = OnceCell::const_new();
static WS_PORT: OnceLock<u16> = OnceLock::new();

/// WS服务状态 — 记录端口号
pub struct WsState {
    pub port: u16,
}

/// 引导WS服务 — 加载配置、启动WS服务器、设置AppHandle、连接LLM回调、初始化Channel注册表
pub async fn bootstrap(app_handle: tauri::AppHandle) -> Result<&'static WsState, String> {
    if let Some(state) = WS_STATE.get() {
        return Ok(state);
    }

    use crate::ws::router;
    use crate::ws::server;

    let cfg = claw_config::config::get_config()
        .await?
        .clone();

    router::init_router(cfg);

    let port = server::start_ws_server()
        .await
        .map_err(|e| format!("WS server start failed: {}", e))?;
    log::info!("[WsBootstrap] WebSocket server started on port {}", port);

    server::set_app_handle(app_handle);

    claw_llm::llm::set_ws_emit_callback(|conv_id, event_type, data| {
        server::emit_stream(conv_id, event_type, data);
    });

    if let Some(registry) = claw_channel::bootstrap::get_registry() {
        crate::ws::channel_handlers::init_registry(registry);
        log::info!("[WsBootstrap] Channel registry wired to WS handlers");
    } else {
        log::warn!("[WsBootstrap] Channel registry not found, channel routes will be unavailable");
    }

    let _ = WS_PORT.set(port);
    let state = WsState { port };
    WS_STATE.set(state)
        .map_err(|_| "WS state already set".to_string())?;

    log::info!("[WsBootstrap] Initialized (port={})", port);
    WS_STATE.get().ok_or_else(|| "WS state not initialized".to_string())
}

/// 获取WS服务器端口
pub fn get_ws_port() -> Option<u16> {
    WS_PORT.get().copied()
}

/// 检查WS服务是否已初始化
pub fn is_initialized() -> bool {
    WS_STATE.get().is_some()
}
