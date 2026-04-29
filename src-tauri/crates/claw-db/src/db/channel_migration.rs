// Claw Desktop - 渠道数据迁移 - 渠道表结构迁移
use sea_orm::{ConnectionTrait, DbErr, Statement};

pub async fn init_channel_tables(conn: &impl ConnectionTrait) -> Result<(), DbErr> {
    log::info!("[ChannelDB] Initializing channel tables...");

    let sql = [
        r#"
        CREATE TABLE IF NOT EXISTS channel_accounts (
            id TEXT PRIMARY KEY,
            channel_type TEXT NOT NULL,
            name TEXT NOT NULL,
            enabled BOOLEAN DEFAULT TRUE,
            config_json TEXT NOT NULL DEFAULT '{}',
            encrypted_fields TEXT,
            dm_policy TEXT,
            group_policy TEXT,
            streaming_config TEXT,
            status TEXT DEFAULT 'configured',
            last_error TEXT,
            last_connected_at DATETIME,
            created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
            updated_at DATETIME DEFAULT CURRENT_TIMESTAMP
        )
        "#,
        r#"
        CREATE TABLE IF NOT EXISTS channel_sessions (
            id TEXT PRIMARY KEY,
            internal_conversation_id TEXT NOT NULL,
            external_chat_id TEXT NOT NULL,
            channel_account_id TEXT NOT NULL,
            chat_type TEXT NOT NULL,
            thread_id TEXT,
            title TEXT,
            metadata TEXT,
            created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
            last_active_at DATETIME DEFAULT CURRENT_TIMESTAMP,
            UNIQUE(external_chat_id, channel_account_id)
        )
        "#,
        r#"
        CREATE TABLE IF NOT EXISTS channel_message_log (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            message_id TEXT NOT NULL,
            channel_account_id TEXT NOT NULL,
            direction TEXT NOT NULL CHECK(direction IN ('inbound', 'outbound')),
            content_summary TEXT,
            full_content TEXT,
            sender_id TEXT,
            target_id TEXT,
            chat_type TEXT,
            metadata TEXT,
            timestamp DATETIME DEFAULT CURRENT_TIMESTAMP
        )
        "#,
        "CREATE INDEX IF NOT EXISTS idx_channel_accounts_type ON channel_accounts(channel_type)",
        "CREATE INDEX IF NOT EXISTS idx_channel_accounts_enabled ON channel_accounts(enabled)",
        "CREATE INDEX IF NOT EXISTS idx_channel_sessions_account ON channel_sessions(channel_account_id)",
        "CREATE INDEX IF NOT EXISTS idx_channel_sessions_external ON channel_sessions(external_chat_id)",
        "CREATE INDEX IF NOT EXISTS idx_channel_message_log_account ON channel_message_log(channel_account_id)",
        "CREATE INDEX IF NOT EXISTS idx_channel_message_log_timestamp ON channel_message_log(timestamp)",
    ];

    for stmt in &sql {
        let stmt = Statement::from_string(conn.get_database_backend(), stmt.to_string());
        if let Err(e) = conn.execute(stmt).await {
            log::warn!("[ChannelDB] Statement warning: {}", e);
        }
    }

    log::info!("[ChannelDB] Channel tables initialized successfully");
    Ok(())
}

pub async fn init_extended_tables(conn: &impl ConnectionTrait) -> Result<(), DbErr> {
    log::info!("[ExtendedDB] Initializing extended tables (cron, hooks, weixin, credentials)...");

    let sql = [
        r#"
        CREATE TABLE IF NOT EXISTS cron_run_log (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            cron_job_id TEXT NOT NULL,
            started_at INTEGER NOT NULL,
            finished_at INTEGER,
            status TEXT NOT NULL DEFAULT 'running',
            result_summary TEXT,
            error_message TEXT
        )
        "#,
        r#"
        CREATE TABLE IF NOT EXISTS weixin_context_tokens (
            account_id TEXT NOT NULL,
            user_id TEXT NOT NULL,
            context_token TEXT NOT NULL,
            updated_at INTEGER NOT NULL,
            PRIMARY KEY (account_id, user_id)
        )
        "#,
        r#"
        CREATE TABLE IF NOT EXISTS weixin_typing_tickets (
            account_id TEXT NOT NULL,
            ticket TEXT NOT NULL,
            updated_at INTEGER NOT NULL,
            PRIMARY KEY (account_id)
        )
        "#,
        r#"
        CREATE TABLE IF NOT EXISTS message_dedup (
            message_id TEXT PRIMARY KEY,
            seen_at INTEGER NOT NULL
        )
        "#,
        "CREATE INDEX IF NOT EXISTS idx_cron_run_log_job ON cron_run_log(cron_job_id)",
        "CREATE INDEX IF NOT EXISTS idx_message_dedup_seen ON message_dedup(seen_at)",
    ];

    for stmt in &sql {
        let stmt = Statement::from_string(conn.get_database_backend(), stmt.to_string());
        if let Err(e) = conn.execute(stmt).await {
            log::warn!("[ExtendedDB] Statement warning: {}", e);
        }
    }

    let alter_sql = [
        "ALTER TABLE cron_jobs ADD COLUMN delivery_channel_id TEXT DEFAULT NULL",
        "ALTER TABLE cron_jobs ADD COLUMN delivery_chat_id TEXT DEFAULT NULL",
        "ALTER TABLE cron_jobs ADD COLUMN silent_on_empty BOOLEAN DEFAULT FALSE",
        "ALTER TABLE cron_jobs ADD COLUMN last_result TEXT DEFAULT NULL",
        "ALTER TABLE cron_jobs ADD COLUMN cron_expr TEXT DEFAULT NULL",
        "ALTER TABLE cron_jobs ADD COLUMN task_command TEXT DEFAULT NULL",
        "ALTER TABLE hooks ADD COLUMN pattern TEXT DEFAULT NULL",
        "ALTER TABLE hooks ADD COLUMN priority INTEGER DEFAULT 0",
        "ALTER TABLE credential_pool ADD COLUMN rate_limit_remaining INTEGER DEFAULT NULL",
        "ALTER TABLE credential_pool ADD COLUMN rate_limit_reset_at INTEGER DEFAULT NULL",
    ];

    for stmt in &alter_sql {
        let stmt = Statement::from_string(conn.get_database_backend(), stmt.to_string());
        if let Err(_) = conn.execute(stmt).await {
            // Column already exists - expected
        }
    }

    let extra_index_sql = [
        "CREATE INDEX IF NOT EXISTS idx_cron_jobs_enabled ON cron_jobs(is_active)",
        "CREATE INDEX IF NOT EXISTS idx_cron_jobs_next_run ON cron_jobs(next_run_at)",
        "CREATE INDEX IF NOT EXISTS idx_hooks_event ON hooks(event_type)",
        "CREATE INDEX IF NOT EXISTS idx_hooks_enabled ON hooks(is_active)",
        "CREATE INDEX IF NOT EXISTS idx_credential_pool_active ON credential_pool(is_active)",
    ];

    for stmt in &extra_index_sql {
        let stmt = Statement::from_string(conn.get_database_backend(), stmt.to_string());
        if let Err(e) = conn.execute(stmt).await {
            log::warn!("[ExtendedDB] Index warning: {}", e);
        }
    }

    log::info!("[ExtendedDB] Extended tables initialized successfully");
    Ok(())
}
