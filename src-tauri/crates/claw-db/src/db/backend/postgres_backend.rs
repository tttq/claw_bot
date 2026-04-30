// Claw Desktop - PostgreSQL后端实现
// 提供PostgreSQL数据库的初始化建表、状态检查、连接测试、pgvector支持检测
use crate::db::backend::schema_validator;
use crate::db::backend::{DatabaseInitResult, DatabaseStatus, TableStatus};
use sea_orm::{ConnectionTrait, DatabaseConnection, QueryResult, Statement};

/// PostgreSQL后端实现
pub struct PostgresBackend;

impl PostgresBackend {
    /// 初始化PostgreSQL数据库 — 连接数据库、启用pgvector扩展、建表
    pub async fn initialize() -> Result<DatabaseInitResult, String> {
        let config = claw_config::config::try_get_config().ok_or("Config not initialized")?;

        if !config.database.is_postgres() {
            return Err("Database backend is not postgres".to_string());
        }

        let url = config.database.connection_url();
        log::info!(
            "[Postgres] Connecting to {}:{} db={}",
            config.database.postgres.host,
            config.database.postgres.port,
            config.database.postgres.database
        );

        let conn = sea_orm::Database::connect(&url)
            .await
            .map_err(|e| format!("PostgreSQL connection failed: {}", e))?;

        Self::ensure_pgvector(&conn).await?;

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

        Self::create_core_tables(&conn).await?;
        Self::create_agent_tables(&conn).await?;
        Self::create_vector_tables(&conn).await?;

        log::info!(
            "[Postgres] Initialization complete | created={} repaired={}",
            tables_created.len(),
            tables_repaired.len()
        );

        Ok(DatabaseInitResult {
            backend: "postgres".to_string(),
            success: true,
            tables_created,
            tables_repaired,
            vector_support: true,
            message: format!("PostgreSQL initialized (pgvector enabled)"),
        })
    }

    /// 检查PostgreSQL数据库状态 — 连接状态、表完整性、行数统计、pgvector支持
    pub async fn check_status() -> Result<DatabaseStatus, String> {
        let db = crate::db::conn::get_db().await;

        let validation = schema_validator::validate_core_tables(db).await;
        let mut tables = Vec::new();
        let mut total_rows = std::collections::HashMap::new();

        for (name, status) in validation {
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
            backend: "postgres".to_string(),
            connected: true,
            vector_support: true,
            tables,
            total_rows,
        })
    }

    /// 测试PostgreSQL连接 — 根据配置参数尝试连接数据库
    pub async fn test_connection(config: &serde_json::Value) -> Result<bool, String> {
        let host = config
            .get("host")
            .and_then(|v| v.as_str())
            .unwrap_or("localhost");
        let port = config.get("port").and_then(|v| v.as_u64()).unwrap_or(5432);
        let database = config
            .get("database")
            .and_then(|v| v.as_str())
            .unwrap_or("claw_desktop");
        let username = config
            .get("username")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        let password = config
            .get("password")
            .and_then(|v| v.as_str())
            .unwrap_or("");

        let url = format!(
            "postgres://{}:{}@{}:{}/{}",
            username, password, host, port, database
        );
        let conn = sea_orm::Database::connect(&url)
            .await
            .map_err(|e| format!("PostgreSQL connection test failed: {}", e))?;

        conn.execute_unprepared("SELECT 1")
            .await
            .map_err(|e| format!("PostgreSQL ping failed: {}", e))?;

        Ok(true)
    }

    /// 确保pgvector扩展已安装和启用
    async fn ensure_pgvector(conn: &DatabaseConnection) -> Result<(), String> {
        let result: Result<sea_orm::ExecResult, sea_orm::DbErr> = conn
            .execute_unprepared("CREATE EXTENSION IF NOT EXISTS vector")
            .await;
        result.map_err(|e| {
            format!(
                "Failed to create pgvector extension: {}. Is pgvector installed?",
                e
            )
        })?;
        log::info!("[Postgres] pgvector extension ensured");
        Ok(())
    }

    /// 创建核心表 — conversations, messages, memory_units, entities, unit_entities, memory_links, entity_cooccurrences, cron_jobs, hooks, credential_pool, avoidance_rules
    async fn create_core_tables(conn: &DatabaseConnection) -> Result<(), String> {
        let backend = conn.get_database_backend();
        let statements = [
            "CREATE TABLE IF NOT EXISTS conversations (
                id TEXT PRIMARY KEY,
                title TEXT NOT NULL DEFAULT 'New Conversation',
                created_at BIGINT NOT NULL,
                updated_at BIGINT NOT NULL,
                message_count INTEGER NOT NULL DEFAULT 0,
                metadata TEXT DEFAULT NULL
            )",
            "CREATE TABLE IF NOT EXISTS messages (
                id TEXT PRIMARY KEY,
                conversation_id TEXT NOT NULL,
                role TEXT NOT NULL,
                content TEXT NOT NULL,
                timestamp BIGINT NOT NULL,
                token_count INTEGER DEFAULT NULL,
                embedding BYTEA DEFAULT NULL,
                is_error INTEGER NOT NULL DEFAULT 0,
                model TEXT DEFAULT NULL,
                metadata TEXT DEFAULT NULL
            )",
            "CREATE INDEX IF NOT EXISTS idx_messages_conv ON messages(conversation_id)",
            "CREATE INDEX IF NOT EXISTS idx_messages_time ON messages(timestamp)",
            "CREATE TABLE IF NOT EXISTS memory_units (
                id TEXT PRIMARY KEY,
                agent_id TEXT NOT NULL,
                conversation_id TEXT,
                text TEXT NOT NULL,
                embedding BYTEA NOT NULL,
                fact_type TEXT NOT NULL DEFAULT 'world',
                context TEXT DEFAULT NULL,
                occurred_at BIGINT DEFAULT NULL,
                mentioned_at BIGINT DEFAULT NULL,
                source_type TEXT NOT NULL DEFAULT 'conversation',
                metadata TEXT DEFAULT NULL,
                tags TEXT DEFAULT NULL,
                importance_score DOUBLE PRECISION NOT NULL DEFAULT 1.0,
                access_count INTEGER NOT NULL DEFAULT 0,
                created_at BIGINT NOT NULL,
                updated_at BIGINT NOT NULL
            )",
            "CREATE TABLE IF NOT EXISTS entities (
                id TEXT PRIMARY KEY,
                agent_id TEXT NOT NULL,
                canonical_name TEXT NOT NULL,
                entity_type TEXT NOT NULL DEFAULT 'general',
                metadata TEXT DEFAULT NULL,
                first_seen BIGINT NOT NULL,
                last_seen BIGINT NOT NULL,
                mention_count INTEGER NOT NULL DEFAULT 1
            )",
            "CREATE TABLE IF NOT EXISTS unit_entities (
                unit_id TEXT NOT NULL,
                entity_id TEXT NOT NULL,
                role TEXT DEFAULT 'subject',
                PRIMARY KEY (unit_id, entity_id)
            )",
            "CREATE TABLE IF NOT EXISTS memory_links (
                id TEXT PRIMARY KEY,
                from_unit_id TEXT NOT NULL,
                to_unit_id TEXT NOT NULL,
                link_type TEXT NOT NULL,
                weight DOUBLE PRECISION NOT NULL DEFAULT 1.0,
                created_at BIGINT NOT NULL
            )",
            "CREATE TABLE IF NOT EXISTS entity_cooccurrences (
                entity_id_1 TEXT NOT NULL,
                entity_id_2 TEXT NOT NULL,
                cooccurrence_count INTEGER NOT NULL DEFAULT 1,
                last_cooccurred BIGINT NOT NULL,
                PRIMARY KEY (entity_id_1, entity_id_2)
            )",
            "CREATE TABLE IF NOT EXISTS cron_jobs (
                id TEXT PRIMARY KEY,
                agent_id TEXT NOT NULL,
                name TEXT NOT NULL,
                cron_expr TEXT NOT NULL,
                task_command TEXT NOT NULL,
                is_active INTEGER NOT NULL DEFAULT 1,
                last_run_at BIGINT DEFAULT NULL,
                next_run_at BIGINT DEFAULT NULL,
                run_count INTEGER NOT NULL DEFAULT 0,
                created_at BIGINT NOT NULL,
                updated_at BIGINT NOT NULL
            )",
            "CREATE TABLE IF NOT EXISTS hooks (
                id TEXT PRIMARY KEY,
                agent_id TEXT NOT NULL,
                event_type TEXT NOT NULL,
                handler_type TEXT NOT NULL DEFAULT 'prompt',
                handler_config TEXT NOT NULL,
                is_active INTEGER NOT NULL DEFAULT 1,
                trigger_count INTEGER NOT NULL DEFAULT 0,
                created_at BIGINT NOT NULL,
                updated_at BIGINT NOT NULL
            )",
            "CREATE TABLE IF NOT EXISTS credential_pool (
                id TEXT PRIMARY KEY,
                provider TEXT NOT NULL,
                api_key_encrypted TEXT NOT NULL,
                base_url TEXT DEFAULT NULL,
                model_name TEXT DEFAULT NULL,
                weight INTEGER NOT NULL DEFAULT 1,
                is_active INTEGER NOT NULL DEFAULT 1,
                use_count INTEGER NOT NULL DEFAULT 0,
                last_used_at BIGINT DEFAULT NULL,
                created_at BIGINT NOT NULL,
                updated_at BIGINT NOT NULL
            )",
            "CREATE TABLE IF NOT EXISTS avoidance_rules (
                id TEXT PRIMARY KEY,
                agent_id TEXT NOT NULL,
                error_pattern TEXT NOT NULL,
                error_category TEXT NOT NULL DEFAULT 'other',
                root_cause TEXT NOT NULL,
                fix_suggestion TEXT NOT NULL,
                trigger_count INTEGER NOT NULL DEFAULT 0,
                is_active INTEGER NOT NULL DEFAULT 1,
                similarity_hash TEXT DEFAULT NULL,
                created_at BIGINT NOT NULL,
                updated_at BIGINT NOT NULL
            )",
            "CREATE INDEX IF NOT EXISTS idx_memory_units_agent ON memory_units(agent_id)",
            "CREATE INDEX IF NOT EXISTS idx_memory_units_conv ON memory_units(conversation_id)",
            "CREATE INDEX IF NOT EXISTS idx_entities_agent_name ON entities(agent_id, canonical_name)",
            "CREATE INDEX IF NOT EXISTS idx_cron_jobs_agent ON cron_jobs(agent_id)",
            "CREATE INDEX IF NOT EXISTS idx_hooks_agent ON hooks(agent_id)",
            "CREATE INDEX IF NOT EXISTS idx_credential_pool_provider ON credential_pool(provider)",
            "CREATE INDEX IF NOT EXISTS idx_avoidance_rules_agent ON avoidance_rules(agent_id)",
        ];

        for stmt in &statements {
            let result: Result<sea_orm::ExecResult, sea_orm::DbErr> = conn
                .execute(Statement::from_string(backend, stmt.to_string()))
                .await;
            result.map_err(|e| format!("Postgres create table failed: {}", e))?;
        }

        log::info!("[Postgres] Core tables created");
        Ok(())
    }

    /// 创建Agent表 — agents, agent_sessions, agent_configs, agent_workspace_files, agent_vectors, agent_profiles
    async fn create_agent_tables(conn: &DatabaseConnection) -> Result<(), String> {
        let backend = conn.get_database_backend();
        let statements = [
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
                temperature DOUBLE PRECISION DEFAULT 0.7,
                workspace_path TEXT DEFAULT NULL,
                is_active INTEGER NOT NULL DEFAULT 1,
                created_at BIGINT NOT NULL,
                updated_at BIGINT NOT NULL,
                conversation_count INTEGER NOT NULL DEFAULT 0,
                total_messages INTEGER NOT NULL DEFAULT 0
            )",
            "CREATE TABLE IF NOT EXISTS agent_sessions (
                id TEXT PRIMARY KEY,
                agent_id TEXT NOT NULL,
                conversation_id TEXT DEFAULT NULL,
                status TEXT NOT NULL DEFAULT 'active',
                turn_count INTEGER NOT NULL DEFAULT 0,
                total_tokens_used DOUBLE PRECISION NOT NULL DEFAULT 0,
                started_at BIGINT NOT NULL,
                last_active BIGINT DEFAULT NULL
            )",
            "CREATE TABLE IF NOT EXISTS agent_configs (
                id SERIAL PRIMARY KEY,
                agent_id TEXT NOT NULL,
                config_key TEXT NOT NULL,
                config_value TEXT NOT NULL,
                updated_at BIGINT NOT NULL,
                UNIQUE(agent_id, config_key)
            )",
            "CREATE TABLE IF NOT EXISTS agent_workspace_files (
                id SERIAL PRIMARY KEY,
                agent_id TEXT NOT NULL,
                session_id TEXT DEFAULT NULL,
                relative_path TEXT NOT NULL,
                full_path TEXT NOT NULL,
                file_size INTEGER NOT NULL DEFAULT 0,
                content_type TEXT DEFAULT NULL,
                indexed_at BIGINT NOT NULL
            )",
            "CREATE TABLE IF NOT EXISTS agent_vectors (
                id SERIAL PRIMARY KEY,
                agent_id TEXT NOT NULL,
                chunk_index INTEGER NOT NULL,
                content_hash TEXT NOT NULL,
                vector BYTEA NOT NULL,
                source_type TEXT DEFAULT NULL,
                created_at BIGINT NOT NULL
            )",
            "CREATE TABLE IF NOT EXISTS agent_profiles (
                agent_id TEXT PRIMARY KEY,
                profile_json TEXT NOT NULL DEFAULT '{}',
                interaction_count INTEGER NOT NULL DEFAULT 0,
                last_updated_at BIGINT NOT NULL DEFAULT 0,
                created_at BIGINT NOT NULL DEFAULT 0
            )",
            "CREATE INDEX IF NOT EXISTS idx_sessions_agent ON agent_sessions(agent_id)",
            "CREATE INDEX IF NOT EXISTS idx_ws_agent ON agent_workspace_files(agent_id)",
            "CREATE INDEX IF NOT EXISTS idx_vectors_agent ON agent_vectors(agent_id)",
        ];

        for stmt in &statements {
            let result: Result<sea_orm::ExecResult, sea_orm::DbErr> = conn
                .execute(Statement::from_string(backend, stmt.to_string()))
                .await;
            result.map_err(|e| format!("Postgres create agent table failed: {}", e))?;
        }

        log::info!("[Postgres] Agent tables created");
        Ok(())
    }

    /// 创建向量索引表 — 为记忆和Agent向量建立pgvector索引
    async fn create_vector_tables(conn: &DatabaseConnection) -> Result<(), String> {
        let dim = claw_types::common::EMBEDDING_DIM;
        let sql = format!(
            "CREATE TABLE IF NOT EXISTS memory_vectors_pgvec (
                id SERIAL PRIMARY KEY,
                embedding vector({}),
                memory_unit_id TEXT NOT NULL,
                agent_id TEXT NOT NULL
            )",
            dim
        );

        let result: Result<sea_orm::ExecResult, sea_orm::DbErr> = conn
            .execute(Statement::from_string(conn.get_database_backend(), sql))
            .await;
        result.map_err(|e| format!("Postgres create vector table failed: {}", e))?;

        let idx_result: Result<sea_orm::ExecResult, sea_orm::DbErr> = conn
            .execute(Statement::from_string(
                conn.get_database_backend(),
                "CREATE INDEX IF NOT EXISTS idx_mv_pgvec_agent ON memory_vectors_pgvec(agent_id)"
                    .to_string(),
            ))
            .await;
        idx_result.ok();

        log::info!("[Postgres] Vector tables created (pgvector dim={})", dim);
        Ok(())
    }

    /// 统计表行数
    async fn count_rows(conn: &DatabaseConnection, table: &str) -> i64 {
        let sql = format!("SELECT COUNT(*) as cnt FROM {}", table);
        let result: Option<QueryResult> = conn
            .query_one(Statement::from_string(conn.get_database_backend(), sql))
            .await
            .ok()
            .flatten();

        result
            .and_then(|row: QueryResult| row.try_get::<i64>("", "cnt").ok())
            .unwrap_or(0)
    }
}
