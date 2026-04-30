// Claw Desktop - 工具执行器 - 执行工具调用并返回结果
// 实现 claw_types::common::ToolExecutor trait，提供工具执行和列表查询功能

use claw_types::common::{ToolDefinition, ToolExecutor as CoreToolExecutor};
use std::sync::Arc;

/// Claw工具执行器 — 实现CoreToolExecutor trait
pub struct ClawToolExecutor;

#[async_trait::async_trait]
impl CoreToolExecutor for ClawToolExecutor {
    /// 执行工具 — 分发到tool_dispatcher并记录耗时
    async fn execute(
        &self,
        name: &str,
        params: &serde_json::Value,
    ) -> Result<serde_json::Value, String> {
        log::info!(
            "[ToolExecutor] 执行工具: {} | 参数: {}",
            name,
            if params.as_object().map(|o| o.len()).unwrap_or(0) > 0 {
                format!("{:?}", params)
            } else {
                "(无)".to_string()
            }
        );
        let start = std::time::Instant::now();
        let result = crate::tool_dispatcher::dispatch_tool(name, params).await;
        let elapsed = start.elapsed();
        match &result {
            Ok(_) => log::info!(
                "[ToolExecutor] ✅ 工具 '{}' 执行成功 ({:.2}ms)",
                name,
                elapsed.as_millis()
            ),
            Err(e) => log::error!(
                "[ToolExecutor] ❌ 工具 '{}' 执行失败 ({:.2}ms): {}",
                name,
                elapsed.as_millis(),
                e
            ),
        }
        result
    }

    /// 列出所有工具 — 阻塞方式调用异步注册表
    fn list_all_tools(&self) -> Vec<ToolDefinition> {
        tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current()
                .block_on(async { crate::tool_registry::list_all_tools().await })
        })
    }

    /// 列出指定Agent可用的工具 — 根据Agent的skills_enabled过滤
    async fn list_tools_for_agent(&self, agent_id: &str) -> Vec<ToolDefinition> {
        crate::tool_registry::list_tools_for_agent(agent_id).await
    }
}

/// 创建并注册工具执行器到全局注入点
pub fn create_and_register_tool_executor() {
    let executor: Arc<dyn CoreToolExecutor> = Arc::new(ClawToolExecutor);
    crate::global_registry::set_tool_executor(executor);
    log::info!("[ToolExecutor] 已注册到全局注入点 (支持 28+ 内置工具)");
}
