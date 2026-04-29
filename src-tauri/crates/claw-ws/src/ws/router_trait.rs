// Claw Desktop - 路由Trait - 定义路由处理器的统一接口
use axum::{Router};

/// 路由处理器Trait - 所有WS路由模块必须实现此接口
/// 每个路由模块通过实现 router() 方法注册自己的路由
pub trait ClawRouter: Send + Sync + 'static {
    /// 返回该路由模块的Axum Router实例
    fn router() -> Router;
}
