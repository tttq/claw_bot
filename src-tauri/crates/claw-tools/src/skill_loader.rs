use serde::{Deserialize, Serialize};
use serde_json::{self, Value as JsonValue, json};
/// Claw Desktop - 技能加载器 - 从磁盘加载SKILL.md格式的技能文件
// 对标 def_claw src/skills/loadSkillsDir.ts
// 功能：从磁盘目录加载 SKILL.md 文件 → 解析 YAML frontmatter → 注册到工具注册表
use std::fs;
use std::path::{Path, PathBuf};

/// 已加载的技能完整定义（从 SKILL.md 解析得到）
/// 包含元数据、正文内容、来源类型和文件路径
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoadedSkill {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub display_name: Option<String>,
    pub description: String,
    #[serde(default)]
    pub when_to_use: Option<String>,
    #[serde(default)]
    pub allowed_tools: Vec<String>,
    #[serde(default)]
    pub argument_hint: Option<String>,
    #[serde(rename = "user-invocable", default = "true_fn")]
    pub user_invocable: bool,
    #[serde(default)]
    pub version: Option<String>,
    #[serde(default)]
    pub model: Option<String>,
    #[serde(default)]
    pub effort: Option<String>,
    #[serde(default)]
    pub paths: Option<Vec<String>>,
    pub content: String,
    pub file_path: String,
    pub source: SkillSource,
}

const fn true_fn() -> bool {
    true
}

/// 技能来源类型（决定注册到工具表时的 ToolSource）
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum SkillSource {
    Bundled,
    User,
    Project,
    Extension,
    Mcp,
}

impl Default for SkillSource {
    fn default() -> Self {
        Self::Bundled
    }
}

/// 解析SKILL.md文件 — 提取YAML frontmatter和Markdown正文
fn parse_skill_file(path: &Path) -> Option<(serde_yaml::Value, String)> {
    let content = fs::read_to_string(path).ok()?;
    let frontmatter_start = content.find("---")?;
    let after_first = &content[frontmatter_start + 3..];
    let frontmatter_end = after_first.find("---")?;
    let yaml_str = &after_first[..frontmatter_end];
    let body = after_first[frontmatter_end + 3..].trim().to_string();
    let fm: serde_yaml::Value = serde_yaml::from_str(yaml_str.trim()).ok()?;
    Some((fm, body))
}

/// 从单个 SKILL.md 文件解析技能定义
/// 解析 YAML frontmatter（元数据）+ Markdown body（正文模板）
/// 跳过 AutomationSkill 格式的文件（含有 app_name 字段）
pub fn load_skill_from_file(path: &Path, source: SkillSource) -> Option<LoadedSkill> {
    if !path.exists() {
        return None;
    }
    let (fm, body) = parse_skill_file(path)?;

    if fm.get("app_name").is_some() {
        log::debug!("[SkillLoader] Skipping AutomationSkill file: {:?}", path);
        return None;
    }
    let dir_name = path.parent()?.file_name()?.to_string_lossy().to_string();

    let get_str = |key: &str| -> Option<String> {
        fm.get(key).and_then(|v: &serde_yaml::Value| match v {
            serde_yaml::Value::String(s) => Some(s.clone()),
            _ => None,
        })
    };

    let get_bool = |key: &str| -> bool {
        fm.get(key)
            .and_then(|v: &serde_yaml::Value| match v {
                serde_yaml::Value::Bool(b) => Some(*b),
                _ => None,
            })
            .unwrap_or(true)
    };

    let get_arr = |key: &str| -> Vec<String> {
        fm.get(key)
            .and_then(|v: &serde_yaml::Value| match v {
                serde_yaml::Value::Sequence(arr) => Some(
                    arr.iter()
                        .filter_map(|i| match i {
                            serde_yaml::Value::String(s) => Some(s.clone()),
                            _ => None,
                        })
                        .collect(),
                ),
                _ => None,
            })
            .unwrap_or_default()
    };

    let name = get_str("name").unwrap_or(dir_name);
    let description = get_str("description")
        .or_else(|| extract_description_from_body(&body))
        .unwrap_or_else(|| format!("Skill: {}", name));
    let when_to_use = get_str("when_to_use").or_else(|| get_str("whenToUse"));
    let allowed_tools = get_str("allowed-tools")
        .map(|_| get_arr("allowed-tools"))
        .unwrap_or_else(|| get_arr("allowedTools"));
    let argument_hint = get_str("argument-hint").or_else(|| get_str("argumentHint"));
    let version = get_str("version");
    let model = get_str("model");
    let effort = get_str("effort");
    let paths_val = fm.get("paths");
    let paths: Option<Vec<String>> = paths_val.and_then(|v: &serde_yaml::Value| match v {
        serde_yaml::Value::Sequence(arr) => Some(
            arr.iter()
                .filter_map(|i| match i {
                    serde_yaml::Value::String(s) => Some(s.clone()),
                    _ => None,
                })
                .collect(),
        ),
        _ => None,
    });

    Some(LoadedSkill {
        name,
        display_name: None,
        description,
        when_to_use,
        allowed_tools,
        argument_hint,
        user_invocable: get_bool("user-invocable"),
        version,
        model,
        effort,
        paths,
        content: body,
        file_path: path.to_string_lossy().to_string(),
        source,
    })
}

/// 从Markdown正文中提取描述 — 取第一个非标题非代码块的文本行
fn extract_description_from_body(body: &str) -> Option<String> {
    for line in body.lines() {
        let trimmed = line.trim();
        if !trimmed.is_empty() && !trimmed.starts_with('#') && !trimmed.starts_with("```") {
            return Some(if trimmed.len() > 120 {
                let safe_end = trimmed
                    .char_indices()
                    .take(120)
                    .last()
                    .map(|(i, _)| i)
                    .unwrap_or(0);
                format!("{}...", &trimmed[..safe_end])
            } else {
                trimmed.to_string()
            });
        }
    }
    None
}

/// 扫描指定目录下所有含 SKILL.md 的子目录，批量加载技能
pub fn load_skills_from_dir(dir: &Path, source: &SkillSource) -> Vec<LoadedSkill> {
    let mut skills = Vec::new();
    if !dir.exists() || !dir.is_dir() {
        return skills;
    }
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let ft = match entry.file_type() {
                Ok(ft) => ft,
                Err(_) => continue,
            };
            if !ft.is_dir() && !ft.is_symlink() {
                continue;
            }
            let skill_md = entry.path().join("SKILL.md");
            if !skill_md.exists() {
                continue;
            }
            if let Some(skill) = load_skill_from_file(&skill_md, source.clone()) {
                log::info!(
                    "[SkillLoader] 加载技能: {} (来源: {:?})",
                    skill.name,
                    source
                );
                skills.push(skill);
            }
        }
    }
    skills
}

/// 从多个目录加载全部技能（自动去重，同名技能只保留第一个）
pub fn load_all_skills(directories: &[(PathBuf, SkillSource)]) -> Vec<LoadedSkill> {
    let mut all_skills: Vec<LoadedSkill> = Vec::new();
    let mut seen_names: std::collections::HashSet<String> = std::collections::HashSet::new();
    for (dir, source) in directories {
        let skills = load_skills_from_dir(dir, source);
        for skill in skills {
            if seen_names.insert(skill.name.clone()) {
                all_skills.push(skill);
            }
        }
    }
    all_skills
}

/// 将已加载的技能注册为工具（ToolDefinition），返回成功注册数量
pub async fn register_skills_as_tools(skills: &[LoadedSkill]) -> usize {
    use crate::tool_registry::{ToolSource as TSrc, register_tool};
    use claw_types::common::ToolDefinition;

    let mut count = 0;
    for skill in skills {
        let tsrc = match skill.source {
            SkillSource::Bundled => TSrc::BuiltIn,
            SkillSource::User | SkillSource::Project => TSrc::Skill,
            SkillSource::Extension => TSrc::Extension,
            SkillSource::Mcp => TSrc::Mcp,
        };

        let def = ToolDefinition {
            name: format!("Skill:{}", skill.name),
            description: if let Some(ref wtu) = skill.when_to_use {
                format!("{} [使用场景: {}]", skill.description, wtu)
            } else {
                skill.description.clone()
            },
            input_schema: json!({
                "type": "object",
                "properties": {
                    "skill": {"type": "string", "const": skill.name},
                    "args": {"type": "string", "description": skill.argument_hint.as_deref().unwrap_or("Skill arguments")},
                },
                "required": ["skill"]
            }),
            category: None,
            tags: Vec::new(),
        };

        if register_tool(def, tsrc, Some(format!("skill:{}", skill.name))).await {
            count += 1;
        }
    }
    log::info!("[SkillLoader] 已注册 {} 个技能工具", count);
    count
}

/// 返回默认技能搜索路径列表：用户目录 → 项目目录 → 内置目录
pub fn default_skill_directories() -> Vec<(PathBuf, SkillSource)> {
    let mut dirs = Vec::new();

    if let Ok(root) = std::env::current_dir() {
        let dev_skills = root.join(".build_temp").join("skills");
        if dev_skills.exists() || root.join("src-tauri").exists() {
            log::info!(
                "[SkillLoader] Dev mode: adding .build_temp/skills @ {}",
                dev_skills.display()
            );
            dirs.push((dev_skills, SkillSource::Bundled));
        }
    }

    let app_skills = claw_config::path_resolver::skills_dir();
    if !app_skills.as_os_str().is_empty() {
        dirs.push((app_skills.clone(), SkillSource::Bundled));
    }

    if let Some(home) = dirs::home_dir() {
        dirs.push((home.join(".claw-desktop").join("skills"), SkillSource::User));
    }

    if let Ok(exe) = std::env::current_exe() {
        dirs.push((
            exe.parent().unwrap_or(Path::new(".")).join("skills"),
            SkillSource::Bundled,
        ));
    }

    dirs
}

/// 便捷入口：发现并加载所有默认目录下的技能，同时注册到工具表
pub async fn discover_and_load_all_skills() -> Vec<LoadedSkill> {
    let mut all_skills = Vec::new();
    let mut seen_names: std::collections::HashSet<String> = std::collections::HashSet::new();

    let bundled = load_bundled_skills();
    for skill in bundled {
        if seen_names.insert(skill.name.clone()) {
            all_skills.push(skill);
        }
    }

    let dirs = default_skill_directories();
    let user_skills = load_all_skills(&dirs);
    for skill in user_skills {
        if seen_names.insert(skill.name.clone()) {
            all_skills.push(skill);
        }
    }

    register_skills_as_tools(&all_skills).await;
    log::info!(
        "[SkillLoader] 总计加载 {} 个技能（内置+用户）",
        all_skills.len()
    );
    all_skills
}

/// Tauri命令：从指定目录加载技能并注册
#[tauri::command]
pub async fn cmd_load_skills_from_dir(
    dir: String,
    source: Option<String>,
) -> Result<JsonValue, String> {
    let src = match source.as_deref() {
        Some("project") => SkillSource::Project,
        Some("extension") => SkillSource::Extension,
        Some("mcp") => SkillSource::Mcp,
        _ => SkillSource::User,
    };
    let path = PathBuf::from(&dir);
    let skills = load_skills_from_dir(&path, &src);
    let registered = register_skills_as_tools(&skills).await;
    Ok(
        json!({"success":true,"directory":dir,"source":format!("{:?}",src),"found":skills.len(),"registered":registered,"skills":skills.iter().map(|s|json!({"name":s.name,"description":s.description,"version":s.version})).collect::<Vec<_>>()}),
    )
}

/// Tauri命令：列出所有已加载的技能（包括已注册和内置的）
#[tauri::command]
pub async fn cmd_list_loaded_skills() -> Result<JsonValue, String> {
    use crate::tool_registry::list_dynamic_tools;
    let mut all_skill_entries: Vec<serde_json::Value> = Vec::new();
    let mut seen_names: std::collections::HashSet<String> = std::collections::HashSet::new();

    let dynamic = list_dynamic_tools().await;
    for t in dynamic.into_iter().filter(|t| t.name.starts_with("Skill:")) {
        let name = t.name.strip_prefix("Skill:").unwrap_or(&t.name).to_string();
        if seen_names.insert(name.clone()) {
            all_skill_entries.push(json!({
                "name": name,
                "description": t.description,
                "source": "registered",
                "context": "Inline"
            }));
        }
    }

    let bundled = crate::bundled_skills::get_all_bundled_skills();
    for bs in bundled {
        if seen_names.insert(bs.name.clone()) {
            all_skill_entries.push(json!({
                "name": bs.name,
                "description": bs.description,
                "source": "Bundled",
                "context": "Inline",
                "path": bs.path
            }));
        }
    }

    Ok(json!({"total":all_skill_entries.len(),"skills":all_skill_entries}))
}

/// 从编译时嵌入的 bundled-skills 目录加载技能
pub fn load_bundled_skills() -> Vec<LoadedSkill> {
    use crate::bundled_skills;

    let bundled = bundled_skills::get_all_bundled_skills();
    let mut skills = Vec::new();

    for info in bundled {
        let (fm, body) = if let Some(pos) = info.content.find("---") {
            let after_first = &info.content[pos + 3..];
            if let Some(end) = after_first.find("---") {
                let yaml_str = &after_first[..end];
                let body_text = after_first[end + 3..].trim().to_string();
                if let Ok(fm) = serde_yaml::from_str::<serde_yaml::Value>(yaml_str) {
                    (Some(fm), body_text)
                } else {
                    (None, body_text)
                }
            } else {
                (None, info.content.clone())
            }
        } else {
            (None, info.content.clone())
        };

        let get_fm_str = |key: &str| -> Option<String> {
            fm.as_ref().and_then(|v| {
                v.get(key).and_then(|v| match v {
                    serde_yaml::Value::String(s) => Some(s.clone()),
                    _ => None,
                })
            })
        };

        let get_fm_bool = |key: &str| -> bool {
            fm.as_ref()
                .and_then(|v| {
                    v.get(key).and_then(|v| match v {
                        serde_yaml::Value::Bool(b) => Some(*b),
                        _ => None,
                    })
                })
                .unwrap_or(true)
        };

        let get_fm_arr = |key: &str| -> Vec<String> {
            fm.as_ref()
                .and_then(|v| {
                    v.get(key).and_then(|v| match v {
                        serde_yaml::Value::Sequence(arr) => Some(
                            arr.iter()
                                .filter_map(|i| match i {
                                    serde_yaml::Value::String(s) => Some(s.clone()),
                                    _ => None,
                                })
                                .collect(),
                        ),
                        _ => None,
                    })
                })
                .unwrap_or_default()
        };

        let name = get_fm_str("name").unwrap_or(info.name.clone());
        let description = get_fm_str("description")
            .or_else(|| extract_description_from_body(&body))
            .unwrap_or_else(|| format!("Skill: {}", name));

        if fm.as_ref().and_then(|v| v.get("app_name")).is_some() {
            log::debug!("[SkillLoader] Skipping bundled AutomationSkill: {}", name);
            continue;
        }

        skills.push(LoadedSkill {
            name,
            display_name: None,
            description,
            when_to_use: get_fm_str("when_to_use").or_else(|| get_fm_str("whenToUse")),
            allowed_tools: get_fm_str("allowed-tools")
                .map(|_| get_fm_arr("allowed-tools"))
                .unwrap_or_else(|| get_fm_arr("allowedTools")),
            argument_hint: get_fm_str("argument-hint").or_else(|| get_fm_str("argumentHint")),
            user_invocable: get_fm_bool("user-invocable"),
            version: get_fm_str("version"),
            model: get_fm_str("model"),
            effort: get_fm_str("effort"),
            paths: None,
            content: body,
            file_path: format!("bundled://{}", info.name),
            source: SkillSource::Bundled,
        });
    }

    log::info!("[SkillLoader] 从内置资源加载了 {} 个技能", skills.len());
    skills
}

static LOADED_SKILLS: std::sync::OnceLock<Vec<LoadedSkill>> = std::sync::OnceLock::new();

/// 获取或初始化技能缓存 — 使用OnceLock保证只加载一次
fn get_or_load_skills() -> &'static Vec<LoadedSkill> {
    LOADED_SKILLS.get_or_init(|| {
        let mut all = Vec::new();
        let mut seen = std::collections::HashSet::new();
        for s in load_bundled_skills() {
            if seen.insert(s.name.clone()) {
                all.push(s);
            }
        }
        let dirs = default_skill_directories();
        for s in load_all_skills(&dirs) {
            if seen.insert(s.name.clone()) {
                all.push(s);
            }
        }
        all
    })
}

/// 加载指定技能的完整内容 — 包含描述、使用场景、允许工具和正文
pub fn load_skill_content(skill_name: &str) -> Option<String> {
    let skills = get_or_load_skills();
    let skill = skills.iter().find(|s| s.name == skill_name)?;
    let mut parts = Vec::new();
    if !skill.description.is_empty() {
        parts.push(format!("Description: {}", skill.description));
    }
    if let Some(ref wtu) = skill.when_to_use {
        parts.push(format!("When to use: {}", wtu));
    }
    if !skill.allowed_tools.is_empty() {
        parts.push(format!("Allowed tools: {}", skill.allowed_tools.join(", ")));
    }
    if !skill.content.is_empty() {
        parts.push(skill.content.clone());
    }
    Some(parts.join("\n\n"))
}

/// 列出所有可用技能的名称
pub fn list_available_skills() -> Vec<String> {
    let skills = get_or_load_skills();
    skills.iter().map(|s| s.name.clone()).collect()
}
