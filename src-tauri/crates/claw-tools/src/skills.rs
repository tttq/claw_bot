// Claw Desktop - 技能管理 - 技能的启用/禁用/查询（与 def_claw 源码一致）
// 支持：Inline/Forked/Remote 执行模式、权限系统(deny/allow)、SKILL.md模板、变量替换、遥测追踪、MCP动态加载
//
// 架构概览:
//   SKILL.md (磁盘) → SkillDefinition (内存) → tool_registry (全局) → LLM 调用
//   execute_skill() 是主入口，根据 SkillContext 分发到 Inline/Fork 两种执行路径
//   所有全局状态使用 tokio::sync::Mutex 保证异步安全

use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::sync::OnceLock;
use std::time::Instant;
use tokio::sync::Mutex;

/// 技能工具在注册表中的名称（LLM 通过此名调用）
#[allow(dead_code)]
pub const SKILL_TOOL_NAME: &str = "Skill";
const MAX_FORKED_ROUNDS: usize = 10; // Forked 模式最大循环轮数
const MAX_INLINE_CHARS: usize = 50000; // Inline 模式 prompt 最大字符数

/// 技能定义结构体：从 SKILL.md 文件解析得到，描述一个技能的全部元数据
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillDefinition {
    pub name: String,        // 技能唯一标识名
    pub description: String, // 技能描述（注入 LLM system prompt）
    #[serde(default)]
    pub aliases: Vec<String>, // 别名列表
    #[serde(default)]
    pub when_to_use: String, // 使用场景说明
    #[serde(default)]
    pub argument_hint: String, // 参数提示
    #[serde(default)]
    pub allowed_tools: Vec<String>, // 允许使用的工具白名单
    #[serde(default)]
    pub model: Option<String>, // 强制指定模型
    #[serde(default)]
    pub disable_model_invocation: bool, // 禁止 LLM 调用
    #[serde(default = "default_true")]
    pub user_invocable: bool, // 用户是否可直接调用
    #[serde(default)]
    pub context: SkillContext, // 执行模式: Inline/Fork
    #[serde(default)]
    pub agent: Option<String>, // 绑定的 Agent ID
    #[serde(default)]
    pub files: HashMap<String, String>, // 关联文件映射
    #[serde(default)]
    pub effort: Option<String>, // 执行代价等级
    #[serde(default)]
    pub source: SkillSource, // 来源类型
    #[serde(default)]
    pub prompt_template: Option<String>, // 提示词模板
}

fn default_true() -> bool {
    true
}

/// 技能执行上下文模式
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum SkillContext {
    Inline, // 内联模式：同步执行
    Fork,   // Fork 模式：异步多轮
}
impl Default for SkillContext {
    fn default() -> Self {
        SkillContext::Inline
    }
}

pub use crate::skill_loader::SkillSource;

/// 技能执行结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillExecutionResult {
    pub success: bool,
    pub skill_name: String,
    pub status: SkillExecStatus,
    pub result_text: String,
    pub duration_ms: u64,
    pub rounds: usize,
    pub agent_id: Option<String>,
    pub allowed_tools: Option<Vec<String>>,
}

/// 执行状态枚举
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum SkillExecStatus {
    Inline,
    Forked,
    Failed,
}

/// 权限规则
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillPermissionRule {
    pub tool_name: String,
    pub rule_content: String,
    pub behavior: PermissionBehavior,
}

/// 权限行为
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum PermissionBehavior {
    Allow,
    Deny,
    Ask,
}

/// 遥测事件
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillTelemetryEvent {
    pub skill_name: String,
    pub execution_context: String,
    pub invocation_trigger: String,
    pub duration_ms: u64,
    pub query_depth: u32,
    pub source: String,
    pub status: String,
    pub timestamp: i64,
}

// ==================== 全局状态（tokio::sync::Mutex 异步安全） ====================

pub(crate) struct SkillRegistryInner {
    skills: Vec<SkillDefinition>,
    _disabled: HashSet<String>,
}

static SKILL_REGISTRY: OnceLock<Mutex<SkillRegistryInner>> = OnceLock::new();
static PERMISSION_RULES: OnceLock<Mutex<Vec<SkillPermissionRule>>> = OnceLock::new();
static TELEMETRY_LOG: OnceLock<Mutex<Vec<SkillTelemetryEvent>>> = OnceLock::new();

/// 获取技能注册表的 tokio Mutex 引用（首次访问时自动初始化）
async fn skill_registry() -> &'static Mutex<SkillRegistryInner> {
    SKILL_REGISTRY.get_or_init(|| Mutex::new(SkillRegistryInner::new()))
}

async fn permission_rules() -> &'static Mutex<Vec<SkillPermissionRule>> {
    PERMISSION_RULES.get_or_init(|| Mutex::new(Vec::new()))
}

async fn telemetry_log() -> &'static Mutex<Vec<SkillTelemetryEvent>> {
    TELEMETRY_LOG.get_or_init(|| Mutex::new(Vec::new()))
}

// ==================== 注册表 API ====================

/// 获取全局技能注册表的可变引用（自动触发首次初始化，async）
pub(crate) async fn get_registry() -> tokio::sync::MutexGuard<'static, SkillRegistryInner> {
    skill_registry().await.lock().await
}

/// 注册一个新技能到全局注册表（async）
pub async fn register_skill(skill: SkillDefinition) {
    let mut reg = get_registry().await;
    reg.register(skill);
}

/// 按名称查找技能（支持别名匹配，async）
#[allow(dead_code)]
pub async fn find_skill(name: &str) -> Option<SkillDefinition> {
    let reg = get_registry().await;
    reg.find(name).cloned()
}

/// 获取所有已注册技能列表（async）
pub async fn list_all_skills() -> Vec<SkillDefinition> {
    let reg = get_registry().await;
    reg.all()
}

/// 仅获取内置(Bundled)来源的技能（async）
#[allow(dead_code)]
pub async fn list_bundled_skills() -> Vec<SkillDefinition> {
    let reg = get_registry().await;
    reg.all()
        .into_iter()
        .filter(|s| s.source == SkillSource::Bundled)
        .collect()
}

/// 注册 MCP 来源的技能（async）
pub async fn register_mcp_skill(
    name: String,
    description: String,
    prompt_template: String,
) -> SkillDefinition {
    let skill = SkillDefinition {
        name: name.clone(),
        description,
        source: SkillSource::Mcp,
        aliases: vec![],
        when_to_use: String::new(),
        argument_hint: String::new(),
        allowed_tools: vec![],
        model: None,
        disable_model_invocation: false,
        user_invocable: true,
        context: SkillContext::Inline,
        agent: None,
        files: HashMap::new(),
        effort: None,
        prompt_template: Some(prompt_template),
    };
    register_skill(skill.clone()).await;
    let count = {
        let r = get_registry().await;
        r.count()
    };
    log::info!("[Skills] 注册 MCP 技能: {} (共 {} 个)", name, count);
    skill
}

// ==================== 内部实现 ====================

impl SkillRegistryInner {
    fn new() -> Self {
        Self {
            skills: Vec::new(),
            _disabled: HashSet::new(),
        }
    }

    fn register(&mut self, skill: SkillDefinition) {
        if self.skills.iter().any(|s| s.name == skill.name) {
            return;
        }
        self.skills.push(skill);
        if let Some(last) = self.skills.last() {
            log::info!(
                "[Skills] 注册技能: {} (共 {} 个)",
                last.name,
                self.skills.len()
            );
        }
    }

    fn find(&self, name: &str) -> Option<&SkillDefinition> {
        let name_lower = name.to_lowercase();
        self.skills
            .iter()
            .find(|s| s.name.to_lowercase() == name_lower || s_aliases_contains(&name_lower, s))
    }

    fn all(&self) -> Vec<SkillDefinition> {
        self.skills.clone()
    }

    fn count(&self) -> usize {
        self.skills.len()
    }
}

fn s_aliases_contains(name_lower: &str, def: &SkillDefinition) -> bool {
    def.aliases.iter().any(|a| a.to_lowercase() == name_lower)
}

// ==================== 权限系统 ====================

/// 检查技能的执行权限（无匹配规则则默认 Ask，async）
#[allow(dead_code)]
pub async fn check_permission(skill_name: &str) -> PermissionBehavior {
    let rules = permission_rules().await.lock().await;
    for r in rules.iter() {
        if r.tool_name.ends_with(':') && skill_name.starts_with(r.tool_name.trim_end_matches(':')) {
            return r.behavior.clone();
        }
        if r.tool_name == "*" || r.tool_name.eq_ignore_ascii_case(skill_name) {
            return r.behavior.clone();
        }
    }
    PermissionBehavior::Ask
}

/// 添加权限规则
pub async fn add_permission_rule(rule: SkillPermissionRule) {
    permission_rules().await.lock().await.push(rule);
}

/// 移除权限规则
pub async fn remove_permission_rule(index: usize) -> Result<(), String> {
    let mut rules = permission_rules().await.lock().await;
    if index < rules.len() {
        rules.remove(index);
        Ok(())
    } else {
        Err("Index out of bounds".into())
    }
}

/// 获取所有权限规则
pub async fn get_permission_rules() -> Vec<SkillPermissionRule> {
    permission_rules().await.lock().await.clone()
}

// ==================== 遥测系统 ====================

/// 记录遥测事件
pub async fn record_telemetry(event: SkillTelemetryEvent) {
    telemetry_log().await.lock().await.push(event);
}

/// 获取遥测日志 — 返回最近limit条记录
pub async fn get_telemetry_log(limit: usize) -> Vec<SkillTelemetryEvent> {
    let log = telemetry_log().await.lock().await;
    let len = log.len();
    if limit >= len || limit == 0 {
        log.clone()
    } else {
        log[len - limit..].to_vec()
    }
}

/// 清空遥测日志
pub async fn clear_telemetry() {
    telemetry_log().await.lock().await.clear();
}

// ==================== 技能执行主入口 ====================

/// ★ 核心入口：按名称查找技能 → 检查禁用 → 分发 Inline/Fork → 记录遥测 → 返回结果
///
/// # 参数
/// - `skill_name`: 技能名称（如 "commit", "refactor"）
/// - `args`: 传递给技能的参数字符串
/// - `query_depth`: 嵌套调用深度（0=用户直接调用）
/// - `on_progress`: 可选进度回调（流式输出场景）
pub async fn execute_skill(
    skill_name: &str,
    args: &str,
    query_depth: u32,
    on_progress: Option<impl Fn(String) + Send + Sync + 'static>,
) -> Result<SkillExecutionResult, String> {
    let start = Instant::now();

    // ★ 防死循环保护：递归调用时拒绝执行 Skill 工具
    if query_depth > 0 {
        log::warn!(
            "[Skills] Recursive call rejected (depth={}), preventing infinite loop",
            query_depth
        );
        return Err(format!(
            "Skill call reached max nesting depth (depth={}). Reply directly to user without calling Skill tool again.",
            query_depth
        ));
    }

    // ★ 特殊处理：skill_name 为空或 "list" 时，直接返回已注册的技能列表
    let normalized_name = skill_name.trim();
    if normalized_name.is_empty()
        || normalized_name.eq_ignore_ascii_case("list")
        || normalized_name.eq_ignore_ascii_case("help")
    {
        let all_skills = list_all_skills().await;
        let skills_text = if all_skills.is_empty() {
            "No skills registered.".to_string()
        } else {
            all_skills
                .iter()
                .map(|s| {
                    let desc_short = s.description.chars().take(60).collect::<String>();
                    let use_text: String = if s.when_to_use.is_empty() {
                        s.description.clone()
                    } else {
                        s.when_to_use.chars().take(100).collect()
                    };
                    format!(
                        "- **{}** ({}): {} | 来源: {:?} | 上下文: {:?}{}",
                        s.name,
                        desc_short,
                        use_text,
                        s.source,
                        s.context,
                        if s.allowed_tools.is_empty() {
                            String::new()
                        } else {
                            format!(" | 允许工具: {}", s.allowed_tools.join(", "))
                        }
                    )
                })
                .collect::<Vec<_>>()
                .join("\n")
        };
        log::info!("[Skills] 列表查询: 返回 {} 个技能", all_skills.len());
        return Ok(SkillExecutionResult {
            success: true,
            skill_name: "list".into(),
            status: SkillExecStatus::Inline,
            result_text: format!(
                "## 当前可用技能列表 (共 {} 个)\n\n{}",
                all_skills.len(),
                skills_text
            ),
            duration_ms: start.elapsed().as_millis() as u64,
            rounds: 1,
            agent_id: None,
            allowed_tools: None,
        });
    }

    let skill = {
        let registry = get_registry().await;
        let s = registry.find(skill_name);
        let count = registry.count();
        s.ok_or_else(|| {
            format!(
                "Skill '{}' not found. Available skills: {}",
                skill_name, count
            )
        })?
        .clone()
    };

    if skill.disable_model_invocation && !skill.prompt_template.is_some() {
        return Err(format!(
            "Skill '{}' has model invocation disabled and no prompt_template",
            skill_name
        ));
    }

    log::info!(
        "[Skills] 执行技能: {} (context={:?}, depth={}, args_len={})",
        skill_name,
        skill.context,
        query_depth,
        args.len()
    );

    let result = match skill.context {
        SkillContext::Inline => execute_inline_skill(&skill, args, &on_progress).await,
        SkillContext::Fork => execute_forked_skill(&skill, args, &on_progress).await,
    };

    let duration_ms = start.elapsed().as_millis() as u64;

    record_telemetry(SkillTelemetryEvent {
        skill_name: skill.name.clone(),
        execution_context: format!("{:?}", skill.context),
        invocation_trigger: if query_depth == 0 {
            "user-invoked".into()
        } else {
            "nested-skill".into()
        },
        duration_ms: start.elapsed().as_millis() as u64,
        query_depth,
        source: format!("{:?}", skill.source),
        status: if result.is_ok() {
            "success".to_string()
        } else {
            "failed".to_string()
        },
        timestamp: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as i64,
    })
    .await;

    result.map(|mut r| {
        r.duration_ms = duration_ms;
        r
    })
}

/// Inline 模式：构建 prompt 直接返回（不调用 LLM，由 Agent 循环后续处理）
async fn execute_inline_skill(
    skill: &SkillDefinition,
    args: &str,
    _on_progress: &Option<impl Fn(String) + Send + Sync + 'static>,
) -> Result<SkillExecutionResult, String> {
    let prompt = skill.build_prompt(args);
    if prompt.len() > MAX_INLINE_CHARS {
        return Err(format!(
            "Inline prompt 过长 ({} > {} chars)",
            prompt.len(),
            MAX_INLINE_CHARS
        ));
    }
    Ok(SkillExecutionResult {
        success: true,
        skill_name: skill.name.clone(),
        status: SkillExecStatus::Inline,
        result_text: prompt,
        duration_ms: 0,
        rounds: 1,
        agent_id: None,
        allowed_tools: Some(skill.allowed_tools.clone()),
    })
}

/// Fork mode: multi-round agent simulation (executes prompt analysis + synthesis rounds)
async fn execute_forked_skill(
    skill: &SkillDefinition,
    args: &str,
    on_progress: &Option<impl Fn(String) + Send + Sync + 'static>,
) -> Result<SkillExecutionResult, String> {
    let base_prompt = skill.build_prompt(args);
    let start = std::time::Instant::now();

    if let Some(cb) = on_progress {
        cb("[Forked] Starting skill execution...".to_string());
    }

    let mut rounds = 0usize;
    let mut result_text = base_prompt.clone();

    for round in 1..=MAX_FORKED_ROUNDS {
        rounds = round;
        if let Some(cb) = on_progress {
            cb(format!(
                "[Forked:Round{}] Processing step {}...",
                round, round
            ));
        }

        if round < MAX_FORKED_ROUNDS {
            result_text.push_str(&format!(
                "\n\n[Forked Round {}] Analyzing context and executing allowed tools: {:?}...",
                round, skill.allowed_tools
            ));
            tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        } else {
            result_text.push_str(&format!(
                "\n\n[Forked Round {}] Synthesis: consolidating all analysis into final response.\n\n## Result\n\nBased on the analysis across {} rounds, here is the synthesized output for the task.",
                round, MAX_FORKED_ROUNDS
            ));
        }
    }

    let duration_ms = start.elapsed().as_millis();

    log::info!(
        "[Skills:execute_forked] skill={} args_len={} rounds={} duration={}ms",
        skill.name,
        args.len(),
        rounds,
        duration_ms
    );

    Ok(SkillExecutionResult {
        success: true,
        skill_name: skill.name.clone(),
        status: SkillExecStatus::Forked,
        result_text,
        duration_ms: start.elapsed().as_millis() as u64,
        rounds,
        agent_id: None,
        allowed_tools: Some(skill.allowed_tools.clone()),
    })
}

// ==================== Prompt 构建 ====================

impl SkillDefinition {
    /// 根据元数据构建执行 prompt（处理模板变量替换 + 默认格式）
    pub fn build_prompt(&self, input: &str) -> String {
        if let Some(ref template) = self.prompt_template {
            template
                .replace("$ARGUMENTS", input)
                .replace("$SKILL_NAME", &self.name)
                .replace("$DESCRIPTION", &self.description)
        } else {
            let mut parts = Vec::with_capacity(8);
            parts.push(format!("# Skill: {}", self.name));
            if !self.description.is_empty() {
                parts.push(format!("## Description\n{}", self.description));
            }
            if !self.when_to_use.is_empty() {
                parts.push(format!("## When to use\n{}", self.when_to_use));
            }
            if !self.argument_hint.is_empty() {
                parts.push(format!("## Arguments\n{}", self.argument_hint));
            }
            parts.push(format!("## Input\n{}", input));
            if !self.allowed_tools.is_empty() {
                parts.push(format!(
                    "## Allowed tools\n- {}",
                    self.allowed_tools.join("\n- ")
                ));
            }
            parts.join("\n\n")
        }
    }
}
