// Claw Desktop - 错误学习 - 捕获错误并生成规避规则
use crate::harness::types::{
    AvoidanceRule, ErrorCategory, ErrorEvent,
    MAX_AVOIDANCE_RULES_PER_AGENT, RULE_SIMILARITY_THRESHOLD,
};
use sea_orm::ConnectionTrait;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// 错误学习引擎 — 捕获错误并生成规避规则
///
/// 核心能力：错误捕获→模式提取→根因分析→修复建议→规则生成
/// 支持规则去重(相似度合并)、规则淘汰(低价值淘汰)、规则过期(30天TTL)
pub struct ErrorLearningEngine {
    rules: Arc<RwLock<HashMap<String, Vec<AvoidanceRule>>>>,
    pending_events: Arc<RwLock<Vec<ErrorEvent>>>,
}

impl ErrorLearningEngine {
    /// 创建新的错误学习引擎实例
    pub fn new() -> Self {
        Self {
            rules: Arc::new(RwLock::new(HashMap::new())),
            pending_events: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// 捕获错误事件 — 将错误事件加入待处理队列
    pub async fn capture_error(&self, event: ErrorEvent) {
        log::info!(
            "[ErrorLearning:capture_error] agent={} category={:?} msg_len={}",
            event.agent_id,
            event.category,
            event.error_message.len()
        );
        let mut pending = self.pending_events.write().await;
        pending.push(event);
    }

    /// 捕获并处理错误 — 一步完成捕获和规则生成
    pub async fn capture_and_process(&self, event: ErrorEvent) -> Result<Option<AvoidanceRule>, String> {
        let agent_id = event.agent_id.clone();
        let category = event.category.clone();
        let error_message = event.error_message.clone();
        let user_input = event.user_input_snippet.clone().unwrap_or_default();
        let context = event.context_snapshot.clone().unwrap_or_default();

        self.capture_error(event).await;

        self.generate_rule(&agent_id, &category, &error_message, &user_input, &context).await
    }

    /// 生成规避规则 — 提取错误模式、分析根因、生成修复建议
    ///
    /// 相似规则会合并(相似度>RULE_SIMILARITY_THRESHOLD)，超过上限则淘汰低价值规则
    async fn generate_rule(
        &self,
        agent_id: &str,
        category: &ErrorCategory,
        error_message: &str,
        _user_input: &str,
        context: &str,
    ) -> Result<Option<AvoidanceRule>, String> {
        let pattern = Self::extract_error_pattern(error_message, category);
        let cause = Self::analyze_root_cause(category, error_message, context);
        let fix = Self::generate_fix_suggestion(category, &pattern, &cause);

        let mut rules = self.rules.write().await;
        let agent_rules = rules.entry(agent_id.to_string()).or_insert_with(Vec::new);

        for existing in agent_rules.iter_mut() {
            if Self::compute_similarity(&existing.pattern, &pattern) > RULE_SIMILARITY_THRESHOLD {
                let merged = Self::merge_rules(existing, &cause, &fix);
                *existing = merged;
                log::info!(
                    "[ErrorLearning:generate_rule] Merged similar rule for agent={} pattern={:?}",
                    agent_id,
                    claw_types::truncate_str_safe(&pattern, 60)
                );
                return Ok(None);
            }
        }

        if agent_rules.len() >= MAX_AVOIDANCE_RULES_PER_AGENT {
            if let Some(oldest_idx) = agent_rules
                .iter()
                .enumerate()
                .filter(|(_, r)| r.trigger_count <= 1)
                .min_by_key(|(_, r)| r.last_triggered_at)
                .map(|(i, _)| i)
            {
                agent_rules.remove(oldest_idx);
                log::info!(
                    "[ErrorLearning:generate_rule] Evicted low-value rule for agent={}",
                    agent_id
                );
            }
        }

        let now = chrono::Utc::now().timestamp();
        let rule = AvoidanceRule {
            id: uuid::Uuid::new_v4().to_string(),
            agent_id: agent_id.to_string(),
            pattern: pattern.clone(),
            category: category.clone(),
            cause,
            fix,
            trigger_count: 1,
            last_triggered_at: now,
            created_at: now,
            expires_at: Some(now + 30 * 24 * 3600),
            is_deprecated: false,
        };

        log::info!(
            "[ErrorLearning:generate_rule] New rule for agent={} pattern={:?}",
            agent_id,
            claw_types::truncate_str_safe(&pattern, 60)
        );

        agent_rules.push(rule.clone());
        Ok(Some(rule))
    }

    /// 检查规避规则 — 返回与提议操作匹配的所有活跃规则
    ///
    /// 匹配阈值: 相似度>0.7，跳过已废弃和已过期的规则
    pub async fn check_avoidance_rules(
        &self,
        agent_id: &str,
        proposed_action: &str,
    ) -> Vec<AvoidanceRule> {
        let rules = self.rules.read().await;
        let agent_rules = match rules.get(agent_id) {
            Some(r) => r,
            None => return Vec::new(),
        };

        let now = chrono::Utc::now().timestamp();
        let mut triggered = Vec::new();

        for rule in agent_rules {
            if rule.is_deprecated {
                continue;
            }
            if let Some(expires) = rule.expires_at {
                if now > expires {
                    continue;
                }
            }
            if Self::compute_similarity(&rule.pattern, proposed_action) > 0.7 {
                triggered.push(rule.clone());
            }
        }

        triggered
    }

    /// 获取用于Prompt注入的规避规则文本 — 将活跃规则格式化为可注入System Prompt的段落
    pub async fn get_rules_for_prompt(&self, agent_id: &str) -> String {
        AvoidanceRule::rules_to_prompt_section(
            &self.get_active_rules(agent_id).await,
            agent_id,
        )
    }

    /// 获取指定Agent的所有活跃规则 — 排除已废弃和已过期的
    pub async fn get_active_rules(&self, agent_id: &str) -> Vec<AvoidanceRule> {
        let rules = self.rules.read().await;
        let agent_rules = match rules.get(agent_id) {
            Some(r) => r,
            None => return Vec::new(),
        };

        let now = chrono::Utc::now().timestamp();
        agent_rules
            .iter()
            .filter(|r| !r.is_deprecated && r.expires_at.map_or(true, |e| now <= e))
            .cloned()
            .collect()
    }

    /// 废弃规则 — 将指定规则标记为已废弃
    pub async fn deprecate_rule(&self, agent_id: &str, rule_id: &str) -> Result<(), String> {
        let mut rules = self.rules.write().await;
        if let Some(agent_rules) = rules.get_mut(agent_id) {
            for rule in agent_rules.iter_mut() {
                if rule.id == rule_id {
                    rule.is_deprecated = true;
                    log::info!("[ErrorLearning:deprecate_rule] Deprecated rule {} for agent={}", rule_id, agent_id);
                    return Ok(());
                }
            }
        }
        Err(format!("Rule {} not found for agent {}", rule_id, agent_id))
    }

    /// 触发规则命中 — 递增规则的触发计数
    pub async fn trigger_rule_hit(&self, rule_id: &str) {
        let mut rules = self.rules.write().await;
        for agent_rules in rules.values_mut() {
            for rule in agent_rules.iter_mut() {
                if rule.id == rule_id {
                    rule.trigger_count += 1;
                    log::info!("[ErrorLearning:trigger_rule_hit] Rule {} hit count={}", rule_id, rule.trigger_count);
                    return;
                }
            }
        }
    }

    /// 捕获并学习 — 便捷方法，一步完成错误捕获和规则生成，返回规则ID
    pub async fn capture_and_learn(
        &self,
        agent_id: &str,
        category: &ErrorCategory,
        error_message: &str,
        context: Option<&str>,
        _user_input: Option<&str>,
    ) -> String {
        let event = ErrorEvent {
            id: uuid::Uuid::new_v4().to_string(),
            agent_id: agent_id.to_string(),
            timestamp: chrono::Utc::now().timestamp_millis() as i64,
            category: category.clone(),
            error_message: error_message.to_string(),
            user_input_snippet: None,
            context_snapshot: context.map(|s| s.to_string()),
            is_processed: false,
            generated_rule_id: None,
        };
        self.capture_error(event).await;
        match self.generate_rule(
            agent_id,
            category,
            error_message,
            &format!("Auto-generated from error: {}", error_message),
            context.unwrap_or(""),
        ).await {
            Ok(Some(rule)) => rule.id,
            Ok(None) => String::new(),
            Err(e) => {
                log::warn!("[ErrorLearning:capture_and_learn] generate_rule failed: {}", e);
                String::new()
            }
        }
    }

    /// 从数据库加载规避规则 — 加载指定Agent的所有未废弃规则
    pub async fn load_rules_from_db(&self, agent_id: &str) -> Result<(), String> {
        let db = claw_db::db::get_db().await;
        let rows: Vec<sea_orm::QueryResult> = db
            .query_all(sea_orm::Statement::from_sql_and_values(
                db.get_database_backend(),
                "SELECT * FROM avoidance_rules WHERE agent_id = ?1 AND is_deprecated = 0 ORDER BY created_at DESC",
                [agent_id.into()],
            ))
            .await
            .map_err(|e: sea_orm::DbErr| e.to_string())?;

        let mut loaded = Vec::new();
        for row in &rows {
            let category_str = row
                .try_get::<String>("", "category")
                .unwrap_or_else(|_| "other".to_string());
            let category = match category_str.as_str() {
                "api" => ErrorCategory::ApiError,
                "tool" => ErrorCategory::ToolError,
                "logic" => ErrorCategory::LogicError,
                "context" => ErrorCategory::ContextError,
                "validation" => ErrorCategory::ValidationError,
                _ => ErrorCategory::Other,
            };

            loaded.push(AvoidanceRule {
                id: row.try_get::<String>("", "id").unwrap_or_default(),
                agent_id: row
                    .try_get::<String>("", "agent_id")
                    .unwrap_or_default(),
                pattern: row
                    .try_get::<String>("", "pattern")
                    .unwrap_or_default(),
                category,
                cause: row
                    .try_get::<String>("", "cause")
                    .unwrap_or_default(),
                fix: row.try_get::<String>("", "fix").unwrap_or_default(),
                trigger_count: row
                    .try_get::<i32>("", "trigger_count")
                    .unwrap_or(0) as u32,
                last_triggered_at: row
                    .try_get::<i64>("", "last_triggered_at")
                    .unwrap_or(0),
                created_at: row
                    .try_get::<i64>("", "created_at")
                    .unwrap_or(0),
                expires_at: row
                    .try_get::<Option<i64>>("", "expires_at")
                    .ok()
                    .flatten(),
                is_deprecated: row
                    .try_get::<bool>("", "is_deprecated")
                    .unwrap_or(false),
            });
        }

        let mut rules = self.rules.write().await;
        rules.insert(agent_id.to_string(), loaded);
        log::info!(
            "[ErrorLearning:load_rules_from_db] Loaded {} rules for agent={}",
            rules.get(agent_id).map(|r| r.len()).unwrap_or(0),
            agent_id
        );
        Ok(())
    }

    /// 保存规则到数据库 — INSERT OR REPLACE方式持久化
    pub async fn save_rule_to_db(&self, rule: &AvoidanceRule) -> Result<(), String> {
        let db = claw_db::db::get_db().await;
        db.execute(sea_orm::Statement::from_sql_and_values(
            db.get_database_backend(),
            "INSERT OR REPLACE INTO avoidance_rules (id, agent_id, pattern, category, cause, fix, trigger_count, last_triggered_at, created_at, expires_at, is_deprecated) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
            [
                rule.id.clone().into(),
                rule.agent_id.clone().into(),
                rule.pattern.clone().into(),
                rule.category.to_string().into(),
                rule.cause.clone().into(),
                rule.fix.clone().into(),
                (rule.trigger_count as i32).into(),
                rule.last_triggered_at.into(),
                rule.created_at.into(),
                rule.expires_at.into(),
                rule.is_deprecated.into(),
            ],
        ))
        .await
        .map_err(|e: sea_orm::DbErr| e.to_string())?;
        Ok(())
    }

    /// 提取错误模式 — 根据错误类别生成带前缀的模式字符串
    fn extract_error_pattern(error_message: &str, category: &ErrorCategory) -> String {
        let msg = error_message.trim();
        let max_len = 200;
        let truncated: String = msg.chars().take(max_len).collect();

        match category {
            ErrorCategory::ApiError => format!("API call failure: {}", truncated),
            ErrorCategory::ToolError => format!("Tool execution failure: {}", truncated),
            ErrorCategory::LogicError => format!("Logic/reasoning error: {}", truncated),
            ErrorCategory::ContextError => format!("Context/stale info error: {}", truncated),
            ErrorCategory::ValidationError => format!("Output validation failure: {}", truncated),
            ErrorCategory::Other => truncated,
        }
    }

    /// 分析根因 — 根据错误类别和消息内容推断根本原因
    fn analyze_root_cause(category: &ErrorCategory, error_message: &str, _context: &str) -> String {
        match category {
            ErrorCategory::ApiError => {
                let lower = error_message.to_lowercase();
                if lower.contains("rate limit") {
                    "API rate limit exceeded".to_string()
                } else if lower.contains("timeout") {
                    "API request timed out".to_string()
                } else if lower.contains("auth") || lower.contains("unauthorized") {
                    "API authentication failed".to_string()
                } else {
                    "API call returned an error".to_string()
                }
            }
            ErrorCategory::ToolError => {
                let lower = error_message.to_lowercase();
                if lower.contains("not found") || lower.contains("no such file") {
                    "Target resource does not exist".to_string()
                } else if lower.contains("permission") || lower.contains("access denied") {
                    "Insufficient permissions".to_string()
                } else if lower.contains("syntax") || lower.contains("parse") {
                    "Invalid input syntax".to_string()
                } else {
                    "Tool execution encountered an error".to_string()
                }
            }
            ErrorCategory::LogicError => "LLM produced logically incorrect output".to_string(),
            ErrorCategory::ContextError => "Decision based on outdated or incorrect context".to_string(),
            ErrorCategory::ValidationError => "Output did not meet expected format or constraints".to_string(),
            ErrorCategory::Other => "Unclassified error occurred".to_string(),
        }
    }

    /// 生成修复建议 — 根据错误类别和根因给出具体的修复方案
    fn generate_fix_suggestion(category: &ErrorCategory, pattern: &str, cause: &str) -> String {
        match category {
            ErrorCategory::ApiError => {
                if cause.contains("rate limit") {
                    "Add delay between API calls or use credential pool rotation".to_string()
                } else if cause.contains("timeout") {
                    "Reduce request complexity or increase timeout setting".to_string()
                } else {
                    "Verify API configuration and credentials".to_string()
                }
            }
            ErrorCategory::ToolError => {
                if cause.contains("not exist") {
                    "Verify target path/resource exists before operating".to_string()
                } else if cause.contains("permission") {
                    "Check file/directory permissions before access".to_string()
                } else {
                    "Validate tool inputs before execution".to_string()
                }
            }
            ErrorCategory::LogicError => {
                format!("Double-check reasoning steps when handling: {}", claw_types::truncate_str_safe(&pattern, 80))
            }
            ErrorCategory::ContextError => "Always verify information freshness before using it".to_string(),
            ErrorCategory::ValidationError => "Ensure output follows the expected format/schema".to_string(),
            ErrorCategory::Other => "Review and handle this error pattern carefully".to_string(),
        }
    }

    /// 计算文本相似度 — 使用Jaccard系数(词集合交集/并集)
    fn compute_similarity(a: &str, b: &str) -> f64 {
        if a == b {
            return 1.0;
        }
        let a_lower = a.to_lowercase();
        let b_lower = b.to_lowercase();

        let a_words: std::collections::HashSet<&str> = a_lower.split_whitespace().collect();
        let b_words: std::collections::HashSet<&str> = b_lower.split_whitespace().collect();

        if a_words.is_empty() && b_words.is_empty() {
            return 1.0;
        }
        if a_words.is_empty() || b_words.is_empty() {
            return 0.0;
        }

        let intersection = a_words.intersection(&b_words).count() as f64;
        let union = a_words.union(&b_words).count() as f64;
        intersection / union
    }

    /// 合并规则 — 将新的原因和修复建议合并到已有规则中，保留更详细的描述
    fn merge_rules(existing: &AvoidanceRule, new_cause: &str, new_fix: &str) -> AvoidanceRule {
        let mut merged = existing.clone();
        merged.trigger_count += 1;
        merged.last_triggered_at = chrono::Utc::now().timestamp();

        if new_cause.len() > merged.cause.len() {
            merged.cause = new_cause.to_string();
        }
        if new_fix.len() > merged.fix.len() {
            merged.fix = new_fix.to_string();
        }

        merged
    }
}

impl Default for ErrorLearningEngine {
    fn default() -> Self {
        Self::new()
    }
}
