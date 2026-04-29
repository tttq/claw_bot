// Claw Desktop - WS模块入口
pub mod server;
pub mod router;
pub mod auth;
pub mod protocol;
pub mod agent_engine;
pub mod channel_handlers;

pub mod app_state;
pub mod middleware;
pub mod response;
pub mod router_trait;
pub mod router_registry;
pub mod keygen;

pub mod routes {
    pub mod auth_routes;
    pub mod config_routes;
    pub mod conversation_routes;
    pub mod tool_routes;
    pub mod git_routes;
    pub mod skill_routes;
    pub mod skill_installer;
    pub mod agent_routes;
    pub mod channel_routes;
    pub mod persona_routes;
    pub mod browser_routes;
    pub mod memory_routes;
    pub mod system_routes;
    pub mod system_agent_routes;
    pub mod multi_agent_routes;
    pub mod fs_skill_routes;
    pub mod iso_routes;
    pub mod cmd_routes;
    pub mod harness_routes;
    pub mod cron_routes;
    pub mod hook_routes;
    pub mod weixin_routes;
    pub mod automation_routes;
}

// Re-export for convenient access from router
pub use channel_handlers::*;
