// Claw Desktop - 可观测性 - 调用统计、事件流、延迟监控
use crate::harness::types::{HarnessEvent, HarnessEventType};
use std::sync::Arc;
use tokio::sync::RwLock;

/// 事件缓冲区最大容量
const MAX_OBSERVABILITY_EVENTS: usize = 1000;

/// 可观测性引擎 — 调用统计、事件流、延迟监控
///
/// 维护一个有界事件环形缓冲区(max_events=MAX_OBSERVABILITY_EVENTS)，支持按Agent和事件类型过滤
pub struct ObservabilityEngine {
    events: Arc<RwLock<Vec<HarnessEvent>>>,
    max_events: usize,
}

impl ObservabilityEngine {
    /// 创建新的可观测性引擎
    pub fn new() -> Self {
        Self {
            events: Arc::new(RwLock::new(Vec::new())),
            max_events: MAX_OBSERVABILITY_EVENTS,
        }
    }

    /// 记录事件 — 将事件加入缓冲区，超过上限时淘汰1/4旧事件
    pub async fn record_event(&self, event: HarnessEvent) {
        log::info!(
            "[Observability:record_event] type={:?} agent={} correlation={:?}",
            event.event_type,
            event.agent_id,
            event.correlation_id
        );

        let mut events = self.events.write().await;
        if events.len() >= self.max_events {
            let drain_count = events.len() / 4;
            events.drain(0..drain_count);
        }
        events.push(event);
    }

    /// 记录简单事件 — 无持续时间的便捷记录方法
    pub async fn record_simple(
        &self,
        event_type: HarnessEventType,
        agent_id: &str,
        correlation_id: Option<&str>,
        payload: Option<&str>,
    ) {
        let event = HarnessEvent {
            id: uuid::Uuid::new_v4().to_string(),
            event_type,
            agent_id: agent_id.to_string(),
            correlation_id: correlation_id.map(|s| s.to_string()),
            payload: payload.map(|s| s.to_string()),
            timestamp: chrono::Utc::now().timestamp(),
            duration_ms: None,
        };
        self.record_event(event).await;
    }

    /// 记录带持续时间的事件 — 用于性能监控
    pub async fn record_with_duration(
        &self,
        event_type: HarnessEventType,
        agent_id: &str,
        correlation_id: Option<&str>,
        payload: Option<&str>,
        duration_ms: u64,
    ) {
        let event = HarnessEvent {
            id: uuid::Uuid::new_v4().to_string(),
            event_type,
            agent_id: agent_id.to_string(),
            correlation_id: correlation_id.map(|s| s.to_string()),
            payload: payload.map(|s| s.to_string()),
            timestamp: chrono::Utc::now().timestamp(),
            duration_ms: Some(duration_ms),
        };
        self.record_event(event).await;
    }

    /// 获取事件列表 — 支持按Agent和事件类型过滤，按时间倒序排列
    pub async fn get_events(
        &self,
        agent_id: Option<&str>,
        event_type: Option<&HarnessEventType>,
        limit: usize,
    ) -> Vec<HarnessEvent> {
        let events = self.events.read().await;
        let mut filtered: Vec<&HarnessEvent> = events
            .iter()
            .filter(|e| {
                if let Some(aid) = agent_id {
                    if e.agent_id != aid {
                        return false;
                    }
                }
                if let Some(et) = event_type {
                    if &e.event_type != et {
                        return false;
                    }
                }
                true
            })
            .collect();

        filtered.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
        filtered.into_iter().take(limit).cloned().collect()
    }

    /// 获取Agent统计 — 返回错误数、完成数、失败数、跨记忆访问数、验证失败数、成功率
    pub async fn get_agent_stats(&self, agent_id: &str) -> serde_json::Value {
        let events = self.events.read().await;
        let agent_events: Vec<&HarnessEvent> = events
            .iter()
            .filter(|e| e.agent_id == agent_id)
            .collect();

        let total = agent_events.len();
        let errors = agent_events
            .iter()
            .filter(|e| matches!(e.event_type, HarnessEventType::ErrorOccurred))
            .count();
        let tasks_completed = agent_events
            .iter()
            .filter(|e| matches!(e.event_type, HarnessEventType::TaskCompleted))
            .count();
        let tasks_failed = agent_events
            .iter()
            .filter(|e| matches!(e.event_type, HarnessEventType::TaskFailed))
            .count();
        let cross_memory_accesses = agent_events
            .iter()
            .filter(|e| matches!(e.event_type, HarnessEventType::CrossMemoryAccessed))
            .count();
        let validation_failures = agent_events
            .iter()
            .filter(|e| matches!(e.event_type, HarnessEventType::ValidationFailed))
            .count();

        serde_json::json!({
            "agent_id": agent_id,
            "total_events": total,
            "errors": errors,
            "tasks_completed": tasks_completed,
            "tasks_failed": tasks_failed,
            "cross_memory_accesses": cross_memory_accesses,
            "validation_failures": validation_failures,
            "success_rate": if tasks_completed + tasks_failed > 0 {
                tasks_completed as f64 / (tasks_completed + tasks_failed) as f64
            } else {
                0.0
            }
        })
    }
}

impl Default for ObservabilityEngine {
    fn default() -> Self {
        Self::new()
    }
}
