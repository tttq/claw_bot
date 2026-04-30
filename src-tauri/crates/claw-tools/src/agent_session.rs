// Claw Desktop - Agent会话 - 管理单个Agent的会话状态（Sea-ORM 1.1 稳定版）

use claw_db::db::agent_entities::agent_configs::Entity as AgentConfigs;
use claw_db::db::agent_entities::agent_sessions::Entity as AgentSessions;
use claw_db::db::agent_entities::agent_vectors::Entity as AgentVectors;
use claw_db::db::agent_entities::agent_workspace_files::Entity as AgentWorkspaceFiles;
use claw_db::db::agent_entities::agents::Entity as Agents;
use claw_db::db::get_agent_db;
use log;
use sea_orm::prelude::*;
use sea_orm::{ActiveModelTrait, EntityTrait, QueryOrder, Set, Statement};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// 隔离Agent数据结构 — 对应数据库agents表的完整业务模型
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct IsolatedAgent {
    pub id: String,
    pub display_name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub purpose: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scope: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model_override: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system_prompt: Option<String>,
    #[serde(default)]
    pub tools_config: serde_json::Value,
    #[serde(default)]
    pub skills_enabled: serde_json::Value,
    pub max_turns: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub workspace_path: Option<String>,
    pub is_active: bool,
    pub created_at: i64,
    pub updated_at: i64,
    pub conversation_count: u64,
    pub total_messages: u64,
}

/// Agent会话数据结构 — 记录Agent的运行会话状态
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentSession {
    pub id: String,
    pub agent_id: String,
    pub conversation_id: Option<String>,
    pub status: String,
    pub turn_count: u32,
    pub total_tokens_used: f64,
    pub started_at: i64,
    pub last_active: Option<i64>,
}

/// 从数据库Model转换为IsolatedAgent业务结构
impl From<claw_db::db::agent_entities::agents::Model> for IsolatedAgent {
    fn from(m: claw_db::db::agent_entities::agents::Model) -> Self {
        IsolatedAgent {
            id: m.id,
            display_name: m.display_name,
            description: m.description,
            purpose: m.purpose,
            scope: m.scope,
            model_override: m.model_override,
            system_prompt: m.system_prompt,
            tools_config: m
                .tools_config
                .and_then(|s: String| serde_json::from_str(s.as_str()).ok())
                .unwrap_or(serde_json::json!({})),
            skills_enabled: m
                .skills_enabled
                .and_then(|s: String| serde_json::from_str(s.as_str()).ok())
                .unwrap_or(serde_json::json!([])),
            max_turns: std::cmp::max(m.max_turns, 0) as u32,
            temperature: m.temperature,
            workspace_path: m.workspace_path,
            is_active: m.is_active != 0,
            created_at: m.created_at,
            updated_at: m.updated_at,
            conversation_count: std::cmp::max(m.conversation_count, 0) as u64,
            total_messages: std::cmp::max(m.total_messages, 0) as u64,
        }
    }
}

/// 从数据库Model转换为AgentSession业务结构
impl From<claw_db::db::agent_entities::agent_sessions::Model> for AgentSession {
    fn from(m: claw_db::db::agent_entities::agent_sessions::Model) -> Self {
        AgentSession {
            id: m.id,
            agent_id: m.agent_id,
            conversation_id: m.conversation_id,
            status: m.status,
            turn_count: std::cmp::max(m.turn_count, 0) as u32,
            total_tokens_used: m.total_tokens_used,
            started_at: m.started_at,
            last_active: m.last_active,
        }
    }
}

/// 生成Agent唯一ID
fn generate_agent_id() -> String {
    Uuid::new_v4().to_string()
}

/// Agent会话管理器 — 提供Agent和会话的CRUD操作
pub struct AgentSessionManager;

impl AgentSessionManager {
    /// 初始化Agent数据库 — 创建所有必要的表结构和索引
    pub async fn init(agent_db_path: &std::path::Path) -> Result<(), anyhow::Error> {
        if let Some(parent) = agent_db_path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| anyhow::anyhow!(e))?;
        }
        let db = claw_db::db::init_agent_db(agent_db_path.to_str().ok_or_else(|| {
            anyhow::anyhow!("agent db path is not valid UTF-8: {:?}", agent_db_path)
        })?)
        .await?;
        log::info!("[AgentDB] 初始化表结构...");
        let backend = db.get_database_backend();
        db.execute(Statement::from_string(
            backend,
            "CREATE TABLE IF NOT EXISTS agents (
                id TEXT PRIMARY KEY,
                display_name TEXT NOT NULL DEFAULT '',
                description TEXT DEFAULT NULL,
                purpose TEXT DEFAULT NULL,
                scope TEXT DEFAULT NULL,
                model_override TEXT DEFAULT NULL,
                system_prompt TEXT DEFAULT NULL,
                tools_config TEXT DEFAULT '{}',
                skills_enabled TEXT DEFAULT '[]',
                max_turns INTEGER NOT NULL DEFAULT 20,
                temperature REAL DEFAULT 0.7,
                workspace_path TEXT DEFAULT NULL,
                is_active INTEGER NOT NULL DEFAULT 1,
                created_at INTEGER NOT NULL,
                updated_at INTEGER NOT NULL,
                conversation_count INTEGER NOT NULL DEFAULT 0,
                total_messages INTEGER NOT NULL DEFAULT 0
            )",
        ))
        .await?;
        db.execute(Statement::from_string(
            backend,
            "CREATE UNIQUE INDEX IF NOT EXISTS idx_agents_id ON agents(id)",
        ))
        .await?;
        db.execute(Statement::from_string(
            backend,
            "CREATE TABLE IF NOT EXISTS agent_sessions (
                id TEXT PRIMARY KEY,
                agent_id TEXT NOT NULL,
                conversation_id TEXT DEFAULT NULL,
                status TEXT NOT NULL DEFAULT 'active',
                turn_count INTEGER NOT NULL DEFAULT 0,
                total_tokens_used REAL NOT NULL DEFAULT 0,
                started_at INTEGER NOT NULL,
                last_active INTEGER DEFAULT NULL
            )",
        ))
        .await?;
        db.execute(Statement::from_string(
            backend,
            "CREATE INDEX IF NOT EXISTS idx_sessions_agent ON agent_sessions(agent_id)",
        ))
        .await?;
        db.execute(Statement::from_string(
            backend,
            "CREATE TABLE IF NOT EXISTS agent_configs (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                agent_id TEXT NOT NULL,
                config_key TEXT NOT NULL,
                config_value TEXT NOT NULL,
                updated_at INTEGER NOT NULL,
                UNIQUE(agent_id, config_key)
            )",
        ))
        .await?;
        db.execute(Statement::from_string(
            backend,
            "CREATE TABLE IF NOT EXISTS agent_workspace_files (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                agent_id TEXT NOT NULL,
                session_id TEXT DEFAULT NULL,
                relative_path TEXT NOT NULL,
                full_path TEXT NOT NULL,
                file_size INTEGER NOT NULL DEFAULT 0,
                content_type TEXT DEFAULT NULL,
                indexed_at INTEGER NOT NULL
            )",
        ))
        .await?;
        db.execute(Statement::from_string(
            backend,
            "CREATE INDEX IF NOT EXISTS idx_ws_agent ON agent_workspace_files(agent_id)",
        ))
        .await?;
        db.execute(Statement::from_string(
            backend,
            "CREATE TABLE IF NOT EXISTS agent_vectors (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                agent_id TEXT NOT NULL,
                chunk_index INTEGER NOT NULL,
                content_hash TEXT NOT NULL,
                vector BLOB NOT NULL,
                source_type TEXT DEFAULT NULL,
                created_at INTEGER NOT NULL
            )",
        ))
        .await?;
        db.execute(Statement::from_string(
            backend,
            "CREATE INDEX IF NOT EXISTS idx_vectors_agent ON agent_vectors(agent_id)",
        ))
        .await?;
        db.execute(Statement::from_string(
            backend,
            "CREATE TABLE IF NOT EXISTS agent_profiles (
                agent_id TEXT PRIMARY KEY,
                profile_json TEXT NOT NULL DEFAULT '{}',
                interaction_count INTEGER NOT NULL DEFAULT 0,
                last_updated_at INTEGER NOT NULL DEFAULT 0,
                created_at INTEGER NOT NULL DEFAULT 0
            )",
        ))
        .await?;
        db.execute(Statement::from_string(backend, "PRAGMA journal_mode=WAL"))
            .await?;
        db.execute(Statement::from_string(backend, "PRAGMA foreign_keys=ON"))
            .await?;
        log::info!("[AgentDB] 所有表初始化完成");
        Ok(())
    }

    // ==================== Agent CRUD ====================

    /// 创建隔离Agent — 根据类别自动生成系统提示词，插入数据库
    pub async fn create_isolated_agent_ext(
        name: &str,
        description: &str,
        system_prompt: &str,
        purpose: Option<&str>,
        scope: Option<&str>,
        category: Option<&str>,
        model_override: Option<&str>,
    ) -> Result<IsolatedAgent, anyhow::Error> {
        let db = get_agent_db().await;
        let now = chrono::Utc::now().timestamp();
        let id = generate_agent_id();
        log::info!(
            "[AgentDB] CREATE agent: name={}, id={}, category={:?}",
            name,
            claw_types::truncate_str_safe(&id, 16),
            category
        );

        let effective_prompt = if system_prompt.is_empty() {
            let cat = category.unwrap_or("general");
            let role_hint = match cat {
                "code" => format!(
                    "You are {}, an expert coding assistant. You write clean, efficient, and well-tested code. You follow best practices and explain your reasoning.",
                    name
                ),
                "search" => format!(
                    "You are {}, a research and information retrieval specialist. You find, analyze, and synthesize information from multiple sources.",
                    name
                ),
                "analysis" => format!(
                    "You are {}, an analytical thinker who breaks down complex problems into structured components. You provide data-driven insights.",
                    name
                ),
                "creative" => format!(
                    "You are {}, a creative assistant skilled in writing, brainstorming, and generating innovative ideas.",
                    name
                ),
                _ => format!("You are {}, a helpful AI assistant.", name),
            };
            Some(role_hint)
        } else {
            Some(system_prompt.to_string())
        };

        let new_agent = claw_db::db::agent_entities::agents::ActiveModel {
            id: Set(id.clone()),
            display_name: Set(name.to_string()),
            description: Set(if description.is_empty() {
                None
            } else {
                Some(description.to_string())
            }),
            purpose: Set(purpose.map(|s| s.to_string())),
            scope: Set(scope.map(|s| s.to_string())),
            model_override: Set(model_override.map(|s| s.to_string())),
            system_prompt: Set(effective_prompt.clone()),
            tools_config: Set(Some(serde_json::json!({"enabled": []}).to_string())),
            skills_enabled: Set(Some(serde_json::json!([]).to_string())),
            max_turns: Set(20),
            temperature: Set(Some(0.7)),
            is_active: Set(1),
            created_at: Set(now),
            updated_at: Set(now),
            ..Default::default()
        };
        new_agent.insert(db).await?;

        if let Some(cat) = category {
            let am = claw_db::db::agent_entities::agent_configs::ActiveModel {
                agent_id: Set(id.clone()),
                config_key: Set("category".to_string()),
                config_value: Set(cat.to_string()),
                updated_at: Set(now),
                ..Default::default()
            };
            am.insert(db).await.ok();
        }

        Ok(IsolatedAgent {
            id: id.clone(),
            display_name: name.to_string(),
            description: if description.is_empty() {
                None
            } else {
                Some(description.to_string())
            },
            purpose: purpose.map(|s| s.to_string()),
            scope: scope.map(|s| s.to_string()),
            model_override: model_override.map(|s| s.to_string()),
            system_prompt: effective_prompt,
            tools_config: serde_json::json!({"enabled": []}),
            skills_enabled: serde_json::json!([]),
            max_turns: 20,
            temperature: Some(0.7),
            is_active: true,
            created_at: now,
            updated_at: now,
            conversation_count: 0,
            total_messages: 0,
            workspace_path: None,
        })
    }

    /// 获取指定ID的Agent
    pub async fn get_agent(agent_id: &str) -> Result<Option<IsolatedAgent>, anyhow::Error> {
        let db = get_agent_db().await;
        log::info!(
            "[AgentDB] SELECT agent WHERE id={}",
            claw_types::truncate_str_safe(agent_id, 16)
        );
        match Agents::find_by_id(agent_id.to_string()).one(db).await? {
            Some(model) => Ok(Some(IsolatedAgent::from(model))),
            None => Ok(None),
        }
    }

    /// 列出所有Agent — 按创建时间降序排列
    pub async fn list_all_agents() -> Result<Vec<IsolatedAgent>, anyhow::Error> {
        let db = get_agent_db().await;
        log::info!("[AgentDB] SELECT * FROM agents ORDER BY created_at DESC");
        let models: Vec<claw_db::db::agent_entities::agents::Model> = Agents::find()
            .order_by_desc(claw_db::db::agent_entities::agents::Column::CreatedAt)
            .all(db)
            .await?;
        Ok(models.into_iter().map(IsolatedAgent::from).collect())
    }

    /// 重命名Agent — 更新显示名称
    pub async fn rename_agent(
        agent_id: &str,
        new_name: &str,
    ) -> Result<IsolatedAgent, anyhow::Error> {
        let db = get_agent_db().await;
        log::info!(
            "[AgentDB] UPDATE agent SET display_name={} WHERE id={}",
            new_name,
            claw_types::truncate_str_safe(agent_id, 16)
        );
        let agent = Agents::find_by_id(agent_id.to_string())
            .one(db)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Agent not found"))?;
        let mut active: claw_db::db::agent_entities::agents::ActiveModel = agent.into();
        active.display_name = Set(new_name.to_string());
        active.updated_at = Set(chrono::Utc::now().timestamp());
        let model = active.update(db).await?;
        Ok(IsolatedAgent::from(model))
    }

    /// 删除Agent — 级联删除关联的会话、配置、工作区文件和向量数据
    pub async fn delete_agent(agent_id: &str) -> Result<(), anyhow::Error> {
        let db = get_agent_db().await;
        log::info!(
            "[AgentDB] CASCADE DELETE agent WHERE id={}",
            claw_types::truncate_str_safe(agent_id, 16)
        );
        AgentSessions::delete_many()
            .filter(claw_db::db::agent_entities::agent_sessions::Column::AgentId.eq(agent_id))
            .exec(db)
            .await?;
        AgentConfigs::delete_many()
            .filter(claw_db::db::agent_entities::agent_configs::Column::AgentId.eq(agent_id))
            .exec(db)
            .await?;
        AgentWorkspaceFiles::delete_many()
            .filter(
                claw_db::db::agent_entities::agent_workspace_files::Column::AgentId.eq(agent_id),
            )
            .exec(db)
            .await?;
        AgentVectors::delete_many()
            .filter(claw_db::db::agent_entities::agent_vectors::Column::AgentId.eq(agent_id))
            .exec(db)
            .await?;
        Agents::delete_by_id(agent_id.to_string()).exec(db).await?;
        Ok(())
    }

    // ==================== Config ====================

    /// 设置Agent的工具配置 — 更新tools_config字段
    pub async fn set_tools_config(
        agent_id: &str,
        config: &serde_json::Value,
    ) -> Result<(), anyhow::Error> {
        let db = get_agent_db().await;
        log::info!(
            "[AgentDB] UPDATE agent SET tools_config={} WHERE id={}",
            config,
            claw_types::truncate_str_safe(agent_id, 16)
        );
        let agent = Agents::find_by_id(agent_id.to_string())
            .one(db)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Agent not found"))?;
        let mut active: claw_db::db::agent_entities::agents::ActiveModel = agent.into();
        active.tools_config = Set(Some(config.to_string()));
        active.updated_at = Set(chrono::Utc::now().timestamp());
        active.update(db).await?;
        Ok(())
    }

    /// 设置Agent启用的技能列表
    pub async fn set_skills_enabled(
        agent_id: &str,
        enabled: &[String],
    ) -> Result<(), anyhow::Error> {
        let db = get_agent_db().await;
        log::info!(
            "[AgentDB] UPDATE agent SET skills_enabled={:?} WHERE id={}",
            enabled,
            claw_types::truncate_str_safe(agent_id, 16)
        );
        let agent = Agents::find_by_id(agent_id.to_string())
            .one(db)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Agent not found"))?;
        let mut active: claw_db::db::agent_entities::agents::ActiveModel = agent.into();
        active.skills_enabled = Set(Some(serde_json::to_string(enabled)?));
        active.updated_at = Set(chrono::Utc::now().timestamp());
        active.update(db).await?;
        Ok(())
    }

    /// 设置Agent配置项 — 支持UPSERT（存在则更新，不存在则插入）
    pub async fn set_config(agent_id: &str, key: &str, value: &str) -> Result<(), anyhow::Error> {
        let db = get_agent_db().await;
        let now = chrono::Utc::now().timestamp();
        log::info!(
            "[AgentDB] UPSERT config: agent={}, key={}",
            claw_types::truncate_str_safe(agent_id, 16),
            key
        );
        let existing: Option<claw_db::db::agent_entities::agent_configs::Model> =
            AgentConfigs::find()
                .filter(claw_db::db::agent_entities::agent_configs::Column::AgentId.eq(agent_id))
                .filter(claw_db::db::agent_entities::agent_configs::Column::ConfigKey.eq(key))
                .one(db)
                .await?;
        match existing {
            Some(m) => {
                let mut am: claw_db::db::agent_entities::agent_configs::ActiveModel = m.into();
                am.config_value = Set(value.to_string());
                am.updated_at = Set(now);
                am.update(db).await?;
            }
            None => {
                claw_db::db::agent_entities::agent_configs::ActiveModel {
                    agent_id: Set(agent_id.to_string()),
                    config_key: Set(key.to_string()),
                    config_value: Set(value.to_string()),
                    updated_at: Set(now),
                    ..Default::default()
                }
                .insert(db)
                .await?;
            }
        }
        Ok(())
    }

    /// 获取Agent配置项 — 若Agent无自定义配置则回退到全局默认值
    pub async fn get_config(agent_id: &str, key: &str) -> Result<Option<String>, anyhow::Error> {
        let db = get_agent_db().await;
        log::info!(
            "[AgentDB] SELECT config: agent={}, key={}",
            claw_types::truncate_str_safe(agent_id, 16),
            key
        );
        let result: Option<claw_db::db::agent_entities::agent_configs::Model> =
            AgentConfigs::find()
                .filter(claw_db::db::agent_entities::agent_configs::Column::AgentId.eq(agent_id))
                .filter(claw_db::db::agent_entities::agent_configs::Column::ConfigKey.eq(key))
                .one(db)
                .await?;

        if let Some(m) = result {
            if !m.config_value.is_empty() {
                return Ok(Some(m.config_value));
            }
        }

        let global_default = Self::get_global_default_for_key(key).await;
        Ok(global_default)
    }

    /// 获取配置键对应的全局默认值 — 从AppConfig中读取模型相关默认配置
    async fn get_global_default_for_key(key: &str) -> Option<String> {
        let config = claw_config::config::AppConfig::load_or_create(
            &claw_config::path_resolver::config_path(),
        )
        .ok()?;
        let model = &config.model;
        match key {
            "agent_model_provider" => Some(model.provider.clone()),
            "agent_model_format" => Some(model.api_format.clone()),
            "agent_model_url" => Some(model.custom_url.clone()),
            "agent_model_key" => Some(model.custom_api_key.clone()),
            "agent_model_name" => Some(model.custom_model_name.clone()),
            "agent_model_default" => Some(model.default_model.clone()),
            "agent_temperature" => Some(model.temperature.to_string()),
            "agent_max_tokens" => Some(model.max_tokens.to_string()),
            "agent_top_p" => Some(model.top_p.to_string()),
            "agent_thinking_budget" => Some(model.thinking_budget.to_string()),
            "agent_stream_mode" => Some(if model.stream_mode {
                "true".to_string()
            } else {
                "false".to_string()
            }),
            _ => None,
        }
    }

    // ==================== Sessions ====================

    /// 创建Agent会话 — 关联Agent和对话ID
    pub async fn create_session(
        agent_id: &str,
        conversation_id: Option<&str>,
    ) -> Result<AgentSession, anyhow::Error> {
        let db = get_agent_db().await;
        let now = chrono::Utc::now().timestamp();
        let sid = generate_agent_id();
        log::info!(
            "[AgentDB] INSERT session: agent={}, conv={}",
            claw_types::truncate_str_safe(agent_id, 16),
            conversation_id.unwrap_or("none")
        );
        let new_session = claw_db::db::agent_entities::agent_sessions::ActiveModel {
            id: Set(sid.clone()),
            agent_id: Set(agent_id.to_string()),
            conversation_id: Set(conversation_id.map(String::from)),
            status: Set("active".into()),
            turn_count: Set(0),
            total_tokens_used: Set(0.0),
            started_at: Set(now),
            last_active: Set(Some(now)),
        };
        let model = new_session.insert(db).await?;
        Ok(AgentSession::from(model))
    }

    /// 列出Agent的所有会话 — 按开始时间降序排列
    pub async fn list_sessions(agent_id: &str) -> Result<Vec<AgentSession>, anyhow::Error> {
        let db = get_agent_db().await;
        log::info!(
            "[AgentDB] SELECT sessions WHERE agent_id={}",
            claw_types::truncate_str_safe(agent_id, 16)
        );
        let models: Vec<claw_db::db::agent_entities::agent_sessions::Model> = AgentSessions::find()
            .filter(claw_db::db::agent_entities::agent_sessions::Column::AgentId.eq(agent_id))
            .order_by_desc(claw_db::db::agent_entities::agent_sessions::Column::StartedAt)
            .all(db)
            .await?;
        Ok(models.into_iter().map(AgentSession::from).collect())
    }

    // ==================== Workspace ====================

    /// 索引Agent工作区文件 — 记录文件路径、大小和内容类型，支持UPSERT
    pub async fn index_workspace_file(
        agent_id: &str,
        relative_path: &str,
        full_path: &str,
        file_size: i64,
        content_type: Option<&str>,
    ) -> Result<(), anyhow::Error> {
        let db = get_agent_db().await;
        let now = chrono::Utc::now().timestamp();
        log::info!(
            "[AgentDB] UPSERT workspace_file: agent={}, path={}",
            claw_types::truncate_str_safe(agent_id, 16),
            relative_path
        );
        let existing: Option<claw_db::db::agent_entities::agent_workspace_files::Model> =
            AgentWorkspaceFiles::find()
                .filter(
                    claw_db::db::agent_entities::agent_workspace_files::Column::AgentId
                        .eq(agent_id),
                )
                .filter(
                    claw_db::db::agent_entities::agent_workspace_files::Column::RelativePath
                        .eq(relative_path),
                )
                .one(db)
                .await?;
        match existing {
            Some(m) => {
                let mut am: claw_db::db::agent_entities::agent_workspace_files::ActiveModel =
                    m.into();
                am.full_path = Set(full_path.to_string());
                am.file_size = Set(file_size);
                am.content_type = Set(content_type.map(String::from));
                am.indexed_at = Set(now);
                am.update(db).await?;
            }
            None => {
                claw_db::db::agent_entities::agent_workspace_files::ActiveModel {
                    agent_id: Set(agent_id.to_string()),
                    relative_path: Set(relative_path.to_string()),
                    full_path: Set(full_path.to_string()),
                    file_size: Set(file_size),
                    content_type: Set(content_type.map(String::from)),
                    indexed_at: Set(now),
                    ..Default::default()
                }
                .insert(db)
                .await?;
            }
        }
        Ok(())
    }

    /// 列出Agent工作区文件索引 — 返回(相对路径, 完整路径, 文件大小)元组
    pub async fn list_workspace_index(
        agent_id: &str,
    ) -> Result<Vec<(String, String, i64)>, anyhow::Error> {
        let db = get_agent_db().await;
        log::info!(
            "[AgentDB] SELECT workspace_files WHERE agent_id={}",
            claw_types::truncate_str_safe(agent_id, 16)
        );
        let files: Vec<claw_db::db::agent_entities::agent_workspace_files::Model> =
            AgentWorkspaceFiles::find()
                .filter(
                    claw_db::db::agent_entities::agent_workspace_files::Column::AgentId
                        .eq(agent_id),
                )
                .order_by_asc(
                    claw_db::db::agent_entities::agent_workspace_files::Column::RelativePath,
                )
                .all(db)
                .await?;
        Ok(files
            .into_iter()
            .map(|f| (f.relative_path, f.full_path, f.file_size))
            .collect())
    }

    // ==================== Cleanup ====================

    /// 清理过期数据 — 删除超过指定天数的会话和向量数据
    pub async fn cleanup_stale_data(days_threshold: i64) -> Result<u64, anyhow::Error> {
        let db = get_agent_db().await;
        let cutoff = chrono::Utc::now().timestamp() - (days_threshold * 86400);
        log::warn!("[AgentDB] 清理 {} 天前的过期数据", days_threshold);
        let del_sessions = AgentSessions::delete_many()
            .filter(claw_db::db::agent_entities::agent_sessions::Column::LastActive.lt(cutoff))
            .exec(db)
            .await?
            .rows_affected;
        let del_vectors = AgentVectors::delete_many()
            .filter(claw_db::db::agent_entities::agent_vectors::Column::CreatedAt.lt(cutoff))
            .exec(db)
            .await?
            .rows_affected;
        log::info!(
            "[AgentDB] 清理完成: sessions={}, vectors={}",
            del_sessions,
            del_vectors
        );
        Ok(del_sessions + del_vectors)
    }
}

// ==================== Tauri Commands (async for Sea-ORM) ====================

/// Tauri命令：初始化Agent数据库
#[tauri::command]
pub async fn iso_init_agent_db(_app_handle: tauri::AppHandle) -> Result<(), String> {
    AgentSessionManager::init(claw_config::path_resolver::agent_db_path().as_path())
        .await
        .map_err(|e| e.to_string())
}

/// Tauri命令：创建隔离Agent
#[tauri::command]
#[allow(non_snake_case)]
pub async fn iso_agent_create(
    name: String,
    description: String,
    systemPrompt: String,
    purpose: Option<String>,
    scope: Option<String>,
    category: Option<String>,
    modelOverride: Option<String>,
) -> Result<IsolatedAgent, String> {
    AgentSessionManager::create_isolated_agent_ext(
        &name,
        &description,
        &systemPrompt,
        purpose.as_deref(),
        scope.as_deref(),
        category.as_deref(),
        modelOverride.as_deref(),
    )
    .await
    .map_err(|e| e.to_string())
}

/// Tauri命令：列出所有Agent
#[tauri::command]
pub async fn iso_agent_list() -> Result<Vec<IsolatedAgent>, String> {
    AgentSessionManager::list_all_agents()
        .await
        .map_err(|e| e.to_string())
}

/// Tauri命令：获取指定Agent
#[tauri::command]
pub async fn iso_agent_get(id: String) -> Result<Option<IsolatedAgent>, String> {
    AgentSessionManager::get_agent(&id)
        .await
        .map_err(|e| e.to_string())
}

/// Tauri命令：重命名Agent
#[tauri::command]
#[allow(non_snake_case)]
pub async fn iso_agent_rename(id: String, newName: String) -> Result<IsolatedAgent, String> {
    AgentSessionManager::rename_agent(&id, &newName)
        .await
        .map_err(|e| e.to_string())
}

/// Tauri命令：删除Agent
#[tauri::command]
pub async fn iso_agent_delete(id: String) -> Result<(), String> {
    AgentSessionManager::delete_agent(&id)
        .await
        .map_err(|e| e.to_string())
}

/// Tauri命令：设置Agent工具配置
#[tauri::command]
#[allow(non_snake_case)]
pub async fn iso_set_tools_config(
    agentId: String,
    config: serde_json::Value,
) -> Result<(), String> {
    AgentSessionManager::set_tools_config(&agentId, &config)
        .await
        .map_err(|e| e.to_string())
}

/// Tauri命令：设置Agent启用的技能列表
#[tauri::command]
#[allow(non_snake_case)]
pub async fn iso_set_skills_enabled(agentId: String, enabled: Vec<String>) -> Result<(), String> {
    AgentSessionManager::set_skills_enabled(&agentId, &enabled)
        .await
        .map_err(|e| e.to_string())
}

/// Tauri命令：更新Agent配置（系统提示词、用途、范围、模型、轮次、温度）
#[tauri::command]
#[allow(non_snake_case)]
pub async fn iso_agent_update_config(
    agentId: String,
    systemPrompt: Option<String>,
    purpose: Option<String>,
    scope: Option<String>,
    modelOverride: Option<String>,
    maxTurns: Option<u32>,
    temperature: Option<f64>,
) -> Result<(), String> {
    let db = claw_db::db::try_get_agent_db().ok_or("Agent DB not initialized")?;
    let now = chrono::Utc::now().timestamp();

    let existing = claw_db::db::agent_entities::agents::Entity::find_by_id(agentId.clone())
        .one(db)
        .await
        .map_err(|e: sea_orm::DbErr| e.to_string())?
        .ok_or_else(|| format!("Agent '{}' not found", agentId))?;

    let mut am = claw_db::db::agent_entities::agents::ActiveModel {
        id: Set(agentId.clone()),
        display_name: Set(existing.display_name),
        description: Set(existing.description),
        updated_at: Set(now),
        ..Default::default()
    };

    if let Some(sp) = systemPrompt {
        am.system_prompt = Set(Some(sp));
    } else {
        am.system_prompt = Set(existing.system_prompt);
    }

    if let Some(p) = purpose {
        am.purpose = Set(Some(p));
    } else {
        am.purpose = Set(existing.purpose);
    }

    if let Some(s) = scope {
        am.scope = Set(Some(s));
    } else {
        am.scope = Set(existing.scope);
    }

    if let Some(m) = modelOverride {
        am.model_override = Set(Some(m));
    } else {
        am.model_override = Set(existing.model_override);
    }

    if let Some(t) = maxTurns {
        am.max_turns = Set(t as i32);
    } else {
        am.max_turns = Set(existing.max_turns);
    }

    if temperature.is_some() {
        am.temperature = Set(temperature);
    } else {
        am.temperature = Set(existing.temperature);
    }

    am.tools_config = Set(existing.tools_config);
    am.skills_enabled = Set(existing.skills_enabled);
    am.is_active = Set(existing.is_active);
    am.created_at = Set(existing.created_at);
    am.workspace_path = Set(existing.workspace_path);

    am.update(db)
        .await
        .map_err(|e: sea_orm::DbErr| e.to_string())?;
    Ok(())
}

/// Tauri命令：设置Agent配置项
#[tauri::command]
#[allow(non_snake_case)]
pub async fn iso_set_config(agentId: String, key: String, value: String) -> Result<(), String> {
    AgentSessionManager::set_config(&agentId, &key, &value)
        .await
        .map_err(|e| e.to_string())
}

/// Tauri命令：获取Agent配置项
#[tauri::command]
#[allow(non_snake_case)]
pub async fn iso_get_config(agentId: String, key: String) -> Result<Option<String>, String> {
    AgentSessionManager::get_config(&agentId, &key)
        .await
        .map_err(|e| e.to_string())
}

/// Tauri命令：创建Agent会话
#[tauri::command]
#[allow(non_snake_case)]
pub async fn iso_create_session(
    agentId: String,
    conversationId: Option<String>,
) -> Result<AgentSession, String> {
    AgentSessionManager::create_session(&agentId, conversationId.as_deref())
        .await
        .map_err(|e| e.to_string())
}

/// Tauri命令：列出Agent的所有会话
#[tauri::command]
#[allow(non_snake_case)]
pub async fn iso_list_sessions(agentId: String) -> Result<Vec<AgentSession>, String> {
    AgentSessionManager::list_sessions(&agentId)
        .await
        .map_err(|e| e.to_string())
}

/// Tauri命令：索引Agent工作区文件
#[tauri::command]
#[allow(non_snake_case)]
pub async fn iso_index_workspace(
    agentId: String,
    relativePath: String,
    fullPath: String,
    fileSize: i64,
    contentType: Option<String>,
) -> Result<(), String> {
    AgentSessionManager::index_workspace_file(
        &agentId,
        &relativePath,
        &fullPath,
        fileSize,
        contentType.as_deref(),
    )
    .await
    .map_err(|e| e.to_string())
}

/// Tauri命令：列出Agent工作区文件索引
#[tauri::command]
#[allow(non_snake_case)]
pub async fn iso_list_workspace(agentId: String) -> Result<Vec<(String, String, i64)>, String> {
    AgentSessionManager::list_workspace_index(&agentId)
        .await
        .map_err(|e| e.to_string())
}

/// Tauri命令：清理过期Agent数据
#[tauri::command]
#[allow(non_snake_case)]
pub async fn iso_cleanup(daysThreshold: i64) -> Result<u64, String> {
    AgentSessionManager::cleanup_stale_data(daysThreshold)
        .await
        .map_err(|e| e.to_string())
}
