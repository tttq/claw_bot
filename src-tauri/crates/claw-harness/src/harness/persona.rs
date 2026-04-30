// Claw Desktop - 人物画像 - Agent性格、专长、沟通风格的建模
// 职责：管理每个 Agent 的独立人物画像，包括 CRUD 操作、
//       System Prompt 注入、以及与 agent.md 文件的同步

use crate::harness::types::{AgentPersona, CommunicationStyle};
use log;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

/// 人物画像管理器（内存缓存 + 文件持久化）
pub struct PersonaManager {
    /// agents/ 基础目录
    base_dir: PathBuf,
    /// 内存缓存: agent_id → Persona
    cache: HashMap<String, AgentPersona>,
}

impl PersonaManager {
    /// 创建新的人物画像管理器
    pub fn new(base_dir: &Path) -> Self {
        let mgr = Self {
            base_dir: base_dir.to_path_buf(),
            cache: HashMap::new(),
        };
        // 确保目录存在
        if !mgr.base_dir.exists() {
            if let Err(e) = fs::create_dir_all(&mgr.base_dir) {
                log::warn!(
                    "[PersonaManager] Failed to create base dir {:?}: {}",
                    mgr.base_dir,
                    e
                );
            }
        }
        mgr
    }

    /// 获取或加载指定 Agent 的人物画像
    pub fn get_persona(&mut self, agent_id: &str) -> Option<&AgentPersona> {
        if !self.cache.contains_key(agent_id) {
            self.load_persona(agent_id);
        }
        self.cache.get(agent_id)
    }

    /// 从 persona.md 文件加载画像
    fn load_persona(&mut self, agent_id: &str) {
        let path = self.persona_path(agent_id);
        match fs::read_to_string(&path) {
            Ok(content) => match self.parse_persona_md(&content, agent_id) {
                Ok(persona) => {
                    self.cache.insert(agent_id.to_string(), persona);
                    log::info!("[PersonaManager] Loaded persona for agent '{}'", agent_id);
                }
                Err(e) => {
                    log::warn!(
                        "[PersonaManager] Failed to parse persona for '{}': {}",
                        agent_id,
                        e
                    );
                    let default = AgentPersona::default_for_agent(agent_id, agent_id);
                    self.cache.insert(agent_id.to_string(), default);
                }
            },
            Err(_) => {
                // 不存在则使用默认画像
                let default = AgentPersona::default_for_agent(agent_id, agent_id);
                self.cache.insert(agent_id.to_string(), default);
                log::debug!(
                    "[PersonaManager] Using default persona for agent '{}'",
                    agent_id
                );
            }
        }
    }

    /// 解析 persona.md Markdown 文件为 AgentPersona 结构体
    fn parse_persona_md(&self, content: &str, agent_id: &str) -> Result<AgentPersona, String> {
        let now = chrono::Utc::now().timestamp();
        let mut display_name = agent_id.to_string();
        let mut personality_traits = Vec::new();
        let mut communication_style = CommunicationStyle::default();
        let mut expertise_domain = String::new();
        let mut behavior_constraints = Vec::new();
        let mut response_tone_instruction = String::new();
        let mut language_preference = "zh-CN".to_string();

        for line in content.lines() {
            let line = line.trim();

            // 解析 # 名称
            if line.starts_with("# ") {
                display_name = line[2..].trim().to_string();
                continue;
            }

            // 解析字段: **Field**: Value 或 Field: Value
            if line.contains(':') || line.contains("：") {
                let lower = line.to_lowercase();
                if lower.contains("personality") || lower.contains("性格") {
                    if let Some(value) = extract_colon_value(line) {
                        personality_traits = value
                            .split(['/', '、', ','])
                            .map(|s| s.trim().to_string())
                            .filter(|s| !s.is_empty())
                            .collect();
                    }
                } else if lower.contains("style") || lower.contains("风格") {
                    if let Some(value) = extract_colon_value(line) {
                        let v = value.trim().to_lowercase();
                        communication_style = match v.as_str() {
                            "formal" | "正式" | "学术" => CommunicationStyle::Formal,
                            "casual" | "随意" | "轻松" => CommunicationStyle::Casual,
                            "technical" | "技术" | "专业" => CommunicationStyle::Technical,
                            "friendly" | "友好" | "亲切" => CommunicationStyle::Friendly,
                            "concise" | "简洁" | "高效" => CommunicationStyle::Concise,
                            "educational" | "教学" | "引导" => CommunicationStyle::Educational,
                            _ => CommunicationStyle::default(),
                        };
                    }
                } else if lower.contains("expertise")
                    || lower.contains("domain")
                    || lower.contains("领域")
                {
                    if let Some(value) = extract_colon_value(line) {
                        expertise_domain = value.trim().to_string();
                    }
                } else if lower.contains("constraint")
                    || lower.contains("约束")
                    || lower.contains("限制")
                {
                    if let Some(value) = extract_colon_value(line) {
                        behavior_constraints = value
                            .split([';', '|'])
                            .map(|s| s.trim().to_string())
                            .filter(|s| !s.is_empty())
                            .collect();
                    }
                } else if lower.contains("tone") || lower.contains("基调") || lower.contains("回复")
                {
                    if let Some(value) = extract_colon_value(line) {
                        response_tone_instruction = value.trim().to_string();
                    }
                } else if lower.contains("language") || lower.contains("语言") {
                    if let Some(value) = extract_colon_value(line) {
                        language_preference = value.trim().to_string();
                    }
                }
            }
        }

        Ok(AgentPersona {
            agent_id: agent_id.to_string(),
            display_name,
            personality_traits,
            communication_style,
            expertise_domain,
            behavior_constraints,
            response_tone_instruction,
            language_preference,
            created_at: now,
            updated_at: now,
        })
    }

    /// 保存或更新 Agent 画像到文件系统
    pub fn save_persona(&mut self, persona: &AgentPersona) -> Result<(), String> {
        let path = self.persona_path(&persona.agent_id);

        // 确保目录存在
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|e| format!("Failed to create dir: {}", e))?;
        }

        let md_content = format!(
            "# {}\n\n\
             > **Personality / 性格**: {}\n\
             > **Communication Style / 风格**: {}\n\
             > **Expertise Domain / 领域**: {}\n\
             > **Behavior Constraints / 约束**: {}\n\
             > **Tone Instruction / 基调**: {}\n\
             > **Language / 语言**: {}\n",
            persona.display_name,
            persona.personality_traits.join(" / "),
            persona.communication_style,
            persona.expertise_domain,
            persona.behavior_constraints.join("; "),
            persona.response_tone_instruction,
            persona.language_preference,
        );

        fs::write(&path, &md_content)
            .map_err(|e| format!("Failed to write persona file {:?}: {}", path, e))?;

        // 更新缓存
        let mut updated = persona.clone();
        updated.updated_at = chrono::Utc::now().timestamp();
        self.cache.insert(persona.agent_id.clone(), updated);

        log::info!(
            "[PersonaManager] Saved persona for agent '{}'",
            persona.agent_id
        );
        Ok(())
    }

    /// 更新指定 Agent 的画像字段
    pub fn update_persona_field(
        &mut self,
        agent_id: &str,
        field: &str,
        value: &str,
    ) -> Result<(), String> {
        // 先 clone persona 数据，释放借用后再调用 save_persona
        let mut persona = match self.get_persona(agent_id) {
            Some(p) => p.clone(),
            None => return Err(format!("Agent '{}' not found", agent_id)),
        };

        match field.to_lowercase().as_str() {
            "name" | "display_name" => {
                persona.display_name = value.to_string();
            }
            "traits" | "personality" => {
                persona.personality_traits = value
                    .split([',', '/', '、'])
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
                    .collect();
            }
            "style" | "communication_style" => {
                persona.communication_style = match value.to_lowercase().as_str() {
                    "formal" | "正式" => CommunicationStyle::Formal,
                    "casual" | "随意" => CommunicationStyle::Casual,
                    "technical" | "技术" => CommunicationStyle::Technical,
                    "friendly" | "友好" => CommunicationStyle::Friendly,
                    "concise" | "简洁" => CommunicationStyle::Concise,
                    "educational" | "教学" => CommunicationStyle::Educational,
                    _ => return Err(format!("Invalid communication style: {}", value)),
                };
            }
            "domain" | "expertise" => {
                persona.expertise_domain = value.to_string();
            }
            "constraints" => {
                persona.behavior_constraints = value
                    .split([',', ';', '|'])
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
                    .collect();
            }
            "tone" => {
                persona.response_tone_instruction = value.to_string();
            }
            "language" => {
                persona.language_preference = value.to_string();
            }
            _ => return Err(format!("Unknown field: {}", field)),
        }

        self.save_persona(&persona)?;
        Ok(())
    }

    /// 删除指定 Agent 的画像文件和缓存
    pub fn delete_persona(&mut self, agent_id: &str) -> Result<(), String> {
        let path = self.persona_path(agent_id);
        if path.exists() {
            fs::remove_file(&path).map_err(|e| format!("Failed to remove persona file: {}", e))?;
        }
        self.cache.remove(agent_id);
        log::info!("[PersonaManager] Deleted persona for agent '{}'", agent_id);
        Ok(())
    }

    /// 列出所有已缓存的画像
    pub fn list_personas(&self) -> Vec<&AgentPersona> {
        self.cache.values().collect()
    }

    /// 构建带画像增强的完整 System Prompt
    /// 将 persona 片段注入基础 system prompt
    pub fn build_enhanced_system_prompt(&mut self, agent_id: &str, base_prompt: &str) -> String {
        let persona_fragment = match self.get_persona(agent_id) {
            Some(p) => p.to_system_prompt_fragment(),
            None => String::new(),
        };

        if persona_fragment.is_empty() {
            base_prompt.to_string()
        } else {
            format!("{}\n\n{}", base_prompt, persona_fragment)
        }
    }

    // ==================== 内部辅助方法 ====================

    /// 返回 persona.md 的路径: {base_dir}/{agent_id}/persona.md
    fn persona_path(&self, agent_id: &str) -> PathBuf {
        let safe_id = sanitize_id(agent_id);
        self.base_dir.join(&safe_id).join("persona.md")
    }
}

/// 辅助函数：从冒号分隔的行中提取值部分
fn extract_colon_value(line: &str) -> Option<String> {
    // 支持 "Key: Value" 和 "**Key**: Value" 格式
    if let Some(pos) = line.find(':') {
        let value = line[pos + 1..]
            .trim()
            .trim_start_matches("**")
            .trim_end_matches("**")
            .trim()
            .to_string();
        if value.is_empty() { None } else { Some(value) }
    } else if let Some(pos) = line.find('：') {
        let value = line[pos + 3..].trim().to_string(); // 中文冒号占3字节 UTF-8
        if value.is_empty() { None } else { Some(value) }
    } else {
        None
    }
}

/// 清理 ID 中的非法字符（用于文件路径）
pub fn sanitize_id(id: &str) -> String {
    id.replace(['/', '\\', ':', '*', '?', '"', '<', '>', '|'], "_")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_persona_generation() {
        let persona = AgentPersona::default_for_agent("test-agent", "Test Agent");
        assert_eq!(persona.agent_id, "test-agent");
        assert_eq!(persona.display_name, "Test Agent");
        assert_eq!(persona.communication_style, CommunicationStyle::Friendly);
        assert!(persona.behavior_constraints.is_empty());
    }

    #[test]
    fn test_persona_to_prompt_fragment() {
        let mut persona = AgentPersona::default_for_agent("code-agent", "Code Expert");
        persona.personality_traits = vec!["严谨".to_string(), "高效".to_string()];
        persona.expertise_domain = "Software Engineering".to_string();
        persona.behavior_constraints = vec!["不写死代码".to_string()];

        let fragment = persona.to_system_prompt_fragment();
        assert!(fragment.contains("Code Expert"));
        assert!(fragment.contains("严谨"));
        assert!(fragment.contains("Software Engineering"));
        assert!(fragment.contains("不写死代码"));
    }

    #[test]
    fn test_sanitize_id() {
        assert_eq!(sanitize_id("my-agent"), "my-agent");
        assert_eq!(sanitize_id("agent/name"), "agent_name");
        assert_eq!(sanitize_id("bad:name*test?"), "bad_name_test_");
    }
}
