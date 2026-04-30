// Claw Desktop - 内置记忆提供者 - 默认的记忆存储和检索实现
// 支持四层记忆架构和统一流水线
use crate::memory_layers::*;
use crate::memory_pipeline::{IngestionRequest, MemoryPipeline};
use crate::memory_provider::*;
use crate::rag::{build_rag_context, hybrid_retrieve};
use async_trait::async_trait;
use claw_db::db::entities::memory_units;
use claw_db::db::get_db;
use sea_orm::{ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter};

/// 内置记忆提供者 — 默认的记忆存储和检索实现
///
/// 实现MemoryProvider trait，支持四层记忆架构和统一流水线
pub struct BuiltinMemoryProvider;

impl BuiltinMemoryProvider {
    /// 创建新的内置记忆提供者实例
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl MemoryProvider for BuiltinMemoryProvider {
    /// 存储记忆 — 通过MemoryPipeline摄入单条记忆
    async fn store(&self, metadata: &MemoryMetadata, text: &str) -> Result<String, String> {
        let request = IngestionRequest {
            agent_id: metadata.agent_id.clone(),
            conversation_id: metadata.conversation_id.clone(),
            text: text.to_string(),
            fact_type: metadata.fact_type.clone(),
            source_type: metadata.source_type.clone(),
            context: metadata.context.clone(),
            tags: metadata.tags.clone(),
            force_layer: metadata.memory_layer,
        };

        let result = MemoryPipeline::ingest(request).await;
        match result.memory_id {
            Some(id) => Ok(id),
            None => Err(result.error.unwrap_or_else(|| "Store failed".to_string())),
        }
    }

    /// 检索记忆 — 使用混合检索(向量+BM25+时间)获取相关记忆
    async fn retrieve(
        &self,
        agent_id: &str,
        query: &str,
        conversation_id: Option<&str>,
        limit: usize,
    ) -> Result<Vec<MemoryUnit>, String> {
        let enhanced = hybrid_retrieve(query, agent_id, conversation_id, limit).await?;
        Ok(enhanced
            .into_iter()
            .map(|e| MemoryUnit {
                id: e.id,
                text: e.text,
                fact_type: e.fact_type,
                context: e.context,
                occurred_at: e.occurred_at,
                source_type: e.source_type,
                tags: e.tags,
                importance_score: e.importance_score,
                semantic_score: e.semantic_score,
                bm25_score: e.bm25_score,
                temporal_score: e.temporal_score,
                final_score: e.final_score,
                memory_layer: e
                    .metadata
                    .as_deref()
                    .and_then(|m| {
                        serde_json::from_str::<
                                std::collections::HashMap<String, serde_json::Value>,
                            >(m)
                            .ok()
                    })
                    .and_then(|m| {
                        m.get("memory_layer")
                            .and_then(|v| v.as_str())
                            .map(|s| MemoryLayer::from_str(s))
                    }),
                expires_at: e
                    .metadata
                    .as_ref()
                    .and_then(|m| {
                        serde_json::from_str::<
                                std::collections::HashMap<String, serde_json::Value>,
                            >(m)
                            .ok()
                    })
                    .and_then(|m| m.get("expires_at").and_then(|v| v.as_i64())),
                access_count: 0,
            })
            .collect())
    }

    /// 删除记忆 — 根据ID直接删除一条记忆
    async fn delete(&self, memory_id: &str) -> Result<(), String> {
        let db = get_db().await;
        memory_units::Entity::delete_by_id(memory_id.to_string())
            .exec(db)
            .await
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    /// 更新记忆文本 — 通过MemoryPipeline带重试地更新记忆内容
    async fn update(&self, memory_id: &str, text: &str) -> Result<(), String> {
        MemoryPipeline::update_with_retry(memory_id, text).await
    }

    /// 构建RAG上下文 — 为对话生成包含相关记忆的上下文文本
    async fn build_context(
        &self,
        agent_id: &str,
        conversation_id: &str,
        query: &str,
    ) -> Result<String, String> {
        build_rag_context(Some(agent_id), conversation_id, query).await
    }

    /// 存储对话交互 — 将用户消息和助手回复一起摄入记忆系统
    async fn store_interaction(
        &self,
        agent_id: &str,
        conversation_id: Option<&str>,
        user_msg: &str,
        assistant_msg: &str,
    ) -> Result<(), String> {
        let results =
            MemoryPipeline::ingest_interaction(agent_id, conversation_id, user_msg, assistant_msg)
                .await;
        let errors: Vec<String> = results.into_iter().filter_map(|r| r.error).collect();
        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors.join("; "))
        }
    }

    /// 返回提供者名称
    fn name(&self) -> &str {
        "builtin"
    }

    /// 返回提供者能力描述 — 语义搜索、全文搜索、实体提取、去重、整合等
    fn capabilities(&self) -> ProviderCapabilities {
        ProviderCapabilities {
            semantic_search: true,
            full_text_search: true,
            entity_extraction: true,
            cross_agent: false,
            max_context_tokens: 6000,
            layer_aware: true,
            deduplication: true,
            consolidation: true,
        }
    }

    /// 按层级检索记忆 — 支持指定层级和权重的分层检索
    async fn retrieve_by_layers(
        &self,
        request: &LayerAwareRetrieveRequest,
    ) -> Result<Vec<MemoryUnit>, String> {
        let layer_weights = if let Some(ref layers) = request.layers {
            let mut weights = std::collections::HashMap::new();
            let weight_per_layer = 1.0 / layers.len() as f64;
            for layer in layers {
                weights.insert(*layer, weight_per_layer);
            }
            Some(weights)
        } else {
            None
        };

        let layered = crate::memory_pipeline::retrieve_by_layers(
            &request.query,
            &request.agent_id,
            request.conversation_id.as_deref(),
            layer_weights.as_ref(),
            request.limit,
        )
        .await?;

        Ok(layered
            .into_iter()
            .map(|u| MemoryUnit {
                id: u.id,
                text: u.text,
                fact_type: u.fact_type,
                context: u.context,
                occurred_at: u.occurred_at,
                source_type: u.source_type,
                tags: u.tags,
                importance_score: u.importance_score,
                semantic_score: 0.0,
                bm25_score: 0.0,
                temporal_score: 0.0,
                final_score: u.importance_score,
                memory_layer: Some(u.layer),
                expires_at: u.expires_at,
                access_count: u.access_count,
            })
            .collect())
    }

    /// 整合记忆 — 对Agent的记忆执行晋升/遗忘/压缩整合
    async fn consolidate(&self, agent_id: &str) -> Result<ConsolidationStats, String> {
        let result = MemoryPipeline::consolidate(agent_id).await?;
        Ok(ConsolidationStats {
            promoted: result.promoted,
            merged: result.merged,
            forgotten: result.forgotten,
            protected: result.protected,
        })
    }

    /// 清理过期记忆 — 删除所有已超过expires_at时间戳的记忆
    async fn cleanup_expired(&self, agent_id: &str) -> Result<u64, String> {
        let db = get_db().await;
        let now = chrono::Utc::now().timestamp();

        let expired = memory_units::Entity::find()
            .filter(memory_units::Column::AgentId.eq(agent_id))
            .filter(memory_units::Column::ExpiresAt.lt(now))
            .all(db)
            .await
            .map_err(|e| e.to_string())?;

        let count = expired.len() as u64;
        for unit in &expired {
            let db = get_db().await;
            let _ = db
                .execute(sea_orm::Statement::from_sql_and_values(
                    db.get_database_backend(),
                    "DELETE FROM memory_vectors WHERE memory_unit_id = ?1",
                    [unit.id.clone().into()],
                ))
                .await;

            let db = get_db().await;
            let _ = db
                .execute(sea_orm::Statement::from_sql_and_values(
                    db.get_database_backend(),
                    "DELETE FROM memory_units_fts WHERE rowid = ?1",
                    [unit.id.clone().into()],
                ))
                .await;

            let db = get_db().await;
            memory_units::Entity::delete_by_id(unit.id.clone())
                .exec(db)
                .await
                .map_err(|e| e.to_string())?;
        }

        if count > 0 {
            log::info!(
                "[BuiltinProvider:cleanup_expired] Cleaned {} expired memories for agent={}",
                count,
                &agent_id[..8.min(agent_id.len())]
            );
        }
        Ok(count)
    }

    /// 获取层级统计 — 返回各层的记忆数量和平均重要性
    async fn get_layer_stats(&self, agent_id: &str) -> Result<Vec<LayerStatEntry>, String> {
        let db = get_db().await;
        let all_units = memory_units::Entity::find()
            .filter(memory_units::Column::AgentId.eq(agent_id))
            .all(db)
            .await
            .map_err(|e| e.to_string())?;

        let mut layer_data: std::collections::HashMap<MemoryLayer, (usize, f64)> =
            std::collections::HashMap::new();
        for unit in &all_units {
            let layer = unit
                .memory_layer
                .as_deref()
                .map(|s| MemoryLayer::from_str(s))
                .unwrap_or_else(|| {
                    classify_to_layer(
                        &unit.fact_type,
                        &unit.source_type,
                        unit.tags.as_deref(),
                        unit.importance_score,
                    )
                });
            let entry = layer_data.entry(layer).or_insert((0, 0.0));
            entry.0 += 1;
            entry.1 += unit.importance_score;
        }

        let mut stats = Vec::new();
        for (layer, (count, total_importance)) in layer_data {
            stats.push(LayerStatEntry {
                layer,
                count,
                avg_importance: if count > 0 {
                    total_importance / count as f64
                } else {
                    0.0
                },
            });
        }
        stats.sort_by(|a, b| b.layer.priority().cmp(&a.layer.priority()));
        Ok(stats)
    }
}
