// Claw Desktop - 渠道处理器 - 处理渠道相关的WS消息
use std::sync::{OnceLock, Arc};
use claw_channel::*;

static CHANNEL_REGISTRY: OnceLock<Arc<ChannelRegistry>> = OnceLock::new();

/// 初始化Channel注册表 — 在WS引导时调用
pub fn init_registry(registry: Arc<ChannelRegistry>) {
    CHANNEL_REGISTRY.get_or_init(|| registry);
}

/// 获取Channel注册表引用
pub fn get_registry() -> Option<&'static ChannelRegistry> {
    CHANNEL_REGISTRY.get().map(|arc| arc.as_ref())
}

// ====== Channel 管理路由 ======

/// 处理渠道列表请求 — 返回已注册渠道和账号信息
pub async fn handle_channel_list(_req: &crate::ws::protocol::WsRequest) -> Result<serde_json::Value, String> {
    let registry = get_registry().ok_or("Channel registry not initialized")?;

    let channels: Vec<ChannelMeta> = registry.list_registered_channels().await;
    let accounts = match registry.config_manager() {
        Some(mgr) => mgr.list_accounts().await.map_err(|e| e.to_string())?,
        None => vec![],
    };

    Ok(serde_json::json!({
        "channels": channels,
        "accounts": accounts,
    }))
}

pub async fn handle_channel_status(req: &crate::ws::protocol::WsRequest) -> Result<serde_json::Value, String> {
    let account_id = req.params["account_id"]
        .as_str()
        .ok_or("Missing account_id")?;

    let registry = get_registry().ok_or("Channel registry not initialized")?;
    let status: ChannelStatus = registry
        .get_account_status(account_id)
        .await
        .map_err(|e| e.to_string())?;

    Ok(serde_json::to_value(status).map_err(|e| format!("{:?}", e))?)
}

pub async fn handle_channel_create_account(req: &crate::ws::protocol::WsRequest) -> Result<serde_json::Value, String> {
    let channel_type_str = req.params["channel_type"]
        .as_str()
        .ok_or("Missing channel_type")?;
    let name = req.params["name"].as_str().ok_or("Missing name")?;
    let config_json = &req.params["config"];

    let channel_id = ChannelId::from_str(channel_type_str);

    let auth_fields: std::collections::HashMap<String, String> =
        serde_json::from_value(config_json.clone()).map_err(|e| e.to_string())?;

    let account_id = uuid::Uuid::new_v4().to_string();

    let account = ChannelAccountConfig {
        id: account_id.clone(),
        channel_id,
        name: name.to_string(),
        enabled: true,
        auth_fields,
        dm_policy: Default::default(),
        group_policy: Default::default(),
        streaming_config: Default::default(),
        status: config::ChannelAccountStatus::Configured,
        last_error: None,
    };

    let registry = get_registry().ok_or("Channel registry not initialized")?;
    let manager = registry
        .config_manager()
        .ok_or("Database not connected")?;

    manager
        .create_account(&account)
        .await
        .map_err(|e| e.to_string())?;

    log::info!("[Channel] Created account: {} ({})", name, account_id);

    Ok(serde_json::json!({
        "success": true,
        "account_id": account_id,
    }))
}

pub async fn handle_channel_update_account(req: &crate::ws::protocol::WsRequest) -> Result<serde_json::Value, String> {
    let account_id = req.params["account_id"]
        .as_str()
        .ok_or("Missing account_id")?;
    let name = req.params["name"]
        .as_str()
        .ok_or("Missing name")?;
    let enabled = req.params["enabled"].as_bool().unwrap_or(true);
    let config_json = &req.params["config"];

    let registry = get_registry().ok_or("Channel registry not initialized")?;
    let mut account: ChannelAccountConfig = registry
        .config_manager()
        .ok_or("Database not connected")?
        .get_account(account_id)
        .await
        .map_err(|e| e.to_string())?;

    account.name = name.to_string();
    account.enabled = enabled;

    if !config_json.is_null() {
        let new_fields: std::collections::HashMap<String, String> =
            serde_json::from_value(config_json.clone()).map_err(|e| e.to_string())?;
        account.auth_fields = new_fields;
    }

    if let Some(dm_policy) = req.params.get("dm_policy") {
        account.dm_policy =
            serde_json::from_value(dm_policy.clone()).map_err(|e| e.to_string())?;
    }

    if let Some(group_policy) = req.params.get("group_policy") {
        account.group_policy =
            serde_json::from_value(group_policy.clone()).map_err(|e| e.to_string())?;
    }

    if let Some(streaming) = req.params.get("streaming_config") {
        account.streaming_config =
            serde_json::from_value(streaming.clone()).map_err(|e| e.to_string())?;
    }

    registry
        .config_manager()
        .ok_or_else(|| "Config manager not initialized".to_string())?
        .update_account(&account)
        .await
        .map_err(|e| e.to_string())?;

    Ok(serde_json::json!({ "success": true }))
}

pub async fn handle_channel_delete_account(req: &crate::ws::protocol::WsRequest) -> Result<serde_json::Value, String> {
    let account_id = req.params["account_id"]
        .as_str()
        .ok_or("Missing account_id")?;

    let registry = get_registry().ok_or("Channel registry not initialized")?;

    // 先停止账户
    let _ = registry.stop_account(account_id).await;

    // 删除配置
    let manager = registry
        .config_manager()
        .ok_or("Database not connected")?;

    manager
        .delete_account(account_id)
        .await
        .map_err(|e| e.to_string())?;

    log::info!("[Channel] Deleted account: {}", account_id);

    Ok(serde_json::json!({ "success": true }))
}

pub async fn handle_channel_toggle(req: &crate::ws::protocol::WsRequest) -> Result<serde_json::Value, String> {
    let account_id = req.params["account_id"]
        .as_str()
        .ok_or("Missing account_id")?;
    let enabled = req.params["enabled"]
        .as_bool()
        .ok_or("Missing enabled")?;

    let registry = get_registry().ok_or("Channel registry not initialized")?;
    let manager = registry
        .config_manager()
        .ok_or("Database not connected")?;

    if enabled {
        manager
            .toggle_account(account_id, true)
            .await
            .map_err(|e| e.to_string())?;
        let _ = registry.start_account(account_id).await;
    } else {
        let _ = registry.stop_account(account_id).await;
        manager
            .toggle_account(account_id, false)
            .await
            .map_err(|e| e.to_string())?;
    }

    Ok(serde_json::json!({ "success": true }))
}

pub async fn handle_channel_test_connection(req: &crate::ws::protocol::WsRequest) -> Result<serde_json::Value, String> {
    let account_id = req.params["account_id"]
        .as_str()
        .ok_or("Missing account_id")?;

    let registry = get_registry().ok_or("Channel registry not initialized")?;

    let result: bool = registry
        .test_connection(account_id)
        .await
        .map_err(|e| e.to_string())?;

    Ok(serde_json::json!({
        "success": true,
        "connected": result,
    }))
}

// ====== Channel 消息路由 ======

pub async fn handle_channel_send_message(req: &crate::ws::protocol::WsRequest) -> Result<serde_json::Value, String> {
    let account_id = req.params["account_id"]
        .as_str()
        .ok_or("Missing account_id")?;
    let target_id = req.params["target_id"]
        .as_str()
        .ok_or("Missing target_id")?;
    let text = req.params["text"]
        .as_str()
        .ok_or("Missing text")?;
    let chat_type_str = req.params["chat_type"]
        .as_str()
        .unwrap_or("direct");

    let chat_type = match chat_type_str {
        "group" => ChatType::Group,
        "channel" => ChatType::Channel,
        _ => ChatType::Direct,
    };

    let registry = get_registry().ok_or("Channel registry not initialized")?;
    let account: ChannelAccountConfig = registry
        .config_manager()
        .ok_or("Database not connected")?
        .get_account(account_id)
        .await
        .map_err(|e| e.to_string())?;

    let msg = OutboundMessage::new(
        account.channel_id.clone(),
        account_id.to_string(),
        target_id.to_string(),
        chat_type,
        MessageContent::Text { text: text.to_string() },
    );

    let result: SendResult = registry
        .send_message(&msg)
        .await
        .map_err(|e| e.to_string())?;

    Ok(serde_json::to_value(result).map_err(|e| format!("{:?}", e))?)
}

pub async fn handle_channel_get_schema(req: &crate::ws::protocol::WsRequest) -> Result<serde_json::Value, String> {
    let channel_type_str = req.params["channel_type"]
        .as_str()
        .ok_or("Missing channel_type")?;

    let registry = get_registry().ok_or("Channel registry not initialized")?;
    let channels: Vec<ChannelMeta> = registry.list_registered_channels().await;

    let schema = channels
        .into_iter()
        .find(|c| c.id.as_str() == channel_type_str)
        .map(|c| c.config_fields)
        .unwrap_or_default();

    Ok(serde_json::json!({ "fields": schema }))
}
