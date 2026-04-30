// Claw Desktop - 错误分类器 - 对LLM API错误进行分类并给出恢复策略建议
// 根据HTTP状态码和错误消息内容，将错误分为：限流、认证、上下文溢出、服务器、超时、网络等类型

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// LLM错误类型枚举 - 覆盖所有可能的API错误场景
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum LlmErrorType {
    RateLimit,         // API调用频率超限 (HTTP 429)
    AuthError,         // 认证失败 (HTTP 401/403)
    ContextOverflow,   // 上下文长度超出模型限制
    ServerError,       // 服务端内部错误 (HTTP 5xx)
    Timeout,           // 请求超时
    NetworkError,      // 网络连接失败
    PayloadTooLarge,   // 请求体过大 (HTTP 413)
    ThinkingSignature, // 思维块签名无效（Claude特有）
    LongContextTier,   // 长上下文层级超限
    EmptyResponse,     // LLM返回空响应
    InvalidResponse,   // LLM返回无效格式响应
    EncodingError,     // 文本编码错误（如代理对字符）
    Unknown,           // 未知错误类型
}

/// 错误分类结果 - 包含错误类型、是否可重试、建议操作等恢复策略信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorClassification {
    pub error_type: LlmErrorType,      // 错误类型
    pub is_retryable: bool,            // 是否可重试
    pub suggested_action: String,      // 建议的恢复操作
    pub should_switch_key: bool,       // 是否应切换API Key
    pub should_compress_context: bool, // 是否应压缩上下文
    pub should_fallback: bool,         // 是否应回退到备选方案
    pub retry_after_secs: Option<u64>, // 建议等待的秒数（限流场景）
}

impl ErrorClassification {
    /// 根据HTTP状态码和错误消息分类错误类型
    /// 返回包含完整恢复策略的ErrorClassification实例
    pub fn classify(status_code: u16, error_message: &str) -> Self {
        let msg_lower = error_message.to_lowercase();

        match status_code {
            429 => {
                let retry_after = Self::extract_retry_after(error_message);
                Self {
                    error_type: LlmErrorType::RateLimit,
                    is_retryable: true,
                    suggested_action: "Switch API key or wait for rate limit reset".to_string(),
                    should_switch_key: true,
                    should_compress_context: false,
                    should_fallback: true,
                    retry_after_secs: retry_after,
                }
            }
            401 | 403 => Self {
                error_type: LlmErrorType::AuthError,
                is_retryable: false,
                suggested_action: "Check API key configuration".to_string(),
                should_switch_key: true,
                should_compress_context: false,
                should_fallback: false,
                retry_after_secs: None,
            },
            413 => Self {
                error_type: LlmErrorType::PayloadTooLarge,
                is_retryable: true,
                suggested_action: "Compress conversation context and retry".to_string(),
                should_switch_key: false,
                should_compress_context: true,
                should_fallback: false,
                retry_after_secs: None,
            },
            400 if Self::is_thinking_signature_error(&msg_lower) => Self {
                error_type: LlmErrorType::ThinkingSignature,
                is_retryable: true,
                suggested_action: "Strip thinking blocks and retry".to_string(),
                should_switch_key: false,
                should_compress_context: false,
                should_fallback: false,
                retry_after_secs: None,
            },
            400 if msg_lower.contains("context")
                || msg_lower.contains("token")
                || msg_lower.contains("too many") =>
            {
                Self {
                    error_type: LlmErrorType::ContextOverflow,
                    is_retryable: true,
                    suggested_action: "Compress conversation context and retry".to_string(),
                    should_switch_key: false,
                    should_compress_context: true,
                    should_fallback: false,
                    retry_after_secs: None,
                }
            }
            400 if Self::is_long_context_tier_error(&msg_lower) => Self {
                error_type: LlmErrorType::LongContextTier,
                is_retryable: true,
                suggested_action: "Reduce context window to standard tier".to_string(),
                should_switch_key: false,
                should_compress_context: true,
                should_fallback: false,
                retry_after_secs: None,
            },
            500 | 502 | 503 => Self {
                error_type: LlmErrorType::ServerError,
                is_retryable: true,
                suggested_action: "Retry with exponential backoff".to_string(),
                should_switch_key: false,
                should_compress_context: false,
                should_fallback: true,
                retry_after_secs: None,
            },
            _ if msg_lower.contains("timeout") || msg_lower.contains("timed out") => Self {
                error_type: LlmErrorType::Timeout,
                is_retryable: true,
                suggested_action: "Retry with longer timeout".to_string(),
                should_switch_key: false,
                should_compress_context: false,
                should_fallback: false,
                retry_after_secs: None,
            },
            _ if msg_lower.contains("connection")
                || msg_lower.contains("network")
                || msg_lower.contains("dns") =>
            {
                Self {
                    error_type: LlmErrorType::NetworkError,
                    is_retryable: true,
                    suggested_action: "Check network connection and retry".to_string(),
                    should_switch_key: false,
                    should_compress_context: false,
                    should_fallback: false,
                    retry_after_secs: None,
                }
            }
            _ if msg_lower.contains("surrogate")
                || msg_lower.contains("encode")
                || msg_lower.contains("ascii") =>
            {
                Self {
                    error_type: LlmErrorType::EncodingError,
                    is_retryable: true,
                    suggested_action: "Sanitize encoding and retry".to_string(),
                    should_switch_key: false,
                    should_compress_context: false,
                    should_fallback: false,
                    retry_after_secs: None,
                }
            }
            _ => Self {
                error_type: LlmErrorType::Unknown,
                is_retryable: false,
                suggested_action: format!("Unhandled error: {}", error_message),
                should_switch_key: false,
                should_compress_context: false,
                should_fallback: false,
                retry_after_secs: None,
            },
        }
    }

    /// 从错误消息中提取retry-after等待时间（秒）
    fn extract_retry_after(error_message: &str) -> Option<u64> {
        let msg_lower = error_message.to_lowercase();

        for keyword in &["retry-after:", "retry_after:", "retry after "] {
            if let Some(pos) = msg_lower.find(keyword) {
                let start = pos + keyword.len();
                let remaining = &error_message[start..];
                let num_str: String = remaining
                    .chars()
                    .take_while(|c| c.is_ascii_digit())
                    .collect();
                if !num_str.is_empty() {
                    if let Ok(secs) = num_str.parse::<u64>() {
                        return Some(secs);
                    }
                }
            }
        }
        None
    }

    /// 检测是否为思维块签名错误（Claude API特有）
    fn is_thinking_signature_error(msg_lower: &str) -> bool {
        msg_lower.contains("thinking")
            && (msg_lower.contains("signature")
                || msg_lower.contains("invalid")
                || msg_lower.contains("tampered"))
    }

    /// 检测是否为长上下文层级超限错误
    fn is_long_context_tier_error(msg_lower: &str) -> bool {
        msg_lower.contains("long context")
            && (msg_lower.contains("tier")
                || msg_lower.contains("extra usage")
                || msg_lower.contains("upgrade"))
    }
}

/// 便捷函数：分类LLM错误并返回错误类型
pub fn classify_llm_error(error_str: &str, status_code: Option<u16>) -> LlmErrorType {
    ErrorClassification::classify(status_code.unwrap_or(0), error_str).error_type
}

/// 从LLM响应消息中提取推理/思考内容
/// 支持 reasoning_content、reasoning、reasoning_details 三种格式
pub fn extract_reasoning(message: &Value) -> Option<String> {
    if let Some(reasoning_content) = message.get("reasoning_content").and_then(|v| v.as_str()) {
        if !reasoning_content.is_empty() {
            return Some(reasoning_content.to_string());
        }
    }

    if let Some(reasoning) = message.get("reasoning").and_then(|v| v.as_str()) {
        if !reasoning.is_empty() {
            return Some(reasoning.to_string());
        }
    }

    if let Some(details) = message.get("reasoning_details").and_then(|v| v.as_array()) {
        let parts: Vec<String> = details
            .iter()
            .filter_map(|d| {
                d.get("text")
                    .and_then(|t| t.as_str())
                    .map(|s| s.to_string())
            })
            .filter(|s| !s.is_empty())
            .collect();
        if !parts.is_empty() {
            return Some(parts.join("\n"));
        }
    }

    None
}

/// 提示缓存守卫 - 防止并发请求破坏提示缓存的一致性
pub struct PromptCacheGuard {
    locked: std::sync::Mutex<()>,
}

impl PromptCacheGuard {
    /// 创建新的提示缓存守卫实例
    pub fn new() -> Self {
        Self {
            locked: std::sync::Mutex::new(()),
        }
    }

    /// 获取互斥锁，如果锁被毒化则恢复
    pub fn lock(&self) -> std::sync::MutexGuard<'_, ()> {
        self.locked.lock().unwrap_or_else(|e| e.into_inner())
    }

    /// 检查锁是否被占用
    pub fn is_locked(&self) -> bool {
        self.locked.try_lock().is_err()
    }
}
