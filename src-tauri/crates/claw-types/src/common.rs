// Claw Desktop - 通用类型 - 订阅ID、会话ID等通用类型定义
use serde::{Deserialize, Serialize};
use async_trait::async_trait;

/// 工具定义结构体（对应 Anthropic/OpenAI tool_use schema 格式）
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ToolDefinition {
    pub name: String,                       // 工具名称
    pub description: String,                // 工具描述
    pub input_schema: serde_json::Value,    // 输入参数的JSON Schema
    #[serde(default)]
    pub category: Option<String>,           // 工具分类
    #[serde(default)]
    pub tags: Vec<String>,                  // 工具标签
}

/// 安全截断字符串到指定字节数（确保不截断在多字节字符中间）
pub fn truncate_str_safe(s: &str, max_bytes: usize) -> &str {
    if s.len() <= max_bytes {
        return s;
    }
    let mut end = max_bytes;
    while end > 0 && !s.is_char_boundary(end) {
        end -= 1;
    }
    &s[..end]
}

/// 订阅ID类型
pub type SubscriptionId = u64;

/// ONNX LocalEmbedder 嵌入向量维度 (all-MiniLM-L6-v2)
pub const EMBEDDING_DIM: usize = 384;

/// 工具执行器 trait（全局工具注册表接口）
/// 由 claw-core::tools 提供运行时实现，claw-llm 通过此 trait 调用工具
#[async_trait]
pub trait ToolExecutor: Send + Sync {
    async fn execute(&self, name: &str, params: &serde_json::Value) -> Result<serde_json::Value, String>;
    fn list_all_tools(&self) -> Vec<ToolDefinition>;
    async fn list_tools_for_agent(&self, agent_id: &str) -> Vec<ToolDefinition>;
}
