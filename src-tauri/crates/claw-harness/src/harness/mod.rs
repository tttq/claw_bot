// Claw Desktop - Harness模块入口
// 基于 Harness Engineering 方法论：Agent = Model + Harness
// 六层架构：Context Engineering → Tool Orchestration → Execution → State → Observability → Human Control
//
// 核心设计原则：
// - 一个 Harness = 一个 Agent（不概念膨胀）
// - 每个 Agent 有独立人物画像（Persona）
// - 错误学习循环：捕获→总结→提取规则→注入Prompt
// - 交叉记忆：@mention 跨 Agent 记忆检索
// - 任务拆分：主Agent拆分→子Agent执行→聚合返回

pub mod agents_md;
pub mod cron;
pub mod cross_memory;
pub mod error_learning;
pub mod hooks;
pub mod observability;
pub mod persona;
pub mod types;
pub mod validation;
