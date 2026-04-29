// Claw Desktop - 技能安装器 - 从市场安装技能的逻辑
use thiserror::Error;
use std::path::{Path, PathBuf};

#[derive(Debug, Error)]
/// 技能安装错误类型
pub enum SkillInstallError {
    #[error("无法确定数据目录")]
    DataDirectoryNotFound,

    #[error("目录操作失败: {0}")]
    DirectoryOperation(String),

    #[error("下载失败: {0}")]
    DownloadFailed(String),

    #[error("HTTP请求失败: 状态码 {status}")]
    HttpError {
        status: reqwest::StatusCode,
    },

    #[error("ZIP文件无效: {0}")]
    InvalidZip(String),

    #[error("配置更新失败: {0}")]
    ConfigUpdate(String),
}

impl From<reqwest::Error> for SkillInstallError {
    fn from(err: reqwest::Error) -> Self {
        SkillInstallError::DownloadFailed(err.to_string())
    }
}

impl From<zip::result::ZipError> for SkillInstallError {
    fn from(err: zip::result::ZipError) -> Self {
        SkillInstallError::InvalidZip(err.to_string())
    }
}

impl From<std::io::Error> for SkillInstallError {
    fn from(err: std::io::Error) -> Self {
        SkillInstallError::DirectoryOperation(err.to_string())
    }
}

/// 准备技能安装目录 — 创建目标目录结构
pub fn prepare_skill_directory(
    agent_id: &str,
    slug: &str,
) -> Result<PathBuf, SkillInstallError> {
    let data_dir = dirs::data_dir()
        .ok_or(SkillInstallError::DataDirectoryNotFound)?;

    let skills_dir = data_dir
        .join("qclaw-desktop")
        .join("agents")
        .join(agent_id)
        .join("skills");

    std::fs::create_dir_all(&skills_dir)?;

    let skill_dir = skills_dir.join(slug);
    if skill_dir.exists() {
        std::fs::remove_dir_all(&skill_dir)?;
    }

    Ok(skill_dir)
}

/// 下载技能包 — 从URL下载ZIP文件
pub async fn download_skill_package(
    download_url: &str,
) -> Result<bytes::Bytes, SkillInstallError> {
    let response = reqwest::get(download_url).await?;

    if !response.status().is_success() {
        return Err(SkillInstallError::HttpError {
            status: response.status(),
        });
    }

    response.bytes().await.map_err(Into::into)
}

/// 解压技能包 — 解压ZIP到目标目录
pub fn extract_skill_package(
    zip_bytes: &[u8],
    skill_dir: &Path,
) -> Result<(), SkillInstallError> {
    std::fs::create_dir_all(skill_dir)?;

    let reader = std::io::Cursor::new(zip_bytes);
    let mut archive = zip::ZipArchive::new(reader)?;

    for i in 0..archive.len() {
        let mut file = archive.by_index(i)?;
        let outpath = skill_dir.join(file.name());

        if file.name().ends_with('/') {
            std::fs::create_dir_all(&outpath)?;
            continue;
        }

        if let Some(parent) = outpath.parent() {
            if !parent.exists() {
                std::fs::create_dir_all(parent)?;
            }
        }

        let mut outfile = std::fs::File::create(&outpath)?;
        std::io::copy(&mut file, &mut outfile)?;
    }

    Ok(())
}

/// 更新技能配置 — 注册新安装的技能
pub async fn update_skill_config(
    agent_id: &str,
    slug: &str,
) -> Result<(), SkillInstallError> {
    let mut enabled_skills: Vec<String> = Vec::new();

    if let Ok(Some(val)) =
        claw_tools::agent_session::AgentSessionManager::get_config(
            agent_id, "skills_enabled",
        )
        .await
    {
        if let Ok(arr) = serde_json::from_str::<Vec<String>>(&val) {
            enabled_skills = arr;
        }
    }

    if !enabled_skills.contains(&slug.to_string()) {
        enabled_skills.push(slug.to_string());
        claw_tools::agent_session::AgentSessionManager::set_config(
            agent_id,
            "skills_enabled",
            &serde_json::to_string(&enabled_skills).unwrap_or_default(),
        )
        .await
        .map_err(|e| SkillInstallError::ConfigUpdate(e.to_string()))?;
    }

    Ok(())
}
