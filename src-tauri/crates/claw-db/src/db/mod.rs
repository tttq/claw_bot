// Claw Desktop - 数据库实体模块入口
/// 数据库模块 - 双数据库架构
/// 主库(claw.db): 会话、消息、向量记忆、Channel 配置
/// Agent库(agent_{id}.db): Agent隔离配置、长期记忆、工作区文件
pub mod conn;
pub mod entities;
pub mod agent_entities;
pub mod channel_migration;
pub mod backend;

pub use conn::{init_main_db, init_agent_db, get_db, get_agent_db, try_get_agent_db, init_core_tables, init_agent_tables};
pub use backend::{DatabaseBackend, DatabaseInitResult, DatabaseStatus, BackendInitializer};
