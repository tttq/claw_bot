// Claw Desktop - WS路由器 - 请求方法到处理函数的映射
use crate::ws::auth;
use crate::ws::protocol::{WsRequest, WsResponse};
use claw_config::config::AppConfig;
use std::sync::OnceLock;
use tokio::sync::Mutex as TokioMutex;

static APP_CONFIG: OnceLock<TokioMutex<AppConfig>> = OnceLock::new();

/// 初始化路由器 — 存储AppConfig到全局静态
pub fn init_router(config: AppConfig) {
    APP_CONFIG.get_or_init(|| TokioMutex::new(config));
}

/// 获取当前AppConfig
pub(crate) async fn get_config() -> AppConfig {
    if let Some(cfg_lock) = APP_CONFIG.get() {
        cfg_lock.lock().await.clone()
    } else {
        AppConfig::default()
    }
}

/// 分发WS请求 — 验证Token后路由到对应处理方法
///
/// 当前WS传输仅用于流式事件推送，所有API调用已迁移到HTTP路由
pub async fn dispatch(req: WsRequest) -> WsResponse {
    if req.method != "auth_handshake" {
        if req.token.is_empty() || !auth::is_token_valid(&req.token) {
            return WsResponse::err(
                &req.id,
                &req.method,
                "Unauthorized: invalid or expired token",
            );
        }
    }

    let result = match req.method.as_str() {
        "auth_handshake" => Err("Use HTTP POST /api/auth/handshake instead".to_string()),
        _ => Err(format!(
            "Method '{}' not available on WS transport. \
             Use HTTP {} /api/{} instead. \
             See src/ws/http.ts ROUTE_MAP for endpoint details.",
            req.method,
            if req.method.starts_with("get_")
                || req.method.starts_with("list")
                || req.method == "tool_list_all"
                || req.method == "agent_list"
                || req.method == "channel_list"
                || req.method == "skill_list"
                || req.method == "memory_list_entities"
                || req.method == "memory_stats"
                || req.method == "get_db_stats"
                || req.method == "get_session_info"
            {
                "GET"
            } else {
                "POST"
            },
            req.method
        )),
    };

    match result {
        Ok(data) => WsResponse::ok(&req.id, &req.method, data),
        Err(e) => WsResponse::err(&req.id, &req.method, e.as_str()),
    }
}
