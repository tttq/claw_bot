// Claw Desktop - 工具执行器Trait - 定义工具执行的统一接口
use claw_types::common::ToolExecutor;
use std::sync::OnceLock;

/// 全局工具执行器实例
static TOOL_EXECUTOR: OnceLock<std::sync::Arc<dyn ToolExecutor>> = OnceLock::new();

/// 注入工具执行器实现（在应用启动时调用一次）
pub fn set_tool_executor(executor: std::sync::Arc<dyn ToolExecutor>) {
    let _ = TOOL_EXECUTOR.set(executor);
}

/// 获取全局工具执行器实例
pub fn get_tool_executor() -> Option<std::sync::Arc<dyn ToolExecutor>> {
    TOOL_EXECUTOR.get().cloned()
}

/// 检查工具执行器是否已注册
pub fn is_tool_executor_registered() -> bool {
    TOOL_EXECUTOR.get().is_some()
}
