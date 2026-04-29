// Claw Desktop - 编码恢复器 - 处理LLM消息中的Unicode编码错误
// 功能：检测并清理代理对字符(surrogate)、非ASCII字符等编码问题
// 当API因编码错误拒绝请求时，自动清理消息并重试

use std::sync::Mutex;

/// 编码恢复状态 - 追踪Unicode清理次数和强制ASCII模式
pub struct EncodingRecoveryState {
    pub unicode_sanitization_passes: Mutex<usize>,   // 已执行的Unicode清理次数
    pub force_ascii_payload: Mutex<bool>,             // 是否强制使用ASCII载荷
    pub max_sanitization_passes: usize,               // 最大允许清理次数
}

impl EncodingRecoveryState {
    /// 创建新的编码恢复状态
    /// max_sanitization_passes: 最大允许的Unicode清理次数
    pub fn new(max_sanitization_passes: usize) -> Self {
        Self {
            unicode_sanitization_passes: Mutex::new(0),
            force_ascii_payload: Mutex::new(false),
            max_sanitization_passes,
        }
    }

    /// 判断是否还应尝试Unicode清理（未超过最大次数）
    pub fn should_attempt_sanitization(&self) -> bool {
        let passes = self.unicode_sanitization_passes.lock().unwrap_or_else(|e| {
            log::error!("[EncodingRecovery:should_attempt_sanitization] Mutex poisoned, recovering: {}", e);
            e.into_inner()
        });
        *passes < self.max_sanitization_passes
    }

    /// 记录一次Unicode清理操作
    pub fn record_sanitization_pass(&self) {
        let mut passes = self.unicode_sanitization_passes.lock().unwrap_or_else(|e| {
            log::error!("[EncodingRecovery:record_sanitization_pass] Mutex poisoned, recovering: {}", e);
            e.into_inner()
        });
        *passes += 1;
    }

    /// 启用强制ASCII模式（移除所有非ASCII字符）
    pub fn enable_force_ascii(&self) {
        let mut flag = self.force_ascii_payload.lock().unwrap_or_else(|e| {
            log::error!("[EncodingRecovery:enable_force_ascii] Mutex poisoned, recovering: {}", e);
            e.into_inner()
        });
        *flag = true;
    }

    /// 检查是否已启用强制ASCII模式
    pub fn is_force_ascii_enabled(&self) -> bool {
        let flag = self.force_ascii_payload.lock().unwrap_or_else(|e| {
            log::error!("[EncodingRecovery:is_force_ascii_enabled] Mutex poisoned, recovering: {}", e);
            e.into_inner()
        });
        *flag
    }

    /// 重置编码恢复状态（API调用成功后调用）
    pub fn reset(&self) {
        let mut passes = self.unicode_sanitization_passes.lock().unwrap_or_else(|e| {
            log::error!("[EncodingRecovery:reset] unicode_sanitization_passes Mutex poisoned, recovering: {}", e);
            e.into_inner()
        });
        *passes = 0;
        let mut flag = self.force_ascii_payload.lock().unwrap_or_else(|e| {
            log::error!("[EncodingRecovery:reset] force_ascii_payload Mutex poisoned, recovering: {}", e);
            e.into_inner()
        });
        *flag = false;
    }
}

/// 判断字符是否为Unicode代理对字符（U+D800至U+DFFF）
/// 代理对字符在UTF-8编码中不合法，会导致API请求失败
pub fn is_surrogate(c: char) -> bool {
    let code = c as u32;
    (0xD800..=0xDFFF).contains(&code)
}

/// 清理字符串中的代理对字符，返回清理后的字符串
pub fn sanitize_surrogates_in_string(input: &str) -> String {
    input.chars()
        .filter(|c| !is_surrogate(*c))
        .collect()
}

/// 清理消息列表中的代理对字符，返回是否发现了需要清理的字符
pub fn sanitize_messages_surrogates(messages: &mut [serde_json::Value]) -> bool {
    let mut found_surrogates = false;
    
    for msg in messages.iter_mut() {
        if let Some(content) = msg.get_mut("content") {
            found_surrogates |= sanitize_value_surrogates(content);
        }
        if let Some(tool_calls) = msg.get_mut("tool_calls") {
            if let Some(tool_calls_arr) = tool_calls.as_array_mut() {
                for tc in tool_calls_arr.iter_mut() {
                    if let Some(function) = tc.get_mut("function") {
                        if let Some(args) = function.get_mut("arguments") {
                            if let Some(args_str) = args.as_str() {
                                let sanitized = sanitize_surrogates_in_string(args_str);
                                if sanitized != args_str {
                                    *args = serde_json::Value::String(sanitized);
                                    found_surrogates = true;
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    
    found_surrogates
}

/// 递归清理JSON值中的代理对字符
fn sanitize_value_surrogates(value: &mut serde_json::Value) -> bool {
    match value {
        serde_json::Value::String(s) => {
            let sanitized = sanitize_surrogates_in_string(s);
            if sanitized != *s {
                *s = sanitized;
                true
            } else {
                false
            }
        }
        serde_json::Value::Array(arr) => {
            let mut found = false;
            for item in arr.iter_mut() {
                found |= sanitize_value_surrogates(item);
            }
            found
        }
        serde_json::Value::Object(obj) => {
            let mut found = false;
            for (_, v) in obj.iter_mut() {
                found |= sanitize_value_surrogates(v);
            }
            found
        }
        _ => false,
    }
}

/// 清理字符串中的非ASCII字符，仅保留ASCII字符
pub fn sanitize_non_ascii_in_string(input: &str) -> String {
    input.chars()
        .filter(|c| c.is_ascii())
        .collect()
}

/// 清理消息列表中的非ASCII字符，返回是否发现了需要清理的字符
pub fn sanitize_messages_non_ascii(messages: &mut [serde_json::Value]) -> bool {
    let mut found_non_ascii = false;
    
    for msg in messages.iter_mut() {
        if let Some(content) = msg.get_mut("content") {
            found_non_ascii |= sanitize_value_non_ascii(content);
        }
    }
    
    found_non_ascii
}

/// 递归清理JSON值中的非ASCII字符
fn sanitize_value_non_ascii(value: &mut serde_json::Value) -> bool {
    match value {
        serde_json::Value::String(s) => {
            let sanitized = sanitize_non_ascii_in_string(s);
            if sanitized != *s {
                *s = sanitized;
                true
            } else {
                false
            }
        }
        serde_json::Value::Array(arr) => {
            let mut found = false;
            for item in arr.iter_mut() {
                found |= sanitize_value_non_ascii(item);
            }
            found
        }
        serde_json::Value::Object(obj) => {
            let mut found = false;
            for (_, v) in obj.iter_mut() {
                found |= sanitize_value_non_ascii(v);
            }
            found
        }
        _ => false,
    }
}
