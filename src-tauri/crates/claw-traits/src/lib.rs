// Claw Desktop - 共享Trait库 - 定义跨crate使用的核心Trait
// 定义跨层通信的 trait 接口和全局注入点
// 解决循环依赖的关键：下层通过 trait 访问上层功能

pub mod automation;
pub mod channel_ops;
pub mod event_bus;
pub mod llm_caller;
pub mod tool_executor;

pub use automation::*;
pub use channel_ops::*;
pub use event_bus::*;
pub use llm_caller::*;
pub use tool_executor::*;
