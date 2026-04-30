// Claw Desktop - LLM核心库 - 大语言模型交互的统一抽象层
// 功能：API 调用、工具循环 (ReAct)、流式处理、Prompt 构建
// ✅ Phase 2 物理迁移完成 — 从 claw-core/src/llm/ 迁移至此

pub mod api_client;
pub mod connection_health;
pub mod constants;
pub mod credential_pool;
pub mod encoding_recovery;
pub mod engine;
pub mod error_classifier;
pub mod llm;
pub mod llm_caller_impl;
pub mod loop_detector;
pub mod message_sanitizer;
pub mod prompt_builder;
pub mod streaming;
pub mod tool_deduplicator;
pub mod tool_executor;
pub mod tool_loop;

pub use llm::build_thinking_param;
pub use llm::effective_temperature;
pub use llm::effective_top_p;
pub use llm::http_client;
pub use llm::model_ignores_temperature;
pub use llm::model_ignores_top_p;
pub use llm::model_supports_thinking;
pub use llm::model_uses_reasoning_effort;
pub use llm_caller_impl::register_llm_caller;
