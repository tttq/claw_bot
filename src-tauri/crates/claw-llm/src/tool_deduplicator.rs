// Claw Desktop - 工具调用去重器 - 检测并过滤同一轮次中的重复工具调用
// 当Agent在同一轮次中多次调用相同工具且参数相同时，自动跳过重复调用

use std::collections::HashSet;

/// 工具调用指纹 - 用于唯一标识一次工具调用（工具名+参数组合）
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ToolCallFingerprint {
    pub name: String,      // 工具名称
    pub arguments: String, // 工具参数的JSON字符串
}

/// 工具调用去重器 - 在单轮工具循环中追踪已执行的工具调用，防止重复执行
pub struct ToolCallDeduplicator {
    seen: HashSet<ToolCallFingerprint>, // 已见过的工具调用指纹集合
    removed_count: usize,               // 已移除的重复调用计数
}

impl ToolCallDeduplicator {
    /// 创建新的去重器实例
    pub fn new() -> Self {
        Self {
            seen: HashSet::new(),
            removed_count: 0,
        }
    }

    /// 检查工具调用是否为重复调用
    /// 如果是重复调用返回true，否则记录并返回false
    pub fn is_duplicate(&mut self, name: &str, arguments: &str) -> bool {
        let fingerprint = ToolCallFingerprint {
            name: name.to_string(),
            arguments: arguments.to_string(),
        };
        let is_new = self.seen.insert(fingerprint);
        if !is_new {
            self.removed_count += 1;
            log::warn!("[Deduplicator] Removed duplicate tool call: {}", name);
        }
        !is_new
    }

    /// 获取已移除的重复调用数量
    pub fn removed_count(&self) -> usize {
        self.removed_count
    }

    /// 重置去重器状态（每轮工具循环开始时调用）
    pub fn reset(&mut self) {
        self.seen.clear();
        self.removed_count = 0;
    }
}
