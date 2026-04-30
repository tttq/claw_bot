// Claw Desktop - 提示词构建器 - 组装系统提示词、RAG上下文、工具定义
use claw_config::config::AppConfig;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiMessage {
    pub role: String,
    pub content: String,
}

pub struct PromptBuilder;

impl PromptBuilder {
    /// 构建系统提示词 — 包含Agent配置和工具目录信息
    pub async fn build_system_prompt(
        config: &AppConfig,
        agent_id: Option<&str>,
        max_turns: Option<usize>,
        tool_count: usize,
        tool_catalog: Option<&str>,
    ) -> String {
        claw_rag::rag::build_system_prompt_with_agent(
            config,
            agent_id,
            max_turns,
            tool_count,
            tool_catalog,
        )
        .await
    }

    /// 构建API消息列表 — 组装系统提示词、RAG上下文和历史消息
    pub fn build_messages(
        history: &[ApiMessage],
        system_prompt: &str,
        rag_context: &str,
        _user_message: &str,
    ) -> Vec<serde_json::Value> {
        let mut msgs = Vec::new();

        let full_system = if rag_context.is_empty() {
            system_prompt.to_string()
        } else {
            format!("{}\n\n{}", system_prompt, rag_context)
        };
        msgs.push(serde_json::json!({"role": "system", "content": full_system}));

        for h in history {
            if h.role == "tool" {
                continue;
            }
            msgs.push(serde_json::json!({"role": h.role, "content": h.content}));
        }
        msgs
    }

    /// 从工具调用信息构建assistant消息的内容块（Anthropic格式）
    pub fn build_assistant_content_from_tool_uses(
        tool_uses: &[crate::llm::ToolCallInfo],
        text: &str,
        reasoning: Option<&str>,
    ) -> Vec<serde_json::Value> {
        let mut content = Vec::new();
        if let Some(rc) = reasoning {
            if !rc.is_empty() {
                content.push(serde_json::json!({"type": "thinking", "thinking": rc}));
            }
        }
        if !text.is_empty() {
            content.push(serde_json::json!({"type": "text", "text": text}));
        }
        for tc in tool_uses {
            content.push(serde_json::json!({
                "type": "tool_use",
                "id": tc.id,
                "name": crate::streaming::sanitize_tool_name_for_api(&tc.name),
                "input": tc.input
            }));
        }
        content
    }
}
