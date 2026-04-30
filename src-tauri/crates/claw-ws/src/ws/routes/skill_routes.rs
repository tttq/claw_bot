// Claw Desktop - 技能路由 - 处理技能管理的WS请求
use axum::{
    Json, Router,
    extract::{Extension, Path, Query},
    routing::{get, post},
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use super::skill_installer;
use crate::ws::app_state::AppState;
use crate::ws::response::ApiResponse;
use crate::ws::router_trait::ClawRouter;
use claw_tools::skills;

/// 技能路由 — 处理技能执行/安装/市场的WS请求
pub struct SkillRoutes;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
/// 技能执行查询参数
pub struct SkillExecuteQuery {
    skill_name: String,
    #[serde(rename = "args")]
    args_str: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
/// 安装技能请求
pub struct InstallSkillRequest {
    agent_id: String,
    slug: String,
    name: String,
    #[serde(default = "default_version")]
    version: String,
    download_url: String,
}

/// 默认版本 — latest
fn default_version() -> String {
    "1.0.0".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
/// 市场列表查询参数
pub struct MarketplaceListQuery {
    #[serde(default = "default_page")]
    page: u16,
    #[serde(default = "default_page_size")]
    page_size: u16,
    keyword: Option<String>,
    category: Option<String>,
    #[serde(default = "default_sort_by")]
    sort_by: String,
    #[serde(default = "default_order")]
    order: String,
}

/// 默认页码 — 1
fn default_page() -> u16 {
    1
}
/// 默认每页数量 — 20
/// 默认页码 — 1
fn default_page_size() -> u16 {
    24
}
/// 默认排序 — updated
fn default_sort_by() -> String {
    "score".to_string()
}
/// 默认排序方向 — desc
fn default_order() -> String {
    "desc".to_string()
}

pub async fn skill_list(
    Extension(_state): Extension<Arc<AppState>>,
) -> Json<ApiResponse<serde_json::Value>> {
    let skills = skills::list_all_skills().await;
    Json(ApiResponse::ok(serde_json::json!({
        "count": skills.len(),
        "skills": skills.iter().map(|s| serde_json::json!({
            "name": s.name,
            "description": s.description,
            "aliases": s.aliases,
            "when_to_use": s.when_to_use,
        })).collect::<Vec<_>>()
    })))
}

/// 执行技能
pub async fn skill_execute(
    Extension(_state): Extension<Arc<AppState>>,
    Query(query): Query<SkillExecuteQuery>,
) -> Json<ApiResponse<serde_json::Value>> {
    let args = query.args_str.unwrap_or_default();
    match skills::execute_skill(&query.skill_name, &args, 0, None::<fn(String)>).await {
        Ok(result) => Json(ApiResponse::ok(serde_json::json!(result))),
        Err(e) => Json(ApiResponse::err(&e.to_string())),
    }
}

/// 安装技能
pub async fn skill_install(
    Extension(_state): Extension<Arc<AppState>>,
    Json(req): Json<InstallSkillRequest>,
) -> Json<ApiResponse<serde_json::Value>> {
    log::info!(
        "[SkillInstall] Installing {} ({}) v{} for agent {}",
        req.name,
        req.slug,
        req.version,
        req.agent_id
    );

    let result: Result<std::path::PathBuf, String> = async {
        let skill_dir = skill_installer::prepare_skill_directory(&req.agent_id, &req.slug)
            .map_err(|e| e.to_string())?;

        log::info!("[SkillInstall] Downloading from {}", req.download_url);
        let zip_bytes = skill_installer::download_skill_package(&req.download_url)
            .await
            .map_err(|e| e.to_string())?;
        log::info!(
            "[SkillInstall] Downloaded {} bytes, extracting...",
            zip_bytes.len()
        );

        skill_installer::extract_skill_package(&zip_bytes, &skill_dir)
            .map_err(|e| e.to_string())?;
        log::info!(
            "[SkillInstall] Skill {} installed successfully to {:?}",
            req.name,
            skill_dir
        );

        skill_installer::update_skill_config(&req.agent_id, &req.slug)
            .await
            .map_err(|e| e.to_string())?;

        Ok(skill_dir)
    }
    .await;

    match result {
        Ok(skill_dir) => Json(ApiResponse::ok(serde_json::json!({
            "success": true,
            "installed": true,
            "skill": {
                "slug": req.slug,
                "name": req.name,
                "version": req.version,
                "path": skill_dir.to_string_lossy().to_string(),
            },
            "message": format!("Skill '{}' (v{}) has been installed for agent", req.name, req.version),
        }))),
        Err(e) => {
            log::error!("[SkillInstall] Installation failed: {}", e);
            Json(ApiResponse::err(&e))
        }
    }
}

/// 列出市场技能
pub async fn skill_marketplace_list(
    Extension(_state): Extension<Arc<AppState>>,
    Query(query): Query<MarketplaceListQuery>,
) -> Json<ApiResponse<serde_json::Value>> {
    let mut url = format!(
        "https://api.skillhub.cn/api/skills?page={}&pageSize={}&sortBy={}&order={}",
        query.page,
        query.page_size,
        urlencoding::encode(&query.sort_by),
        urlencoding::encode(&query.order)
    );

    if let Some(ref k) = query.keyword {
        if !k.is_empty() {
            url.push_str(&format!("&keyword={}", urlencoding::encode(k)));
        }
    }
    if let Some(ref c) = query.category {
        if !c.is_empty() {
            url.push_str(&format!("&category={}", urlencoding::encode(c)));
        }
    }

    log::info!("[SkillMarketplace] Proxying: {}", url);
    let client = match reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
    {
        Ok(c) => c,
        Err(e) => {
            return Json(ApiResponse::err(&format!(
                "Failed to build HTTP client: {}",
                e
            )));
        }
    };

    match client
        .get(&url)
        .header("Accept", "application/json")
        .send()
        .await
    {
        Ok(resp) => {
            if !resp.status().is_success() {
                return Json(ApiResponse::err(&format!(
                    "API returned HTTP {}",
                    resp.status()
                )));
            }
            match resp.json().await {
                Ok(data) => Json(ApiResponse::ok(data)),
                Err(e) => Json(ApiResponse::err(&format!(
                    "Failed to parse response: {}",
                    e
                ))),
            }
        }
        Err(e) => Json(ApiResponse::err(&format!("Request failed: {}", e))),
    }
}

/// 获取市场技能文件列表
pub async fn skill_marketplace_files(
    Extension(_state): Extension<Arc<AppState>>,
    Path(slug): Path<String>,
) -> Json<ApiResponse<serde_json::Value>> {
    let url = format!("https://api.skillhub.cn/api/v1/skills/{}/files", slug);
    log::info!("[SkillMarketplace] Proxying files: {}", url);
    let client = match reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
    {
        Ok(c) => c,
        Err(e) => {
            return Json(ApiResponse::err(&format!(
                "Failed to build HTTP client: {}",
                e
            )));
        }
    };

    match client
        .get(&url)
        .header("Accept", "application/json")
        .send()
        .await
    {
        Ok(resp) => {
            if !resp.status().is_success() {
                return Json(ApiResponse::err(&format!(
                    "API returned HTTP {}",
                    resp.status()
                )));
            }
            match resp.json().await {
                Ok(data) => Json(ApiResponse::ok(data)),
                Err(e) => Json(ApiResponse::err(&format!(
                    "Failed to parse response: {}",
                    e
                ))),
            }
        }
        Err(e) => Json(ApiResponse::err(&format!("Request failed: {}", e))),
    }
}

impl ClawRouter for SkillRoutes {
    fn router() -> Router {
        Router::new()
            .route("/api/skills/list", get(skill_list))
            .route("/api/skills/execute", get(skill_execute))
            .route("/api/skills/install", post(skill_install))
            .route("/api/skills/marketplace", get(skill_marketplace_list))
            .route(
                "/api/skills/marketplace/:slug/files",
                get(skill_marketplace_files),
            )
            .route("/api/skills/permission/add", post(skill_permission_add))
            .route(
                "/api/skills/permission/remove",
                post(skill_permission_remove),
            )
            .route("/api/skills/permission/list", get(skill_permissions_list))
            .route("/api/skills/telemetry/list", get(skill_telemetry_list))
            .route("/api/skills/telemetry/clear", post(skill_telemetry_clear))
            .route("/api/skills/register-mcp", post(skill_register_mcp))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
/// 添加权限请求
pub struct PermissionAddRequest {
    tool_name: String,
    rule_content: String,
    behavior: String,
}

/// 添加技能权限
pub async fn skill_permission_add(
    Extension(_state): Extension<Arc<AppState>>,
    Json(req): Json<PermissionAddRequest>,
) -> Json<ApiResponse<serde_json::Value>> {
    let beh = match req.behavior.as_str() {
        "allow" => claw_tools::skills::PermissionBehavior::Allow,
        "deny" => claw_tools::skills::PermissionBehavior::Deny,
        _ => claw_tools::skills::PermissionBehavior::Ask,
    };
    claw_tools::skills::add_permission_rule(claw_tools::skills::SkillPermissionRule {
        tool_name: req.tool_name,
        rule_content: req.rule_content,
        behavior: beh,
    })
    .await;
    Json(ApiResponse::ok(serde_json::json!({ "success": true })))
}

#[derive(Debug, Clone, Serialize, Deserialize)]
/// 移除权限请求
pub struct PermissionRemoveRequest {
    index: usize,
}

/// 移除技能权限
pub async fn skill_permission_remove(
    Extension(_state): Extension<Arc<AppState>>,
    Json(req): Json<PermissionRemoveRequest>,
) -> Json<ApiResponse<serde_json::Value>> {
    match claw_tools::skills::remove_permission_rule(req.index).await {
        Ok(_) => Json(ApiResponse::ok(serde_json::json!({ "success": true }))),
        Err(e) => Json(ApiResponse::err(&e)),
    }
}

/// 列出技能权限
pub async fn skill_permissions_list(
    Extension(_state): Extension<Arc<AppState>>,
) -> Json<ApiResponse<serde_json::Value>> {
    let rules = claw_tools::skills::get_permission_rules().await;
    Json(ApiResponse::ok(serde_json::json!({ "rules": rules })))
}

#[derive(Debug, Clone, Serialize, Deserialize)]
/// 遥测列表查询参数
pub struct TelemetryListQuery {
    #[serde(default = "default_telemetry_limit")]
    limit: usize,
}

/// 默认遥测限制 — 100
fn default_telemetry_limit() -> usize {
    50
}

/// 列出技能遥测
pub async fn skill_telemetry_list(
    Extension(_state): Extension<Arc<AppState>>,
    Query(query): Query<TelemetryListQuery>,
) -> Json<ApiResponse<serde_json::Value>> {
    let events = claw_tools::skills::get_telemetry_log(query.limit).await;
    Json(ApiResponse::ok(
        serde_json::json!({ "count": events.len(), "events": events }),
    ))
}

/// 清除技能遥测
pub async fn skill_telemetry_clear(
    Extension(_state): Extension<Arc<AppState>>,
) -> Json<ApiResponse<serde_json::Value>> {
    claw_tools::skills::clear_telemetry().await;
    Json(ApiResponse::ok(serde_json::json!({ "success": true })))
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
/// 注册MCP服务请求
pub struct RegisterMcpRequest {
    name: String,
    description: String,
    prompt_template: String,
}

/// 注册MCP服务
pub async fn skill_register_mcp(
    Extension(_state): Extension<Arc<AppState>>,
    Json(req): Json<RegisterMcpRequest>,
) -> Json<ApiResponse<serde_json::Value>> {
    let skill =
        claw_tools::skills::register_mcp_skill(req.name, req.description, req.prompt_template)
            .await;
    Json(ApiResponse::ok(
        serde_json::json!({ "registered": true, "skill": { "name": skill.name } }),
    ))
}
