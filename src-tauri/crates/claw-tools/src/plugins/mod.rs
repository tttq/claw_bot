// Claw Desktop - 工具插件模块入口
// Claw Tools - Plugins 模块
// 所有工具实现的集合，按功能分类组织
//
// 目录结构:
//   plugins/
//   ├── shell/      - Shell 命令执行 (bash, bash_cancel)
//   ├── file/       - 文件操作 (read, write, edit)
//   ├── search/     - 搜索工具 (glob, grep)
//   ├── git/        - Git 操作 (status, diff, commit, log, branch...)
//   ├── web/        - 网络工具 (fetch, search)
//   ├── agent/      - Agent 编排 (agent, todo, task, workflow, skill...)
//   └── misc/       - 杂项工具 (list_all, env, code_review...)

pub mod agent;
pub mod file;
pub mod git;
pub mod misc;
pub mod search;
pub mod shell;
pub mod web;

// Re-export for backward compatibility
pub use agent::*;
pub use file::*;
pub use git::*;
pub use misc::*;
pub use search::*;
pub use shell::*;
pub use web::*;
