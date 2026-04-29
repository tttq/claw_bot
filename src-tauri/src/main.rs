// Claw Desktop - 程序入口
// 职责：初始化日志、构建Tauri应用、注册命令、启动WebSocket服务器、打开主窗口
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use tauri::{Emitter, Manager};

mod app_state;
mod commands;

fn main() {
    env_logger::init();

    log::info!("[Main:main] ========================================");
    log::info!("[Main:main] Claw Desktop starting...");
    log::info!("[Main:main] Version: {}", env!("CARGO_PKG_VERSION"));
    log::info!("[Main:main] Platform: {}", std::env::consts::OS);
    log::info!("[Main:main] ========================================");

    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .setup(|app| {
            tauri::async_runtime::block_on(async {
                // 1. 初始化路径解析器 + 配置
                claw_config::path_resolver::init(app)?;

                claw_config::path_resolver::register_skills_export_callback(
                    claw_tools::bundled_skills::export_bundled_skills
                );

                let config = claw_config::config::get_config().await?.clone();

                // 2. 使用 State<T> 模式注入核心应用状态
                let app_state = app_state::ClawAppState::new(app.handle().clone(), config.clone()).await;
                app.manage(app_state);

                // 3. 启动 WebSocket 服务
                let ws_state = claw_ws::bootstrap(app.handle().clone()).await?;
                log::info!("[Main] WebSocket server started on port {}", ws_state.port);

                // 3.5 初始化自动化引擎（UI Automation）- 在 config 被移动前执行
                if config.tools.automation {
                    let resolved_key = config.resolve_api_key().ok();
                    let base_url = config.get_base_url().to_string();
                    let auto_config = claw_automatically::AutomaticallyConfig {
                        manop_enabled: true,
                        manop_version: "quantized_4b".to_string(),
                        manop_auto_download: true,
                        manop_auto_initialize: true,
                        manop_cloud_api_url: "https://mano.mininglamp.com".to_string(),
                        manop_cloud_api_key: None,
                        inference_timeout_secs: 30,
                        max_action_steps: 50,
                        confidence_threshold: 0.75,
                        ocr_language: "chi_sim+eng".to_string(),
                        llm_api_endpoint: if base_url.is_empty() { "https://api.openai.com/v1/chat/completions".to_string() } else {
                            let trimmed = base_url.trim_end_matches('/');
                            if trimmed.ends_with("/chat/completions") { trimmed.to_string() } else { format!("{}/chat/completions", trimmed) }
                        },
                        llm_api_key: resolved_key,
                        llm_model: if config.model.custom_model_name.is_empty() { config.model.default_model.clone() } else { config.model.custom_model_name.clone() },
                        screen_capture_fps: 30,
                        session_ttl_seconds: 7200,
                        cua_enabled: true,
                    };
                    match claw_automatically::commands::init_engine_with_config(auto_config) {
                        Ok(()) => log::info!("[Main] AutomationEngine registered (automation enabled)"),
                        Err(e) => log::warn!("[Main] AutomationEngine registration failed: {}", e),
                    }
                } else {
                    log::info!("[Main] Automation disabled in settings, skipping engine init");
                }

                // 4. 启动 HTTP API 服务
                let http_app_state = claw_ws::app_state::AppState::new(config);
                let http_app_state_arc = std::sync::Arc::new(http_app_state);

                match claw_ws::router_registry::start_http_server(http_app_state_arc, 1421).await {
                    Ok(http_port) => {
                        log::info!("[Main] HTTP server started on port {}", http_port);
                        if let Err(e) = app.emit("http-server-ready", serde_json::json!({ "port": http_port })) {
                            log::warn!("[Main] Failed to emit http-server-ready event: {}", e);
                        }
                    }
                    Err(e) => {
                        log::error!("[Main] Failed to start HTTP server: {}", e);
                    }
                }

                // 5. 初始化通道系统
                claw_channel::bootstrap().await?;

                // 6. 初始化内置工具
                claw_tools::tool_registry::init_builtin_tools().await;
                log::info!("[Main] Built-in tools registered");

                // 7. 注册 ToolExecutor（使 LLM 工具循环可用）
                claw_tools::tool_executor::create_and_register_tool_executor();
                log::info!("[Main] ToolExecutor registered to global injection point");

                // 7.5 注册 LlmCaller（使 claw-tools 可通过 trait 调用 LLM，解决循环依赖）
                claw_llm::register_llm_caller();
                log::info!("[Main] LlmCaller registered to global injection point");

                // 8. 加载技能并初始化工具/技能记忆
                let loaded_skills = claw_tools::skill_loader::discover_and_load_all_skills().await;
                log::info!("[Main] Loaded {} skills", loaded_skills.len());

                let all_tools = claw_tools::tool_registry::list_all_tools().await;
                let tool_entries: Vec<claw_rag::rag::ToolMemoryEntry> = all_tools.iter().map(|t| {
                    claw_rag::rag::ToolMemoryEntry {
                        name: t.name.clone(),
                        description: t.description.clone(),
                        category: t.category.clone(),
                    }
                }).collect();
                let skill_entries: Vec<claw_rag::rag::SkillMemoryEntry> = loaded_skills.iter().map(|s| {
                    claw_rag::rag::SkillMemoryEntry {
                        name: s.name.clone(),
                        description: s.description.clone(),
                        when_to_use: s.when_to_use.clone().unwrap_or_default(),
                        allowed_tools: s.allowed_tools.clone(),
                        user_invocable: s.user_invocable,
                    }
                }).collect();

                match claw_rag::rag::initialize_tool_skill_memories(&tool_entries, &skill_entries).await {
                    Ok(count) => log::info!("[Main] Initialized {} tool/skill memory entries", count),
                    Err(e) => log::warn!("[Main] Failed to initialize tool/skill memories: {}", e),
                }

                // 9. 清理过期的工具/技能记忆（工具已被卸载但记忆仍在）
                match claw_rag::rag::cleanup_stale_tool_memories(&tool_entries, &skill_entries).await {
                    Ok(count) => log::info!("[Main] Cleaned up {} stale tool/skill memories", count),
                    Err(e) => log::warn!("[Main] Failed to cleanup stale memories: {}", e),
                }

                // 10. 启动时检查并压缩记忆（如果超过阈值）
                match claw_rag::rag::compact_all_agents().await {
                    Ok(count) => log::info!("[Main] Compacted memories for {} agents", count),
                    Err(e) => log::warn!("[Main] Memory compaction check failed: {}", e),
                }

                // 11. 启动记忆定期压缩后台任务（每6小时检查一次）
                tauri::async_runtime::spawn(async move {
                    let interval = std::time::Duration::from_secs(6 * 3600);
                    loop {
                        tokio::time::sleep(interval).await;
                        log::info!("[Main:MemoryMaintenance] Starting periodic memory compaction...");
                        match claw_rag::rag::compact_all_agents().await {
                            Ok(count) => log::info!("[Main:MemoryMaintenance] Compacted {} agents", count),
                            Err(e) => log::warn!("[Main:MemoryMaintenance] Compaction failed: {}", e),
                        }
                    }
                });

                // 7. 输出 State<T> 迁移状态摘要
                log::info!("[Main] State<T> migration status:");
                log::info!("[Main]   ✅ AppConfig → State<ClawAppState>.config");
                log::info!("[Main]   ✅ EventBus → State<ClawAppState>.event_bus");
                log::info!("[Main]   ⏳ DB_CONN → OnceCell (write-once, correct pattern)");
                log::info!("[Main]   ⏳ ToolExecutor → OnceLock (cross-crate access)");
                log::info!("[Main]   ⏳ ChannelOps → OnceLock (cross-crate access)");
                log::info!("[Main]   ⏳ AutomationExecutor → OnceLock (cross-crate access)");

                log::info!("[Main] All systems initialized");
                Ok(())
            })
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_config,
            commands::save_config,
            commands::send_message,
            commands::list_conversations,
            commands::create_conversation,
            commands::get_messages,
            claw_automatically::commands::init_automatically_engine,
            claw_automatically::commands::execute_automation_instruction,
            claw_automatically::commands::execute_cua_instruction,
            claw_automatically::commands::capture_screen,
            claw_automatically::commands::ocr_recognize_screen,
            claw_automatically::commands::mouse_click,
            claw_automatically::commands::mouse_double_click,
            claw_automatically::commands::mouse_right_click,
            claw_automatically::commands::mouse_scroll,
            claw_automatically::commands::mouse_drag,
            claw_automatically::commands::keyboard_type,
            claw_automatically::commands::keyboard_press,
            claw_automatically::commands::list_installed_apps,
            claw_automatically::commands::launch_application,
            claw_automatically::commands::get_active_window,
            claw_automatically::commands::get_window_title,
            claw_automatically::commands::list_windows,
            claw_automatically::commands::focus_window,
            claw_automatically::commands::get_screen_size,
            claw_automatically::commands::get_automation_config,
            claw_automatically::commands::init_mano_p_model,
            claw_automatically::commands::get_mano_p_status,
            claw_automatically::commands::download_mano_p_model,
            claw_automatically::commands::execute_mano_p_instruction,
            claw_automatically::commands::configure_mano_p_cloud,
            claw_automatically::commands::get_mano_p_status,
            claw_automatically::commands::search_apps,
            claw_automatically::commands::find_app,
            claw_automatically::commands::get_all_indexed_apps,
            claw_automatically::commands::refresh_app_index,
            claw_automatically::commands::get_app_index_stats,
            claw_ws::get_server_public_key,
            claw_ws::auth_handshake,
            claw_ws::auth_validate,
            claw_ws::get_ws_url,
        ])
        .run(tauri::generate_context!())
        .expect("Tauri 应用启动失败");
}
