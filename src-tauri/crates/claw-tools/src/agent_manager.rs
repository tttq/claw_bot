// Claw Desktop - Agent管理器 - 管理Agent的创建、配置、工作区（文件系统驱动）
// 支持：agents/ 目录扫描/加载、Markdown元数据、Workspace隔离、环境感知路径
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

/// Agent管理器 — 负责Agent的发现、加载、创建、删除和工作区文件管理
pub struct AgentManager {
    agents_dir: PathBuf,
    loaded_agents: HashMap<String, AgentDefinition>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentDefinition {
    pub id: String,
    pub name: String,
    pub description: String,
    #[serde(default)]
    pub purpose: String,
    #[serde(default)]
    pub scope: String,
    #[serde(default)]
    pub model: Option<String>,
    #[serde(default)]
    pub system_prompt: String,
    #[serde(default)]
    pub tools: Vec<String>,
    #[serde(default)]
    pub max_turns: Option<u32>,
    #[serde(default)]
    pub temperature: Option<f64>,
    #[serde(default)]
    pub created_at: i64,
    #[serde(default)]
    pub updated_at: i64,
    #[serde(default)]
    pub source: String,
    pub workspace_path: String,
    pub metadata_file: String,
    #[serde(default)]
    pub is_active: bool,
    #[serde(default)]
    pub conversation_count: u32,
}

impl AgentManager {
    /// 创建Agent管理器，初始化agents目录
    pub fn new() -> Self {
        let dir = dirs::home_dir()
            .map(|h| h.join(".claw-desktop").join("agents"))
            .unwrap_or_else(|| PathBuf::from(".claw-desktop/agents"));
        let mgr = Self {
            agents_dir: dir.clone(),
            loaded_agents: HashMap::new(),
        };
        mgr.ensure_dirs();
        mgr
    }

    /// 确保agents目录存在
    fn ensure_dirs(&self) {
        if !self.agents_dir.exists() {
            fs::create_dir_all(&self.agents_dir).ok();
            log::info!(
                "[AgentManager] Created agents directory at {:?}",
                self.agents_dir
            );
        }
    }

    /// 扫描并加载所有Agent定义 — 支持目录型(Markdown/JSON)和单文件型(.md/.json)
    pub fn scan_and_load(&mut self) -> Result<Vec<AgentDefinition>, String> {
        self.loaded_agents.clear();

        if !self.agents_dir.exists() {
            return Ok(Vec::new());
        }

        let mut found = Vec::new();

        for entry in fs::read_dir(&self.agents_dir).map_err(|e| e.to_string())? {
            let entry = entry.map_err(|e| e.to_string())?;
            let path = entry.path();

            if path.is_dir() {
                for candidate in &[
                    path.join("agent.md"),
                    path.join("agent.json"),
                    path.join("definition.md"),
                ] {
                    if candidate.exists() {
                        if let Some(agent) = self.load_agent_from_dir(&path, candidate)? {
                            found.push(agent.clone());
                            self.loaded_agents.insert(agent.id.clone(), agent);
                        }
                        break;
                    }
                }
            } else if path.is_file() {
                let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
                if ["md", "json"].contains(&ext) {
                    if let Some(agent) = self.load_single_file_agent(&path)? {
                        found.push(agent.clone());
                        self.loaded_agents.insert(agent.id.clone(), agent);
                    }
                }
            }
        }

        log::info!(
            "[AgentManager] Scanned and loaded {} agents from {:?}",
            found.len(),
            self.agents_dir
        );
        Ok(found)
    }

    /// 从目录加载Agent定义 — 解析元数据文件并设置工作区路径
    fn load_agent_from_dir(
        &self,
        dir: &Path,
        meta_path: &Path,
    ) -> Result<Option<AgentDefinition>, String> {
        let ext = meta_path.extension().and_then(|e| e.to_str()).unwrap_or("");
        let dir_name = dir
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown");
        let workspace = dir.join("workspace");

        fs::create_dir_all(&workspace).ok();

        let agent = match ext {
            "md" => self.parse_markdown_agent(meta_path)?,
            "json" => self.parse_json_agent(meta_path)?,
            _ => return Ok(None),
        };

        let metadata = fs::metadata(meta_path).ok();
        let modified = metadata
            .and_then(|m| m.modified().ok())
            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0);

        let mut final_agent = agent;
        final_agent.id = if final_agent.id.is_empty() {
            dir_name.to_string()
        } else {
            final_agent.id.clone()
        };
        final_agent.workspace_path = workspace.to_string_lossy().to_string();
        final_agent.metadata_file = meta_path.to_string_lossy().to_string();
        final_agent.source = format!("dir://{}", dir.display());
        final_agent.created_at = modified;
        final_agent.updated_at = modified;

        Ok(Some(final_agent))
    }

    /// 从单文件加载Agent定义 — 支持.md和.json格式
    fn load_single_file_agent(&self, path: &Path) -> Result<Option<AgentDefinition>, String> {
        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
        let filename = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown");

        let mut agent = match ext {
            "md" => self.parse_markdown_agent(path)?,
            "json" => self.parse_json_agent(path)?,
            _ => return Ok(None),
        };

        if agent.id.is_empty() {
            agent.id = filename.to_string();
        }
        if agent.name.is_empty() {
            agent.name = filename.to_string();
        }

        let parent = path.parent().unwrap_or_else(|| Path::new("."));
        let ws = parent.join(filename).join("workspace");
        fs::create_dir_all(&ws).ok();

        agent.workspace_path = ws.to_string_lossy().to_string();
        agent.metadata_file = path.to_string_lossy().to_string();
        agent.source = format!("file://{}", path.display());

        Ok(Some(agent))
    }

    /// 解析Markdown格式的Agent定义文件 — 提取标题、描述、用途、工具列表等元数据
    fn parse_markdown_agent(&self, path: &Path) -> Result<AgentDefinition, String> {
        let content = fs::read_to_string(path).map_err(|e| e.to_string())?;
        let filename = path.file_stem().and_then(|s| s.to_str()).unwrap_or("agent");

        let mut name = filename.to_string();
        let mut description = String::new();
        let mut purpose = String::new();
        let mut scope = String::new();
        let mut system_prompt = None;
        let mut model = None;
        let mut tools = Vec::new();
        let mut max_turns = None;

        let mut in_body = false;
        let mut body_lines = Vec::new();

        for line in content.lines() {
            if !in_body {
                if line.starts_with("# ") && name == filename {
                    name = line[2..].trim().to_string();
                } else if line.starts_with("> ") || line.starts_with("**Description:**") {
                    let desc_part = if line.starts_with("> ") {
                        &line[2..]
                    } else {
                        line.split(':').last().unwrap_or("").trim()
                    };
                    if !description.is_empty() {
                        description.push(' ');
                    }
                    description.push_str(desc_part.trim());
                } else if line.contains("Purpose:") || line.contains("purpose:") {
                    purpose = line.split(':').last().unwrap_or("").trim().to_string();
                } else if line.contains("Scope:") || line.contains("scope:") {
                    scope = line.split(':').last().unwrap_or("").trim().to_string();
                } else if line.contains("Model:") || line.contains("model:") {
                    model = Some(line.split(':').last().unwrap_or("").trim().to_string());
                } else if line.contains("Tools:") || line.contains("tools:") {
                    let tools_str = line.split(':').last().unwrap_or("");
                    tools = tools_str
                        .split(',')
                        .map(|t| t.trim().trim_matches('"').trim_matches('\'').to_string())
                        .filter(|t| !t.is_empty())
                        .collect();
                } else if line.contains("MaxTurns:") || line.contains("max_turns:") {
                    max_turns = line
                        .split(':')
                        .last()
                        .unwrap_or("")
                        .trim()
                        .parse::<u32>()
                        .ok();
                } else if line == "---" {
                    in_body = true;
                    continue;
                }
            } else {
                body_lines.push(line);
            }
        }

        if system_prompt.is_none() && !body_lines.is_empty() {
            system_prompt = Some(body_lines.join("\n"));
        }

        if description.is_empty() {
            description = claw_types::truncate_str_safe(&content, 300)
                .replace('\n', " ")
                .to_string();
        }

        Ok(AgentDefinition {
            id: String::new(),
            name,
            description: description.trim().to_string(),
            purpose,
            scope,
            model,
            system_prompt: system_prompt.unwrap_or_default(),
            tools,
            max_turns,
            temperature: None,
            created_at: 0,
            updated_at: 0,
            source: String::new(),
            workspace_path: String::new(),
            metadata_file: String::new(),
            is_active: false,
            conversation_count: 0,
        })
    }

    /// 解析JSON格式的Agent定义文件 — 反序列化为AgentDefinition
    fn parse_json_agent(&self, path: &Path) -> Result<AgentDefinition, String> {
        let content = fs::read_to_string(path).map_err(|e| e.to_string())?;
        let mut agent: AgentDefinition = serde_json::from_str(&content)
            .map_err(|e| format!("Invalid JSON in {:?}: {}", path, e))?;

        if agent.system_prompt.is_empty() {
            agent.system_prompt = format!(
                "You are {}, an AI assistant. {}",
                agent.name, agent.description
            );
        }

        Ok(agent)
    }

    /// 创建新Agent — 生成ID、创建目录和agent.md文件
    pub fn create_agent(&mut self, agent: &AgentDefinition) -> Result<String, String> {
        let safe_id = agent
            .id
            .replace(['/', '\\', ':', '*', '?', '"', '<', '>', '|'], "_");
        let agent_dir = self.agents_dir.join(&safe_id);

        fs::create_dir_all(&agent_dir)
            .map_err(|e| format!("Failed to create agent dir {:?}: {}", agent_dir, e))?;

        let workspace = agent_dir.join("workspace");
        fs::create_dir_all(&workspace)
            .map_err(|e| format!("Failed to create workspace {:?}: {}", workspace, e))?;

        let md_content = format!(
            "# {}\n\n> **Description**: {}\n\n\
             > **Purpose**: {}\n\n\
             > **Scope**: {}\n\n\
             ---\n\n\
             {}",
            agent.name,
            agent.description,
            if agent.purpose.is_empty() {
                "TBD".into()
            } else {
                agent.purpose.clone()
            },
            if agent.scope.is_empty() {
                "General".into()
            } else {
                agent.scope.clone()
            },
            agent.system_prompt
        );

        let md_path = agent_dir.join("agent.md");
        fs::write(&md_path, &md_content)
            .map_err(|e| format!("Failed to write agent.md {:?}: {}", md_path, e))?;

        let mut final_agent = agent.clone();
        final_agent.id = safe_id.clone();
        final_agent.workspace_path = workspace.to_string_lossy().to_string();
        final_agent.metadata_file = md_path.to_string_lossy().to_string();
        final_agent.source = format!("dir://{}", agent_dir.display());

        self.loaded_agents
            .insert(safe_id.clone(), final_agent.clone());

        log::info!(
            "[AgentManager] Created agent '{}' at {:?}",
            agent.name,
            agent_dir
        );
        Ok(agent_dir.to_string_lossy().to_string())
    }

    /// 删除Agent — 删除目录及所有文件
    pub fn remove_agent(&mut self, id: &str) -> Result<(), String> {
        let safe_id = id.replace(['/', '\\', ':', '*', '?', '"', '<', '>', '|'], "_");
        let agent_path = self.agents_dir.join(&safe_id);

        if agent_path.exists() && agent_path.is_dir() {
            fs::remove_dir_all(&agent_path)
                .map_err(|e| format!("Failed to remove agent dir {:?}: {}", agent_path, e))?;
        }

        self.loaded_agents.remove(id);
        log::info!("[AgentManager] Removed agent '{}'", id);
        Ok(())
    }

    /// 列出所有已加载的Agent
    pub fn list_loaded(&self) -> Vec<AgentDefinition> {
        self.loaded_agents.values().cloned().collect()
    }

    /// 获取指定ID的Agent
    pub fn get_agent(&self, id: &str) -> Option<&AgentDefinition> {
        self.loaded_agents.get(id)
    }

    /// 列出Agent工作区文件
    pub fn list_workspace_files(&self, id: &str) -> Result<Vec<WorkspaceFileEntry>, String> {
        let agent = self
            .loaded_agents
            .get(id)
            .ok_or(format!("Agent '{}' not found", id))?;
        let ws_path = Path::new(&agent.workspace_path);

        if !ws_path.exists() {
            return Ok(Vec::new());
        }

        let mut entries = Vec::new();
        self.scan_workspace_dir(ws_path, "", &mut entries)?;
        Ok(entries)
    }

    /// 递归扫描工作区目录 — 收集文件和子目录条目
    fn scan_workspace_dir(
        &self,
        dir: &Path,
        prefix: &str,
        entries: &mut Vec<WorkspaceFileEntry>,
    ) -> Result<(), String> {
        for entry in fs::read_dir(dir).map_err(|e| e.to_string())? {
            let entry = entry.map_err(|e| e.to_string())?;
            let path = entry.path();
            let name = entry.file_name().to_string_lossy().to_string();
            let rel_path = if prefix.is_empty() {
                name.clone()
            } else {
                format!("{}/{}", prefix, name)
            };

            if path.is_dir() {
                entries.push(WorkspaceFileEntry {
                    name: name.clone(),
                    relative_path: rel_path.clone(),
                    full_path: path.to_string_lossy().to_string(),
                    is_dir: true,
                    size: 0,
                    modified: 0,
                });
                self.scan_workspace_dir(&path, &rel_path, entries)?;
            } else if let Ok(metadata) = fs::metadata(&path) {
                entries.push(WorkspaceFileEntry {
                    name: name.clone(),
                    relative_path: rel_path,
                    full_path: path.to_string_lossy().to_string(),
                    is_dir: false,
                    size: metadata.len(),
                    modified: metadata
                        .modified()
                        .ok()
                        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                        .map(|d| d.as_secs() as i64)
                        .unwrap_or(0),
                });
            }
        }
        Ok(())
    }

    /// 写入Agent工作区文件
    pub fn write_workspace_file(
        &self,
        id: &str,
        file_rel_path: &str,
        content: &str,
    ) -> Result<String, String> {
        let agent = self
            .loaded_agents
            .get(id)
            .ok_or(format!("Agent '{}' not found", id))?;
        let ws_path = Path::new(&agent.workspace_path);

        let full_path = ws_path.join(file_rel_path.replace('\\', "/"));
        if let Some(parent) = full_path.parent() {
            fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }

        fs::write(&full_path, content).map_err(|e| e.to_string())?;
        log::info!(
            "[AgentManager] Agent {} wrote {} ({} bytes)",
            id,
            file_rel_path,
            content.len()
        );
        Ok(full_path.to_string_lossy().to_string())
    }

    /// 读取Agent工作区文件
    pub fn read_workspace_file(&self, id: &str, file_rel_path: &str) -> Result<String, String> {
        let agent = self
            .loaded_agents
            .get(id)
            .ok_or(format!("Agent '{}' not found", id))?;
        let full_path = Path::new(&agent.workspace_path).join(file_rel_path.replace('\\', "/"));
        fs::read_to_string(&full_path).map_err(|e| e.to_string())
    }

    /// 删除Agent工作区文件或目录
    pub fn delete_workspace_file(&self, id: &str, file_rel_path: &str) -> Result<(), String> {
        let agent = self
            .loaded_agents
            .get(id)
            .ok_or(format!("Agent '{}' not found", id))?;
        let full_path = Path::new(&agent.workspace_path).join(file_rel_path.replace('\\', "/"));
        if full_path.exists() {
            if full_path.is_dir() {
                fs::remove_dir_all(&full_path).map_err(|e| e.to_string())?;
            } else {
                fs::remove_file(&full_path).map_err(|e| e.to_string())?;
            }
        }
        Ok(())
    }

    /// 热重载Agent列表 — 返回新增和移除的Agent信息
    pub fn hot_reload(&mut self) -> Result<AgentReloadResult, String> {
        let before: Vec<String> = self.loaded_agents.keys().cloned().collect();
        let new_agents = self.scan_and_load()?;
        let after: Vec<String> = self.loaded_agents.keys().cloned().collect();

        Ok(AgentReloadResult {
            total: new_agents.len(),
            added: after
                .iter()
                .filter(|n| !before.contains(n))
                .cloned()
                .collect(),
            removed: before
                .iter()
                .filter(|n| !after.contains(n))
                .cloned()
                .collect(),
            agents: new_agents,
        })
    }

    /// 获取agents目录路径
    pub fn agents_dir_path(&self) -> String {
        self.agents_dir.to_string_lossy().to_string()
    }
}

/// 工作区文件条目 — 表示Agent工作区中的一个文件或目录
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceFileEntry {
    pub name: String,
    pub relative_path: String,
    pub full_path: String,
    pub is_dir: bool,
    pub size: u64,
    pub modified: i64,
}

/// Agent重载结果 — 记录热重载后新增和移除的Agent
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentReloadResult {
    pub total: usize,
    pub added: Vec<String>,
    pub removed: Vec<String>,
    pub agents: Vec<AgentDefinition>,
}
