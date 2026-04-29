// Claw Desktop - LLM模块入口 - 导出所有LLM子模块
// 整合了原 claw-llm crate 的所有功能
// 封装 Anthropic Claude API 和 OpenAI 兼容 API 的调用逻辑
// 子模块分工:
//   constants.rs     - 常量 + 错误分类枚举 + 重试策略
//   loop_detector.rs - LoopDetector 死循环检测
//   credential_pool.rs - CredentialPool API Key 轮转池 (已完成)
//   error_classifier.rs - 错误分类器 (已完成)
//   api_client.rs    - API 调用 (Anthropic/OpenAI) 非流式
//   streaming.rs     - 流式响应解析 + SSE 推送 + emit 事件
//   prompt_builder.rs - System Prompt 构建 + 消息组装
//   tool_loop.rs     - 工具循环主逻辑 + 工具执行 + 结果处理
//   llm.rs           - 入口: 类型定义 + 公共API + 辅助函数

pub mod constants;
pub mod loop_detector;
pub mod credential_pool;
pub mod error_classifier;
pub mod api_client;
pub mod streaming;
pub mod prompt_builder;
pub mod tool_loop;
pub mod llm;
pub mod engine;

pub use constants::*;
pub use loop_detector::{LoopDetector, LoopStatus};
pub use credential_pool::CredentialPool;
pub use error_classifier::{LlmErrorType, ErrorClassification};
pub use engine::LlmEngine;
pub use llm::{ChatResponse, send_chat_message, send_chat_message_streaming, test_llm_connection_detailed};
