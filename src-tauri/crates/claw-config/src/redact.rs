// Claw Desktop - 脱敏处理 - 日志中API Key等敏感信息的脱敏
use regex::Regex;
use std::sync::LazyLock;

static REDACT_PATTERNS: LazyLock<Vec<Regex>> = LazyLock::new(|| {
    vec![
        Regex::new(r"sk-[a-zA-Z0-9]{20,}").expect("Invalid sk- pattern"),
        Regex::new(r"sk-ant-[a-zA-Z0-9]{20,}").expect("Invalid sk-ant- pattern"),
        Regex::new(r"(?i)(api[_\-]?key\s*[:=]\s*)\S+").expect("Invalid api key pattern"),
        Regex::new(r"(?i)(token\s*[:=]\s*)\S+").expect("Invalid token pattern"),
        Regex::new(r"(?i)(secret\s*[:=]\s*)\S+").expect("Invalid secret pattern"),
        Regex::new(r"(?i)(password\s*[:=]\s*)\S+").expect("Invalid password pattern"),
        Regex::new(r"Bearer\s+[a-zA-Z0-9\-._~+/]+=*").expect("Invalid bearer pattern"),
    ]
});

/// 脱敏处理文本中的敏感信息（API Key、Token、密码等）
pub fn redact_secrets(text: &str) -> String {
    let mut result = text.to_string();
    for pattern in REDACT_PATTERNS.iter() {
        result = pattern.replace_all(&result, "***REDACTED***").to_string();
    }
    result
}

/// 日志脱敏格式化器 - 自动脱敏日志记录中的敏感信息
pub struct RedactingFormatter;

impl RedactingFormatter {
    /// 格式化日志记录，自动脱敏敏感信息
    pub fn format_record(record: &log::Record) -> String {
        let raw = format!(
            "[{}][{}] {}",
            record.level(),
            record.target(),
            record.args()
        );
        redact_secrets(&raw)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_redact_openai_key() {
        let input = "Using key sk-1234567890abcdefghijklmnop";
        let output = redact_secrets(input);
        assert!(!output.contains("sk-1234567890"));
        assert!(output.contains("REDACTED"));
    }

    #[test]
    fn test_redact_bearer() {
        let input = "Authorization: Bearer abc123token456";
        let output = redact_secrets(input);
        assert!(!output.contains("abc123token456"));
    }

    #[test]
    fn test_redact_api_key_assignment() {
        let input = "api_key=sk-test1234567890123456789012";
        let output = redact_secrets(input);
        assert!(!output.contains("sk-test123"));
    }
}
