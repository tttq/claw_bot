// Claw Desktop - 数据库后端抽象层
// 定义数据库后端枚举（SQLite/PostgreSQL/Qdrant）、初始化结果/状态/表状态结构体，
// 提供统一的初始化、状态检查、连接测试接口
pub mod postgres_backend;
pub mod qdrant_backend;
pub mod schema_validator;
pub mod sqlite_backend;

use sea_orm::DatabaseConnection;
use serde::{Deserialize, Serialize};

/// 数据库后端类型枚举
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum DatabaseBackend {
    Sqlite,
    Postgres,
    Qdrant,
}

impl std::fmt::Display for DatabaseBackend {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DatabaseBackend::Sqlite => write!(f, "sqlite"),
            DatabaseBackend::Postgres => write!(f, "postgres"),
            DatabaseBackend::Qdrant => write!(f, "qdrant"),
        }
    }
}

impl From<&str> for DatabaseBackend {
    fn from(s: &str) -> Self {
        match s {
            "postgres" => DatabaseBackend::Postgres,
            "qdrant" => DatabaseBackend::Qdrant,
            _ => DatabaseBackend::Sqlite,
        }
    }
}

/// 数据库初始化结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseInitResult {
    pub backend: String,
    pub success: bool,
    pub tables_created: Vec<String>,
    pub tables_repaired: Vec<String>,
    pub vector_support: bool,
    pub message: String,
}

/// 数据库状态信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseStatus {
    pub backend: String,
    pub connected: bool,
    pub vector_support: bool,
    pub tables: Vec<TableStatus>,
    pub total_rows: std::collections::HashMap<String, i64>,
}

/// 单表状态信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TableStatus {
    pub name: String,
    pub exists: bool,
    pub row_count: i64,
    pub columns_valid: bool,
    pub needs_repair: bool,
}

/// 后端初始化器 — 根据后端类型分发初始化/状态检查/连接测试
pub struct BackendInitializer;

impl BackendInitializer {
    /// 初始化指定后端数据库（建表、启用扩展）
    pub async fn initialize(backend: &DatabaseBackend) -> Result<DatabaseInitResult, String> {
        match backend {
            DatabaseBackend::Sqlite => sqlite_backend::SqliteBackend::initialize().await,
            DatabaseBackend::Postgres => postgres_backend::PostgresBackend::initialize().await,
            DatabaseBackend::Qdrant => qdrant_backend::QdrantBackend::initialize().await,
        }
    }

    /// 检查指定后端数据库状态（连接、表、行数）
    pub async fn check_status(backend: &DatabaseBackend) -> Result<DatabaseStatus, String> {
        match backend {
            DatabaseBackend::Sqlite => sqlite_backend::SqliteBackend::check_status().await,
            DatabaseBackend::Postgres => postgres_backend::PostgresBackend::check_status().await,
            DatabaseBackend::Qdrant => qdrant_backend::QdrantBackend::check_status().await,
        }
    }

    /// 测试指定后端数据库连接
    pub async fn test_connection(
        backend: &DatabaseBackend,
        config: &serde_json::Value,
    ) -> Result<bool, String> {
        match backend {
            DatabaseBackend::Sqlite => sqlite_backend::SqliteBackend::test_connection(config).await,
            DatabaseBackend::Postgres => {
                postgres_backend::PostgresBackend::test_connection(config).await
            }
            DatabaseBackend::Qdrant => qdrant_backend::QdrantBackend::test_connection(config).await,
        }
    }
}

/// 获取主数据库连接 — 根据配置创建SQLite或PostgreSQL连接
pub async fn get_main_db_connection() -> Result<DatabaseConnection, String> {
    let config = claw_config::config::try_get_config().ok_or("Config not initialized")?;

    match DatabaseBackend::from(config.database.backend.as_str()) {
        DatabaseBackend::Postgres => {
            let url = config.database.connection_url();
            sea_orm::Database::connect(&url)
                .await
                .map_err(|e| e.to_string())
        }
        _ => {
            let db_path = if config.database.sqlite.db_path.is_empty() {
                claw_config::path_resolver::db_path()
            } else {
                std::path::PathBuf::from(&config.database.sqlite.db_path)
            };
            let url = format!("sqlite://{}?mode=rwc", db_path.display());
            sea_orm::Database::connect(&url)
                .await
                .map_err(|e| e.to_string())
        }
    }
}
