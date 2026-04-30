// Claw Desktop - 工具系统库 - 提供工具注册、调度、执行等核心能力
// 整合 28+ 个工具为统一接口，分为 7 大类 + 动态扩展系统
// 从 claw-core 中拆分出来，解决循环依赖问题
//
// 架构 (v2.0 Plugins):
//   ┌─────────────────────────────────────────────┐
//   │              claw-tools                    │
//   ├─────────────────────────────────────────────┤
//   │  core/           核心基础设施               │
//   │    ├── tool_executor    ToolExecutor 实现   │
//   │    ├── tool_dispatcher  工具分发器          │
//   │    ├── tool_registry    动态注册表          │
//   │    ├── registry         静态工具定义        │
//   │    ├── extension_manager 扩展管理          │
//   │    ├── skill_loader     技能加载器          │
//   │    ├── skills           技能系统            │
//   │    ├── bundled_skills   内置技能           │
//   │    ├── agent_manager    Agent 管理         │
//   │    ├── agent_session    Agent 会话         │
//   │    ├── browser_manager  浏览器管理         │
//   │    ├── chrome_cdp       Chrome CDP         │
//   │    └── session_priority_queue             │
//   ├─────────────────────────────────────────────┤
//   │  plugins/        工具实现 (按功能分类)      │
//   │    ├── shell/         Shell 命令执行        │
//   │    ├── file/          文件操作              │
//   │    ├── search/        搜索工具              │
//   │    ├── git/           Git 操作              │
//   │    ├── web/           网络工具              │
//   │    ├── agent/         Agent 编排            │
//   │    └── misc/          杂项辅助              │
//   └─────────────────────────────────────────────┘

// ==================== Core 基础设施 ====================
pub mod agent_manager;
pub mod agent_session;
pub mod browser_manager;
pub mod bundled_skills;
pub mod chrome_cdp;
pub mod extension_manager;
pub mod global_registry;
pub mod mcp_client;
pub mod registry;
pub mod session_priority_queue;
pub mod skill_loader;
pub mod skills;
pub mod tool_dispatcher;
pub mod tool_executor;
pub mod tool_registry;

// ==================== Plugins 工具实现 ====================
pub mod plugins;

// Re-export types from claw-types for convenience
pub use claw_types::common::ToolDefinition;
pub use registry::*;
pub use tool_registry::*;

// Backward compatibility: re-export all plugin functions
pub use plugins::agent::*;
pub use plugins::file::*;
pub use plugins::git::*;
pub use plugins::misc::*;
pub use plugins::search::*;
pub use plugins::shell::*;
pub use plugins::web::*;
