// Claw Desktop - 验证器 - Agent输出验证和校验
use crate::harness::types::{
    ValidationCheckType, ValidationResult, ValidationSeverity,
};
use regex::Regex;
use std::time::Instant;

/// 验证引擎 — Agent输出的多维度验证和校验
///
/// 支持六种检查类型：格式、安全、事实一致性、工具参数、长度、自定义正则
pub struct ValidationEngine;

impl ValidationEngine {
    /// 验证输出 — 对输出文本执行指定的检查列表，返回所有验证结果
    pub fn validate_output(output: &str, checks: &[ValidationCheckType]) -> Vec<ValidationResult> {
        let mut results = Vec::new();

        for check in checks {
            let start = Instant::now();
            let result = match check {
                ValidationCheckType::FormatCheck => Self::check_format(output),
                ValidationCheckType::SafetyCheck => Self::check_safety(output),
                ValidationCheckType::FactConsistencyCheck => Self::check_fact_consistency(output),
                ValidationCheckType::ToolArgumentCheck => Self::check_tool_arguments(output),
                ValidationCheckType::LengthCheck => Self::check_length(output),
                ValidationCheckType::CustomRegexCheck => Self::check_custom_regex(output),
            };
            let duration_ms = start.elapsed().as_millis() as u64;

            results.push(ValidationResult {
                check_type: check.clone(),
                is_passed: result.is_passed,
                message: result.message,
                severity: result.severity,
                fix_suggestion: result.fix_suggestion,
                duration_ms,
            });
        }

        results
    }

    /// 格式检查 — 检测空输出、无效JSON、未闭合代码块
    fn check_format(output: &str) -> ValidationResult {
        let trimmed = output.trim();

        if trimmed.is_empty() {
            return ValidationResult {
                check_type: ValidationCheckType::FormatCheck,
                is_passed: false,
                message: "Output is empty".to_string(),
                severity: ValidationSeverity::Warn,
                fix_suggestion: Some("Provide a non-empty response".to_string()),
                duration_ms: 0,
            };
        }

        if trimmed.starts_with('{') || trimmed.starts_with('[') {
            match serde_json::from_str::<serde_json::Value>(trimmed) {
                Ok(_) => {}
                Err(e) => {
                    return ValidationResult {
                        check_type: ValidationCheckType::FormatCheck,
                        is_passed: false,
                        message: format!("Invalid JSON: {}", e),
                        severity: ValidationSeverity::Error,
                        fix_suggestion: Some(
                            "Fix JSON syntax errors or wrap in markdown code block".to_string(),
                        ),
                        duration_ms: 0,
                    };
                }
            }
        }

        let has_unterminated_code_block = {
            let count = trimmed.matches("```").count();
            count % 2 != 0
        };
        if has_unterminated_code_block {
            return ValidationResult {
                check_type: ValidationCheckType::FormatCheck,
                is_passed: false,
                message: "Unterminated markdown code block detected".to_string(),
                severity: ValidationSeverity::Warn,
                fix_suggestion: Some("Close all code blocks with ```".to_string()),
                duration_ms: 0,
            };
        }

        ValidationResult {
            check_type: ValidationCheckType::FormatCheck,
            is_passed: true,
            message: "Format validation passed".to_string(),
            severity: ValidationSeverity::Info,
            fix_suggestion: None,
            duration_ms: 0,
        }
    }

    /// 安全检查 — 检测API密钥、密码、Token、私钥等敏感信息泄露
    fn check_safety(output: &str) -> ValidationResult {
        let safety_checks = [
            (r#"(?i)(api[_-]?key|apikey)\s*[:=]\s*['"]?sk-[a-zA-Z0-9]{20,}"#, "safety.api_key", ValidationSeverity::Critical),
            (r#"(?i)(password|passwd|pwd)\s*[:=]\s*['"]?[^\s'"]{8,}"#, "safety.password", ValidationSeverity::Critical),
            (r#"(?i)(secret|token|bearer)\s*[:=]\s*['"]?[a-zA-Z0-9._-]{20,}"#, "safety.token", ValidationSeverity::Warn),
            (r#"(?i)-----BEGIN\s+(?:RSA\s+|EC\s+)?PRIVATE\s+KEY-----"#, "safety.private_key", ValidationSeverity::Critical),
            (r#"(?i)(?:Bearer|Basic)\s+[a-zA-Z0-9._-]{20,}"#, "safety.auth_header", ValidationSeverity::Warn),
        ];

        for (pattern, name, severity) in &safety_checks {
            if let Ok(re) = Regex::new(pattern) {
                if re.is_match(output) {
                    return ValidationResult {
                        check_type: ValidationCheckType::SafetyCheck,
                        is_passed: false,
                        message: format!("Potential {} detected in output", name),
                        severity: severity.clone(),
                        fix_suggestion: Some(format!(
                            "Remove or mask the {} before presenting to user",
                            name
                        )),
                        duration_ms: 0,
                    };
                }
            }
        }

        ValidationResult {
            check_type: ValidationCheckType::SafetyCheck,
            is_passed: true,
            message: "No sensitive information detected".to_string(),
            severity: ValidationSeverity::Info,
            fix_suggestion: None,
            duration_ms: 0,
        }
    }

    /// 事实一致性检查 — 检测输出中的自相矛盾模式
    fn check_fact_consistency(output: &str) -> ValidationResult {
        let contradiction_patterns = [
            (r"(?i)yes.*but.*no", "Possible yes/no contradiction"),
            (r"(?i)correct.*however.*incorrect", "Possible correct/incorrect contradiction"),
            (r"(?i)always.*except.*never", "Possible always/never contradiction"),
        ];

        for (pattern, desc) in &contradiction_patterns {
            if let Ok(re) = Regex::new(pattern) {
                if re.is_match(output) {
                    return ValidationResult {
                        check_type: ValidationCheckType::FactConsistencyCheck,
                        is_passed: false,
                        message: desc.to_string(),
                        severity: ValidationSeverity::Warn,
                        fix_suggestion: Some(
                            "Review for potential contradictions in the response".to_string(),
                        ),
                        duration_ms: 0,
                    };
                }
            }
        }

        ValidationResult {
            check_type: ValidationCheckType::FactConsistencyCheck,
            is_passed: true,
            message: "No obvious contradictions detected".to_string(),
            severity: ValidationSeverity::Info,
            fix_suggestion: None,
            duration_ms: 0,
        }
    }

    /// 工具参数检查 — 检测路径遍历等危险参数
    fn check_tool_arguments(output: &str) -> ValidationResult {
        if let Ok(re) = Regex::new(r#""input"\s*:\s*\{[^}]*\}"#) {
            for cap in re.captures_iter(output) {
                let json_str = cap.get(0).map(|m| m.as_str()).unwrap_or("");
                if json_str.contains("\"path\"") && json_str.contains("..") {
                    return ValidationResult {
                        check_type: ValidationCheckType::ToolArgumentCheck,
                        is_passed: false,
                        message: "Path traversal detected in tool arguments".to_string(),
                        severity: ValidationSeverity::Error,
                        fix_suggestion: Some(
                            "Use absolute paths instead of relative path traversal".to_string(),
                        ),
                        duration_ms: 0,
                    };
                }
            }
        }

        ValidationResult {
            check_type: ValidationCheckType::ToolArgumentCheck,
            is_passed: true,
            message: "Tool arguments appear safe".to_string(),
            severity: ValidationSeverity::Info,
            fix_suggestion: None,
            duration_ms: 0,
        }
    }

    /// 长度检查 — 超过50000字符为错误，超过30000字符为警告
    fn check_length(output: &str) -> ValidationResult {
        let max_output_chars = 50000;
        let warn_output_chars = 30000;

        if output.len() > max_output_chars {
            return ValidationResult {
                check_type: ValidationCheckType::LengthCheck,
                is_passed: false,
                message: format!(
                    "Output exceeds maximum length ({} > {} chars)",
                    output.len(),
                    max_output_chars
                ),
                severity: ValidationSeverity::Error,
                fix_suggestion: Some("Summarize or truncate the response".to_string()),
                duration_ms: 0,
            };
        }

        if output.len() > warn_output_chars {
            return ValidationResult {
                check_type: ValidationCheckType::LengthCheck,
                is_passed: true,
                message: format!(
                    "Output is long ({} chars, warning threshold: {})",
                    output.len(),
                    warn_output_chars
                ),
                severity: ValidationSeverity::Warn,
                fix_suggestion: Some(
                    "Consider summarizing for better readability".to_string(),
                ),
                duration_ms: 0,
            };
        }

        ValidationResult {
            check_type: ValidationCheckType::LengthCheck,
            is_passed: true,
            message: format!("Output length is acceptable ({} chars)", output.len()),
            severity: ValidationSeverity::Info,
            fix_suggestion: None,
            duration_ms: 0,
        }
    }

    /// 自定义正则检查 — 预留接口
    ///
    /// 扩展方式：在ValidationEngine中添加自定义正则模式注册方法，
    /// 将用户定义的正则表达式存入Vec<(String, Regex)>，
    /// 此方法遍历已注册模式进行匹配检查
    fn check_custom_regex(_output: &str) -> ValidationResult {
        ValidationResult {
            check_type: ValidationCheckType::CustomRegexCheck,
            is_passed: true,
            message: "No custom regex patterns configured".to_string(),
            severity: ValidationSeverity::Info,
            fix_suggestion: None,
            duration_ms: 0,
        }
    }

    /// 判断是否存在严重失败 — 任何Error及以上级别的未通过检查
    pub fn has_critical_failure(results: &[ValidationResult]) -> bool {
        results
            .iter()
            .any(|r| !r.is_passed && r.severity >= ValidationSeverity::Error)
    }

    /// 格式化验证报告 — 生成Markdown格式的验证结果摘要
    pub fn format_validation_report(results: &[ValidationResult]) -> String {
        if results.is_empty() {
            return String::new();
        }

        let mut report = String::from("\n## Validation Report\n");
        let passed = results.iter().filter(|r| r.is_passed).count();
        let failed = results.len() - passed;

        report.push_str(&format!(
            "Checks: {} passed, {} failed\n\n",
            passed, failed
        ));

        for result in results {
            let status = if result.is_passed { "PASS" } else { "FAIL" };
            report.push_str(&format!(
                "- [{}] {:?}: {} ({:?})",
                status, result.check_type, result.message, result.severity
            ));
            if let Some(ref fix) = result.fix_suggestion {
                report.push_str(&format!(" | Fix: {}", fix));
            }
            report.push('\n');
        }

        report
    }
}
