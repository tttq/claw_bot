// Claw Desktop - 数据库Schema验证器
// 检查数据库表结构是否符合预期，识别缺失列、无效列，提供修复建议
use sea_orm::{ConnectionTrait, DatabaseConnection, Statement};
use std::collections::HashMap;

/// 列验证结果
#[derive(Debug, Clone)]
pub struct ColumnValidation {
    pub exists: bool,
    pub columns_valid: bool,
}

/// 表Schema定义类型：(列名, 列类型) 列表
type TableSchema = Vec<(&'static str, &'static str)>;

/// 核心表Schema定义 — 对话、消息、记忆、实体、链接、定时任务、钩子、凭证、避错规则
fn core_table_schemas() -> HashMap<&'static str, TableSchema> {
    let mut schemas = HashMap::new();
    schemas.insert(
        "conversations",
        vec![
            ("id", "TEXT"),
            ("title", "TEXT"),
            ("created_at", "INTEGER"),
            ("updated_at", "INTEGER"),
            ("message_count", "INTEGER"),
            ("metadata", "TEXT"),
        ],
    );
    schemas.insert(
        "messages",
        vec![
            ("id", "TEXT"),
            ("conversation_id", "TEXT"),
            ("role", "TEXT"),
            ("content", "TEXT"),
            ("timestamp", "INTEGER"),
            ("token_count", "INTEGER"),
            ("embedding", "BLOB"),
            ("is_error", "INTEGER"),
            ("model", "TEXT"),
            ("metadata", "TEXT"),
        ],
    );
    schemas.insert(
        "memory_units",
        vec![
            ("id", "TEXT"),
            ("agent_id", "TEXT"),
            ("conversation_id", "TEXT"),
            ("text", "TEXT"),
            ("embedding", "BLOB"),
            ("fact_type", "TEXT"),
            ("context", "TEXT"),
            ("occurred_at", "INTEGER"),
            ("mentioned_at", "INTEGER"),
            ("source_type", "TEXT"),
            ("metadata", "TEXT"),
            ("tags", "TEXT"),
            ("importance_score", "REAL"),
            ("access_count", "INTEGER"),
            ("memory_layer", "TEXT"),
            ("expires_at", "INTEGER"),
            ("created_at", "INTEGER"),
            ("updated_at", "INTEGER"),
        ],
    );
    schemas.insert(
        "entities",
        vec![
            ("id", "TEXT"),
            ("agent_id", "TEXT"),
            ("canonical_name", "TEXT"),
            ("entity_type", "TEXT"),
            ("metadata", "TEXT"),
            ("first_seen", "INTEGER"),
            ("last_seen", "INTEGER"),
            ("mention_count", "INTEGER"),
        ],
    );
    schemas.insert(
        "unit_entities",
        vec![("unit_id", "TEXT"), ("entity_id", "TEXT"), ("role", "TEXT")],
    );
    schemas.insert(
        "memory_links",
        vec![
            ("id", "TEXT"),
            ("from_unit_id", "TEXT"),
            ("to_unit_id", "TEXT"),
            ("link_type", "TEXT"),
            ("weight", "REAL"),
            ("created_at", "INTEGER"),
        ],
    );
    schemas.insert(
        "entity_cooccurrences",
        vec![
            ("entity_id_1", "TEXT"),
            ("entity_id_2", "TEXT"),
            ("cooccurrence_count", "INTEGER"),
            ("last_cooccurred", "INTEGER"),
        ],
    );
    schemas.insert(
        "cron_jobs",
        vec![
            ("id", "TEXT"),
            ("agent_id", "TEXT"),
            ("name", "TEXT"),
            ("cron_expr", "TEXT"),
            ("task_command", "TEXT"),
            ("is_active", "INTEGER"),
            ("last_run_at", "INTEGER"),
            ("next_run_at", "INTEGER"),
            ("run_count", "INTEGER"),
            ("created_at", "INTEGER"),
            ("updated_at", "INTEGER"),
        ],
    );
    schemas.insert(
        "hooks",
        vec![
            ("id", "TEXT"),
            ("agent_id", "TEXT"),
            ("event_type", "TEXT"),
            ("handler_type", "TEXT"),
            ("handler_config", "TEXT"),
            ("is_active", "INTEGER"),
            ("trigger_count", "INTEGER"),
            ("created_at", "INTEGER"),
            ("updated_at", "INTEGER"),
        ],
    );
    schemas.insert(
        "credential_pool",
        vec![
            ("id", "TEXT"),
            ("provider", "TEXT"),
            ("api_key_encrypted", "TEXT"),
            ("base_url", "TEXT"),
            ("model_name", "TEXT"),
            ("weight", "INTEGER"),
            ("is_active", "INTEGER"),
            ("use_count", "INTEGER"),
            ("last_used_at", "INTEGER"),
            ("created_at", "INTEGER"),
            ("updated_at", "INTEGER"),
        ],
    );
    schemas.insert(
        "avoidance_rules",
        vec![
            ("id", "TEXT"),
            ("agent_id", "TEXT"),
            ("error_pattern", "TEXT"),
            ("error_category", "TEXT"),
            ("root_cause", "TEXT"),
            ("fix_suggestion", "TEXT"),
            ("trigger_count", "INTEGER"),
            ("is_active", "INTEGER"),
            ("similarity_hash", "TEXT"),
            ("created_at", "INTEGER"),
            ("updated_at", "INTEGER"),
        ],
    );
    schemas
}

/// Agent表Schema定义 — Agent配置、会话、配置项、工作区文件、向量、画像
fn agent_table_schemas() -> HashMap<&'static str, TableSchema> {
    let mut schemas = HashMap::new();
    schemas.insert(
        "agents",
        vec![
            ("id", "TEXT"),
            ("display_name", "TEXT"),
            ("description", "TEXT"),
            ("purpose", "TEXT"),
            ("scope", "TEXT"),
            ("model_override", "TEXT"),
            ("system_prompt", "TEXT"),
            ("tools_config", "TEXT"),
            ("skills_enabled", "TEXT"),
            ("max_turns", "INTEGER"),
            ("temperature", "REAL"),
            ("workspace_path", "TEXT"),
            ("is_active", "INTEGER"),
            ("created_at", "INTEGER"),
            ("updated_at", "INTEGER"),
            ("conversation_count", "INTEGER"),
            ("total_messages", "INTEGER"),
        ],
    );
    schemas.insert(
        "agent_sessions",
        vec![
            ("id", "TEXT"),
            ("agent_id", "TEXT"),
            ("conversation_id", "TEXT"),
            ("status", "TEXT"),
            ("turn_count", "INTEGER"),
            ("total_tokens_used", "REAL"),
            ("started_at", "INTEGER"),
            ("last_active", "INTEGER"),
        ],
    );
    schemas.insert(
        "agent_configs",
        vec![
            ("id", "INTEGER"),
            ("agent_id", "TEXT"),
            ("config_key", "TEXT"),
            ("config_value", "TEXT"),
            ("updated_at", "INTEGER"),
        ],
    );
    schemas.insert(
        "agent_workspace_files",
        vec![
            ("id", "INTEGER"),
            ("agent_id", "TEXT"),
            ("session_id", "TEXT"),
            ("relative_path", "TEXT"),
            ("full_path", "TEXT"),
            ("file_size", "INTEGER"),
            ("content_type", "TEXT"),
            ("indexed_at", "INTEGER"),
        ],
    );
    schemas.insert(
        "agent_vectors",
        vec![
            ("id", "INTEGER"),
            ("agent_id", "TEXT"),
            ("chunk_index", "INTEGER"),
            ("content_hash", "TEXT"),
            ("vector", "BLOB"),
            ("source_type", "TEXT"),
            ("created_at", "INTEGER"),
        ],
    );
    schemas.insert(
        "agent_profiles",
        vec![
            ("agent_id", "TEXT"),
            ("profile_json", "TEXT"),
            ("interaction_count", "INTEGER"),
            ("last_updated_at", "INTEGER"),
            ("created_at", "INTEGER"),
        ],
    );
    schemas
}

/// 验证核心表结构 — 检查所有核心表是否存在且列定义完整
pub async fn validate_core_tables(conn: &DatabaseConnection) -> Vec<(String, ColumnValidation)> {
    validate_tables(conn, core_table_schemas()).await
}

/// 验证Agent表结构 — 检查所有Agent相关表是否存在且列定义完整
pub async fn validate_agent_tables(conn: &DatabaseConnection) -> Vec<(String, ColumnValidation)> {
    validate_tables(conn, agent_table_schemas()).await
}

/// 通用表验证 — 遍历所有Schema，检查表存在性和列完整性
async fn validate_tables(
    conn: &DatabaseConnection,
    schemas: HashMap<&str, TableSchema>,
) -> Vec<(String, ColumnValidation)> {
    let mut results = Vec::new();

    for (table_name, expected_columns) in &schemas {
        let exists = check_table_exists(conn, table_name).await;
        let columns_valid = if exists {
            check_columns_exist(conn, table_name, expected_columns).await
        } else {
            false
        };

        results.push((
            table_name.to_string(),
            ColumnValidation {
                exists,
                columns_valid,
            },
        ));
    }

    results
}

/// 检查表是否存在 — 根据后端类型查询系统表
async fn check_table_exists(conn: &DatabaseConnection, table_name: &str) -> bool {
    let backend = conn.get_database_backend();
    let sql = match backend {
        sea_orm::DatabaseBackend::Sqlite => {
            format!(
                "SELECT name FROM sqlite_master WHERE type='table' AND name='{}'",
                table_name
            )
        }
        sea_orm::DatabaseBackend::Postgres => {
            format!(
                "SELECT table_name FROM information_schema.tables WHERE table_schema='public' AND table_name='{}'",
                table_name
            )
        }
        _ => {
            format!(
                "SELECT name FROM sqlite_master WHERE type='table' AND name='{}'",
                table_name
            )
        }
    };

    conn.query_one(Statement::from_string(backend, sql))
        .await
        .is_ok()
}

/// 检查列是否完整 — 对比期望列与实际列，发现缺失列时记录警告
async fn check_columns_exist(
    conn: &DatabaseConnection,
    table_name: &str,
    expected: &[(&str, &str)],
) -> bool {
    let backend = conn.get_database_backend();
    let sql = match backend {
        sea_orm::DatabaseBackend::Sqlite => {
            format!("PRAGMA table_info({})", table_name)
        }
        sea_orm::DatabaseBackend::Postgres => {
            format!(
                "SELECT column_name, data_type FROM information_schema.columns WHERE table_schema='public' AND table_name='{}'",
                table_name
            )
        }
        _ => {
            format!("PRAGMA table_info({})", table_name)
        }
    };

    let rows = conn.query_all(Statement::from_string(backend, sql)).await;

    match rows {
        Ok(rows) => {
            let existing_columns: std::collections::HashSet<String> = rows
                .iter()
                .filter_map(|row| {
                    if backend == sea_orm::DatabaseBackend::Postgres {
                        row.try_get::<String>("", "column_name").ok()
                    } else {
                        row.try_get::<String>("", "name").ok()
                    }
                })
                .collect();

            for (col_name, _) in expected {
                if !existing_columns.contains(*col_name) {
                    log::warn!(
                        "[SchemaValidator] Table '{}' missing column '{}'",
                        table_name,
                        col_name
                    );
                    return false;
                }
            }
            true
        }
        Err(_) => false,
    }
}
