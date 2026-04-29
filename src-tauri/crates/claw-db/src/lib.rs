// Claw Desktop - 数据库库 - 提供数据库连接和操作封装
// 双数据库架构：主库(claw.db) + Agent库(agent_{id}.db)
// ✅ Phase 2 物理迁移完成 — 从 claw-core/src/db/ 迁移至此

pub mod db;
pub mod database;
pub mod vector_store;

pub use db::*;
pub use database::Database;
