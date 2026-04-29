// Claw Desktop - 数据库模块入口
// Claw Core - 数据库模块
// 双数据库架构：主库(claw.db) + Agent库(agent_{id}.db)
// 自动初始化：首次访问时自动完成数据库连接和表创建

pub mod db;
pub mod database;

pub use db::*;
pub use database::Database;
