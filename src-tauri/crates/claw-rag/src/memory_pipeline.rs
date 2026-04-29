// Claw Desktop - 记忆流水线 - 摄入→提取→整合→存储 四步闭环
// 核心理念：统一的记忆摄入管道，不再散落在 llm.rs/tool_loop.rs 多处
// 生产级能力：去重、容错、重试、一致性保护

use crate::memory_layers::*;
use crate::rag::{
    store_enhanced_memory, hybrid_retrieve, embed_text, vector_to_bytes,
    bytes_to_vector, cosine_similarity, calc_importance_score,
};
use claw_db::db::get_db;
use claw_db::db::entities::memory_units;
use sea_orm::{EntityTrait, ColumnTrait, QueryFilter, Set, ActiveModelTrait, QueryOrder, QuerySelect, ConnectionTrait};
use std::collections::HashMap;

const DEDUP_CANDIDATE_LIMIT: usize = 20;
const MAX_PIPELINE_RETRIES: usize = 3;
const RETRY_DELAY_MS: u64 = 200;

/// 记忆摄入请求 — 描述一条待摄入记忆的所有元数据
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct IngestionRequest {
    pub agent_id: String,
    pub conversation_id: Option<String>,
    pub text: String,
    pub fact_type: String,
    pub source_type: String,
    pub context: Option<String>,
    pub tags: Option<String>,
    pub force_layer: Option<MemoryLayer>,
}

/// 记忆摄入结果 — 记录摄入操作的完整结果信息
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct IngestionResult {
    pub memory_id: Option<String>,
    pub layer: MemoryLayer,
    pub importance_score: f64,
    pub was_deduplicated: bool,
    pub was_updated: bool,
    pub duplicate_of: Option<String>,
    pub error: Option<String>,
}

/// 提取结果 — 从文本中提取的事实和实体
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ExtractionResult {
    pub facts: Vec<ExtractedFact>,
    pub entities: Vec<ExtractedEntity>,
    pub sentiment: Option<String>,
}

/// 提取的事实 — 包含文本、类型和置信度
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ExtractedFact {
    pub text: String,
    pub fact_type: String,
    pub confidence: f64,
    pub source_span: Option<(usize, usize)>,
}

/// 提取的实体 — 识别出的命名实体
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ExtractedEntity {
    pub name: String,
    pub entity_type: String,
    pub confidence: f64,
}

/// 整合结果 — 记忆整合操作的统计信息
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ConsolidationResult {
    pub promoted: usize,
    pub merged: usize,
    pub forgotten: usize,
    pub protected: usize,
}

/// 记忆流水线 — 统一的记忆摄入管道
///
/// 提供摄入→提取→整合→存储四步闭环，
/// 内置去重、容错、重试、一致性保护等生产级能力
pub struct MemoryPipeline;

impl MemoryPipeline {
    /// 摄入单条记忆 — 核心入口方法
    ///
    /// 流程：计算分层 → 去重检查 → 存储新记忆/更新已有记忆
    /// 支持强制指定层级(force_layer)、自动重试(MAX_PIPELINE_RETRIES次)
    pub async fn ingest(request: IngestionRequest) -> IngestionResult {
        let layer = request.force_layer.unwrap_or_else(|| {
            let importance = calc_importance_score(
                &request.fact_type,
                &request.source_type,
                request.tags.as_deref(),
                request.text.len(),
            );
            classify_to_layer(
                &request.fact_type,
                &request.source_type,
                request.tags.as_deref(),
                importance,
            )
        });

        let importance = calc_importance_score(
            &request.fact_type,
            &request.source_type,
            request.tags.as_deref(),
            request.text.len(),
        );

        let layer_config = LayerConfig::for_layer(layer);

        let dedup_result = Self::check_dedup(
            &request.agent_id,
            &request.text,
            layer,
            layer_config.dedup_similarity_threshold,
        ).await;

        if let Some((existing_id, existing_text)) = dedup_result {
            log::info!(
                "[MemoryPipeline:ingest] Dedup hit: layer={}, existing={}, similarity_threshold={}",
                layer, &existing_id[..8.min(existing_id.len())],
                layer_config.dedup_similarity_threshold
            );

            if should_update_existing(&existing_text, &request.text) {
                if let Err(e) = Self::update_with_retry(&existing_id, &request.text).await {
                    return IngestionResult {
                        memory_id: Some(existing_id.clone()),
                        layer,
                        importance_score: importance,
                        was_deduplicated: true,
                        was_updated: false,
                        duplicate_of: None,
                        error: Some(e),
                    };
                }
                return IngestionResult {
                    memory_id: Some(existing_id),
                    layer,
                    importance_score: importance,
                    was_deduplicated: true,
                    was_updated: true,
                    duplicate_of: None,
                    error: None,
                };
            }

            Self::increment_access_count(&existing_id).await.ok();

            return IngestionResult {
                memory_id: Some(existing_id.clone()),
                layer,
                importance_score: importance,
                was_deduplicated: true,
                was_updated: false,
                duplicate_of: Some(existing_id),
                error: None,
            };
        }

        let expires_at = Self::calc_expires_at(&layer, &layer_config);

        let mut last_error = String::new();
        for attempt in 0..MAX_PIPELINE_RETRIES {
            match store_enhanced_memory(
                &request.agent_id,
                request.conversation_id.as_deref(),
                &request.text,
                &request.fact_type,
                &request.source_type,
                request.context.as_deref(),
                request.tags.as_deref(),
            ).await {
                Ok(id) => {
                    if expires_at.is_some() {
                        Self::set_expires_at(&id, expires_at).await.ok();
                    }
                    Self::set_memory_layer(&id, &layer).await.ok();

                    log::info!(
                        "[MemoryPipeline:ingest] Stored: layer={}, id={:.8}, importance={:.1}",
                        layer, id, importance
                    );

                    return IngestionResult {
                        memory_id: Some(id),
                        layer,
                        importance_score: importance,
                        was_deduplicated: false,
                        was_updated: false,
                        duplicate_of: None,
                        error: None,
                    };
                }
                Err(e) => {
                    last_error = e.clone();
                    log::warn!(
                        "[MemoryPipeline:ingest] Store attempt {}/{} failed: {}",
                        attempt + 1, MAX_PIPELINE_RETRIES, e
                    );
                    if attempt < MAX_PIPELINE_RETRIES - 1 {
                        tokio::time::sleep(
                            std::time::Duration::from_millis(RETRY_DELAY_MS * (attempt as u64 + 1))
                        ).await;
                    }
                }
            }
        }

        IngestionResult {
            memory_id: None,
            layer,
            importance_score: importance,
            was_deduplicated: false,
            was_updated: false,
            duplicate_of: None,
            error: Some(last_error),
        }
    }

    /// 摄入一次对话交互 — 将用户消息和助手回复分别摄入记忆系统
    ///
    /// 用户消息存入Working层，助手回复提取事实后自动分层，
    /// 同时将整体对话作为experience存入Episodic层
    pub async fn ingest_interaction(
        agent_id: &str,
        conversation_id: Option<&str>,
        user_msg: &str,
        assistant_msg: &str,
    ) -> Vec<IngestionResult> {
        let mut results = Vec::new();

        let user_request = IngestionRequest {
            agent_id: agent_id.to_string(),
            conversation_id: conversation_id.map(|s| s.to_string()),
            text: format!("User: {}", user_msg),
            fact_type: "observation".to_string(),
            source_type: "conversation_summary".to_string(),
            context: None,
            tags: None,
            force_layer: Some(MemoryLayer::Working),
        };
        results.push(Self::ingest(user_request).await);

        let extracted = Self::extract(assistant_msg);

        for fact in &extracted.facts {
            let fact_request = IngestionRequest {
                agent_id: agent_id.to_string(),
                conversation_id: conversation_id.map(|s| s.to_string()),
                text: fact.text.clone(),
                fact_type: fact.fact_type.clone(),
                source_type: "conversation".to_string(),
                context: None,
                tags: None,
                force_layer: None,
            };
            results.push(Self::ingest(fact_request).await);
        }

        if !extracted.facts.is_empty() {
            let combined = format!("Assistant: {}", assistant_msg);
            let experience_request = IngestionRequest {
                agent_id: agent_id.to_string(),
                conversation_id: conversation_id.map(|s| s.to_string()),
                text: combined,
                fact_type: "experience".to_string(),
                source_type: "conversation_summary".to_string(),
                context: None,
                tags: None,
                force_layer: Some(MemoryLayer::Episodic),
            };
            results.push(Self::ingest(experience_request).await);
        }

        results
    }

    /// 从文本中提取事实和实体 — 基于模式匹配的轻量级提取
    ///
    /// 识别包含"是/指/定义为/总是/从不"等关键词的句子作为事实，
    /// 识别技术关键词(Rust/Python/Docker等)作为实体
    pub fn extract(text: &str) -> ExtractionResult {
        let mut facts = Vec::new();
        let mut entities = Vec::new();

        let sentences: Vec<&str> = text.split(|c: char| c == '.' || c == '。' || c == '\n')
            .map(|s| s.trim())
            .filter(|s| s.len() > 15)
            .collect();

        let fact_patterns = [
            ("is", "world", 0.7),
            ("are", "world", 0.7),
            ("means", "world", 0.8),
            ("refers to", "world", 0.8),
            ("defined as", "world", 0.9),
            ("always", "mental_model", 0.6),
            ("never", "mental_model", 0.6),
            ("should", "mental_model", 0.5),
            ("must", "mental_model", 0.6),
            ("是", "world", 0.7),
            ("指", "world", 0.8),
            ("定义为", "world", 0.9),
            ("总是", "mental_model", 0.6),
            ("从不", "mental_model", 0.6),
            ("应该", "mental_model", 0.5),
        ];

        for sentence in &sentences {
            let lower = sentence.to_lowercase();
            for (pattern, fact_type, confidence) in &fact_patterns {
                if lower.contains(pattern) {
                    facts.push(ExtractedFact {
                        text: sentence.to_string(),
                        fact_type: fact_type.to_string(),
                        confidence: *confidence,
                        source_span: None,
                    });
                    break;
                }
            }
        }

        let tech_keywords = [
            ("Rust", "technology"), ("Python", "technology"), ("TypeScript", "technology"),
            ("React", "technology"), ("Docker", "technology"), ("Kubernetes", "technology"),
            ("API", "concept"), ("REST", "concept"), ("GraphQL", "concept"),
        ];

        for (keyword, entity_type) in &tech_keywords {
            if text.contains(keyword) {
                entities.push(ExtractedEntity {
                    name: keyword.to_string(),
                    entity_type: entity_type.to_string(),
                    confidence: 0.8,
                });
            }
        }

        ExtractionResult {
            facts,
            entities,
            sentiment: None,
        }
    }

    /// 整合记忆 — 对指定Agent的所有记忆执行整合操作
    ///
    /// 整合包括：晋升(低层→高层)、遗忘(过期/低分)、压缩(超量层合并)
    /// 返回各类操作的统计计数
    pub async fn consolidate(agent_id: &str) -> Result<ConsolidationResult, String> {
        let db = get_db().await;
        let mut promoted = 0;
        let mut merged = 0;
        let mut forgotten = 0;
        let mut protected = 0;

        let all_units = memory_units::Entity::find()
            .filter(memory_units::Column::AgentId.eq(agent_id))
            .all(db).await
            .map_err(|e| e.to_string())?;

        let layered_units: Vec<LayeredMemoryUnit> = all_units.iter().map(|u| {
            let layer = u.memory_layer.as_deref()
                .map(|s| MemoryLayer::from_str(s))
                .unwrap_or_else(|| classify_to_layer(
                    &u.fact_type, &u.source_type, u.tags.as_deref(), u.importance_score
                ));

            let expires_at = u.expires_at;

            LayeredMemoryUnit {
                id: u.id.clone(),
                layer,
                text: u.text.clone(),
                fact_type: u.fact_type.clone(),
                context: u.context.clone(),
                occurred_at: u.occurred_at,
                expires_at,
                source_type: u.source_type.clone(),
                tags: u.tags.clone(),
                importance_score: u.importance_score,
                access_count: u.access_count,
                agent_id: u.agent_id.clone(),
                conversation_id: u.conversation_id.clone(),
                metadata: u.metadata.clone(),
            }
        }).collect();

        let plans = plan_consolidation(&layered_units);

        for plan in &plans {
            for unit_id in &plan.unit_ids {
                if let Some(_unit) = layered_units.iter().find(|u| u.id == *unit_id) {
                    if plan.target_layer == MemoryLayer::Procedural {
                        protected += 1;
                    }
                    match Self::promote_memory(unit_id, &plan.target_layer).await {
                        Ok(_) => {
                            promoted += 1;
                            log::info!(
                                "[MemoryPipeline:consolidate] Promoted {} → {} ({})",
                                plan.source_layer, plan.target_layer,
                                &unit_id[..8.min(unit_id.len())]
                            );
                        }
                        Err(e) => {
                            log::warn!(
                                "[MemoryPipeline:consolidate] Promotion failed: {}", e
                            );
                        }
                    }
                }
            }
        }

        for unit in &layered_units {
            let config = LayerConfig::for_layer(unit.layer);
            if should_forget(unit, &config) {
                match Self::forget_memory(&unit.id).await {
                    Ok(_) => {
                        forgotten += 1;
                    }
                    Err(e) => {
                        log::warn!(
                            "[MemoryPipeline:consolidate] Forget failed for {}: {}",
                            &unit.id[..8.min(unit.id.len())], e
                        );
                    }
                }
            }
        }

        let mut layer_counts: HashMap<MemoryLayer, usize> = HashMap::new();
        for unit in &layered_units {
            *layer_counts.entry(unit.layer).or_insert(0) += 1;
        }

        for (layer, count) in &layer_counts {
            let config = LayerConfig::for_layer(*layer);
            let trigger = (config.max_units as f64 * config.compaction_trigger_ratio) as usize;
            if *count > trigger {
                let to_remove = count.saturating_sub(
                    (config.max_units as f64 * config.compaction_retain_ratio) as usize
                );
                if to_remove > 0 {
                    match Self::compact_layer(agent_id, *layer, to_remove).await {
                        Ok(removed) => {
                            merged += removed;
                        }
                        Err(e) => {
                            log::warn!(
                                "[MemoryPipeline:consolidate] Layer compaction failed: {}", e
                            );
                        }
                    }
                }
            }
        }

        log::info!(
            "[MemoryPipeline:consolidate] Agent={}: promoted={}, merged={}, forgotten={}, protected={}",
            &agent_id[..8.min(agent_id.len())], promoted, merged, forgotten, protected
        );

        Ok(ConsolidationResult {
            promoted,
            merged,
            forgotten,
            protected,
        })
    }

    /// 去重检查 — 基于向量相似度检测是否已存在相同/相似记忆
    ///
    /// 优先使用SQLite向量扩展进行高效查询，降级到逐条余弦相似度比较
    /// 返回匹配的(记忆ID, 文本)对，未匹配返回None
    async fn check_dedup(
        agent_id: &str,
        text: &str,
        _layer: MemoryLayer,
        threshold: f64,
    ) -> Option<(String, String)> {
        let query_vec = embed_text(text);
        let query_bytes = vector_to_bytes(&query_vec);

        let db = get_db().await;

        match db.query_all(sea_orm::Statement::from_sql_and_values(
            db.get_database_backend(),
            "SELECT mv.memory_unit_id, mu.text, \
             1.0 - vector_distance_cosine(mv.embedding, ?1) AS similarity \
             FROM memory_vectors mv \
             JOIN memory_units mu ON mu.id = mv.memory_unit_id \
             WHERE mv.agent_id = ?2 \
             AND 1.0 - vector_distance_cosine(mv.embedding, ?1) > ?3 \
             ORDER BY similarity DESC LIMIT ?4",
            [
                query_bytes.into(),
                agent_id.to_string().into(),
                threshold.into(),
                (DEDUP_CANDIDATE_LIMIT as i64).into(),
            ],
        )).await {
            Ok(rows) => {
                for row in rows {
                    let Some(id) = row.try_get::<String>("", "memory_unit_id").ok() else { continue; };
                    let Some(existing_text) = row.try_get::<String>("", "text").ok() else { continue; };
                    let Some(sim) = row.try_get::<f64>("", "similarity").ok() else { continue; };
                    if sim >= threshold {
                        return Some((id, existing_text));
                    }
                }
                None
            }
            Err(_) => {
                let units = memory_units::Entity::find()
                    .filter(memory_units::Column::AgentId.eq(agent_id))
                    .order_by_desc(memory_units::Column::CreatedAt)
                    .limit(DEDUP_CANDIDATE_LIMIT as u64)
                    .all(db).await
                    .ok()?;

                for unit in units {
                    let stored_vec = bytes_to_vector(&unit.embedding);
                    if stored_vec.len() == query_vec.len() {
                        let sim = cosine_similarity(&stored_vec, &query_vec);
                        if sim >= threshold {
                            return Some((unit.id.clone(), unit.text.clone()));
                        }
                    }
                }
                None
            }
        }
    }

    /// 带重试的记忆更新 — 更新已有记忆的文本和嵌入向量
    ///
    /// 同时更新memory_units表、memory_vectors向量索引、memory_units_fts全文索引
    /// 最多重试MAX_PIPELINE_RETRIES次，每次间隔RETRY_DELAY_MS
    pub async fn update_with_retry(memory_id: &str, new_text: &str) -> Result<(), String> {
        let mut last_error = String::new();
        for attempt in 0..MAX_PIPELINE_RETRIES {
            let db = get_db().await;
            let unit = memory_units::Entity::find_by_id(memory_id.to_string())
                .one(db).await
                .map_err(|e| e.to_string())?;

            if let Some(model) = unit {
                let vector = embed_text(new_text);
                let embedding_bytes = vector_to_bytes(&vector);
                let now = chrono::Utc::now().timestamp();

                let mut am: memory_units::ActiveModel = model.into();
                am.text = Set(new_text.to_string());
                am.embedding = Set(embedding_bytes.clone());
                am.updated_at = Set(now);
                am.mentioned_at = Set(Some(now));

                let db = get_db().await;
                match am.update(db).await {
                    Ok(_) => {
                        let db = get_db().await;
                        let _ = db.execute(sea_orm::Statement::from_sql_and_values(
                            db.get_database_backend(),
                            "INSERT OR REPLACE INTO memory_vectors(rowid, embedding, memory_unit_id, agent_id) \
                             VALUES((SELECT rowid FROM memory_units WHERE id = ?1), ?2, ?1, \
                             (SELECT agent_id FROM memory_units WHERE id = ?1))",
                            [memory_id.into(), embedding_bytes.into()],
                        )).await;

                        let db = get_db().await;
                        let _ = db.execute(sea_orm::Statement::from_sql_and_values(
                            db.get_database_backend(),
                            "INSERT INTO memory_units_fts(rowid, text) VALUES(?1, ?2) \
                             ON CONFLICT(rowid) DO UPDATE SET text = excluded.text",
                            [memory_id.into(), new_text.into()],
                        )).await;

                        return Ok(());
                    }
                    Err(e) => {
                        last_error = e.to_string();
                        if attempt < MAX_PIPELINE_RETRIES - 1 {
                            tokio::time::sleep(
                                std::time::Duration::from_millis(RETRY_DELAY_MS)
                            ).await;
                        }
                    }
                }
            } else {
                return Err(format!("Memory unit {} not found", memory_id));
            }
        }
        Err(last_error)
    }

    /// 递增访问计数 — 记录记忆被访问的次数，用于遗忘评分
    async fn increment_access_count(memory_id: &str) -> Result<(), String> {
        let db = get_db().await;
        let unit = memory_units::Entity::find_by_id(memory_id.to_string())
            .one(db).await
            .map_err(|e| e.to_string())?;

        if let Some(model) = unit {
            let mut am: memory_units::ActiveModel = model.into();
            let current: i32 = match am.access_count {
                sea_orm::ActiveValue::Set(v) | sea_orm::ActiveValue::Unchanged(v) => v,
                _ => 0,
            };
            am.access_count = Set(current + 1);
            am.mentioned_at = Set(Some(chrono::Utc::now().timestamp()));

            let db = get_db().await;
            am.update(db).await.map_err(|e| e.to_string())?;
        }
        Ok(())
    }

    /// 设置过期时间 — 在metadata中写入expires_at时间戳
    async fn set_expires_at(memory_id: &str, expires_at: Option<i64>) -> Result<(), String> {
        let db = get_db().await;
        let unit = memory_units::Entity::find_by_id(memory_id.to_string())
            .one(db).await
            .map_err(|e| e.to_string())?;

        if let Some(model) = unit {
            let mut metadata_map: HashMap<String, serde_json::Value> = model.metadata
                .as_deref()
                .and_then(|m| serde_json::from_str(m).ok())
                .unwrap_or_default();

            if let Some(ea) = expires_at {
                metadata_map.insert("expires_at".to_string(), serde_json::json!(ea));
            }

            let mut am: memory_units::ActiveModel = model.into();
            am.metadata = Set(Some(serde_json::to_string(&metadata_map).unwrap_or_default()));

            let db = get_db().await;
            am.update(db).await.map_err(|e| e.to_string())?;
        }
        Ok(())
    }

    /// 设置记忆层级 — 在metadata中写入memory_layer字段
    async fn set_memory_layer(memory_id: &str, layer: &MemoryLayer) -> Result<(), String> {
        let db = get_db().await;
        let unit = memory_units::Entity::find_by_id(memory_id.to_string())
            .one(db).await
            .map_err(|e| e.to_string())?;

        if let Some(model) = unit {
            let mut metadata_map: HashMap<String, serde_json::Value> = model.metadata
                .as_deref()
                .and_then(|m| serde_json::from_str(m).ok())
                .unwrap_or_default();

            metadata_map.insert("memory_layer".to_string(), serde_json::json!(layer.as_str()));

            let mut am: memory_units::ActiveModel = model.into();
            am.metadata = Set(Some(serde_json::to_string(&metadata_map).unwrap_or_default()));

            let db = get_db().await;
            am.update(db).await.map_err(|e| e.to_string())?;
        }
        Ok(())
    }

    /// 晋升记忆 — 将记忆从低层提升到高层，同时更新fact_type和importance_score
    ///
    /// 晋升时importance_boost: Procedural=1.5, Semantic=1.0, Episodic=0.5, Working=0.0
    /// 最终importance_score上限为5.0
    async fn promote_memory(memory_id: &str, target_layer: &MemoryLayer) -> Result<(), String> {
        Self::set_memory_layer(memory_id, target_layer).await?;

        let db = get_db().await;
        let unit = memory_units::Entity::find_by_id(memory_id.to_string())
            .one(db).await
            .map_err(|e| e.to_string())?;

        if let Some(model) = unit {
            let new_fact_type = layer_to_fact_type_hint(*target_layer).to_string();
            let mut am: memory_units::ActiveModel = model.into();
            am.fact_type = Set(new_fact_type);

            let importance_boost = match target_layer {
                MemoryLayer::Procedural => 1.5,
                MemoryLayer::Semantic => 1.0,
                MemoryLayer::Episodic => 0.5,
                MemoryLayer::Working => 0.0,
            };
            let current: f64 = match am.importance_score {
                sea_orm::ActiveValue::Set(v) | sea_orm::ActiveValue::Unchanged(v) => v,
                _ => 1.0,
            };
            am.importance_score = Set((current + importance_boost).min(5.0));

            let db = get_db().await;
            am.update(db).await.map_err(|e| e.to_string())?;
        }
        Ok(())
    }

    /// 遗忘记忆 — 彻底删除一条记忆及其所有关联索引
    ///
    /// 依次删除: memory_vectors → memory_units_fts → unit_entities → memory_units
    async fn forget_memory(memory_id: &str) -> Result<(), String> {
        let db = get_db().await;

        let _ = db.execute(sea_orm::Statement::from_sql_and_values(
            db.get_database_backend(),
            "DELETE FROM memory_vectors WHERE memory_unit_id = ?1",
            [memory_id.into()],
        )).await;

        let db = get_db().await;
        let _ = db.execute(sea_orm::Statement::from_sql_and_values(
            db.get_database_backend(),
            "DELETE FROM memory_units_fts WHERE rowid = ?1",
            [memory_id.into()],
        )).await;

        let db = get_db().await;
        let _ = db.execute(sea_orm::Statement::from_sql_and_values(
            db.get_database_backend(),
            "DELETE FROM unit_entities WHERE unit_id = ?1",
            [memory_id.into()],
        )).await;

        let db = get_db().await;
        memory_units::Entity::delete_by_id(memory_id.to_string())
            .exec(db).await
            .map_err(|e| e.to_string())?;

        Ok(())
    }

    /// 压缩层级 — 当某层记忆数量超过阈值时，合并低分记忆为摘要
    ///
    /// 策略：importance<1.0的直接遗忘，其余每5条合并为一条摘要
    /// 跳过Procedural层和tool_knowledge/skill_knowledge标签的记忆
    async fn compact_layer(
        agent_id: &str,
        layer: MemoryLayer,
        to_remove: usize,
    ) -> Result<usize, String> {
        let db = get_db().await;

        let layer_str = layer.as_str();
        let units = memory_units::Entity::find()
            .filter(memory_units::Column::AgentId.eq(agent_id))
            .all(db).await
            .map_err(|e| e.to_string())?;

        let mut candidates: Vec<_> = units.iter()
            .filter(|u| {
                let unit_layer = u.metadata.as_deref()
                    .and_then(|m| serde_json::from_str::<HashMap<String, serde_json::Value>>(m).ok())
                    .and_then(|m| m.get("memory_layer").and_then(|v| v.as_str()).map(|s| s.to_string()))
                    .unwrap_or_else(|| classify_to_layer(
                        &u.fact_type, &u.source_type, u.tags.as_deref(), u.importance_score
                    ).as_str().to_string());
                unit_layer == layer_str
            })
            .filter(|u| {
                if layer == MemoryLayer::Procedural { return false; }
                u.tags.as_deref() != Some("tool_knowledge") && u.tags.as_deref() != Some("skill_knowledge")
            })
            .collect();

        candidates.sort_by(|a, b| {
            let score_a = a.importance_score * a.access_count as f64;
            let score_b = b.importance_score * b.access_count as f64;
            score_a.partial_cmp(&score_b).unwrap_or(std::cmp::Ordering::Equal)
        });

        let mut removed = 0;
        let mut group: Vec<&claw_db::db::entities::memory_units::Model> = Vec::new();

        for unit in &candidates {
            if removed >= to_remove { break; }

            if unit.importance_score < 1.0 {
                Self::forget_memory(&unit.id).await.ok();
                removed += 1;
            } else {
                group.push(unit);
                if group.len() >= 5 {
                    let combined: String = group.iter()
                        .map(|u| u.text.as_str())
                        .collect::<Vec<&str>>()
                        .join("\n");

                    let summary = summarize_for_layer(&combined, &layer);

                    store_enhanced_memory(
                        agent_id,
                        None,
                        &summary,
                        layer_to_fact_type_hint(layer),
                        "compaction",
                        Some(&format!("layer_compaction:{}", layer_str)),
                        None,
                    ).await.ok();

                    for u in &group {
                        Self::forget_memory(&u.id).await.ok();
                        removed += 1;
                    }
                    group.clear();
                }
            }
        }

        if !group.is_empty() && removed < to_remove {
            let combined: String = group.iter()
                .map(|u| u.text.as_str())
                .collect::<Vec<&str>>()
                .join("\n");
            let summary = summarize_for_layer(&combined, &layer);

            store_enhanced_memory(
                agent_id,
                None,
                &summary,
                layer_to_fact_type_hint(layer),
                "compaction",
                Some(&format!("layer_compaction:{}", layer_str)),
                None,
            ).await.ok();

            for u in &group {
                Self::forget_memory(&u.id).await.ok();
                removed += 1;
            }
        }

        Ok(removed)
    }

    /// 计算过期时间 — 根据层级配置的TTL计算expires_at时间戳
    fn calc_expires_at(_layer: &MemoryLayer, config: &LayerConfig) -> Option<i64> {
        config.ttl_seconds.map(|ttl| {
            chrono::Utc::now().timestamp() + ttl
        })
    }
}

/// 判断是否应更新已有记忆 — 当新文本比旧文本长2倍或包含>30%新关键词时更新
fn should_update_existing(existing_text: &str, new_text: &str) -> bool {
    if new_text.len() > existing_text.len() * 2 {
        return true;
    }

    let existing_keywords: std::collections::HashSet<String> = existing_text
        .split_whitespace()
        .filter(|w| w.len() > 4)
        .map(|w| w.to_lowercase())
        .collect();

    let new_keywords: std::collections::HashSet<String> = new_text
        .split_whitespace()
        .filter(|w| w.len() > 4)
        .map(|w| w.to_lowercase())
        .collect();

    let novel_ratio = if !new_keywords.is_empty() {
        let novel_count = new_keywords.difference(&existing_keywords).count();
        novel_count as f64 / new_keywords.len() as f64
    } else {
        0.0
    };

    novel_ratio > 0.3
}

/// 为层级生成摘要 — 提取关键词和关键句，生成压缩后的记忆文本
fn summarize_for_layer(text: &str, layer: &MemoryLayer) -> String {
    let prefix = match layer {
        MemoryLayer::Working => "[Working Memory Compressed]",
        MemoryLayer::Episodic => "[Episodic Memory Compressed]",
        MemoryLayer::Semantic => "[Semantic Memory Compressed]",
        MemoryLayer::Procedural => "[Procedural Memory Compressed]",
    };

    let sentences: Vec<&str> = text.split(|c: char| c == '\n' || c == '.')
        .map(|s| s.trim())
        .filter(|s| s.len() > 10)
        .collect();

    if sentences.is_empty() {
        return format!("{} {}", prefix, &text[..text.len().min(200)]);
    }

    let mut keywords = std::collections::HashMap::new();
    for sentence in &sentences {
        for word in sentence.split_whitespace() {
            let w: String = word.to_lowercase().chars().filter(|c| c.is_alphanumeric()).collect();
            if w.len() > 3 {
                *keywords.entry(w).or_insert(0) += 1;
            }
        }
    }

    let mut top_keywords: Vec<_> = keywords.iter().collect();
    top_keywords.sort_by(|a, b| b.1.cmp(a.1));
    let top_kw: Vec<&str> = top_keywords.iter().take(6).map(|(k, _)| k.as_str()).collect();

    let key_points: Vec<&str> = sentences.iter()
        .filter(|s| top_kw.iter().any(|kw| s.to_lowercase().contains(*kw)))
        .take(4)
        .cloned()
        .collect();

    if key_points.is_empty() {
        format!("{} Key topics: {}. {}", prefix, top_kw.join(", "), &sentences[0])
    } else {
        format!("{} Key topics: {}\n{}", prefix, top_kw.join(", "), key_points.join(". "))
    }
}

/// 按层级检索记忆 — 带层级权重的混合检索
///
/// 先用hybrid_retrieve获取候选集，再根据各层retrieval_weight加权排序
/// 支持自定义层级权重，默认使用calc_layer_retrieval_weights()
pub async fn retrieve_by_layers(
    query: &str,
    agent_id: &str,
    conversation_id: Option<&str>,
    layer_weights: Option<&HashMap<MemoryLayer, f64>>,
    limit: usize,
) -> Result<Vec<LayeredMemoryUnit>, String> {
    let weights = layer_weights.cloned().unwrap_or_else(calc_layer_retrieval_weights);

    let all_results = hybrid_retrieve(query, agent_id, conversation_id, limit * 3).await?;

    let mut layered: Vec<LayeredMemoryUnit> = all_results.into_iter().map(|r| {
        let layer = r.metadata.as_ref()
            .and_then(|m| serde_json::from_str::<HashMap<String, serde_json::Value>>(m).ok())
            .and_then(|m| m.get("memory_layer").and_then(|v| v.as_str()).map(|s| MemoryLayer::from_str(s)))
            .unwrap_or_else(|| classify_to_layer(&r.fact_type, &r.source_type, r.tags.as_deref(), r.importance_score));

        let expires_at = r.metadata.as_ref()
            .and_then(|m| serde_json::from_str::<HashMap<String, serde_json::Value>>(m).ok())
            .and_then(|m| m.get("expires_at").and_then(|v| v.as_i64()));

        LayeredMemoryUnit {
            id: r.id,
            layer,
            text: r.text,
            fact_type: r.fact_type,
            context: r.context,
            occurred_at: r.occurred_at,
            expires_at,
            source_type: r.source_type,
            tags: r.tags,
            importance_score: r.importance_score,
            access_count: 0,
            agent_id: agent_id.to_string(),
            conversation_id: conversation_id.map(|s| s.to_string()),
            metadata: None,
        }
    }).collect();

    for unit in &mut layered {
        let layer_weight = weights.get(&unit.layer).copied().unwrap_or(0.25);
        unit.importance_score *= layer_weight;
    }

    layered.sort_by(|a, b| {
        b.importance_score.partial_cmp(&a.importance_score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    layered.truncate(limit);
    Ok(layered)
}
