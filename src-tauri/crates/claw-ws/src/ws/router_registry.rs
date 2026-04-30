// Claw Desktop - 路由注册表 - 管理所有路由处理器的注册
use axum::{Router, extract::Extension, middleware};
use std::sync::Arc;
use tower_http::cors::{Any, CorsLayer};

use crate::ws::app_state::AppState;
use crate::ws::middleware::auth_middleware;
use crate::ws::router_trait::ClawRouter;

pub fn create_app_router(state: Arc<AppState>) -> Router {
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any)
        .expose_headers(Any);

    let app = crate::routes::auth_routes::AuthRoutes::router()
        .merge(crate::routes::config_routes::ConfigRoutes::router())
        .merge(crate::routes::conversation_routes::ConversationRoutes::router())
        .merge(crate::routes::tool_routes::ToolRoutes::router())
        .merge(crate::routes::git_routes::GitRoutes::router())
        .merge(crate::routes::skill_routes::SkillRoutes::router())
        .merge(crate::routes::agent_routes::AgentRoutes::router())
        .merge(crate::routes::channel_routes::ChannelRoutes::router())
        .merge(crate::routes::persona_routes::PersonaRoutes::router())
        .merge(crate::routes::browser_routes::BrowserRoutes::router())
        .merge(crate::routes::memory_routes::MemoryRoutes::router())
        .merge(crate::routes::system_routes::SystemRoutes::router())
        .merge(crate::routes::system_agent_routes::router())
        .merge(crate::routes::multi_agent_routes::MultiAgentRoutes::router())
        .merge(crate::routes::fs_skill_routes::FsSkillRoutes::router())
        .merge(crate::routes::iso_routes::IsoRoutes::router())
        .merge(crate::routes::cmd_routes::CmdRoutes::router())
        .merge(crate::routes::harness_routes::HarnessRoutes::router())
        .merge(crate::routes::cron_routes::CronRoutes::router())
        .merge(crate::routes::hook_routes::HookRoutes::router())
        .merge(crate::routes::weixin_routes::WeixinRoutes::router())
        .merge(crate::routes::automation_routes::AutomationRoutes::router())
        .layer(cors)
        .layer(middleware::from_fn(auth_middleware))
        .layer(Extension(state));

    log::info!("[RouterRegistry] ✅ All 22 route modules merged successfully!");

    app
}

pub async fn start_http_server(
    state: Arc<AppState>,
    port: u16,
) -> Result<u16, Box<dyn std::error::Error>> {
    let app = create_app_router(state);

    let listener = tokio::net::TcpListener::bind(format!("127.0.0.1:{}", port)).await?;
    let actual_port = listener.local_addr()?.port();

    log::info!("[HTTP] Starting server on http://127.0.0.1:{}", actual_port);

    tokio::spawn(async move {
        if let Err(e) = axum::serve(listener, app).await {
            log::error!("[HTTP] Server error: {}", e);
        }
    });

    Ok(actual_port)
}
