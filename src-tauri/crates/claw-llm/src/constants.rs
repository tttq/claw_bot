// Claw Desktop - 常量定义 - LLM相关的硬编码常量（最大轮次、超时时间等）
pub const MAX_TOOL_ROUNDS: usize = 15;
pub const TOTAL_LOOP_TIMEOUT_SECS: u64 = 180;
pub const MAX_SAME_TOOL_CONSECUTIVE: usize = 3;
pub const MAX_API_RETRIES: usize = 3;
pub const RETRY_DELAY_BASE_MS: u64 = 1000;
pub const CONTEXT_OVERFLOW_MAX_RETRIES: usize = 2;
pub const INCREMENTAL_SAVE_INTERVAL: usize = 3;

pub use crate::error_classifier::LlmErrorType;

/// 根据错误信息和HTTP状态码分类LLM错误类型
pub fn classify_llm_error(error_str: &str, status_code: Option<u16>) -> LlmErrorType {
    crate::error_classifier::ErrorClassification::classify(
        status_code.unwrap_or(0),
        error_str,
    ).error_type
}

/// 判断指定错误类型在当前重试次数下是否应该重试
pub fn should_retry_error(error_type: &LlmErrorType, retry_count: usize) -> bool {
    match error_type {
        LlmErrorType::RateLimit => retry_count < MAX_API_RETRIES,
        LlmErrorType::ServerError => retry_count < 2,
        LlmErrorType::NetworkError => retry_count < MAX_API_RETRIES,
        LlmErrorType::Timeout => retry_count < 2,
        LlmErrorType::ContextOverflow => retry_count < CONTEXT_OVERFLOW_MAX_RETRIES,
        LlmErrorType::PayloadTooLarge => retry_count < 3,
        LlmErrorType::EncodingError => retry_count < 2,
        _ => false,
    }
}

/// 根据错误类型和重试次数计算重试延迟时间（毫秒）
pub fn get_retry_delay_ms(error_type: &LlmErrorType, attempt: usize) -> u64 {
    match error_type {
        LlmErrorType::RateLimit => RETRY_DELAY_BASE_MS * 2u64.pow(attempt as u32 + 1),
        LlmErrorType::ServerError => 1000 * attempt as u64 + 500,
        LlmErrorType::NetworkError => RETRY_DELAY_BASE_MS * 2u64.pow(attempt as u32),
        LlmErrorType::Timeout => 2000 * attempt as u64 + 1000,
        LlmErrorType::PayloadTooLarge => 2000,
        LlmErrorType::EncodingError => 500,
        _ => RETRY_DELAY_BASE_MS,
    }
}

/// 根据错误类型生成用户友好的错误提示信息
pub fn format_error_for_user(error_type: &LlmErrorType, original: &str) -> String {
    match error_type {
        LlmErrorType::RateLimit => "API rate limit reached. Please wait a moment and try again.".to_string(),
        LlmErrorType::AuthError => "Authentication failed. Please check your API key configuration.".to_string(),
        LlmErrorType::ContextOverflow => "Conversation too long for the model. Try starting a new session or use /reset to clear history.".to_string(),
        LlmErrorType::Timeout => "The request timed out. The server may be busy or the task may be complex. Please try again.".to_string(),
        LlmErrorType::NetworkError => "Network connection failed. Please check your internet connection.".to_string(),
        LlmErrorType::ServerError => "The AI service encountered an internal error. This is usually temporary. Please try again.".to_string(),
        LlmErrorType::PayloadTooLarge => "Request payload too large. Context compression is being applied automatically.".to_string(),
        LlmErrorType::ThinkingSignature => "Invalid thinking block detected. Retrying with corrected format.".to_string(),
        LlmErrorType::LongContextTier => "Long context tier exceeded. Reducing context window automatically.".to_string(),
        LlmErrorType::EncodingError => "Text encoding issue detected. Sanitizing and retrying.".to_string(),
        LlmErrorType::EmptyResponse => "AI returned an empty response. Retrying...".to_string(),
        LlmErrorType::InvalidResponse => "AI returned an invalid response format. Retrying...".to_string(),
        LlmErrorType::Unknown => original.chars().take(300).collect::<String>(),
    }
}
