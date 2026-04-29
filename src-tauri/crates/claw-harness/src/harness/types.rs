// Claw Desktop - Harness类型 - 错误规则、画像、Hook等类型定义
// 定义所有 Harness 模块共享的数据结构、枚举和常量

use serde::{Deserialize, Serialize};

// ==================== 常量定义 ====================

/// 每个 Agent 最大规避规则数（防膨胀）
pub const MAX_AVOIDANCE_RULES_PER_AGENT: usize = 50;
/// 规则相似度阈值（超过此值视为重复，自动合并）
pub const RULE_SIMILARITY_THRESHOLD: f64 = 0.85;
/// 交叉记忆检索的最大上下文字符数
pub const CROSS_MEMORY_MAX_CHARS: usize = 2000;

// ==================== Agent 人物画像 (L1 Context Engineering) ====================

/// Agent 人物画像：定义 Agent 的性格、风格和专业特征
/// 影响 Agent 的回复风格、决策倾向和行为约束
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentPersona {
    pub agent_id: String,
    /// 显示名称
    pub display_name: String,
    /// 性格特征标签：["严谨", "幽默", "耐心", "直接"]
    pub personality_traits: Vec<String>,
    /// 沟通风格
    pub communication_style: CommunicationStyle,
    /// 专业领域/知识背景
    pub expertise_domain: String,
    /// 行为约束列表：["不主动提供建议", "避免技术术语"]
    pub behavior_constraints: Vec<String>,
    /// 回复基调指令（直接注入 system prompt）
    pub response_tone_instruction: String,
    /// 语言偏好
    pub language_preference: String,
    /// 创建时间
    pub created_at: i64,
    /// 更新时间
    pub updated_at: i64,
}

/// 沟通风格枚举
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CommunicationStyle {
    /// 正式学术风
    Formal,
    /// 轻松随意风
    Casual,
    /// 技术专业风
    Technical,
    /// 友好亲切风
    Friendly,
    /// 简洁高效风
    Concise,
    /// 教学引导风
    Educational,
}

impl Default for CommunicationStyle {
    fn default() -> Self { CommunicationStyle::Friendly }
}

impl std::fmt::Display for CommunicationStyle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CommunicationStyle::Formal => write!(f, "formal"),
            CommunicationStyle::Casual => write!(f, "casual"),
            CommunicationStyle::Technical => write!(f, "technical"),
            CommunicationStyle::Friendly => write!(f, "friendly"),
            CommunicationStyle::Concise => write!(f, "concise"),
            CommunicationStyle::Educational => write!(f, "educational"),
        }
    }
}

impl AgentPersona {
    /// 创建默认人物画像
    pub fn default_for_agent(agent_id: &str, name: &str) -> Self {
        let now = chrono::Utc::now().timestamp();
        Self {
            agent_id: agent_id.to_string(),
            display_name: name.to_string(),
            personality_traits: vec!["专业".to_string(), "乐于助人".to_string()],
            communication_style: CommunicationStyle::Friendly,
            expertise_domain: "General AI Assistant".to_string(),
            behavior_constraints: vec![],
            response_tone_instruction: String::new(),
            language_preference: "zh-CN".to_string(),
            created_at: now,
            updated_at: now,
        }
    }

    /// 将画像转换为 System Prompt 片段（注入到 LLM system prompt 中）
    pub fn to_system_prompt_fragment(&self) -> String {
        let mut parts = Vec::new();

        parts.push(format!("## Your Persona: {}\n", self.display_name));

        if !self.personality_traits.is_empty() {
            parts.push(format!("- Personality traits: {}", self.personality_traits.join(", ")));
        }
        parts.push(format!("- Communication style: {}", self.communication_style));
        if !self.expertise_domain.is_empty() {
            parts.push(format!("- Expertise domain: {}", self.expertise_domain));
        }
        if !self.behavior_constraints.is_empty() {
            parts.push(format!("- Constraints: {}", self.behavior_constraints.join("; ")));
        }
        if !self.response_tone_instruction.is_empty() {
            parts.push(format!("- Tone instruction: {}", self.response_tone_instruction));
        }
        if !self.language_preference.is_empty() {
            parts.push(format!("- Respond in: {}", self.language_preference));
        }

        parts.join("\n")
    }
}

// ==================== 错误学习与规避规则 (L6 Human Control + Error Recovery) ====================

/// 错误分类
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ErrorCategory {
    /// API 调用错误（超时、限流、认证失败）
    ApiError,
    /// 工具执行错误（文件不存在、命令失败）
    ToolError,
    /// 逻辑推理错误（输出格式错误、幻觉）
    LogicError,
    /// 上下文相关错误（信息过时、上下文污染）
    ContextError,
    /// 验证失败（输出不符合预期）
    ValidationError,
    /// 其他未分类错误
    Other,
}

impl std::fmt::Display for ErrorCategory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ErrorCategory::ApiError => write!(f, "api"),
            ErrorCategory::ToolError => write!(f, "tool"),
            ErrorCategory::LogicError => write!(f, "logic"),
            ErrorCategory::ContextError => write!(f, "context"),
            ErrorCategory::ValidationError => write!(f, "validation"),
            ErrorCategory::Other => write!(f, "other"),
        }
    }
}

/// 规避规则条目：从历史错误中提取的"不要这样做"规则
/// 自动写入 avoidance_rules.md，下次会话启动时注入 System Prompt
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AvoidanceRule {
    pub id: String,
    /// 关联的 Agent ID
    pub agent_id: String,
    /// 错误模式描述（人类可读）
    pub pattern: String,
    /// 错误分类
    pub category: ErrorCategory,
    /// 根因分析
    pub cause: String,
    /// 修复建议 / 规避指令
    pub fix: String,
    /// 该模式触发的次数（用于评估规则有效性）
    pub trigger_count: u32,
    /// 最后触发时间
    pub last_triggered_at: i64,
    /// 规则创建时间
    pub created_at: i64,
    /// 过期时间（None 表示永不过期）
    pub expires_at: Option<i64>,
    /// 是否已降权标记
    pub is_deprecated: bool,
}

impl AvoidanceRule {
    /// 将单条规则转换为 Prompt 指令格式
    pub fn to_prompt_instruction(&self) -> String {
        if self.is_deprecated {
            return String::new();
        }
        format!(
            "- [AVOID] {} | Fix: {} (triggered {}x)",
            self.pattern, self.fix, self.trigger_count
        )
    }

    /// 将多条规则集合转换为完整的 System Prompt 注入段
    pub fn rules_to_prompt_section(rules: &[AvoidanceRule], agent_id: &str) -> String {
        let active_rules: Vec<&AvoidanceRule> = rules
            .iter()
            .filter(|r| r.agent_id == agent_id && !r.is_deprecated)
            .collect();

        if active_rules.is_empty() {
            return String::new();
        }

        let mut section = String::from("\n## Learned Avoidance Rules (from past errors)\n");
        section.push_str("IMPORTANT: Follow these rules to avoid repeating known mistakes:\n\n");

        for rule in &active_rules {
            section.push_str(&rule.to_prompt_instruction());
            section.push('\n');
        }

        section.push_str("--- End Avoidance Rules ---\n");
        section
    }
}

/// 错误事件记录（用于学习循环的原始输入）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorEvent {
    pub id: String,
    pub agent_id: String,
    /// 错误分类
    pub category: ErrorCategory,
    /// 错误消息原文
    pub error_message: String,
    /// 出错时的用户输入（截断）
    pub user_input_snippet: Option<String>,
    /// 出错时的系统状态摘要
    pub context_snapshot: Option<String>,
    /// 时间戳
    pub timestamp: i64,
    /// 是否已处理为规避规则
    pub is_processed: bool,
    /// 生成的规则 ID（处理后填充）
    pub generated_rule_id: Option<String>,
}

// ==================== 交叉记忆 (L1 Context Engineering - Cross-Agent) ====================

/// 记忆访问权限级别
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MemoryVisibility {
    /// 仅自己可见
    Private,
    /// 同团队 Agent 可见
    Team,
    /// 所有 Agent 可见（默认）
    Public,
}

impl std::fmt::Display for MemoryVisibility {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MemoryVisibility::Private => write!(f, "private"),
            MemoryVisibility::Team => write!(f, "team"),
            MemoryVisibility::Public => write!(f, "public"),
        }
    }
}

impl Default for MemoryVisibility {
    fn default() -> Self { MemoryVisibility::Public }
}

/// 交叉记忆请求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrossMemoryRequest {
    /// 请求源 Agent ID
    pub source_agent_id: String,
    /// 目标 Agent ID 列表（@mention 的 Agent）
    pub target_agent_ids: Vec<String>,
    /// 检索查询文本
    pub query: String,
    /// 返回上下文的最大字符数
    pub context_limit: Option<usize>,
    /// 最小可见性要求（只能访问 >= 此级别的记忆）
    pub min_visibility: MemoryVisibility,
}

/// 交叉记忆响应条目
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrossMemoryEntry {
    /// 来源 Agent ID
    pub source_agent_id: String,
    /// 来源 Agent 名称
    pub source_agent_name: String,
    /// 记忆内容片段
    pub content: String,
    /// 相关度分数
    pub relevance_score: f64,
    /// 记忆类型
    pub fact_type: String,
    /// 时间戳
    pub occurred_at: Option<i64>,
}

// ==================== 验证引擎 (L5 Observability + Validation) ====================

/// 验证检查类型
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ValidationCheckType {
    /// 输出格式验证（JSON/Markdown/代码）
    FormatCheck,
    /// 内容安全验证（无敏感信息泄露）
    SafetyCheck,
    /// 事实一致性验证（不矛盾已知事实）
    FactConsistencyCheck,
    /// 工具调用参数验证
    ToolArgumentCheck,
    /// 输出长度验证（不超过限制）
    LengthCheck,
    /// 自定义正则验证
    CustomRegexCheck,
}

/// 验证结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationResult {
    pub check_type: ValidationCheckType,
    pub is_passed: bool,
    pub message: String,
    /// 严重程度：info/warn/error/critical
    pub severity: ValidationSeverity,
    /// 修复建议（如果未通过）
    pub fix_suggestion: Option<String>,
    /// 验证耗时（毫秒）
    pub duration_ms: u64,
}

/// 验证严重程度
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ValidationSeverity {
    Info,
    Warn,
    Error,
    Critical,
}

impl std::fmt::Display for ValidationSeverity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ValidationSeverity::Info => write!(f, "info"),
            ValidationSeverity::Warn => write!(f, "warn"),
            ValidationSeverity::Error => write!(f, "error"),
            ValidationSeverity::Critical => write!(f, "critical"),
        }
    }
}

// ==================== 可观测性追踪 (L5 Observability) ====================

/// Harness 事件类型（用于可观测性追踪）
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HarnessEventType {
    // === 生命周期事件 ===
    AgentStarted,
    AgentStopped,
    SessionCreated,
    SessionEnded,

    // === 任务执行事件 ===
    TaskDecomposed,       // 主Agent拆分任务
    TaskAssigned,          // 分配给子Agent
    TaskStarted,           // 子Agent开始执行
    TaskCompleted,         // 子Agent完成
    TaskFailed,            // 子Agent失败
    TaskAggregated,        // 结果聚合完成

    // === 记忆事件 ===
    MemoryStored,
    MemoryRetrieved,
    CrossMemoryAccessed,   // 交叉记忆被访问

    // === 错误事件 ===
    ErrorOccurred,
    ErrorRuleGenerated,    // 规避规则生成
    ErrorRuleTriggered,    // 规避规则被触发（成功避免）

    // === 验证事件 ===
    ValidationPerformed,
    ValidationFailed,

    // === 协作事件 ===
    MentionDetected,       // @mention 检测到
    CoordinationMessage,   // Agent间消息传递
}

impl std::fmt::Display for HarnessEventType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            HarnessEventType::AgentStarted => write!(f, "agent_started"),
            HarnessEventType::AgentStopped => write!(f, "agent_stopped"),
            HarnessEventType::SessionCreated => write!(f, "session_created"),
            HarnessEventType::SessionEnded => write!(f, "session_ended"),
            HarnessEventType::TaskDecomposed => write!(f, "task_decomposed"),
            HarnessEventType::TaskAssigned => write!(f, "task_assigned"),
            HarnessEventType::TaskStarted => write!(f, "task_started"),
            HarnessEventType::TaskCompleted => write!(f, "task_completed"),
            HarnessEventType::TaskFailed => write!(f, "task_failed"),
            HarnessEventType::TaskAggregated => write!(f, "task_aggregated"),
            HarnessEventType::MemoryStored => write!(f, "memory_stored"),
            HarnessEventType::MemoryRetrieved => write!(f, "memory_retrieved"),
            HarnessEventType::CrossMemoryAccessed => write!(f, "cross_memory_accessed"),
            HarnessEventType::ErrorOccurred => write!(f, "error_occurred"),
            HarnessEventType::ErrorRuleGenerated => write!(f, "error_rule_generated"),
            HarnessEventType::ErrorRuleTriggered => write!(f, "error_rule_triggered"),
            HarnessEventType::ValidationPerformed => write!(f, "validation_performed"),
            HarnessEventType::ValidationFailed => write!(f, "validation_failed"),
            HarnessEventType::MentionDetected => write!(f, "mention_detected"),
            HarnessEventType::CoordinationMessage => write!(f, "coordination_message"),
        }
    }
}

/// Harness 可观测性事件记录
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HarnessEvent {
    pub id: String,
    pub event_type: HarnessEventType,
    pub agent_id: String,
    /// 关联的任务/会话 ID
    pub correlation_id: Option<String>,
    /// 事件负载（JSON 字符串）
    pub payload: Option<String>,
    /// 时间戳
    pub timestamp: i64,
    /// 事件耗时（毫秒，可选）
    pub duration_ms: Option<u64>,
}

// ==================== 类型导出结束 ====================
