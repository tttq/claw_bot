// Claw Desktop - 向量存储 - SQLite向量存储实现（用于RAG检索）
// 统一管理 sqlite-vec 加速路径和 BLOB 降级路径
// 运行时自动检测可用性，透明切换

use sea_orm::{DatabaseConnection, ConnectionTrait, Statement, FromQueryResult};
use once_cell::sync::OnceCell;

/// sqlite-vec扩展是否可用的全局标志
static VEC0_AVAILABLE: OnceCell<bool> = OnceCell::new();

/// 检查sqlite-vec扩展是否可用
pub fn is_vec0_available() -> bool {
    *VEC0_AVAILABLE.get().unwrap_or(&false)
}

/// 标记sqlite-vec扩展的可用性
pub fn mark_vec0_available(available: bool) {
    let _ = VEC0_AVAILABLE.set(available);
}

/// 向量搜索结果
#[derive(Debug, Clone, FromQueryResult)]
pub struct VectorSearchResult {
    pub memory_unit_id: String,
    pub similarity: f64,
}

/// 初始化向量扩展（在数据库连接建立后调用）
pub async fn init_vector_extension(conn: &DatabaseConnection) -> Result<bool, String> {
    match conn.execute_unprepared("SELECT load_extension('sqlite_vec');").await {
        Ok(_) => {
            log::info!("[VectorStore] sqlite-vec 扩展加载成功");
            
            // 创建 vec0 虚拟表
            conn.execute_unprepared(
                &format!(
                    "CREATE VIRTUAL TABLE IF NOT EXISTS memory_vectors USING vec0(
                        embedding float[{}],
                        memory_unit_id TEXT,
                        agent_id TEXT
                    );",
                    claw_types::common::EMBEDDING_DIM
                )
            ).await.map_err(|e| format!("Failed to create vec0 virtual table: {}", e))?;
            
            conn.execute_unprepared(
                "CREATE INDEX IF NOT EXISTS idx_mv_agent ON memory_vectors(agent_id);"
            ).await.ok();
            
            mark_vec0_available(true);
            Ok(true)
        }
        Err(e) => {
            log::warn!("[VectorStore] sqlite-vec 不可用，使用 BLOB + 余弦相似度降级方案: {}", e);
            mark_vec0_available(false);
            Ok(false)
        }
    }
}

/// 存储向量到最优路径
pub async fn store_vector(
    conn: &DatabaseConnection,
    memory_unit_id: &str,
    embedding: &[f32],
    agent_id: &str,
) -> Result<(), String> {
    let embedding_bytes = vector_to_bytes(embedding);

    if is_vec0_available() {
        // 路径 A: sqlite-vec 虚拟表（加速查询）
        conn.execute(Statement::from_sql_and_values(
            conn.get_database_backend(),
            "INSERT OR REPLACE INTO memory_vectors(rowid, embedding, memory_unit_id, agent_id) VALUES (?1, ?2, ?3, ?4)",
            [memory_unit_id.into(), embedding_bytes.clone().into(), agent_id.into()],
        )).await.map_err(|e| e.to_string())?;
    } else {
        // 路径 B: BLOB 字段（通过 sea-orm entity）
        // 由调用方通过 Database::update_embedding() 处理
    }

    Ok(())
}

/// 向量相似度搜索（自动选择最优路径）
pub async fn vector_search(
    conn: &DatabaseConnection,
    query_embedding: &[f32],
    agent_id: &str,
    limit: usize,
    threshold: f64,
) -> Result<Vec<VectorSearchResult>, String> {
    let query_bytes = vector_to_bytes(query_embedding);

    if is_vec0_available() {
        // 路径 A: sqlite-vec hardware 加速
        let sql = format!(
            "SELECT memory_unit_id, \
             1.0 - (vector_distance_cosine(mv.embedding, ?1)) AS similarity \
             FROM memory_vectors mv \
             WHERE mv.agent_id = ?2 \
             AND 1.0 - (vector_distance_cosine(mv.embedding, ?1)) > ?3 \
             ORDER BY similarity DESC \
             LIMIT {}",
            limit
        );

        let rows = conn.query_all(Statement::from_sql_and_values(
            conn.get_database_backend(),
            &sql,
            [query_bytes.into(), agent_id.into(), threshold.into()],
        )).await.map_err(|e| e.to_string())?;

        let results = rows.iter()
            .filter_map(|row| {
                let id = row.try_get::<String>("", "memory_unit_id").ok()?;
                let sim = row.try_get::<f64>("", "similarity").ok()?;
                Some(VectorSearchResult { memory_unit_id: id, similarity: sim })
            })
            .collect();

        Ok(results)
    } else {
        // 路径 B: 降级 — 从 memory_units 表读取全部，内存中计算余弦相似度
        let rows = conn.query_all(Statement::from_sql_and_values(
            conn.get_database_backend(),
            "SELECT id, embedding FROM memory_units WHERE agent_id = ?1 AND embedding IS NOT NULL",
            [agent_id.into()],
        )).await.map_err(|e| e.to_string())?;

        let mut results: Vec<VectorSearchResult> = Vec::new();
        
        for row in rows {
            if let (Ok(id), Ok(blob)) = (
                row.try_get::<String>("", "id"),
                row.try_get::<Vec<u8>>("", "embedding"),
            ) {
                let stored = bytes_to_vector(&blob);
                let sim = cosine_similarity(&stored, query_embedding);
                if sim >= threshold {
                    results.push(VectorSearchResult { memory_unit_id: id, similarity: sim });
                }
            }
        }

        results.sort_by(|a, b| b.similarity.partial_cmp(&a.similarity).unwrap_or(std::cmp::Ordering::Equal));
        results.truncate(limit);
        Ok(results)
    }
}

/// 删除指定记忆单元的向量
pub async fn delete_vector(conn: &DatabaseConnection, memory_unit_id: &str) -> Result<(), String> {
    if is_vec0_available() {
        conn.execute(Statement::from_sql_and_values(
            conn.get_database_backend(),
            "DELETE FROM memory_vectors WHERE memory_unit_id = ?1",
            [memory_unit_id.into()],
        )).await.map_err(|e| e.to_string())?;
    }
    Ok(())
}

// ==================== 工具函数 ====================

/// 将f32向量转换为字节数组（小端序）
pub fn vector_to_bytes(vec: &[f32]) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(vec.len() * 4);
    for &f in vec { bytes.extend_from_slice(&f.to_le_bytes()); }
    bytes
}

/// 将字节数组转换回f32向量（小端序）
pub fn bytes_to_vector(bytes: &[u8]) -> Vec<f32> {
    bytes.chunks_exact(4).filter_map(|c| c.try_into().ok().map(f32::from_le_bytes)).collect()
}

/// 计算两个向量的余弦相似度
pub fn cosine_similarity(a: &[f32], b: &[f32]) -> f64 {
    if a.len() != b.len() || a.is_empty() { return 0.0; }
    
    let dot_product: f64 = a.iter().zip(b.iter()).map(|(x, y)| (*x as f64) * (*y as f64)).sum();
    let norm_a: f64 = a.iter().map(|x| (*x as f64).powi(2)).sum::<f64>().sqrt();
    let norm_b: f64 = b.iter().map(|x| (*x as f64).powi(2)).sum::<f64>().sqrt();
    
    if norm_a == 0.0 || norm_b == 0.0 { return 0.0; }
    
    dot_product / (norm_a * norm_b)
}
