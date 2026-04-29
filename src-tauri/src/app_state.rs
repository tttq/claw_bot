// Claw Desktop - 应用全局状态
// 管理Tauri应用的生命周期状态：配置、数据库、事件总线、WebSocket服务器句柄等
use std::sync::{Arc, Mutex};
use std::collections::HashMap;
use tokio::sync::Mutex as TokioMutex;
use tauri::{AppHandle, Emitter};

use async_trait::async_trait;
use serde_json::Value;
use claw_types::common::SubscriptionId;
use claw_traits::event_bus::{EventBus, EventHandler};

pub struct ClawAppState {
    pub config: Mutex<claw_config::config::AppConfig>,
    #[allow(dead_code)]
    pub event_bus: Arc<dyn EventBus>,
}

struct SubscriptionEntry {
    pattern: String,
    handler: Box<dyn EventHandler>,
}

struct TauriEventBus {
    handle: AppHandle,
    next_id: Mutex<u64>,
    subscriptions: TokioMutex<HashMap<SubscriptionId, SubscriptionEntry>>,
}

impl TauriEventBus {
    fn allocate_id(&self) -> SubscriptionId {
        let mut id = self.next_id.lock().expect("next_id lock poisoned");
        *id += 1;
        *id
    }

    fn event_matches_pattern(event_type: &str, pattern: &str) -> bool {
        if pattern == "*" { return true; }
        if pattern == event_type { return true; }
        if pattern.ends_with(".*") {
            let prefix = &pattern[..pattern.len() - 2];
            return event_type.starts_with(prefix) &&
                event_type[prefix.len()..].starts_with('.');
        }
        if pattern.ends_with(".>") {
            let prefix = &pattern[..pattern.len() - 2];
            return event_type.starts_with(prefix);
        }
        false
    }
}

#[async_trait]
impl EventBus for TauriEventBus {
    async fn publish(&self, event: claw_types::events::AppEvent) {
        let _ = self.handle.emit("event-bus", serde_json::to_value(&event).unwrap_or(Value::Null));

        let event_type = serde_json::to_value(&event)
            .ok()
            .and_then(|v| v.get("type").and_then(|t| t.as_str()).map(String::from))
            .unwrap_or_default();

        let matching_ids: Vec<SubscriptionId> = {
            let subs = self.subscriptions.lock().await;
            subs.iter()
                .filter(|(_id, entry)| Self::event_matches_pattern(&event_type, &entry.pattern))
                .map(|(id, _)| *id)
                .collect()
        };

        for id in &matching_ids {
            let subs = self.subscriptions.lock().await;
            if let Some(entry) = subs.get(id) {
                entry.handler.handle(&event).await;
            }
        }

        log::debug!("[EventBus] Published | type={}", event_type);
    }

    async fn subscribe(
        &self,
        event_pattern: &str,
        handler: Box<dyn EventHandler>,
    ) -> SubscriptionId {
        let id = self.allocate_id();
        let entry = SubscriptionEntry {
            pattern: event_pattern.to_string(),
            handler,
        };
        self.subscriptions.lock().await.insert(id, entry);
        log::info!("[EventBus] Subscribed | id={}, pattern={}", id, event_pattern);
        id
    }

    async fn unsubscribe(&self, id: SubscriptionId) {
        let removed = self.subscriptions.lock().await.remove(&id);
        if removed.is_some() {
            log::info!("[EventBus] Unsubscribed | id={}", id);
        } else {
            log::warn!("[EventBus] Unsubscribe failed: id={} not found", id);
        }
    }
}

impl ClawAppState {
    fn create_event_bus(handle: AppHandle) -> Arc<dyn EventBus> {
        Arc::new(TauriEventBus {
            handle,
            next_id: Mutex::new(0),
            subscriptions: TokioMutex::new(HashMap::new()),
        })
    }

    pub async fn new(app_handle: AppHandle, config: claw_config::config::AppConfig) -> Self {
        log::info!("[AppState] Initializing...");
        let event_bus = Self::create_event_bus(app_handle);
        log::info!("[AppState] Ready — config={}, event_bus=✅", config.model.default_model);
        Self { config: Mutex::new(config), event_bus }
    }

    pub fn get_config(&self) -> claw_config::config::AppConfig {
        self.config.lock().expect("AppConfig lock poisoned").clone()
    }

    #[allow(dead_code)]
    pub fn get_event_bus(&self) -> Arc<dyn EventBus> {
        self.event_bus.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use claw_config::config::AppConfig;

    #[test]
    fn test_event_matches_pattern() {
        assert!(TauriEventBus::event_matches_pattern("tool.executed", "*"));
        assert!(TauriEventBus::event_matches_pattern("tool.executed", "tool.executed"));
        assert!(TauriEventBus::event_matches_pattern("tool.executed", "tool.*"));
        assert!(!TauriEventBus::event_matches_pattern("tool.executed", "channel.*"));
        assert!(TauriEventBus::event_matches_pattern("tool.executed.completed", "tool.>"));
        assert!(!TauriEventBus::event_matches_pattern("tool.executed", "channel.>"));
    }

    #[test]
    fn test_config_thread_safety() {
        let config = AppConfig::default();
        let state = ClawAppState {
            config: Mutex::new(config.clone()),
            event_bus: create_mock_event_bus(),
        };

        let handle = std::thread::spawn(move || {
            let retrieved = state.get_config();
            assert_eq!(retrieved.model.default_model, config.model.default_model);
        });

        handle.join().expect("Thread panicked");
    }

    #[test]
    fn test_get_config_returns_clone() {
        let config = AppConfig::default();
        let state = ClawAppState {
            config: Mutex::new(config),
            event_bus: create_mock_event_bus(),
        };

        let config1 = state.get_config();
        let config2 = state.get_config();

        assert_eq!(config1.model.default_model, config2.model.default_model);
    }

    #[test]
    fn test_event_bus_clone_independence() {
        let state = ClawAppState {
            config: Mutex::new(AppConfig::default()),
            event_bus: create_mock_event_bus(),
        };

        let bus1 = state.get_event_bus();
        let bus2 = state.get_event_bus();

        assert!(Arc::ptr_eq(&bus1, &bus2), "EventBus clones should point to same instance");
    }

    fn create_mock_event_bus() -> Arc<dyn EventBus> {
        struct MockEventBus;

        #[async_trait]
        impl EventBus for MockEventBus {
            async fn publish(&self, _event: claw_types::events::AppEvent) {}
            async fn subscribe(
                &self,
                _pattern: &str,
                _handler: Box<dyn EventHandler>,
            ) -> SubscriptionId {
                0
            }
            async fn unsubscribe(&self, _id: SubscriptionId) {}
        }

        Arc::new(MockEventBus)
    }
}
