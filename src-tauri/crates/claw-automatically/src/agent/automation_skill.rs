// Claw Desktop - 自动化技能系统
// 定义应用级操作知识，让CUA Agent能像人一样操作特定应用
// 技能文件格式: .md + YAML frontmatter（与项目SKILL.md格式一致）
// 开发环境: .build_temp/skills/
// 生产环境: 安装目录/skills/ + ~/.claw-desktop/skills/

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use once_cell::sync::Lazy;

/// 自动化技能 — 定义一个应用的完整操作知识
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutomationSkill {
    pub name: String,
    pub app_name: String,
    #[serde(default)]
    pub aliases: Vec<String>,
    pub description: String,
    #[serde(default)]
    pub shortcuts: HashMap<String, String>,
    #[serde(default)]
    pub operations: HashMap<String, SkillOperation>,
    #[serde(default)]
    pub ui_hints: Vec<UiHint>,
    #[serde(default)]
    pub error_states: Vec<ErrorState>,
    #[serde(skip)]
    pub source_path: Option<PathBuf>,
}

/// 技能操作 — 定义一个具体的操作流程
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillOperation {
    pub description: String,
    #[serde(default)]
    pub steps: Vec<OperationStep>,
}

/// 操作步骤 — 单个操作动作
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OperationStep {
    pub action: String,
    #[serde(default)]
    pub target: Option<String>,
    #[serde(default)]
    pub shortcut: Option<String>,
    #[serde(default)]
    pub input_placeholder: Option<String>,
    #[serde(default = "default_wait_ms")]
    pub wait_after_ms: u64,
    #[serde(default)]
    pub note: Option<String>,
}

/// UI提示 — 描述应用中的关键UI元素位置和外观
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UiHint {
    pub element: String,
    pub description: String,
    #[serde(default)]
    pub typical_position: Option<String>,
    #[serde(default)]
    pub look_for: Option<String>,
}

/// 错误状态 — 定义应用可能出现的错误状态和恢复方法
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorState {
    pub name: String,
    pub detection: String,
    pub recovery: String,
}

fn default_wait_ms() -> u64 {
    500
}

static SKILL_REGISTRY: Lazy<std::sync::Mutex<SkillRegistry>> =
    Lazy::new(|| std::sync::Mutex::new(SkillRegistry::new()));

pub struct SkillRegistry {
    skills: HashMap<String, AutomationSkill>,
    alias_index: HashMap<String, String>,
    loaded: bool,
}

impl SkillRegistry {
    fn new() -> Self {
        Self {
            skills: HashMap::new(),
            alias_index: HashMap::new(),
            loaded: false,
        }
    }

    fn register(&mut self, skill: AutomationSkill) {
        let key = skill.name.to_lowercase();
        for alias in &skill.aliases {
            self.alias_index.insert(alias.to_lowercase(), key.clone());
        }
        self.alias_index.insert(skill.app_name.to_lowercase(), key.clone());
        self.skills.insert(key, skill);
    }

    fn match_skill(&self, instruction: &str) -> Option<&AutomationSkill> {
        let lower = instruction.to_lowercase();

        let mut best_match: Option<&AutomationSkill> = None;
        let mut best_score = 0usize;

        for skill in self.skills.values() {
            let mut score = 0usize;

            if lower.contains(&skill.app_name.to_lowercase()) {
                score += 10;
            }

            for alias in &skill.aliases {
                if lower.contains(&alias.to_lowercase()) {
                    score += 8;
                }
            }

            for op in skill.operations.keys() {
                if lower.contains(&op.to_lowercase()) {
                    score += 5;
                }
            }

            if score > best_score {
                best_score = score;
                best_match = Some(skill);
            }
        }

        best_match
    }
}

/// 初始化技能注册表 — 扫描所有技能目录并加载 .md 文件
pub fn init_skills() {
    let mut registry = SKILL_REGISTRY.lock().unwrap();
    if registry.loaded {
        return;
    }

    load_builtin_skills(&mut registry);
    scan_skill_directories(&mut registry);

    registry.loaded = true;
    log::info!("[AutomationSkill] Initialized with {} skills", registry.skills.len());
}

/// 强制重新加载技能 — 用于热扫描新增/修改的技能文件
pub fn reload_skills() {
    let mut registry = SKILL_REGISTRY.lock().unwrap();
    registry.skills.clear();
    registry.alias_index.clear();
    registry.loaded = false;

    load_builtin_skills(&mut registry);
    scan_skill_directories(&mut registry);

    registry.loaded = true;
    log::info!("[AutomationSkill] Reloaded with {} skills", registry.skills.len());
}

pub fn match_skill(instruction: &str) -> Option<AutomationSkill> {
    init_skills();
    let registry = SKILL_REGISTRY.lock().unwrap();
    registry.match_skill(instruction).cloned()
}

pub fn list_skills() -> Vec<AutomationSkill> {
    init_skills();
    let registry = SKILL_REGISTRY.lock().unwrap();
    registry.skills.values().cloned().collect()
}

/// 获取技能搜索目录列表 — 开发环境 + 生产环境 + 用户目录
fn skill_search_paths() -> Vec<(PathBuf, &'static str)> {
    let mut paths = Vec::new();

    if let Ok(cwd) = std::env::current_dir() {
        let dev_skills = cwd.join(".build_temp").join("skills");
        if dev_skills.exists() || cwd.join("src-tauri").exists() {
            paths.push((dev_skills, "Dev"));
        }

        let bundled_skills = cwd.join("src-tauri").join("bundled-skills");
        if bundled_skills.exists() {
            paths.push((bundled_skills, "Bundled"));
        }
    }

    if let Ok(exe) = std::env::current_exe() {
        if let Some(exe_dir) = exe.parent() {
            let prod_skills = exe_dir.join("skills");
            if prod_skills.exists() {
                paths.push((prod_skills, "Production"));
            }
        }
    }

    let app_skills = claw_config::path_resolver::skills_dir();
    if app_skills.exists() {
        paths.push((app_skills, "AppData"));
    }

    if let Some(home) = dirs::home_dir() {
        let user_skills = home.join(".claw-desktop").join("skills");
        if user_skills.exists() {
            paths.push((user_skills, "User"));
        }
    }

    paths
}

/// 扫描所有技能目录，加载 .md 文件
fn scan_skill_directories(registry: &mut SkillRegistry) {
    let search_paths = skill_search_paths();
    let mut seen_names: std::collections::HashSet<String> = std::collections::HashSet::new();

    for (dir, source) in &search_paths {
        if !dir.exists() {
            continue;
        }
        log::info!("[AutomationSkill] Scanning {} skills from: {:?}", source, dir);
        scan_skill_dir(dir, source, registry, &mut seen_names);
    }
}

/// 递归扫描目录中的 .md 技能文件
fn scan_skill_dir(
    dir: &PathBuf,
    source: &str,
    registry: &mut SkillRegistry,
    seen_names: &mut std::collections::HashSet<String>,
) {
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                let skill_file = path.join("SKILL.md");
                if skill_file.exists() {
                    if let Some(skill) = parse_skill_md(&skill_file) {
                        if seen_names.insert(skill.name.clone()) {
                            log::info!(
                                "[AutomationSkill] Loaded {} skill: {} from {:?}",
                                source, skill.name, skill_file
                            );
                            registry.register(skill);
                        } else {
                            log::debug!(
                                "[AutomationSkill] Skipped duplicate skill: {} from {:?}",
                                skill.name, skill_file
                            );
                        }
                    }
                } else {
                    scan_skill_dir(&path, source, registry, seen_names);
                }
            } else if path.extension().map_or(false, |e| e == "md") {
                if let Some(skill) = parse_skill_md(&path) {
                    if seen_names.insert(skill.name.clone()) {
                        log::info!(
                            "[AutomationSkill] Loaded {} skill: {} from {:?}",
                            source, skill.name, path
                        );
                        registry.register(skill);
                    }
                }
            }
        }
    }
}

/// 解析 .md 技能文件 — 提取 YAML frontmatter 并反序列化
fn parse_skill_md(path: &PathBuf) -> Option<AutomationSkill> {
    let content = std::fs::read_to_string(path).ok()?;
    let (frontmatter, _body) = extract_frontmatter(&content)?;
    let mut skill: AutomationSkill = serde_yaml::from_str(&frontmatter).ok()?;
    skill.source_path = Some(path.clone());
    Some(skill)
}

/// 从 Markdown 内容中提取 YAML frontmatter
/// 格式: ---\nyaml\n---\nmarkdown body
fn extract_frontmatter(content: &str) -> Option<(String, String)> {
    let trimmed = content.trim_start();
    if !trimmed.starts_with("---") {
        return None;
    }

    let after_first = &trimmed[3..];
    if let Some(end_pos) = after_first.find("\n---") {
        let yaml = after_first[..end_pos].trim();
        let body = after_first[end_pos + 4..].trim().to_string();
        Some((yaml.to_string(), body))
    } else {
        None
    }
}

/// 将技能知识格式化为CUA Agent可用的提示词片段
pub fn format_skill_for_prompt(skill: &AutomationSkill) -> String {
    let mut prompt = format!(
        "\n\n=== APPLICATION-SPECIFIC KNOWLEDGE: {} ===\n",
        skill.app_name
    );

    prompt.push_str(&format!("Description: {}\n\n", skill.description));

    if !skill.shortcuts.is_empty() {
        prompt.push_str("KEYBOARD SHORTCUTS:\n");
        for (action, shortcut) in &skill.shortcuts {
            prompt.push_str(&format!("  - {}: {}\n", action, shortcut));
        }
        prompt.push_str("\n");
    }

    if !skill.operations.is_empty() {
        prompt.push_str("KNOWN OPERATIONS (follow these step sequences when applicable):\n");
        for (op_name, op) in &skill.operations {
            prompt.push_str(&format!("\n  Operation: {} — {}\n", op_name, op.description));
            for (i, step) in op.steps.iter().enumerate() {
                let mut step_desc = format!("    {}. {}", i + 1, step.action);
                if let Some(ref target) = step.target {
                    step_desc.push_str(&format!(" (target: {})", target));
                }
                if let Some(ref shortcut) = step.shortcut {
                    step_desc.push_str(&format!(" [shortcut: {}]", shortcut));
                }
                if step.wait_after_ms > 500 {
                    step_desc.push_str(&format!(" [wait {}ms]", step.wait_after_ms));
                }
                if let Some(ref note) = step.note {
                    step_desc.push_str(&format!(" — {}", note));
                }
                prompt.push_str(&step_desc);
                prompt.push('\n');
            }
        }
        prompt.push_str("\n");
    }

    if !skill.ui_hints.is_empty() {
        prompt.push_str("UI ELEMENT HINTS (what to look for on screen):\n");
        for hint in &skill.ui_hints {
            prompt.push_str(&format!("  - {}: {}", hint.element, hint.description));
            if let Some(ref pos) = hint.typical_position {
                prompt.push_str(&format!(" (typically: {})", pos));
            }
            if let Some(ref look) = hint.look_for {
                prompt.push_str(&format!(" [look for: {}]", look));
            }
            prompt.push('\n');
        }
        prompt.push_str("\n");
    }

    if !skill.error_states.is_empty() {
        prompt.push_str("ERROR STATES AND RECOVERY:\n");
        for err in &skill.error_states {
            prompt.push_str(&format!(
                "  - {}: Detection: {} → Recovery: {}\n",
                err.name, err.detection, err.recovery
            ));
        }
        prompt.push_str("\n");
    }

    prompt.push_str("=== END APPLICATION KNOWLEDGE ===\n");
    prompt
}

// ==================== 内置技能（硬编码回退） ====================
// 当磁盘上没有对应的 .md 文件时，使用这些内置定义

fn load_builtin_skills(registry: &mut SkillRegistry) {
    let builtin_skills = vec![
        create_wechat_skill(),
        create_qq_skill(),
        create_dingtalk_skill(),
        create_feishu_skill(),
        create_chrome_skill(),
        create_vscode_skill(),
        create_excel_skill(),
        create_word_skill(),
        create_notepad_skill(),
        create_explorer_skill(),
    ];

    for skill in builtin_skills {
        registry.register(skill);
    }
}

fn create_wechat_skill() -> AutomationSkill {
    let mut shortcuts = HashMap::new();
    shortcuts.insert("搜索".to_string(), "Ctrl+F".to_string());
    shortcuts.insert("发送消息".to_string(), "Enter".to_string());
    shortcuts.insert("换行".to_string(), "Shift+Enter".to_string());
    shortcuts.insert("截图".to_string(), "Alt+A".to_string());
    shortcuts.insert("文件传输".to_string(), "Ctrl+Shift+F".to_string());

    let mut operations = HashMap::new();
    operations.insert("send_message".to_string(), SkillOperation {
        description: "发送消息给联系人".to_string(),
        steps: vec![
            OperationStep { action: "确认微信已打开".to_string(), target: Some("微信窗口/任务栏图标".into()), shortcut: None, input_placeholder: None, wait_after_ms: 2000, note: Some("若未打开则使用open_app打开微信".into()) },
            OperationStep { action: "检查登录状态".to_string(), target: Some("登录二维码/主界面".into()), shortcut: None, input_placeholder: None, wait_after_ms: 1000, note: Some("若显示二维码则提示用户扫码".into()) },
            OperationStep { action: "打开搜索框".to_string(), target: Some("搜索框".into()), shortcut: Some("Ctrl+F".into()), input_placeholder: None, wait_after_ms: 500, note: None },
            OperationStep { action: "输入联系人名称".to_string(), target: Some("搜索输入框".into()), shortcut: None, input_placeholder: Some("contact_name".into()), wait_after_ms: 800, note: Some("输入后等待搜索结果".into()) },
            OperationStep { action: "点击搜索结果中的联系人".to_string(), target: Some("搜索结果列表".into()), shortcut: None, input_placeholder: None, wait_after_ms: 500, note: Some("点击第一个匹配结果".into()) },
            OperationStep { action: "点击聊天输入框".to_string(), target: Some("消息输入框".into()), shortcut: None, input_placeholder: None, wait_after_ms: 300, note: None },
            OperationStep { action: "输入消息内容".to_string(), target: Some("消息输入框".into()), shortcut: None, input_placeholder: Some("message".into()), wait_after_ms: 300, note: None },
            OperationStep { action: "发送消息".to_string(), target: None, shortcut: Some("Enter".into()), input_placeholder: None, wait_after_ms: 500, note: Some("确认消息已发送".into()) },
        ],
    });
    operations.insert("open_chat".to_string(), SkillOperation {
        description: "打开与某人的聊天窗口".to_string(),
        steps: vec![
            OperationStep { action: "确认微信已打开并登录".to_string(), target: None, shortcut: None, input_placeholder: None, wait_after_ms: 2000, note: None },
            OperationStep { action: "打开搜索".to_string(), target: None, shortcut: Some("Ctrl+F".into()), input_placeholder: None, wait_after_ms: 500, note: None },
            OperationStep { action: "输入联系人名称".to_string(), target: Some("搜索框".into()), shortcut: None, input_placeholder: Some("contact_name".into()), wait_after_ms: 800, note: None },
            OperationStep { action: "点击联系人".to_string(), target: Some("搜索结果".into()), shortcut: None, input_placeholder: None, wait_after_ms: 500, note: None },
        ],
    });

    AutomationSkill {
        name: "wechat".to_string(),
        app_name: "微信".to_string(),
        aliases: vec!["WeChat".to_string(), "wechat".to_string(), "weixin".to_string()],
        description: "微信桌面客户端 — 支持发送消息、搜索联系人、文件传输等操作".to_string(),
        shortcuts,
        operations,
        ui_hints: vec![
            UiHint { element: "搜索框".to_string(), description: "微信主界面顶部的搜索区域".to_string(), typical_position: Some("窗口顶部居中".into()), look_for: Some("搜索图标或\"搜索\"文字".into()) },
            UiHint { element: "聊天输入框".to_string(), description: "聊天窗口底部的文字输入区域".to_string(), typical_position: Some("窗口底部".into()), look_for: Some("空白输入区域或\"按Enter发送\"提示".into()) },
            UiHint { element: "联系人列表".to_string(), description: "左侧的聊天/联系人列表".to_string(), typical_position: Some("窗口左侧".into()), look_for: Some("头像和名称列表".into()) },
        ],
        error_states: vec![
            ErrorState { name: "未登录".to_string(), detection: "显示二维码登录界面".to_string(), recovery: "提示用户需要扫码登录，使用fail结束任务".to_string() },
            ErrorState { name: "联系人不存在".to_string(), detection: "搜索结果为空或显示\"无搜索结果\"".to_string(), recovery: "告知用户找不到该联系人，使用fail结束".to_string() },
            ErrorState { name: "消息发送失败".to_string(), detection: "消息旁显示红色感叹号".to_string(), recovery: "等待几秒后重试发送".to_string() },
        ],
        source_path: None,
    }
}

fn create_qq_skill() -> AutomationSkill {
    let mut shortcuts = HashMap::new();
    shortcuts.insert("搜索".to_string(), "Ctrl+F".to_string());
    shortcuts.insert("发送消息".to_string(), "Enter".to_string());
    shortcuts.insert("换行".to_string(), "Ctrl+Enter".to_string());

    let mut operations = HashMap::new();
    operations.insert("send_message".to_string(), SkillOperation {
        description: "发送消息给QQ好友".to_string(),
        steps: vec![
            OperationStep { action: "确认QQ已打开".to_string(), target: None, shortcut: None, input_placeholder: None, wait_after_ms: 2000, note: None },
            OperationStep { action: "检查登录状态".to_string(), target: Some("登录界面/主面板".into()), shortcut: None, input_placeholder: None, wait_after_ms: 1000, note: Some("若显示登录界面则提示用户登录".into()) },
            OperationStep { action: "打开搜索".to_string(), target: None, shortcut: Some("Ctrl+F".into()), input_placeholder: None, wait_after_ms: 500, note: None },
            OperationStep { action: "输入联系人名称或QQ号".to_string(), target: Some("搜索框".into()), shortcut: None, input_placeholder: Some("contact_name".into()), wait_after_ms: 800, note: None },
            OperationStep { action: "点击搜索结果中的联系人".to_string(), target: Some("搜索结果".into()), shortcut: None, input_placeholder: None, wait_after_ms: 500, note: None },
            OperationStep { action: "点击聊天输入框".to_string(), target: Some("消息输入框".into()), shortcut: None, input_placeholder: None, wait_after_ms: 300, note: None },
            OperationStep { action: "输入消息内容".to_string(), target: None, shortcut: None, input_placeholder: Some("message".into()), wait_after_ms: 300, note: None },
            OperationStep { action: "发送消息".to_string(), target: None, shortcut: Some("Enter".into()), input_placeholder: None, wait_after_ms: 500, note: None },
        ],
    });

    AutomationSkill {
        name: "qq".to_string(),
        app_name: "QQ".to_string(),
        aliases: vec!["腾讯QQ".to_string(), "qq".to_string(), "TIM".to_string(), "tim".to_string()],
        description: "QQ桌面客户端 — 支持发送消息、搜索好友等操作".to_string(),
        shortcuts,
        operations,
        ui_hints: vec![
            UiHint { element: "搜索框".to_string(), description: "QQ主面板顶部的搜索区域".to_string(), typical_position: Some("面板顶部".into()), look_for: Some("搜索图标".into()) },
            UiHint { element: "聊天输入框".to_string(), description: "聊天窗口底部的输入区域".to_string(), typical_position: Some("窗口底部".into()), look_for: Some("空白输入区域".into()) },
        ],
        error_states: vec![
            ErrorState { name: "未登录".to_string(), detection: "显示登录界面".to_string(), recovery: "提示用户需要登录".to_string() },
        ],
        source_path: None,
    }
}

fn create_dingtalk_skill() -> AutomationSkill {
    let mut shortcuts = HashMap::new();
    shortcuts.insert("搜索".to_string(), "Ctrl+F".to_string());
    shortcuts.insert("发送消息".to_string(), "Enter".to_string());

    let mut operations = HashMap::new();
    operations.insert("send_message".to_string(), SkillOperation {
        description: "发送钉钉消息给联系人".to_string(),
        steps: vec![
            OperationStep { action: "确认钉钉已打开".to_string(), target: None, shortcut: None, input_placeholder: None, wait_after_ms: 2000, note: None },
            OperationStep { action: "检查登录状态".to_string(), target: None, shortcut: None, input_placeholder: None, wait_after_ms: 1000, note: Some("若显示登录界面则提示用户登录".into()) },
            OperationStep { action: "打开搜索".to_string(), target: None, shortcut: Some("Ctrl+F".into()), input_placeholder: None, wait_after_ms: 500, note: None },
            OperationStep { action: "输入联系人名称".to_string(), target: Some("搜索框".into()), shortcut: None, input_placeholder: Some("contact_name".into()), wait_after_ms: 800, note: None },
            OperationStep { action: "点击搜索结果".to_string(), target: Some("搜索结果".into()), shortcut: None, input_placeholder: None, wait_after_ms: 500, note: None },
            OperationStep { action: "点击聊天输入框".to_string(), target: Some("消息输入框".into()), shortcut: None, input_placeholder: None, wait_after_ms: 300, note: None },
            OperationStep { action: "输入消息内容".to_string(), target: None, shortcut: None, input_placeholder: Some("message".into()), wait_after_ms: 300, note: None },
            OperationStep { action: "发送消息".to_string(), target: None, shortcut: Some("Enter".into()), input_placeholder: None, wait_after_ms: 500, note: None },
        ],
    });

    AutomationSkill {
        name: "dingtalk".to_string(),
        app_name: "钉钉".to_string(),
        aliases: vec!["DingTalk".to_string(), "dingtalk".to_string()],
        description: "钉钉桌面客户端 — 支持发送消息、搜索联系人等操作".to_string(),
        shortcuts,
        operations,
        ui_hints: vec![
            UiHint { element: "搜索框".to_string(), description: "钉钉主界面顶部的搜索区域".to_string(), typical_position: Some("窗口顶部".into()), look_for: Some("搜索图标".into()) },
        ],
        error_states: vec![
            ErrorState { name: "未登录".to_string(), detection: "显示登录界面".to_string(), recovery: "提示用户需要登录".to_string() },
        ],
        source_path: None,
    }
}

fn create_feishu_skill() -> AutomationSkill {
    let mut shortcuts = HashMap::new();
    shortcuts.insert("搜索".to_string(), "Ctrl+K".to_string());
    shortcuts.insert("发送消息".to_string(), "Enter".to_string());

    let mut operations = HashMap::new();
    operations.insert("send_message".to_string(), SkillOperation {
        description: "发送飞书消息给联系人".to_string(),
        steps: vec![
            OperationStep { action: "确认飞书已打开".to_string(), target: None, shortcut: None, input_placeholder: None, wait_after_ms: 2000, note: None },
            OperationStep { action: "检查登录状态".to_string(), target: None, shortcut: None, input_placeholder: None, wait_after_ms: 1000, note: Some("若显示登录界面则提示用户登录".into()) },
            OperationStep { action: "打开搜索".to_string(), target: None, shortcut: Some("Ctrl+K".into()), input_placeholder: None, wait_after_ms: 500, note: None },
            OperationStep { action: "输入联系人名称".to_string(), target: Some("搜索框".into()), shortcut: None, input_placeholder: Some("contact_name".into()), wait_after_ms: 800, note: None },
            OperationStep { action: "点击搜索结果".to_string(), target: Some("搜索结果".into()), shortcut: None, input_placeholder: None, wait_after_ms: 500, note: None },
            OperationStep { action: "点击聊天输入框".to_string(), target: Some("消息输入框".into()), shortcut: None, input_placeholder: None, wait_after_ms: 300, note: None },
            OperationStep { action: "输入消息内容".to_string(), target: None, shortcut: None, input_placeholder: Some("message".into()), wait_after_ms: 300, note: None },
            OperationStep { action: "发送消息".to_string(), target: None, shortcut: Some("Enter".into()), input_placeholder: None, wait_after_ms: 500, note: None },
        ],
    });

    AutomationSkill {
        name: "feishu".to_string(),
        app_name: "飞书".to_string(),
        aliases: vec!["Feishu".to_string(), "feishu".to_string(), "Lark".to_string(), "lark".to_string()],
        description: "飞书桌面客户端 — 支持发送消息、搜索联系人等操作".to_string(),
        shortcuts,
        operations,
        ui_hints: vec![
            UiHint { element: "搜索框".to_string(), description: "飞书主界面顶部的搜索区域".to_string(), typical_position: Some("窗口顶部".into()), look_for: Some("搜索图标或Ctrl+K提示".into()) },
        ],
        error_states: vec![
            ErrorState { name: "未登录".to_string(), detection: "显示登录界面".to_string(), recovery: "提示用户需要登录".to_string() },
        ],
        source_path: None,
    }
}

fn create_chrome_skill() -> AutomationSkill {
    let mut shortcuts = HashMap::new();
    shortcuts.insert("新建标签页".to_string(), "Ctrl+T".to_string());
    shortcuts.insert("关闭标签页".to_string(), "Ctrl+W".to_string());
    shortcuts.insert("地址栏".to_string(), "Ctrl+L".to_string());
    shortcuts.insert("查找".to_string(), "Ctrl+F".to_string());
    shortcuts.insert("刷新".to_string(), "F5".to_string());
    shortcuts.insert("强制刷新".to_string(), "Ctrl+F5".to_string());
    shortcuts.insert("开发者工具".to_string(), "F12".to_string());

    let mut operations = HashMap::new();
    operations.insert("open_url".to_string(), SkillOperation {
        description: "在Chrome中打开URL".to_string(),
        steps: vec![
            OperationStep { action: "确认Chrome已打开".to_string(), target: None, shortcut: None, input_placeholder: None, wait_after_ms: 1500, note: None },
            OperationStep { action: "聚焦地址栏".to_string(), target: None, shortcut: Some("Ctrl+L".into()), input_placeholder: None, wait_after_ms: 300, note: None },
            OperationStep { action: "输入URL".to_string(), target: Some("地址栏".into()), shortcut: None, input_placeholder: Some("url".into()), wait_after_ms: 300, note: None },
            OperationStep { action: "导航到页面".to_string(), target: None, shortcut: Some("Enter".into()), input_placeholder: None, wait_after_ms: 2000, note: Some("等待页面加载".into()) },
        ],
    });
    operations.insert("search".to_string(), SkillOperation {
        description: "在Chrome中搜索内容".to_string(),
        steps: vec![
            OperationStep { action: "确认Chrome已打开".to_string(), target: None, shortcut: None, input_placeholder: None, wait_after_ms: 1500, note: None },
            OperationStep { action: "新建标签页".to_string(), target: None, shortcut: Some("Ctrl+T".into()), input_placeholder: None, wait_after_ms: 500, note: None },
            OperationStep { action: "输入搜索关键词".to_string(), target: Some("地址栏".into()), shortcut: None, input_placeholder: Some("query".into()), wait_after_ms: 300, note: None },
            OperationStep { action: "执行搜索".to_string(), target: None, shortcut: Some("Enter".into()), input_placeholder: None, wait_after_ms: 2000, note: None },
        ],
    });

    AutomationSkill {
        name: "chrome".to_string(),
        app_name: "Chrome".to_string(),
        aliases: vec!["Google Chrome".to_string(), "chrome".to_string(), "谷歌浏览器".to_string()],
        description: "Google Chrome浏览器 — 支持打开URL、搜索、标签页管理等操作".to_string(),
        shortcuts,
        operations,
        ui_hints: vec![
            UiHint { element: "地址栏".to_string(), description: "浏览器顶部的URL输入框".to_string(), typical_position: Some("窗口顶部".into()), look_for: Some("URL文字或搜索提示".into()) },
        ],
        error_states: vec![],
        source_path: None,
    }
}

fn create_vscode_skill() -> AutomationSkill {
    let mut shortcuts = HashMap::new();
    shortcuts.insert("命令面板".to_string(), "Ctrl+Shift+P".to_string());
    shortcuts.insert("文件搜索".to_string(), "Ctrl+P".to_string());
    shortcuts.insert("全局搜索".to_string(), "Ctrl+Shift+F".to_string());
    shortcuts.insert("保存".to_string(), "Ctrl+S".to_string());
    shortcuts.insert("终端".to_string(), "Ctrl+`".to_string());

    AutomationSkill {
        name: "vscode".to_string(),
        app_name: "VSCode".to_string(),
        aliases: vec!["Visual Studio Code".to_string(), "vscode".to_string(), "Code".to_string()],
        description: "Visual Studio Code编辑器 — 支持打开文件、搜索、运行命令等操作".to_string(),
        shortcuts,
        operations: HashMap::new(),
        ui_hints: vec![],
        error_states: vec![],
        source_path: None,
    }
}

fn create_excel_skill() -> AutomationSkill {
    let mut shortcuts = HashMap::new();
    shortcuts.insert("保存".to_string(), "Ctrl+S".to_string());
    shortcuts.insert("另存为".to_string(), "F12".to_string());
    shortcuts.insert("查找".to_string(), "Ctrl+F".to_string());
    shortcuts.insert("全选".to_string(), "Ctrl+A".to_string());

    AutomationSkill {
        name: "excel".to_string(),
        app_name: "Excel".to_string(),
        aliases: vec!["Microsoft Excel".to_string(), "excel".to_string()],
        description: "Microsoft Excel电子表格 — 支持编辑单元格、保存文件等操作".to_string(),
        shortcuts,
        operations: HashMap::new(),
        ui_hints: vec![],
        error_states: vec![],
        source_path: None,
    }
}

fn create_word_skill() -> AutomationSkill {
    let mut shortcuts = HashMap::new();
    shortcuts.insert("保存".to_string(), "Ctrl+S".to_string());
    shortcuts.insert("另存为".to_string(), "F12".to_string());
    shortcuts.insert("查找".to_string(), "Ctrl+F".to_string());
    shortcuts.insert("全选".to_string(), "Ctrl+A".to_string());

    AutomationSkill {
        name: "word".to_string(),
        app_name: "Word".to_string(),
        aliases: vec!["Microsoft Word".to_string(), "word".to_string()],
        description: "Microsoft Word文档编辑器 — 支持输入文字、保存文件等操作".to_string(),
        shortcuts,
        operations: HashMap::new(),
        ui_hints: vec![],
        error_states: vec![],
        source_path: None,
    }
}

fn create_notepad_skill() -> AutomationSkill {
    let mut shortcuts = HashMap::new();
    shortcuts.insert("保存".to_string(), "Ctrl+S".to_string());
    shortcuts.insert("另存为".to_string(), "Ctrl+Shift+S".to_string());
    shortcuts.insert("查找".to_string(), "Ctrl+F".to_string());

    let mut operations = HashMap::new();
    operations.insert("write_text".to_string(), SkillOperation {
        description: "在记事本中输入文字".to_string(),
        steps: vec![
            OperationStep { action: "确认记事本已打开".to_string(), target: None, shortcut: None, input_placeholder: None, wait_after_ms: 1000, note: None },
            OperationStep { action: "点击编辑区域".to_string(), target: Some("文本编辑区".into()), shortcut: None, input_placeholder: None, wait_after_ms: 300, note: None },
            OperationStep { action: "输入文字内容".to_string(), target: None, shortcut: None, input_placeholder: Some("text".into()), wait_after_ms: 300, note: None },
        ],
    });

    AutomationSkill {
        name: "notepad".to_string(),
        app_name: "记事本".to_string(),
        aliases: vec!["Notepad".to_string(), "notepad".to_string()],
        description: "Windows记事本 — 支持输入文字、保存文件等操作".to_string(),
        shortcuts,
        operations,
        ui_hints: vec![],
        error_states: vec![],
        source_path: None,
    }
}

fn create_explorer_skill() -> AutomationSkill {
    let mut shortcuts = HashMap::new();
    shortcuts.insert("地址栏".to_string(), "Ctrl+L".to_string());
    shortcuts.insert("新建文件夹".to_string(), "Ctrl+Shift+N".to_string());
    shortcuts.insert("复制".to_string(), "Ctrl+C".to_string());
    shortcuts.insert("粘贴".to_string(), "Ctrl+V".to_string());
    shortcuts.insert("删除".to_string(), "Delete".to_string());

    AutomationSkill {
        name: "explorer".to_string(),
        app_name: "文件资源管理器".to_string(),
        aliases: vec!["Explorer".to_string(), "explorer".to_string(), "文件管理器".to_string()],
        description: "Windows文件资源管理器 — 支持导航目录、创建文件夹、复制粘贴文件等操作".to_string(),
        shortcuts,
        operations: HashMap::new(),
        ui_hints: vec![],
        error_states: vec![],
        source_path: None,
    }
}
