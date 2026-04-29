// Claw Desktop - Hook系统 - 事件钩子的注册和触发
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tokio::sync::RwLock;
use claw_db::db::get_db;
use sea_orm::{ConnectionTrait, Statement};

/// 钩子事件类型 — 定义所有可触发Hook的事件
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum HookEvent {
    PreToolCall,
    PostToolCall,
    PreLlmCall,
    PostLlmCall,
    OnSessionStart,
    OnSessionEnd,
    OnSessionReset,
    OnMessageReceived,
    OnMessageSent,
}

impl HookEvent {
    /// 返回事件对应的字符串标识
    pub fn as_str(&self) -> &str {
        match self {
            HookEvent::PreToolCall => "pre_tool_call",
            HookEvent::PostToolCall => "post_tool_call",
            HookEvent::PreLlmCall => "pre_llm_call",
            HookEvent::PostLlmCall => "post_llm_call",
            HookEvent::OnSessionStart => "on_session_start",
            HookEvent::OnSessionEnd => "on_session_end",
            HookEvent::OnSessionReset => "on_session_reset",
            HookEvent::OnMessageReceived => "on_message_received",
            HookEvent::OnMessageSent => "on_message_sent",
        }
    }

    /// 从字符串解析事件类型，未知字符串返回None
    pub fn from_str_value(s: &str) -> Option<Self> {
        match s {
            "pre_tool_call" => Some(HookEvent::PreToolCall),
            "post_tool_call" => Some(HookEvent::PostToolCall),
            "pre_llm_call" => Some(HookEvent::PreLlmCall),
            "post_llm_call" => Some(HookEvent::PostLlmCall),
            "on_session_start" => Some(HookEvent::OnSessionStart),
            "on_session_end" => Some(HookEvent::OnSessionEnd),
            "on_session_reset" => Some(HookEvent::OnSessionReset),
            "on_message_received" => Some(HookEvent::OnMessageReceived),
            "on_message_sent" => Some(HookEvent::OnMessageSent),
            _ => None,
        }
    }
}

/// 钩子上下文 — 事件触发时传递的数据
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HookContext {
    pub event: HookEvent,
    pub data: HashMap<String, serde_json::Value>,
}

/// 钩子动作 — Hook处理后的返回动作
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum HookAction {
    Continue,
    Skip,
    Modify(HashMap<String, serde_json::Value>),
}

/// 钩子定义 — 单个Hook的配置信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HookDefinition {
    pub id: String,
    pub name: String,
    pub event: HookEvent,
    pub pattern: Option<String>,
    pub handler_type: String,
    pub handler_config: serde_json::Value,
    pub priority: i32,
    pub enabled: bool,
}

/// 钩子注册表 — 管理所有Hook的注册、触发和持久化
pub struct HookRegistry {
    hooks: RwLock<Vec<HookDefinition>>,
}

impl HookRegistry {
    /// 创建空的钩子注册表
    pub fn new() -> Self {
        Self { hooks: RwLock::new(Vec::new()) }
    }

    /// 从数据库加载所有活跃的Hook定义
    pub async fn load_from_db(&self) -> Result<(), String> {
        let db = get_db().await;
        let rows = db.query_all(Statement::from_sql_and_values(
            db.get_database_backend(),
            "SELECT * FROM hooks WHERE is_active = 1 ORDER BY priority ASC",
            [],
        )).await.map_err(|e| e.to_string())?;

        let mut hooks = Vec::new();
        for row in rows {
            let event_str = row.try_get::<String>("", "event_type").unwrap_or_default();
            if let Some(event) = HookEvent::from_str_value(&event_str) {
                hooks.push(HookDefinition {
                    id: row.try_get::<String>("", "id").unwrap_or_default(),
                    name: row.try_get::<String>("", "name").unwrap_or_default(),
                    event,
                    pattern: row.try_get::<Option<String>>("", "pattern").ok().flatten(),
                    handler_type: row.try_get::<String>("", "handler_type").unwrap_or_default(),
                    handler_config: row.try_get::<String>("", "handler_config")
                        .ok()
                        .and_then(|s| serde_json::from_str(&s).ok())
                        .unwrap_or(serde_json::Value::Object(Default::default())),
                    priority: 0,
                    enabled: row.try_get::<i32>("", "is_active").unwrap_or(1) == 1,
                });
            }
        }

        let mut guard = self.hooks.write().await;
        *guard = hooks;
        log::info!("[HookRegistry] Loaded {} hooks from DB", guard.len());
        Ok(())
    }

    /// 注册新Hook — 同时写入数据库和内存，按优先级排序
    pub async fn register(&self, hook: HookDefinition) -> Result<(), String> {
        let db = get_db().await;
        let now = chrono::Utc::now().timestamp();
        let config_str = serde_json::to_string(&hook.handler_config).unwrap_or_default();
        db.execute(Statement::from_sql_and_values(
            db.get_database_backend(),
            "INSERT OR REPLACE INTO hooks (id, agent_id, event_type, pattern, handler_type, handler_config, is_active, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?8)",
            [
                hook.id.clone().into(), String::new().into(), hook.event.as_str().into(),
                hook.pattern.clone().into(), hook.handler_type.clone().into(), config_str.into(),
                hook.enabled.into(), now.into(),
            ],
        )).await.map_err(|e| e.to_string())?;

        let mut guard = self.hooks.write().await;
        guard.push(hook);
        guard.sort_by(|a, b| a.priority.cmp(&b.priority));
        Ok(())
    }

    /// 注销Hook — 从数据库和内存中同时删除
    pub async fn unregister(&self, id: &str) -> Result<(), String> {
        let db = get_db().await;
        db.execute(Statement::from_sql_and_values(
            db.get_database_backend(),
            "DELETE FROM hooks WHERE id = ?1",
            [id.into()],
        )).await.map_err(|e| e.to_string())?;

        let mut guard = self.hooks.write().await;
        guard.retain(|h| h.id != id);
        Ok(())
    }

    /// 触发事件 — 按优先级依次执行匹配的Hook处理器
    ///
    /// 支持三种处理器类型: log(日志记录)、filter(过滤跳过)、modify(修改上下文)
    /// pattern匹配: 检查上下文数据中是否包含pattern字符串
    pub async fn fire_event(&self, event: HookEvent, mut context: HookContext) -> HookAction {
        let hooks = self.hooks.read().await;
        for hook in hooks.iter().filter(|h| h.event == event && h.enabled) {
            if let Some(pattern) = &hook.pattern {
                let matches = context.data.values().any(|v| {
                    v.as_str().map(|s| s.contains(pattern)).unwrap_or(false)
                });
                if !matches { continue; }
            }

            match hook.handler_type.as_str() {
                "log" => {
                    log::info!("[Hook:{}] Fired for {:?}", hook.name, event);
                }
                "filter" => {
                    if let Some(filter_key) = hook.handler_config.get("key").and_then(|v| v.as_str()) {
                        if context.data.contains_key(filter_key) {
                            return HookAction::Skip;
                        }
                    }
                }
                "modify" => {
                    if let Some(mods) = hook.handler_config.get("modifications").and_then(|v| v.as_object()) {
                        for (k, v) in mods {
                            context.data.insert(k.clone(), v.clone());
                        }
                        return HookAction::Modify(context.data.clone());
                    }
                }
                _ => {}
            }
        }
        HookAction::Continue
    }

    /// 列出所有已注册的Hook
    pub async fn list_hooks(&self) -> Vec<HookDefinition> {
        self.hooks.read().await.clone()
    }
}

impl Default for HookRegistry {
    fn default() -> Self {
        Self::new()
    }
}
