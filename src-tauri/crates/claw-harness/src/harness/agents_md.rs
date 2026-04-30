// Claw Desktop - Agent MD - 从AGENTS.md加载Agent定义
use crate::harness::persona::sanitize_id;
use crate::harness::types::AgentPersona;
use log;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

/// Agent MD管理器 — 管理Agent的agent.md定义文件
///
/// 支持生成、保存、加载和自动刷新agent.md文件
/// 使用内存缓存避免频繁磁盘IO
pub struct AgentsMdManager {
    base_dir: PathBuf,
    cache: HashMap<String, String>,
}

impl AgentsMdManager {
    /// 创建Agent MD管理器，自动创建基础目录
    pub fn new(base_dir: &Path) -> Self {
        let mgr = Self {
            base_dir: base_dir.to_path_buf(),
            cache: HashMap::new(),
        };
        if !mgr.base_dir.exists() {
            if let Err(e) = fs::create_dir_all(&mgr.base_dir) {
                log::warn!(
                    "[AgentsMdManager:new] Failed to create base dir {:?}: {}",
                    mgr.base_dir,
                    e
                );
            }
        }
        mgr
    }

    /// 生成agent.md内容 — 将AgentPersona转换为Markdown格式
    pub fn generate_agent_md(&self, persona: &AgentPersona) -> String {
        let mut md = String::new();

        md.push_str(&format!("# {}\n\n", persona.display_name));

        md.push_str(&format!("> Agent ID: `{}`\n\n", persona.agent_id));

        if !persona.personality_traits.is_empty() {
            md.push_str("## Personality\n\n");
            for trait_item in &persona.personality_traits {
                md.push_str(&format!("- {}\n", trait_item));
            }
            md.push('\n');
        }

        md.push_str(&format!(
            "## Communication Style\n\n{}\n\n",
            persona.communication_style
        ));

        if !persona.expertise_domain.is_empty() {
            md.push_str(&format!("## Expertise\n\n{}\n\n", persona.expertise_domain));
        }

        if !persona.behavior_constraints.is_empty() {
            md.push_str("## Constraints\n\n");
            for constraint in &persona.behavior_constraints {
                md.push_str(&format!("- {}\n", constraint));
            }
            md.push('\n');
        }

        if !persona.response_tone_instruction.is_empty() {
            md.push_str(&format!(
                "## Tone Instruction\n\n{}\n\n",
                persona.response_tone_instruction
            ));
        }

        md.push_str(&format!("## Language\n\n{}\n", persona.language_preference));

        md
    }

    /// 保存agent.md — 将Persona写入磁盘文件并更新缓存
    pub fn save_agent_md(&mut self, persona: &AgentPersona) -> Result<(), String> {
        let path = self.agent_md_path(&persona.agent_id);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|e| format!("Failed to create dir: {}", e))?;
        }

        let content = self.generate_agent_md(persona);
        fs::write(&path, &content).map_err(|e| format!("Failed to write agent.md: {}", e))?;

        self.cache.insert(persona.agent_id.clone(), content);

        log::info!(
            "[AgentsMdManager:save_agent_md] Saved agent.md for agent={}",
            persona.agent_id
        );
        Ok(())
    }

    /// 加载agent.md — 优先从缓存读取，缓存未命中则从磁盘加载
    pub fn load_agent_md(&mut self, agent_id: &str) -> Option<String> {
        if let Some(cached) = self.cache.get(agent_id) {
            return Some(cached.clone());
        }

        let path = self.agent_md_path(agent_id);
        match fs::read_to_string(&path) {
            Ok(content) => {
                self.cache.insert(agent_id.to_string(), content.clone());
                Some(content)
            }
            Err(_) => None,
        }
    }

    /// 自动刷新 — 扫描基础目录下所有agent.md文件并更新缓存
    pub fn auto_refresh(&mut self) -> Result<usize, String> {
        let mut refreshed = 0;

        if !self.base_dir.exists() {
            return Ok(0);
        }

        let entries =
            fs::read_dir(&self.base_dir).map_err(|e| format!("Failed to read base dir: {}", e))?;

        for entry in entries.flatten() {
            if entry.file_type().map(|t| t.is_dir()).unwrap_or(false) {
                let agent_md_path = entry.path().join("agent.md");
                if agent_md_path.exists() {
                    let agent_id = entry.file_name().to_string_lossy().to_string();
                    if let Ok(content) = fs::read_to_string(&agent_md_path) {
                        self.cache.insert(agent_id.clone(), content);
                        refreshed += 1;
                    }
                }
            }
        }

        log::info!(
            "[AgentsMdManager:auto_refresh] Refreshed {} agent.md files",
            refreshed
        );
        Ok(refreshed)
    }

    /// 计算agent.md文件路径 — {base_dir}/{safe_agent_id}/agent.md
    fn agent_md_path(&self, agent_id: &str) -> PathBuf {
        let safe_id = sanitize_id(agent_id);
        self.base_dir.join(&safe_id).join("agent.md")
    }
}
