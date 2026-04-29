// Claw Desktop - 共享类型库 - 定义跨crate使用的核心数据类型
// 提供统一的错误类型、事件定义、通用结构体
// 零循环依赖，作为所有crate的基础层

pub mod error;
pub mod events;
pub mod common;

pub use error::{ClawError, ClawResult};
pub use events::*;
pub use common::*;
