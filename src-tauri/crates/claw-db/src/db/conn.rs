// Claw Desktop - Sea-ORM 数据库连接管理 + SQL 日志
// 统一数据库初始化、连接获取、SQL 打印
// 双数据库架构：主库(DB_CONN) + Agent隔离库(AGENT_DB_CONN)
// 自动初始化：首次调用 get_db() 时自动完成所有数据库初始化
// 向量扩展：集成 sqlite-vec 用于高效向量相似度搜索

use sea_orm::{ConnectionTrait, Database, DatabaseConnection, DbErr, Statement};
use tokio::sync::OnceCell;

static DB_CONN: OnceCell<DatabaseConnection> = OnceCell::const_new();
static AGENT_DB_CONN: OnceCell<DatabaseConnection> = OnceCell::const_new();
static DB_INITIALIZED: OnceCell<Result<(), String>> = OnceCell::const_new();

/// 初始化主数据库连接（SQLite，路径含 mode=rwc 自动建表）
/// 返回连接实例并缓存到全局 DB_CONN
/// 注册 sqlite-vec 扩展用于向量相似度搜索
pub async fn init_main_db(db_path: &str) -> Result<DatabaseConnection, DbErr> {
    let url = format!("sqlite://{}?mode=rwc", db_path);
    log::info!("[DB] 初始化主数据库: {}", db_path);
    let conn = Database::connect(&url).await?;
    conn.execute_unprepared("PRAGMA journal_mode=WAL;")
        .await
        .ok();
    conn.execute_unprepared("PRAGMA busy_timeout=5000;")
        .await
        .ok();
    conn.execute_unprepared("PRAGMA synchronous=NORMAL;")
        .await
        .ok();

    if let Err(e) = crate::vector_store::init_vector_extension(&conn).await {
        log::warn!("[DB] 向量扩展初始化失败: {}", e);
    }

    log::info!("[DB] 主数据库连接成功 (WAL mode)");
    DB_CONN.set(conn.clone()).expect("DB_CONN already set");
    Ok(conn)
}

/// 初始化 Agent 隔离数据库连接（每个 Agent 独立一个 .db 文件）
/// 返回连接实例并缓存到全局 AGENT_DB_CONN
pub async fn init_agent_db(db_path: &str) -> Result<DatabaseConnection, DbErr> {
    let url = format!("sqlite://{}?mode=rwc", db_path);
    log::info!("[AgentDB] 初始化 Agent 隔离数据库: {}", db_path);
    let conn = Database::connect(&url).await?;
    conn.execute_unprepared("PRAGMA journal_mode=WAL;")
        .await
        .ok();
    conn.execute_unprepared("PRAGMA busy_timeout=5000;")
        .await
        .ok();
    conn.execute_unprepared("PRAGMA synchronous=NORMAL;")
        .await
        .ok();
    log::info!("[AgentDB] Agent 数据库连接成功 (WAL mode)");
    AGENT_DB_CONN
        .set(conn.clone())
        .expect("AGENT_DB_CONN already set");
    Ok(conn)
}

/// 获取主数据库连接（全局单例，首次访问时自动触发初始化）
/// 如果初始化失败会 panic 并记录详细错误信息
pub async fn get_db() -> &'static DatabaseConnection {
    match ensure_db_initialized().await {
        Ok(()) => DB_CONN
            .get()
            .expect("DB_CONN should be set after initialization"),
        Err(e) => {
            log::error!("[DB] Auto-initialization failed: {}", e);
            panic!(
                "Database auto-initialization failed: {}. Ensure claw_config::path_resolver::init() is called in main.rs setup.",
                e
            )
        }
    }
}

/// 获取 Agent 隔离数据库连接（全局单例，首次访问时自动触发初始化）
/// 如果初始化失败会 panic 并记录详细错误信息
pub async fn get_agent_db() -> &'static DatabaseConnection {
    match ensure_db_initialized().await {
        Ok(()) => AGENT_DB_CONN
            .get()
            .expect("AGENT_DB_CONN should be set after initialization"),
        Err(e) => {
            log::error!("[DB] Agent DB auto-initialization failed: {}", e);
            panic!(
                "Agent database auto-initialization failed: {}. Ensure claw_config::path_resolver::init() is called in main.rs setup.",
                e
            )
        }
    }
}

/// 安全获取 Agent 数据库连接（未初始化时返回 None 而非 panic）
pub fn try_get_agent_db() -> Option<&'static DatabaseConnection> {
    AGENT_DB_CONN.get()
}

/// 初始化主库核心表（conversations, messages, memory_units 等）
pub async fn init_core_tables(conn: &DatabaseConnection) -> Result<(), DbErr> {
    log::info!("[DB] Initializing core tables (conversations, messages, memory)...");
    let backend = conn.get_database_backend();

    let create_sql = [
        "CREATE TABLE IF NOT EXISTS conversations (
            id TEXT PRIMARY KEY,
            title TEXT NOT NULL DEFAULT 'New Conversation',
            created_at INTEGER NOT NULL,
            updated_at INTEGER NOT NULL,
            message_count INTEGER NOT NULL DEFAULT 0,
            metadata TEXT DEFAULT NULL
        )",
        "CREATE TABLE IF NOT EXISTS messages (
            id TEXT PRIMARY KEY,
            conversation_id TEXT NOT NULL,
            role TEXT NOT NULL,
            content TEXT NOT NULL,
            timestamp INTEGER NOT NULL,
            token_count INTEGER DEFAULT NULL,
            embedding BLOB DEFAULT NULL,
            is_error INTEGER NOT NULL DEFAULT 0,
            model TEXT DEFAULT NULL,
            metadata TEXT DEFAULT NULL,
            FOREIGN KEY(conversation_id) REFERENCES conversations(id) ON DELETE CASCADE
        )",
        "CREATE TABLE IF NOT EXISTS memory_units (
            id TEXT PRIMARY KEY,
            agent_id TEXT NOT NULL,
            conversation_id TEXT,
            text TEXT NOT NULL,
            embedding BLOB NOT NULL,
            fact_type TEXT NOT NULL DEFAULT 'world',
            context TEXT DEFAULT NULL,
            occurred_at INTEGER DEFAULT NULL,
            mentioned_at INTEGER DEFAULT NULL,
            source_type TEXT NOT NULL DEFAULT 'conversation',
            metadata TEXT DEFAULT NULL,
            tags TEXT DEFAULT NULL,
            importance_score REAL NOT NULL DEFAULT 1.0,
            access_count INTEGER NOT NULL DEFAULT 0,
            memory_layer TEXT DEFAULT NULL,
            expires_at INTEGER DEFAULT NULL,
            created_at INTEGER NOT NULL,
            updated_at INTEGER NOT NULL
        )",
        "CREATE TABLE IF NOT EXISTS entities (
            id TEXT PRIMARY KEY,
            agent_id TEXT NOT NULL,
            canonical_name TEXT NOT NULL,
            entity_type TEXT NOT NULL DEFAULT 'general',
            metadata TEXT DEFAULT NULL,
            first_seen INTEGER NOT NULL,
            last_seen INTEGER NOT NULL,
            mention_count INTEGER NOT NULL DEFAULT 1
        )",
        "CREATE TABLE IF NOT EXISTS unit_entities (
            unit_id TEXT NOT NULL,
            entity_id TEXT NOT NULL,
            role TEXT DEFAULT 'subject',
            PRIMARY KEY (unit_id, entity_id),
            FOREIGN KEY (unit_id) REFERENCES memory_units(id) ON DELETE CASCADE,
            FOREIGN KEY (entity_id) REFERENCES entities(id) ON DELETE CASCADE
        )",
        "CREATE TABLE IF NOT EXISTS memory_links (
            id TEXT PRIMARY KEY,
            from_unit_id TEXT NOT NULL,
            to_unit_id TEXT NOT NULL,
            link_type TEXT NOT NULL,
            weight REAL NOT NULL DEFAULT 1.0,
            created_at INTEGER NOT NULL,
            FOREIGN KEY (from_unit_id) REFERENCES memory_units(id) ON DELETE CASCADE,
            FOREIGN KEY (to_unit_id) REFERENCES memory_units(id) ON DELETE CASCADE
        )",
        "CREATE TABLE IF NOT EXISTS entity_cooccurrences (
            entity_id_1 TEXT NOT NULL,
            entity_id_2 TEXT NOT NULL,
            cooccurrence_count INTEGER NOT NULL DEFAULT 1,
            last_cooccurred INTEGER NOT NULL,
            PRIMARY KEY (entity_id_1, entity_id_2),
            FOREIGN KEY (entity_id_1) REFERENCES entities(id) ON DELETE CASCADE,
            FOREIGN KEY (entity_id_2) REFERENCES entities(id) ON DELETE CASCADE
        )",
        "CREATE TABLE IF NOT EXISTS cron_jobs (
            id TEXT PRIMARY KEY,
            agent_id TEXT,
            name TEXT NOT NULL,
            schedule TEXT NOT NULL,
            prompt TEXT NOT NULL,
            delivery_channel_id TEXT,
            delivery_chat_id TEXT,
            enabled INTEGER NOT NULL DEFAULT 1,
            silent_on_empty INTEGER NOT NULL DEFAULT 0,
            last_run_at INTEGER DEFAULT NULL,
            next_run_at INTEGER DEFAULT NULL,
            run_count INTEGER NOT NULL DEFAULT 0,
            last_result TEXT DEFAULT NULL,
            created_at INTEGER NOT NULL,
            updated_at INTEGER NOT NULL
        )",
        "CREATE TABLE IF NOT EXISTS hooks (
            id TEXT PRIMARY KEY,
            agent_id TEXT NOT NULL,
            event_type TEXT NOT NULL,
            handler_type TEXT NOT NULL DEFAULT 'prompt',
            handler_config TEXT NOT NULL,
            is_active INTEGER NOT NULL DEFAULT 1,
            trigger_count INTEGER NOT NULL DEFAULT 0,
            created_at INTEGER NOT NULL,
            updated_at INTEGER NOT NULL
        )",
        "CREATE TABLE IF NOT EXISTS credential_pool (
            id TEXT PRIMARY KEY,
            provider TEXT NOT NULL,
            api_key TEXT NOT NULL,
            base_url TEXT DEFAULT NULL,
            model_name TEXT DEFAULT NULL,
            weight INTEGER NOT NULL DEFAULT 1,
            is_active INTEGER NOT NULL DEFAULT 1,
            rate_limit_remaining INTEGER DEFAULT NULL,
            rate_limit_reset_at INTEGER DEFAULT NULL,
            use_count INTEGER NOT NULL DEFAULT 0,
            last_used_at INTEGER DEFAULT NULL,
            created_at INTEGER NOT NULL,
            updated_at INTEGER NOT NULL
        )",
        "CREATE TABLE IF NOT EXISTS avoidance_rules (
            id TEXT PRIMARY KEY,
            agent_id TEXT NOT NULL,
            pattern TEXT NOT NULL,
            category TEXT NOT NULL DEFAULT 'other',
            cause TEXT NOT NULL,
            fix TEXT NOT NULL,
            trigger_count INTEGER NOT NULL DEFAULT 0,
            last_triggered_at INTEGER NOT NULL DEFAULT 0,
            is_deprecated INTEGER NOT NULL DEFAULT 0,
            expires_at INTEGER DEFAULT NULL,
            is_active INTEGER NOT NULL DEFAULT 1,
            similarity_hash TEXT DEFAULT NULL,
            created_at INTEGER NOT NULL,
            updated_at INTEGER NOT NULL
        )",
    ];

    for stmt in &create_sql {
        if let Err(e) = conn
            .execute(Statement::from_string(backend, stmt.to_string()))
            .await
        {
            log::warn!("[DB] Core table create warning: {}", e);
        }
    }

    let alter_sql = [
        "ALTER TABLE conversations ADD COLUMN agent_id TEXT DEFAULT NULL",
        "ALTER TABLE messages ADD COLUMN model TEXT DEFAULT NULL",
        "ALTER TABLE messages ADD COLUMN metadata TEXT DEFAULT NULL",
        "ALTER TABLE messages ADD COLUMN is_error INTEGER NOT NULL DEFAULT 0",
        "ALTER TABLE memory_units ADD COLUMN agent_id TEXT NOT NULL DEFAULT ''",
        "ALTER TABLE memory_units ADD COLUMN conversation_id TEXT DEFAULT NULL",
        "ALTER TABLE memory_units ADD COLUMN fact_type TEXT NOT NULL DEFAULT 'world'",
        "ALTER TABLE memory_units ADD COLUMN context TEXT DEFAULT NULL",
        "ALTER TABLE memory_units ADD COLUMN occurred_at INTEGER DEFAULT NULL",
        "ALTER TABLE memory_units ADD COLUMN mentioned_at INTEGER DEFAULT NULL",
        "ALTER TABLE memory_units ADD COLUMN source_type TEXT NOT NULL DEFAULT 'conversation'",
        "ALTER TABLE memory_units ADD COLUMN metadata TEXT DEFAULT NULL",
        "ALTER TABLE memory_units ADD COLUMN tags TEXT DEFAULT NULL",
        "ALTER TABLE memory_units ADD COLUMN importance_score REAL NOT NULL DEFAULT 1.0",
        "ALTER TABLE memory_units ADD COLUMN access_count INTEGER NOT NULL DEFAULT 0",
        "ALTER TABLE memory_units ADD COLUMN memory_layer TEXT DEFAULT NULL",
        "ALTER TABLE memory_units ADD COLUMN expires_at INTEGER DEFAULT NULL",
        "ALTER TABLE entities ADD COLUMN agent_id TEXT NOT NULL DEFAULT ''",
        "ALTER TABLE entities ADD COLUMN entity_type TEXT NOT NULL DEFAULT 'general'",
        "ALTER TABLE entities ADD COLUMN metadata TEXT DEFAULT NULL",
        "ALTER TABLE entities ADD COLUMN mention_count INTEGER NOT NULL DEFAULT 1",
        "ALTER TABLE unit_entities ADD COLUMN role TEXT DEFAULT 'subject'",
        "ALTER TABLE memory_links ADD COLUMN weight REAL NOT NULL DEFAULT 1.0",
        "ALTER TABLE cron_jobs ADD COLUMN agent_id TEXT NOT NULL DEFAULT ''",
        "ALTER TABLE cron_jobs ADD COLUMN delivery TEXT DEFAULT NULL",
        "ALTER TABLE hooks ADD COLUMN agent_id TEXT NOT NULL DEFAULT ''",
        "ALTER TABLE hooks ADD COLUMN handler_type TEXT NOT NULL DEFAULT 'prompt'",
        "ALTER TABLE hooks ADD COLUMN trigger_count INTEGER NOT NULL DEFAULT 0",
        "ALTER TABLE credential_pool ADD COLUMN base_url TEXT DEFAULT NULL",
        "ALTER TABLE credential_pool ADD COLUMN model_name TEXT DEFAULT NULL",
        "ALTER TABLE credential_pool ADD COLUMN weight INTEGER NOT NULL DEFAULT 1",
        "ALTER TABLE credential_pool ADD COLUMN rate_limit INTEGER DEFAULT NULL",
        "ALTER TABLE avoidance_rules ADD COLUMN agent_id TEXT NOT NULL DEFAULT ''",
        "ALTER TABLE avoidance_rules ADD COLUMN error_category TEXT NOT NULL DEFAULT 'other'",
        "ALTER TABLE avoidance_rules ADD COLUMN similarity_hash TEXT DEFAULT NULL",
    ];

    for stmt in &alter_sql {
        if let Err(_) = conn
            .execute(Statement::from_string(backend, stmt.to_string()))
            .await
        {
            // ALTER TABLE ADD COLUMN fails if column already exists - this is expected
        }
    }

    let fts_sql = "CREATE VIRTUAL TABLE IF NOT EXISTS memory_units_fts USING fts5(text, content='memory_units', content_rowid='id')";
    if let Err(e) = conn
        .execute(Statement::from_string(backend, fts_sql.to_string()))
        .await
    {
        log::warn!("[DB] FTS5 virtual table warning: {}", e);
    }

    let index_sql = [
        "CREATE INDEX IF NOT EXISTS idx_messages_conv ON messages(conversation_id)",
        "CREATE INDEX IF NOT EXISTS idx_messages_time ON messages(timestamp)",
        "CREATE INDEX IF NOT EXISTS idx_memory_units_agent ON memory_units(agent_id)",
        "CREATE INDEX IF NOT EXISTS idx_memory_units_conv ON memory_units(conversation_id)",
        "CREATE INDEX IF NOT EXISTS idx_memory_units_fact_type ON memory_units(fact_type)",
        "CREATE INDEX IF NOT EXISTS idx_memory_units_occurred ON memory_units(occurred_at)",
        "CREATE INDEX IF NOT EXISTS idx_memory_units_importance ON memory_units(importance_score DESC)",
        "CREATE INDEX IF NOT EXISTS idx_memory_units_layer ON memory_units(memory_layer)",
        "CREATE INDEX IF NOT EXISTS idx_memory_units_expires ON memory_units(expires_at)",
        "CREATE INDEX IF NOT EXISTS idx_entities_agent_name ON entities(agent_id, canonical_name)",
        "CREATE INDEX IF NOT EXISTS idx_memory_links_from ON memory_links(from_unit_id)",
        "CREATE INDEX IF NOT EXISTS idx_memory_links_to ON memory_links(to_unit_id)",
        "CREATE INDEX IF NOT EXISTS idx_memory_links_type ON memory_links(link_type)",
        "CREATE INDEX IF NOT EXISTS idx_cron_jobs_agent ON cron_jobs(agent_id)",
        "CREATE INDEX IF NOT EXISTS idx_hooks_agent ON hooks(agent_id)",
        "CREATE INDEX IF NOT EXISTS idx_hooks_event ON hooks(event_type)",
        "CREATE INDEX IF NOT EXISTS idx_credential_pool_provider ON credential_pool(provider)",
        "CREATE INDEX IF NOT EXISTS idx_avoidance_rules_agent ON avoidance_rules(agent_id)",
        "CREATE INDEX IF NOT EXISTS idx_avoidance_rules_category ON avoidance_rules(error_category)",
    ];

    for stmt in &index_sql {
        if let Err(e) = conn
            .execute(Statement::from_string(backend, stmt.to_string()))
            .await
        {
            log::warn!("[DB] Index create warning: {}", e);
        }
    }

    conn.execute_unprepared("PRAGMA foreign_keys=ON").await.ok();
    log::info!("[DB] Core tables initialized successfully");
    Ok(())
}

/// 初始化 Agent 隔离库表（agents, agent_sessions, agent_configs 等）
pub async fn init_agent_tables(conn: &DatabaseConnection) -> Result<(), DbErr> {
    log::info!("[AgentDB] Initializing agent tables...");
    let backend = conn.get_database_backend();

    let create_sql = [
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
        "CREATE TABLE IF NOT EXISTS agent_configs (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            agent_id TEXT NOT NULL,
            config_key TEXT NOT NULL,
            config_value TEXT NOT NULL,
            updated_at INTEGER NOT NULL,
            UNIQUE(agent_id, config_key)
        )",
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
        "CREATE TABLE IF NOT EXISTS agent_vectors (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            agent_id TEXT NOT NULL,
            chunk_index INTEGER NOT NULL,
            content_hash TEXT NOT NULL,
            vector BLOB NOT NULL,
            source_type TEXT DEFAULT NULL,
            created_at INTEGER NOT NULL
        )",
        "CREATE TABLE IF NOT EXISTS agent_profiles (
            agent_id TEXT PRIMARY KEY,
            profile_json TEXT NOT NULL DEFAULT '{}',
            interaction_count INTEGER NOT NULL DEFAULT 0,
            last_updated_at INTEGER NOT NULL DEFAULT 0,
            created_at INTEGER NOT NULL DEFAULT 0
        )",
    ];

    for stmt in &create_sql {
        if let Err(e) = conn
            .execute(Statement::from_string(backend, stmt.to_string()))
            .await
        {
            log::warn!("[AgentDB] Table create warning: {}", e);
        }
    }

    let alter_sql = [
        "ALTER TABLE agents ADD COLUMN display_name TEXT NOT NULL DEFAULT ''",
        "ALTER TABLE agents ADD COLUMN description TEXT DEFAULT NULL",
        "ALTER TABLE agents ADD COLUMN purpose TEXT DEFAULT NULL",
        "ALTER TABLE agents ADD COLUMN scope TEXT DEFAULT NULL",
        "ALTER TABLE agents ADD COLUMN model_override TEXT DEFAULT NULL",
        "ALTER TABLE agents ADD COLUMN system_prompt TEXT DEFAULT NULL",
        "ALTER TABLE agents ADD COLUMN tools_config TEXT DEFAULT '{}'",
        "ALTER TABLE agents ADD COLUMN skills_enabled TEXT DEFAULT '[]'",
        "ALTER TABLE agents ADD COLUMN max_turns INTEGER NOT NULL DEFAULT 20",
        "ALTER TABLE agents ADD COLUMN temperature REAL DEFAULT 0.7",
        "ALTER TABLE agents ADD COLUMN workspace_path TEXT DEFAULT NULL",
        "ALTER TABLE agents ADD COLUMN conversation_count INTEGER NOT NULL DEFAULT 0",
        "ALTER TABLE agents ADD COLUMN total_messages INTEGER NOT NULL DEFAULT 0",
        "ALTER TABLE agent_sessions ADD COLUMN conversation_id TEXT DEFAULT NULL",
        "ALTER TABLE agent_sessions ADD COLUMN turn_count INTEGER NOT NULL DEFAULT 0",
        "ALTER TABLE agent_sessions ADD COLUMN total_tokens_used REAL NOT NULL DEFAULT 0",
        "ALTER TABLE agent_workspace_files ADD COLUMN content_type TEXT DEFAULT NULL",
        "ALTER TABLE agent_vectors ADD COLUMN source_type TEXT DEFAULT NULL",
        "ALTER TABLE agent_profiles ADD COLUMN interaction_count INTEGER NOT NULL DEFAULT 0",
    ];

    for stmt in &alter_sql {
        if let Err(_) = conn
            .execute(Statement::from_string(backend, stmt.to_string()))
            .await
        {
            // Column already exists - expected
        }
    }

    let index_sql = [
        "CREATE UNIQUE INDEX IF NOT EXISTS idx_agents_id ON agents(id)",
        "CREATE INDEX IF NOT EXISTS idx_sessions_agent ON agent_sessions(agent_id)",
        "CREATE INDEX IF NOT EXISTS idx_ws_agent ON agent_workspace_files(agent_id)",
        "CREATE INDEX IF NOT EXISTS idx_vectors_agent ON agent_vectors(agent_id)",
    ];

    for stmt in &index_sql {
        if let Err(e) = conn
            .execute(Statement::from_string(backend, stmt.to_string()))
            .await
        {
            log::warn!("[AgentDB] Index create warning: {}", e);
        }
    }

    conn.execute_unprepared("PRAGMA foreign_keys=ON").await.ok();
    log::info!("[AgentDB] Agent tables initialized successfully");
    Ok(())
}

/// 内部数据库初始化函数（仅调用一次，依赖 config 模块已初始化）
async fn ensure_db_initialized() -> Result<(), String> {
    DB_INITIALIZED
        .get_or_init(|| async {
            use crate::db::channel_migration;
            use claw_config::path_resolver;

            let db_path = path_resolver::db_path();
            if let Some(parent) = db_path.parent() {
                std::fs::create_dir_all(parent)
                    .map_err(|e| format!("Failed to create DB dir: {}", e))?;
            }

            let db_path_str = db_path.to_str().ok_or("db path is not valid UTF-8")?;
            init_main_db(db_path_str)
                .await
                .map_err(|e| format!("MainDB init failed: {}", e))?;
            log::info!("[DB] Main database initialized at {}", db_path.display());

            let agent_db_path = path_resolver::agent_db_path();
            let agent_db_path_str = agent_db_path
                .to_str()
                .ok_or("agent db path is not valid UTF-8")?;
            init_agent_db(agent_db_path_str)
                .await
                .map_err(|e| format!("AgentDB init failed: {}", e))?;
            log::info!(
                "[DB] Agent database initialized at {}",
                agent_db_path.display()
            );

            let db_ref = DB_CONN.get().ok_or("DB_CONN not set after init_main_db")?;
            if let Err(e) = init_core_tables(db_ref).await {
                log::warn!("[DB] Core table init warning: {}", e);
            }

            let agent_db_ref = AGENT_DB_CONN
                .get()
                .ok_or("AGENT_DB_CONN not set after init_agent_db")?;
            if let Err(e) = init_agent_tables(agent_db_ref).await {
                log::warn!("[DB] Agent table init warning: {}", e);
            }

            if let Err(e) = channel_migration::init_channel_tables(db_ref).await {
                log::warn!("[DB] Channel table migration warning: {}", e);
            }
            if let Err(e) = channel_migration::init_extended_tables(db_ref).await {
                log::warn!("[DB] Extended table migration warning: {}", e);
            }

            log::info!("[DB] All databases initialized successfully");

            if let Ok(mut config) = claw_config::config::get_config().await.map(|c| c.clone()) {
                if !config.database.initialized {
                    config.database.initialized = true;
                    if let Err(e) = config.save(claw_config::path_resolver::get_app_root()) {
                        log::warn!("[DB] Failed to mark database as initialized: {}", e);
                    } else {
                        log::info!("[DB] Database marked as initialized in config");
                    }
                }
            }

            Ok(())
        })
        .await
        .clone()
}

/// 检查数据库是否已初始化
pub fn is_db_initialized() -> bool {
    matches!(DB_INITIALIZED.get(), Some(Ok(())))
}
