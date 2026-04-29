// Claw Desktop - 循环检测器 - 检测Agent是否陷入重复调用循环
// 当Agent反复调用相同工具且参数/结果不变时，判定为循环并终止执行
// 检测模式：同一工具连续调用、乒乓模式（A→B→A→B）、无进展模式

use super::constants::MAX_SAME_TOOL_CONSECUTIVE;

/// 工具调用记录 - 记录单次工具调用的名称、参数哈希和结果预览
#[derive(Debug, Clone)]
struct ToolCallRecord {
    tool_name: String,       // 工具名称
    args_hash: u64,          // 参数的简单哈希值
    result_preview: String,  // 结果的前200字符预览
}

/// 循环检测状态枚举
#[derive(Debug, Clone, PartialEq)]
pub enum LoopStatus {
    Normal,              // 正常状态，未检测到循环
    Warning(String),     // 警告：检测到可能的循环趋势
    Broken(String),      // 严重：同一工具连续调用超过阈值
    Blocked(String),     // 阻断：乒乓模式或无进展模式被检测到
}

/// 循环检测器 - 追踪工具调用历史，检测Agent是否陷入重复调用循环
pub struct LoopDetector {
    history: Vec<ToolCallRecord>,    // 工具调用历史记录
    same_tool_count: usize,          // 同一工具连续调用次数
    status: LoopStatus,              // 当前检测状态
    warning_count: usize,            // 累计警告次数
}

impl LoopDetector {
    /// 创建新的循环检测器实例
    pub fn new() -> Self {
        Self { history: vec![], same_tool_count: 0, status: LoopStatus::Normal, warning_count: 0 }
    }

    /// 记录一次工具调用并返回当前循环检测状态
    /// tool_name: 工具名称, args_json: 参数JSON字符串, result_preview: 结果预览
    pub fn record(&mut self, tool_name: &str, args_json: &str, result_preview: &str) -> LoopStatus {
        let args_hash = self.simple_hash(args_json);
        let record = ToolCallRecord { tool_name: tool_name.to_string(), args_hash, result_preview: result_preview.chars().take(200).collect() };

        if let Some(last) = self.history.last() {
            if last.tool_name == tool_name && last.args_hash == args_hash {
                self.same_tool_count += 1;
            } else {
                self.same_tool_count = 1;
            }
        } else {
            self.same_tool_count = 1;
        }

        self.history.push(record);

        let status = self.evaluate();
        self.status = status.clone();
        status
    }

    /// 获取当前循环检测状态（不记录新调用）
    pub fn check(&self) -> &LoopStatus {
        &self.status
    }

    /// 重置循环检测器状态（新的工具循环开始时调用）
    pub fn reset(&mut self) {
        self.history.clear();
        self.same_tool_count = 0;
        self.status = LoopStatus::Normal;
        self.warning_count = 0;
    }

    /// 评估当前工具调用历史是否构成循环
    /// 检测逻辑：1.同一工具连续调用 2.乒乓模式 3.无进展模式
    fn evaluate(&mut self) -> LoopStatus {
        if self.same_tool_count >= MAX_SAME_TOOL_CONSECUTIVE {
            log::warn!("[LoopDetector] BROKEN: same tool '{}' repeated {} times consecutively", self.history.last().map(|t| t.tool_name.as_str()).unwrap_or("?"), self.same_tool_count);
            return LoopStatus::Broken(format!("same tool '{}' repeated {} times", self.history.last().map(|t| t.tool_name.as_str()).unwrap_or("?"), self.same_tool_count))
        }

        if self.is_ping_pong(2) || self.is_ping_pong(3) {
            log::warn!("[LoopDetector] BLOCKED: ping-pong pattern detected (history len={})", self.history.len());
            return LoopStatus::Blocked("ping-pong pattern detected".to_string())
        }

        if self.no_progress_in_last_n(3) {
            self.warning_count += 1;
            if self.warning_count >= 2 {
                log::warn!("[LoopDetector] BLOCKED: no progress in last 3 calls, warnings={}", self.warning_count);
                return LoopStatus::Blocked("no progress in last 3 calls".to_string())
            }
            log::info!("[LoopDetector] WARNING: no progress detected");
            return LoopStatus::Warning("no progress detected".to_string())
        }

        LoopStatus::Normal
    }

    /// 检测乒乓模式 - 周期为period的交替调用模式（如A→B→A→B）
    fn is_ping_pong(&self, period: usize) -> bool {
        if self.history.len() < period * 2 { return false }
        let n = self.history.len();
        for i in 0..period {
            if self.history[n - 1 - i].tool_name != self.history[n - 1 - period - i].tool_name { return false; }
            if self.history[n - 1 - i].args_hash != self.history[n - 1 - period - i].args_hash { return false; }
        }
        true
    }

    /// 检测无进展模式 - 最近N次调用的结果长度变化极小
    fn no_progress_in_last_n(&self, n: usize) -> bool {
        if self.history.len() < n + 1 { return false }
        let mut base_len: Option<usize> = None;
        for i in (0..=n).rev() {
            let idx = self.history.len() - 1 - i;
            let preview_len = self.history[idx].result_preview.len();
            if preview_len > 10 {
                if let Some(bl) = base_len {
                    if (preview_len as isize - bl as isize).abs() > 200 { return false; }
                } else {
                    base_len = Some(preview_len);
                }
            }
        }
        base_len.is_some()
    }

    /// 简单字符串哈希函数 - 用于快速比较工具调用参数是否相同
    fn simple_hash(&self, s: &str) -> u64 {
        s.bytes().fold(0u64, |acc, b| acc.wrapping_mul(31).wrapping_add(b as u64))
    }
}
