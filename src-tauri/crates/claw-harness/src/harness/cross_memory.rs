// Claw Desktop - 交叉记忆 - Agent间的记忆共享和检索
use crate::harness::types::{
    CrossMemoryEntry, CrossMemoryRequest, MemoryVisibility, CROSS_MEMORY_MAX_CHARS,
};
use sea_orm::{ConnectionTrait, EntityTrait};

/// 交叉记忆引擎 — Agent间的记忆共享和检索
///
/// 支持按可见性(Private/Team/Public)过滤、Jaccard相关性排序、
/// 上下文长度限制、@mention解析
pub struct CrossMemoryEngine;

impl CrossMemoryEngine {
    /// 检索交叉记忆 — 从目标Agent的记忆中检索与查询相关的条目
    ///
    /// 按相关性排序，截断到context_limit字符数
    pub async fn retrieve(request: &CrossMemoryRequest) -> Result<Vec<CrossMemoryEntry>, String> {
        log::info!(
            "[CrossMemory:retrieve] source={} targets={:?} query_len={}",
            request.source_agent_id,
            request.target_agent_ids,
            request.query.len()
        );

        let context_limit = request
            .context_limit
            .unwrap_or(CROSS_MEMORY_MAX_CHARS);

        let mut all_entries = Vec::new();

        for target_id in &request.target_agent_ids {
            if target_id == &request.source_agent_id {
                continue;
            }

            match Self::retrieve_from_agent(target_id, &request.query, &request.min_visibility).await {
                Ok(entries) => {
                    all_entries.extend(entries);
                }
                Err(e) => {
                    log::warn!(
                        "[CrossMemory:retrieve] Failed to retrieve from agent={}: {}",
                        target_id,
                        e
                    );
                }
            }
        }

        all_entries.sort_by(|a, b| {
            b.relevance_score
                .partial_cmp(&a.relevance_score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        let mut total_chars = 0usize;
        let mut result = Vec::new();
        for entry in all_entries {
            if total_chars + entry.content.len() > context_limit {
                let remaining = context_limit.saturating_sub(total_chars);
                if remaining > 50 {
                    let truncated: String = entry.content.chars().take(remaining).collect();
                    let mut e = entry;
                    e.content = format!("{}...", truncated);
                    result.push(e);
                }
                break;
            }
            total_chars += entry.content.len();
            result.push(entry);
        }

        log::info!(
            "[CrossMemory:retrieve] Returning {} entries ({} chars) for source={}",
            result.len(),
            total_chars,
            request.source_agent_id
        );

        Ok(result)
    }

    /// 从单个Agent检索记忆 — 按可见性过滤，使用Jaccard+重要性计算相关性
    async fn retrieve_from_agent(
        agent_id: &str,
        query: &str,
        min_visibility: &MemoryVisibility,
    ) -> Result<Vec<CrossMemoryEntry>, String> {
        let db = claw_db::db::get_db().await;

        let (sql, values): (String, Vec<sea_orm::Value>) = match min_visibility {
            MemoryVisibility::Private => (
                "SELECT id, agent_id, text, fact_type, occurred_at, importance_score, tags \
                 FROM memory_units \
                 WHERE agent_id = ?1 AND visibility IN ('private', 'team', 'public') \
                 ORDER BY importance_score DESC \
                 LIMIT 10".to_string(),
                vec![agent_id.into()],
            ),
            MemoryVisibility::Team => (
                "SELECT id, agent_id, text, fact_type, occurred_at, importance_score, tags \
                 FROM memory_units \
                 WHERE agent_id = ?1 AND visibility IN ('team', 'public') \
                 ORDER BY importance_score DESC \
                 LIMIT 10".to_string(),
                vec![agent_id.into()],
            ),
            MemoryVisibility::Public => (
                "SELECT id, agent_id, text, fact_type, occurred_at, importance_score, tags \
                 FROM memory_units \
                 WHERE agent_id = ?1 AND visibility = 'public' \
                 ORDER BY importance_score DESC \
                 LIMIT 10".to_string(),
                vec![agent_id.into()],
            ),
        };

        let rows: Vec<sea_orm::QueryResult> = db
            .query_all(sea_orm::Statement::from_sql_and_values(
                db.get_database_backend(),
                &sql,
                values,
            ))
            .await
            .map_err(|e: sea_orm::DbErr| e.to_string())?;

        let query_lower = query.to_lowercase();
        let query_words: std::collections::HashSet<&str> =
            query_lower.split_whitespace().collect();

        let mut entries = Vec::new();
        for row in &rows {
            let text = row
                .try_get::<String>("", "text")
                .unwrap_or_default();
            let fact_type = row
                .try_get::<String>("", "fact_type")
                .unwrap_or_else(|_| "observation".to_string());
            let occurred_at = row
                .try_get::<Option<i64>>("", "occurred_at")
                .ok()
                .flatten();
            let importance = row
                .try_get::<f64>("", "importance_score")
                .unwrap_or(1.0);

            let text_lower = text.to_lowercase();
            let text_words: std::collections::HashSet<&str> =
                text_lower.split_whitespace().collect();

            let intersection = query_words.intersection(&text_words).count() as f64;
            let union = query_words.union(&text_words).count() as f64;
            let jaccard = if union > 0.0 { intersection / union } else { 0.0 };

            let relevance = jaccard * 0.6 + (importance / 5.0).min(1.0) * 0.4;

            if relevance > 0.1 {
                let agent_name = Self::get_agent_name(agent_id).await;
                entries.push(CrossMemoryEntry {
                    source_agent_id: agent_id.to_string(),
                    source_agent_name: agent_name,
                    content: text,
                    relevance_score: relevance,
                    fact_type,
                    occurred_at,
                });
            }
        }

        Ok(entries)
    }

    /// 获取Agent显示名称 — 从agent_db查询display_name
    async fn get_agent_name(agent_id: &str) -> String {
        if let Some(agent_db) = claw_db::db::try_get_agent_db() {
            if let Ok(Some(agent)) =
                claw_db::db::agent_entities::agents::Entity::find_by_id(agent_id.to_string())
                    .one(agent_db)
                    .await
            {
                return agent.display_name.clone();
            }
        }
        agent_id.to_string()
    }

    /// 格式化交叉记忆上下文 — 将检索结果格式化为Markdown段落
    pub fn format_cross_memory_context(entries: &[CrossMemoryEntry]) -> String {
        if entries.is_empty() {
            return String::new();
        }

        let mut context = String::from("\n## Cross-Agent Memory (from other agents)\n");
        context.push_str("The following information was retrieved from other agents' memories:\n\n");

        for entry in entries {
            context.push_str(&format!(
                "- **[{}]** (from {}, relevance: {:.2}): {}\n",
                entry.fact_type, entry.source_agent_name, entry.relevance_score, entry.content
            ));
        }

        context.push_str("--- End Cross-Agent Memory ---\n");
        context
    }

    /// 解析@mention — 从文本中提取所有@AgentName引用
    pub fn parse_mentions(text: &str) -> Vec<serde_json::Value> {
        let mut mentions = Vec::new();
        for (start, end) in Self::find_mentions(text) {
            let name = text[start..end].trim_start_matches('@').to_string();
            mentions.push(serde_json::json!({
                "agentName": name,
                "startIndex": start,
                "endIndex": end,
            }));
        }
        mentions
    }

    /// 查找@mention位置 — 返回所有@name的(start, end)索引对
    fn find_mentions(text: &str) -> Vec<(usize, usize)> {
        let mut results = Vec::new();
        let chars: Vec<char> = text.chars().collect();
        let mut i = 0;
        while i < chars.len() {
            if chars[i] == '@' && i + 1 < chars.len() && chars[i + 1].is_alphabetic() {
                let start = i;
                i += 1;
                while i < chars.len() && (chars[i].is_alphanumeric() || chars[i] == '_' || chars[i] == '-') {
                    i += 1;
                }
                results.push((start, i));
            } else {
                i += 1;
            }
        }
        results
    }
}
