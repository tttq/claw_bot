// Claw Desktop - 渠道引导 - 启动和初始化消息渠道
use std::sync::{Arc, OnceLock};

static REGISTRY: OnceLock<Arc<crate::ChannelRegistry>> = OnceLock::new();

pub async fn bootstrap() -> Result<Arc<crate::ChannelRegistry>, String> {
    if let Some(registry) = REGISTRY.get() {
        return Ok(registry.clone());
    }

    use crate::plugins::{DiscordPlugin, TelegramPlugin, WeixinPlugin};
    use claw_db::db::get_db;

    let db_conn = get_db().await;
    let registry = crate::ChannelRegistry::with_db((*db_conn).clone());

    registry.register(TelegramPlugin::new()).await;
    registry.register(DiscordPlugin::new()).await;
    registry.register(WeixinPlugin::new()).await;

    log::info!("[ChannelBootstrap] Initialized with Telegram, Discord & WeChat plugins");

    let arc_registry = Arc::new(registry);
    REGISTRY
        .set(arc_registry.clone())
        .map_err(|_| "ChannelRegistry already initialized".to_string())?;
    Ok(arc_registry)
}

pub fn get_registry() -> Option<Arc<crate::ChannelRegistry>> {
    REGISTRY.get().map(|r| r.clone())
}

pub fn is_initialized() -> bool {
    REGISTRY.get().is_some()
}
