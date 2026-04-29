// Claw Desktop - Qdrant向量数据库后端实现
// 提供Qdrant集合的初始化、状态检查、连接测试功能
use crate::db::backend::{DatabaseInitResult, DatabaseStatus, TableStatus};

/// Qdrant向量数据库后端实现
pub struct QdrantBackend;

impl QdrantBackend {
    /// 初始化Qdrant — 测试连接、确保集合存在（含向量维度配置）
    pub async fn initialize() -> Result<DatabaseInitResult, String> {
        let config = claw_config::config::try_get_config()
            .ok_or("Config not initialized")?;

        if !config.database.is_qdrant() {
            return Err("Database backend is not qdrant".to_string());
        }

        let url = &config.database.qdrant.url;
        let collection = &config.database.qdrant.collection;
        let api_key = &config.database.qdrant.api_key;

        log::info!("[Qdrant] Connecting to {} collection={}", url, collection);

        Self::test_qdrant_connection(url, api_key).await?;

        let dim = claw_types::common::EMBEDDING_DIM;
        Self::ensure_collection(url, api_key, collection, dim).await?;

        log::info!("[Qdrant] Initialization complete | collection={} dim={}", collection, dim);

        Ok(DatabaseInitResult {
            backend: "qdrant".to_string(),
            success: true,
            tables_created: vec![collection.clone()],
            tables_repaired: vec![],
            vector_support: true,
            message: format!("Qdrant initialized (collection={}, dim={})", collection, dim),
        })
    }

    /// 检查Qdrant状态 — 连接状态、集合是否存在
    pub async fn check_status() -> Result<DatabaseStatus, String> {
        let config = claw_config::config::try_get_config()
            .ok_or("Config not initialized")?;

        let url = &config.database.qdrant.url;
        let collection = &config.database.qdrant.collection;
        let api_key = &config.database.qdrant.api_key;

        let connected = Self::test_qdrant_connection(url, api_key).await.is_ok();
        let collection_exists = Self::collection_exists(url, api_key, collection).await.unwrap_or(false);

        let mut tables = Vec::new();
        tables.push(TableStatus {
            name: collection.clone(),
            exists: collection_exists,
            row_count: 0,
            columns_valid: collection_exists,
            needs_repair: !collection_exists,
        });

        Ok(DatabaseStatus {
            backend: "qdrant".to_string(),
            connected,
            vector_support: connected,
            tables,
            total_rows: std::collections::HashMap::new(),
        })
    }

    /// 测试Qdrant连接 — 根据配置参数尝试健康检查
    pub async fn test_connection(config: &serde_json::Value) -> Result<bool, String> {
        let url = config.get("url").and_then(|v| v.as_str()).unwrap_or("http://localhost:6333");
        let api_key = config.get("api_key").and_then(|v| v.as_str()).unwrap_or("");
        Self::test_qdrant_connection(url, api_key).await
    }

    /// 测试Qdrant连接 — 请求/healthz端点
    async fn test_qdrant_connection(url: &str, _api_key: &str) -> Result<bool, String> {
        let client = reqwest::Client::new();
        let health_url = format!("{}/healthz", url);

        let response = client.get(&health_url)
            .timeout(std::time::Duration::from_secs(5))
            .send()
            .await
            .map_err(|e| format!("Qdrant connection failed: {}", e))?;

        if response.status().is_success() {
            Ok(true)
        } else {
            Err(format!("Qdrant health check failed: {}", response.status()))
        }
    }

    /// 检查集合是否存在 — 请求/collections/{name}端点
    async fn collection_exists(url: &str, _api_key: &str, collection: &str) -> Result<bool, String> {
        let client = reqwest::Client::new();
        let check_url = format!("{}/collections/{}", url, collection);

        let response = client.get(&check_url)
            .timeout(std::time::Duration::from_secs(5))
            .send()
            .await
            .map_err(|e| format!("Qdrant collection check failed: {}", e))?;

        Ok(response.status().is_success())
    }

    /// 确保集合存在 — 不存在则创建（配置向量维度和Cosine距离度量）
    async fn ensure_collection(url: &str, _api_key: &str, collection: &str, dim: usize) -> Result<(), String> {
        if Self::collection_exists(url, _api_key, collection).await? {
            log::info!("[Qdrant] Collection '{}' already exists", collection);
            return Ok(());
        }

        let client = reqwest::Client::new();
        let create_url = format!("{}/collections/{}", url, collection);

        let body = serde_json::json!({
            "vectors": {
                "size": dim,
                "distance": "Cosine"
            }
        });

        let response = client.put(&create_url)
            .json(&body)
            .timeout(std::time::Duration::from_secs(10))
            .send()
            .await
            .map_err(|e| format!("Qdrant create collection failed: {}", e))?;

        if response.status().is_success() {
            log::info!("[Qdrant] Collection '{}' created (dim={})", collection, dim);
            Ok(())
        } else {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            Err(format!("Qdrant create collection failed: {} - {}", status, text))
        }
    }
}
