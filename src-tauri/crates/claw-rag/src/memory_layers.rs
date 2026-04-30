// Claw Desktop - 四层记忆架构 - Agent Memory 分层系统
// 核心理念："记忆不是存出来的，是算出来的"
// 四层架构：工作记忆(短期) → 情景记忆(中期) → 语义记忆(长期) → 程序记忆(核心)
// 每层有独立的存储策略、检索策略、生命周期和遗忘机制

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

pub const MEMORY_LAYER_WORKING: &str = "working";
pub const MEMORY_LAYER_EPISODIC: &str = "episodic";
pub const MEMORY_LAYER_SEMANTIC: &str = "semantic";
pub const MEMORY_LAYER_PROCEDURAL: &str = "procedural";

/// 记忆层级枚举 — 四层记忆架构的核心类型
///
/// Working(工作记忆/短期) → Episodic(情景记忆/中期) → Semantic(语义记忆/长期) → Procedural(程序记忆/核心)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum MemoryLayer {
    Working,
    Episodic,
    Semantic,
    Procedural,
}

impl MemoryLayer {
    /// 返回层级对应的字符串标识
    pub fn as_str(&self) -> &'static str {
        match self {
            MemoryLayer::Working => MEMORY_LAYER_WORKING,
            MemoryLayer::Episodic => MEMORY_LAYER_EPISODIC,
            MemoryLayer::Semantic => MEMORY_LAYER_SEMANTIC,
            MemoryLayer::Procedural => MEMORY_LAYER_PROCEDURAL,
        }
    }

    /// 从字符串解析层级，未知字符串默认返回Episodic
    pub fn from_str(s: &str) -> Self {
        match s {
            MEMORY_LAYER_WORKING => MemoryLayer::Working,
            MEMORY_LAYER_EPISODIC => MemoryLayer::Episodic,
            MEMORY_LAYER_SEMANTIC => MemoryLayer::Semantic,
            MEMORY_LAYER_PROCEDURAL => MemoryLayer::Procedural,
            _ => MemoryLayer::Episodic,
        }
    }

    /// 返回层级优先级 — Procedural(4) > Semantic(3) > Episodic(2) > Working(1)
    pub fn priority(&self) -> u8 {
        match self {
            MemoryLayer::Procedural => 4,
            MemoryLayer::Semantic => 3,
            MemoryLayer::Episodic => 2,
            MemoryLayer::Working => 1,
        }
    }

    /// 返回所有层级的数组
    pub fn all() -> &'static [MemoryLayer] {
        &[
            MemoryLayer::Working,
            MemoryLayer::Episodic,
            MemoryLayer::Semantic,
            MemoryLayer::Procedural,
        ]
    }
}

impl std::fmt::Display for MemoryLayer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// 层级配置 — 每层记忆的存储策略、检索策略、生命周期参数
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LayerConfig {
    pub layer: MemoryLayer,
    pub max_units: usize,
    pub ttl_seconds: Option<i64>,
    pub compaction_trigger_ratio: f64,
    pub compaction_retain_ratio: f64,
    pub retrieval_weight: f64,
    pub forgetting_half_life_days: f64,
    pub dedup_similarity_threshold: f64,
    pub auto_consolidate: bool,
}

impl LayerConfig {
    /// 获取指定层级的配置参数
    ///
    /// Working: 100条上限, 1小时TTL, 0.042天半衰期
    /// Episodic: 300条上限, 无TTL, 30天半衰期
    /// Semantic: 500条上限, 无TTL, 365天半衰期
    /// Procedural: 200条上限, 永不过期, 无限半衰期
    pub fn for_layer(layer: MemoryLayer) -> Self {
        match layer {
            MemoryLayer::Working => LayerConfig {
                layer: MemoryLayer::Working,
                max_units: 100,
                ttl_seconds: Some(3600),
                compaction_trigger_ratio: 0.9,
                compaction_retain_ratio: 0.5,
                retrieval_weight: 0.4,
                forgetting_half_life_days: 0.042,
                dedup_similarity_threshold: 0.95,
                auto_consolidate: true,
            },
            MemoryLayer::Episodic => LayerConfig {
                layer: MemoryLayer::Episodic,
                max_units: 300,
                ttl_seconds: None,
                compaction_trigger_ratio: 0.8,
                compaction_retain_ratio: 0.6,
                retrieval_weight: 0.3,
                forgetting_half_life_days: 30.0,
                dedup_similarity_threshold: 0.90,
                auto_consolidate: true,
            },
            MemoryLayer::Semantic => LayerConfig {
                layer: MemoryLayer::Semantic,
                max_units: 500,
                ttl_seconds: None,
                compaction_trigger_ratio: 0.85,
                compaction_retain_ratio: 0.7,
                retrieval_weight: 0.2,
                forgetting_half_life_days: 365.0,
                dedup_similarity_threshold: 0.85,
                auto_consolidate: false,
            },
            MemoryLayer::Procedural => LayerConfig {
                layer: MemoryLayer::Procedural,
                max_units: 200,
                ttl_seconds: None,
                compaction_trigger_ratio: 1.0,
                compaction_retain_ratio: 1.0,
                retrieval_weight: 0.1,
                forgetting_half_life_days: f64::MAX,
                dedup_similarity_threshold: 0.95,
                auto_consolidate: false,
            },
        }
    }
}

/// 分层记忆单元 — 带层级信息的记忆数据结构
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LayeredMemoryUnit {
    pub id: String,
    pub layer: MemoryLayer,
    pub text: String,
    pub fact_type: String,
    pub context: Option<String>,
    pub occurred_at: Option<i64>,
    pub expires_at: Option<i64>,
    pub source_type: String,
    pub tags: Option<String>,
    pub importance_score: f64,
    pub access_count: i32,
    pub agent_id: String,
    pub conversation_id: Option<String>,
    pub metadata: Option<String>,
}

/// 层级检索结果 — 包含分层记忆单元和各层评分
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LayerRetrievalResult {
    pub units: Vec<LayeredMemoryUnit>,
    pub layer_scores: HashMap<String, f64>,
    pub total_candidates: usize,
}

/// 将记忆分类到对应层级 — 基于fact_type/source_type/tags/importance_score
///
/// 规则：tool_knowledge/skill_knowledge → Procedural, world → Semantic,
/// mental_model → Procedural, experience → Episodic, observation按重要性分层
pub fn classify_to_layer(
    fact_type: &str,
    source_type: &str,
    tags: Option<&str>,
    importance_score: f64,
) -> MemoryLayer {
    if let Some(t) = tags {
        if t == "tool_knowledge" || t == "skill_knowledge" {
            return MemoryLayer::Procedural;
        }
    }

    if source_type == "tool_init" {
        return MemoryLayer::Procedural;
    }

    match fact_type {
        "world" => MemoryLayer::Semantic,
        "mental_model" => MemoryLayer::Procedural,
        "experience" => MemoryLayer::Episodic,
        "observation" => {
            if importance_score >= 3.0 {
                MemoryLayer::Episodic
            } else {
                MemoryLayer::Working
            }
        }
        _ => {
            if importance_score >= 3.5 {
                MemoryLayer::Semantic
            } else if importance_score >= 1.5 {
                MemoryLayer::Episodic
            } else {
                MemoryLayer::Working
            }
        }
    }
}

/// 层级到事实类型的映射 — 每层对应一个默认的fact_type
pub fn layer_to_fact_type_hint(layer: MemoryLayer) -> &'static str {
    match layer {
        MemoryLayer::Working => "observation",
        MemoryLayer::Episodic => "experience",
        MemoryLayer::Semantic => "world",
        MemoryLayer::Procedural => "mental_model",
    }
}

/// 计算遗忘评分 — 基于时间衰减、访问频率和重要性
///
/// 公式: decay × importance × access_boost
/// decay = 2^(-days_elapsed / half_life_days)
/// access_boost = 1 + ln(access_count) × 0.1
pub fn calc_forgetting_score(
    occurred_at: i64,
    access_count: i32,
    importance_score: f64,
    half_life_days: f64,
) -> f64 {
    let now = chrono::Utc::now().timestamp();
    let days_elapsed = ((now - occurred_at) as f64) / 86400.0;
    let decay = 2.0_f64.powf(-days_elapsed / half_life_days);
    let access_boost = 1.0 + (access_count as f64).ln().max(0.0) * 0.1;
    decay * importance_score * access_boost
}

/// 判断是否应遗忘 — Procedural层永不遗忘，过期或遗忘评分<0.01的应遗忘
pub fn should_forget(unit: &LayeredMemoryUnit, config: &LayerConfig) -> bool {
    if unit.layer == MemoryLayer::Procedural {
        return false;
    }

    if let Some(expires_at) = unit.expires_at {
        let now = chrono::Utc::now().timestamp();
        if now > expires_at {
            return true;
        }
    }

    if let Some(occurred_at) = unit.occurred_at {
        let score = calc_forgetting_score(
            occurred_at,
            unit.access_count,
            unit.importance_score,
            config.forgetting_half_life_days,
        );
        if score < 0.01 {
            return true;
        }
    }

    false
}

/// 计算各层检索权重 — 基于各层配置的retrieval_weight归一化
pub fn calc_layer_retrieval_weights() -> HashMap<MemoryLayer, f64> {
    let mut weights = HashMap::new();
    for layer in MemoryLayer::all() {
        let config = LayerConfig::for_layer(*layer);
        weights.insert(*layer, config.retrieval_weight);
    }

    let total: f64 = weights.values().sum();
    if total > 0.0 {
        for v in weights.values_mut() {
            *v /= total;
        }
    }
    weights
}

/// 记忆整合计划 — 描述一条记忆的晋升目标
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryConsolidationPlan {
    pub source_layer: MemoryLayer,
    pub target_layer: MemoryLayer,
    pub unit_ids: Vec<String>,
    pub reason: String,
    pub consolidated_text: Option<String>,
}

/// 规划记忆整合 — 根据重要性和访问频率决定哪些记忆应晋升
///
/// Working→Episodic: importance≥2.0 或 access_count≥3
/// Episodic→Semantic: importance≥3.5 且 access_count≥5
/// Semantic→Procedural: fact_type为mental_model 或 importance≥4.5
pub fn plan_consolidation(units: &[LayeredMemoryUnit]) -> Vec<MemoryConsolidationPlan> {
    let mut plans = Vec::new();

    for unit in units {
        match unit.layer {
            MemoryLayer::Working => {
                if unit.importance_score >= 2.0 || unit.access_count >= 3 {
                    plans.push(MemoryConsolidationPlan {
                        source_layer: MemoryLayer::Working,
                        target_layer: MemoryLayer::Episodic,
                        unit_ids: vec![unit.id.clone()],
                        reason: format!(
                            "Working memory promoted: importance={:.1}, access_count={}",
                            unit.importance_score, unit.access_count
                        ),
                        consolidated_text: None,
                    });
                }
            }
            MemoryLayer::Episodic => {
                if unit.importance_score >= 3.5 && unit.access_count >= 5 {
                    plans.push(MemoryConsolidationPlan {
                        source_layer: MemoryLayer::Episodic,
                        target_layer: MemoryLayer::Semantic,
                        unit_ids: vec![unit.id.clone()],
                        reason: format!(
                            "Episodic memory crystallized: importance={:.1}, access_count={}",
                            unit.importance_score, unit.access_count
                        ),
                        consolidated_text: None,
                    });
                }
            }
            MemoryLayer::Semantic => {
                if unit.fact_type == "mental_model" || unit.importance_score >= 4.5 {
                    plans.push(MemoryConsolidationPlan {
                        source_layer: MemoryLayer::Semantic,
                        target_layer: MemoryLayer::Procedural,
                        unit_ids: vec![unit.id.clone()],
                        reason: format!(
                            "Semantic memory internalized: importance={:.1}",
                            unit.importance_score
                        ),
                        consolidated_text: None,
                    });
                }
            }
            MemoryLayer::Procedural => {}
        }
    }

    plans
}

/// 层级统计 — 单层的记忆统计信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LayerStats {
    pub layer: MemoryLayer,
    pub total_units: usize,
    pub total_importance: f64,
    pub avg_importance: f64,
    pub oldest_timestamp: Option<i64>,
    pub newest_timestamp: Option<i64>,
    pub expired_count: usize,
    pub forgettable_count: usize,
}

impl LayerStats {
    /// 创建指定层级的空统计实例
    pub fn new(layer: MemoryLayer) -> Self {
        Self {
            layer,
            total_units: 0,
            total_importance: 0.0,
            avg_importance: 0.0,
            oldest_timestamp: None,
            newest_timestamp: None,
            expired_count: 0,
            forgettable_count: 0,
        }
    }

    /// 更新统计 — 累加一条记忆的统计信息
    pub fn update(&mut self, unit: &LayeredMemoryUnit, config: &LayerConfig) {
        self.total_units += 1;
        self.total_importance += unit.importance_score;

        if let Some(t) = unit.occurred_at {
            self.oldest_timestamp = Some(self.oldest_timestamp.map_or(t, |old| old.min(t)));
            self.newest_timestamp = Some(self.newest_timestamp.map_or(t, |old| old.max(t)));
        }

        if let Some(expires_at) = unit.expires_at {
            let now = chrono::Utc::now().timestamp();
            if now > expires_at {
                self.expired_count += 1;
            }
        }

        if should_forget(unit, config) {
            self.forgettable_count += 1;
        }
    }

    /// 完成统计 — 计算平均重要性
    pub fn finalize(&mut self) {
        if self.total_units > 0 {
            self.avg_importance = self.total_importance / self.total_units as f64;
        }
    }
}
