// Claw Desktop - 事件总线Trait - 定义发布/订阅事件总线接口
use async_trait::async_trait;
use claw_types::common::SubscriptionId;
use claw_types::events::AppEvent;
use std::sync::OnceLock;

/// 事件处理器 trait
#[async_trait]
pub trait EventHandler: Send + Sync {
    async fn handle(&self, event: &AppEvent);
}

/// 事件总线 trait（由主程序实现）
#[async_trait]
pub trait EventBus: Send + Sync {
    /// 发布事件（异步，不阻塞发布者）
    async fn publish(&self, event: AppEvent);

    /// 订阅事件（支持通配符过滤）
    async fn subscribe(
        &self,
        event_pattern: &str,
        handler: Box<dyn EventHandler>,
    ) -> SubscriptionId;

    /// 取消订阅
    async fn unsubscribe(&self, id: SubscriptionId);
}

// ===== 全局注入点 =====
static EVENT_BUS: OnceLock<Box<dyn EventBus>> = OnceLock::new();

/// 注入 EventBus 实现（在 main.rs 中调用一次）
pub fn set_event_bus(bus: impl EventBus + 'static) -> Result<(), String> {
    EVENT_BUS
        .set(Box::new(bus))
        .map_err(|_| "EventBus already initialized".to_string())
}

/// 获取 EventBus 实例
pub fn event_bus() -> Option<&'static dyn EventBus> {
    EVENT_BUS.get().map(|b| b.as_ref())
}

/// 便捷函数：发布事件
pub async fn publish_event(event: AppEvent) {
    if let Some(bus) = event_bus() {
        bus.publish(event).await;
    }
}
