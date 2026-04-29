// Claw Desktop - SQLite后端实现
// 提供SQLite数据库的初始化建表、状态检查、连接测试、向量支持检测
use sea_orm::{ConnectionTrait, DatabaseConnection, Statement, QueryResult};
use crate::db::conn::{init_core_tables, init_agent_tables};
use crate::db::backend::{DatabaseInitResult, DatabaseStatus, TableStatus};
use crate::db::backend::schema_validator;

/// SQLite后端实现
pub struct SqliteBackend;

impl SqliteBackend {
    /// 初始化SQLite数据库 — 连接数据库、建表、启用vec0向量扩展
    pub async fn initialize() -> Result<DatabaseInitResult, String> {
        let config = claw_config::config::try_get_config()
            .ok_or("Config not initialized")?;

        let db_path = if config.database.sqlite.db_path.is_empty() {
            claw_config::path_resolver::db_path()
        } else {
            std::path::PathBuf::from(&config.database.sqlite.db_path)
        };

        if let Some(parent) = db_path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| format!("Failed to create DB dir: {}", e))?;
        }

        let db_path_str = db_path.to_str().ok_or("db path is not valid UTF-8")?;
        let url = format!("sqlite://{}?mode=rwc", db_path_str);

        let conn = sea_orm::Database::connect(&url).await
            .map_err(|e| format!("SQLite connection failed: {}", e))?;

        conn.execute_unprepared("PRAGMA journal_mode=WAL;").await.ok();
        conn.execute_unprepared("PRAGMA busy_timeout=5000;").await.ok();
        conn.execute_unprepared("PRAGMA synchronous=NORMAL;").await.ok();
        conn.execute_unprepared("PRAGMA foreign_keys=ON;").await.ok();

        log::info!("[SQLite] Connected to {}", db_path.display());

        let mut tables_created = Vec::new();
        let mut tables_repaired = Vec::new();

        let validation = schema_validator::validate_core_tables(&conn).await;
        for (table, status) in &validation {
            if !status.exists {
                tables_created.push(table.clone());
            } else if !status.columns_valid {
                tables_repaired.push(table.clone());
            }
        }

        if let Err(e) = init_core_tables(&conn).await {
            log::warn!("[SQLite] Core table init warning: {}", e);
        }

        let mut vector_support = false;
        if config.database.sqlite.enable_vec {
            vector_support = crate::vector_store::init_vector_extension(&conn).await.unwrap_or(false);
        }

        let agent_db_path = claw_config::path_resolver::agent_db_path();
        let agent_db_path_str = agent_db_path.to_str().ok_or("agent db path is not valid UTF-8")?;
        let agent_url = format!("sqlite://{}?mode=rwc", agent_db_path_str);

        let agent_conn = sea_orm::Database::connect(&agent_url).await
            .map_err(|e| format!("Agent SQLite connection failed: {}", e))?;

        agent_conn.execute_unprepared("PRAGMA journal_mode=WAL;").await.ok();
        agent_conn.execute_unprepared("PRAGMA busy_timeout=5000;").await.ok();
        agent_conn.execute_unprepared("PRAGMA synchronous=NORMAL;").await.ok();
        agent_conn.execute_unprepared("PRAGMA foreign_keys=ON;").await.ok();

        let agent_validation = schema_validator::validate_agent_tables(&agent_conn).await;
        for (table, status) in &agent_validation {
            if !status.exists {
                tables_created.push(table.clone());
            } else if !status.columns_valid {
                tables_repaired.push(table.clone());
            }
        }

        if let Err(e) = init_agent_tables(&agent_conn).await {
            log::warn!("[SQLite] Agent table init warning: {}", e);
        }

        if let Err(e) = crate::db::channel_migration::init_channel_tables(&conn).await {
            log::warn!("[SQLite] Channel table migration warning: {}", e);
        }
        if let Err(e) = crate::db::channel_migration::init_extended_tables(&conn).await {
            log::warn!("[SQLite] Extended table migration warning: {}", e);
        }

        log::info!("[SQLite] Initialization complete | created={} repaired={} vec={}",
            tables_created.len(), tables_repaired.len(), vector_support);

        Ok(DatabaseInitResult {
            backend: "sqlite".to_string(),
            success: true,
            tables_created,
            tables_repaired,
            vector_support,
            message: format!("SQLite initialized at {} (vec={})", db_path.display(), vector_support),
        })
    }

    /// 检查SQLite数据库状态 — 连接状态、表完整性、行数统计、向量支持
    pub async fn check_status() -> Result<DatabaseStatus, String> {
        let db = crate::db::conn::get_db().await;
        let agent_db = crate::db::conn::get_agent_db().await;

        let core_validation = schema_validator::validate_core_tables(db).await;
        let agent_validation = schema_validator::validate_agent_tables(agent_db).await;

        let mut tables = Vec::new();
        let mut total_rows = std::collections::HashMap::new();

        for (name, status) in core_validation.into_iter().chain(agent_validation.into_iter()) {
            let row_count = Self::count_rows(db, &name).await;
            total_rows.insert(name.clone(), row_count);
            tables.push(TableStatus {
                name,
                exists: status.exists,
                row_count,
                columns_valid: status.columns_valid,
                needs_repair: !status.exists || !status.columns_valid,
            });
        }

        Ok(DatabaseStatus {
            backend: "sqlite".to_string(),
            connected: true,
            vector_support: crate::vector_store::is_vec0_available(),
            tables,
            total_rows,
        })
    }

    /// 测试SQLite连接 — 尝试打开数据库文件并执行简单查询
    pub async fn test_connection(_config: &serde_json::Value) -> Result<bool, String> {
        let config = claw_config::config::try_get_config()
            .ok_or("Config not initialized")?;

        let db_path = if config.database.sqlite.db_path.is_empty() {
            claw_config::path_resolver::db_path()
        } else {
            std::path::PathBuf::from(&config.database.sqlite.db_path)
        };

        if let Some(parent) = db_path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| format!("Failed to create DB dir: {}", e))?;
        }

        let url = format!("sqlite://{}?mode=rwc", db_path.display());
        let conn = sea_orm::Database::connect(&url).await
            .map_err(|e| format!("SQLite connection test failed: {}", e))?;

        conn.execute_unprepared("SELECT 1").await
            .map_err(|e| format!("SQLite ping failed: {}", e))?;

        Ok(true)
    }

    /// 统计表行数
    async fn count_rows(conn: &DatabaseConnection, table: &str) -> i64 {
        let sql = format!("SELECT COUNT(*) as cnt FROM {}", table);
        let result: Option<QueryResult> = conn.query_one(Statement::from_string(
            conn.get_database_backend(),
            sql,
        )).await.ok().flatten();

        result
            .and_then(|row: QueryResult| row.try_get::<i64>("", "cnt").ok())
            .unwrap_or(0)
    }
}
