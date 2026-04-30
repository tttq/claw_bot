// Claw Desktop - 连接健康检查器 - 监控LLM API连接状态，触发恢复策略
// 功能：追踪连续失败次数、估算token数量、判断是否需要预压缩上下文

use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, Instant};

/// 连接健康检查器 - 追踪API调用的成功/失败状态，当连续失败超过阈值时触发恢复
pub struct ConnectionHealthChecker {
    consecutive_failures: AtomicUsize, // 连续失败次数（原子操作）
    last_success: std::sync::Mutex<Option<Instant>>, // 上次成功时间
    max_consecutive_failures: usize,   // 触发恢复的最大连续失败次数
}

impl ConnectionHealthChecker {
    /// 创建新的健康检查器
    /// max_consecutive_failures: 触发恢复的最大连续失败次数阈值
    pub fn new(max_consecutive_failures: usize) -> Self {
        Self {
            consecutive_failures: AtomicUsize::new(0),
            last_success: std::sync::Mutex::new(Some(Instant::now())),
            max_consecutive_failures,
        }
    }

    /// 记录一次成功的API调用，重置连续失败计数
    pub fn record_success(&self) {
        self.consecutive_failures.store(0, Ordering::SeqCst);
        let mut last_success = self.last_success.lock().unwrap_or_else(|e| {
            log::error!(
                "[ConnectionHealth:record_success] Mutex poisoned, recovering: {}",
                e
            );
            e.into_inner()
        });
        *last_success = Some(Instant::now());
    }

    /// 记录一次失败的API调用，返回当前连续失败次数
    pub fn record_failure(&self) -> usize {
        let failures = self.consecutive_failures.fetch_add(1, Ordering::SeqCst) + 1;
        log::warn!(
            "[ConnectionHealth:record_failure] Consecutive failures: {}/{}",
            failures,
            self.max_consecutive_failures
        );
        failures
    }

    /// 判断是否应触发连接恢复（连续失败次数达到阈值）
    pub fn should_trigger_recovery(&self) -> bool {
        self.consecutive_failures.load(Ordering::SeqCst) >= self.max_consecutive_failures
    }

    /// 获取当前连续失败次数
    pub fn get_consecutive_failures(&self) -> usize {
        self.consecutive_failures.load(Ordering::SeqCst)
    }

    /// 获取距离上次成功调用的时间间隔
    pub fn time_since_last_success(&self) -> Option<Duration> {
        let last_success = self.last_success.lock().unwrap_or_else(|e| {
            log::error!(
                "[ConnectionHealth:time_since_last_success] Mutex poisoned, recovering: {}",
                e
            );
            e.into_inner()
        });
        last_success.map(|instant| instant.elapsed())
    }

    /// 重置健康检查器状态
    pub fn reset(&self) {
        self.consecutive_failures.store(0, Ordering::SeqCst);
        let mut last_success = self.last_success.lock().unwrap_or_else(|e| {
            log::error!("[ConnectionHealth:reset] Mutex poisoned, recovering: {}", e);
            e.into_inner()
        });
        *last_success = Some(Instant::now());
    }
}

/// 粗略估算消息列表的token数量（按4字符≈1token计算）
pub fn estimate_tokens_approx(messages: &[serde_json::Value]) -> usize {
    let mut total_chars = 0;
    for msg in messages {
        if let Some(content) = msg.get("content") {
            total_chars += content.to_string().len();
        }
        if let Some(tool_calls) = msg.get("tool_calls") {
            total_chars += tool_calls.to_string().len();
        }
    }
    total_chars / 4
}

/// 判断是否应在发送API请求前进行上下文预压缩
/// 当估算token数超过阈值且消息数量足够多时返回true
/// protect_first_n/protect_last_n: 保护前N条和后N条消息不被压缩
pub fn should_compress_preflight(
    messages: &[serde_json::Value],
    context_threshold: usize,
    protect_first_n: usize,
    protect_last_n: usize,
) -> bool {
    if messages.len() <= protect_first_n + protect_last_n + 1 {
        return false;
    }

    let estimated_tokens = estimate_tokens_approx(messages);
    estimated_tokens >= context_threshold
}
