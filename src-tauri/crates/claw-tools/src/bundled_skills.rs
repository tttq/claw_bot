// Claw Desktop - 内置技能 - 编译时嵌入的技能定义
use include_dir::{Dir, include_dir};
use serde::{Deserialize, Serialize};
use std::path::Path;

pub static BUNDLED_SKILLS_DIR: Dir<'static> = include_dir!("bundled-skills");

/// 内置技能信息 — 包含名称、描述、内容和路径
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BundledSkillInfo {
    pub name: String,
    pub description: String,
    pub content: String,
    pub path: String,
}

/// 获取内置技能目录的静态引用
#[allow(dead_code)]
pub fn get_bundled_skills_dir() -> &'static Dir<'static> {
    &BUNDLED_SKILLS_DIR
}

/// 列出所有内置技能的名称
#[allow(dead_code)]
pub fn list_bundled_skill_names() -> Vec<String> {
    let mut names = Vec::new();
    for entry in BUNDLED_SKILLS_DIR.dirs() {
        names.push(
            entry
                .path()
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string(),
        );
    }
    names.sort();
    names
}

/// 获取指定内置技能的内容 — 读取SKILL.md文件
pub fn get_bundled_skill_content(skill_name: &str) -> Option<String> {
    let skill_path = Path::new(skill_name).join("SKILL.md");
    if let Some(file) = BUNDLED_SKILLS_DIR.get_file(&skill_path) {
        Some(file.contents_utf8()?.to_string())
    } else {
        None
    }
}

/// 获取所有内置技能列表 — 解析每个技能的SKILL.md并提取描述
pub fn get_all_bundled_skills() -> Vec<BundledSkillInfo> {
    let mut skills = Vec::new();

    for dir in BUNDLED_SKILLS_DIR.dirs() {
        let skill_name = dir
            .path()
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();

        let skill_md_path = Path::new(&skill_name).join("SKILL.md");

        if let Some(file) = BUNDLED_SKILLS_DIR.get_file(&skill_md_path) {
            let content = match file.contents_utf8() {
                Some(c) => c.to_string(),
                None => continue,
            };

            let description = extract_description_from_content(&content);

            skills.push(BundledSkillInfo {
                name: skill_name.clone(),
                description,
                content,
                path: format!("bundled://{}", skill_name),
            });
        }
    }

    skills.sort_by(|a, b| a.name.cmp(&b.name));
    skills
}

/// 从SKILL.md内容中提取description字段 — 解析YAML frontmatter
fn extract_description_from_content(content: &str) -> String {
    let frontmatter_start = content.find("---");
    if frontmatter_start.is_none() {
        return String::new();
    }

    let after_first = &content[frontmatter_start.unwrap() + 3..];
    let frontmatter_end = after_first.find("---");
    if frontmatter_end.is_none() {
        return String::new();
    }

    let yaml_str = &after_first[..frontmatter_end.unwrap()];

    for line in yaml_str.lines() {
        let line = line.trim();
        if line.starts_with("description:") || line.starts_with("description :") {
            let desc = line
                .trim_start_matches("description:")
                .trim_start_matches("description :")
                .trim();
            let desc = desc
                .trim_start_matches('"')
                .trim_end_matches('"')
                .trim_start_matches('\'')
                .trim_end_matches('\'');
            return desc.to_string();
        }
    }

    String::new()
}

/// Tauri命令：列出所有内置技能
#[tauri::command]
pub async fn cmd_list_bundled_skills() -> Result<serde_json::Value, String> {
    let skills = get_all_bundled_skills();
    Ok(serde_json::json!({
        "total": skills.len(),
        "skills": skills.iter().map(|s| serde_json::json!({
            "name": s.name,
            "description": s.description
        })).collect::<Vec<_>>()
    }))
}

/// Tauri命令：获取指定内置技能的内容
#[tauri::command]
pub async fn cmd_get_bundled_skill_content(name: String) -> Result<String, String> {
    get_bundled_skill_content(&name).ok_or_else(|| format!("Bundled skill '{}' not found", name))
}

/// 导出内置技能到指定目录 — 跳过已存在的技能目录
pub fn export_bundled_skills(target_dir: &Path) -> Result<usize, String> {
    let skills = get_all_bundled_skills();
    let mut count = 0;

    std::fs::create_dir_all(target_dir)
        .map_err(|e| format!("Failed to create skills dir: {}", e))?;

    for skill in &skills {
        let skill_dir = target_dir.join(&skill.name);
        if skill_dir.exists() {
            continue;
        }

        std::fs::create_dir_all(&skill_dir)
            .map_err(|e| format!("Failed to create skill dir '{}': {}", skill.name, e))?;

        let skill_md_path = skill_dir.join("SKILL.md");
        std::fs::write(&skill_md_path, &skill.content)
            .map_err(|e| format!("Failed to write skill '{}': {}", skill.name, e))?;

        log::info!(
            "[BundledSkills] Exported '{}' to {}",
            skill.name,
            skill_dir.display()
        );
        count += 1;
    }

    log::info!(
        "[BundledSkills] Exported {} bundled skills to {}",
        count,
        target_dir.display()
    );
    Ok(count)
}
