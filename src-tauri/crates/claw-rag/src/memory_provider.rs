// Claw Desktop - 记忆提供者Trait - 定义记忆存储和检索的统一接口
// 支持四层记忆架构：工作记忆/情景记忆/语义记忆/程序记忆
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::memory_layers::MemoryLayer;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryMetadata {
    pub agent_id: String,
    pub conversation_id: Option<String>,
    pub fact_type: String,
    pub source_type: String,
    pub context: Option<String>,
    pub tags: Option<String>,
    pub memory_layer: Option<MemoryLayer>,
    pub expires_at: Option<i64>,
}

impl Default for MemoryMetadata {
    fn default() -> Self {
        Self {
            agent_id: "default".to_string(),
            conversation_id: None,
            fact_type: "world".to_string(),
            source_type: "conversation".to_string(),
            context: None,
            tags: None,
            memory_layer: None,
            expires_at: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderCapabilities {
    pub semantic_search: bool,
    pub full_text_search: bool,
    pub entity_extraction: bool,
    pub cross_agent: bool,
    pub max_context_tokens: usize,
    pub layer_aware: bool,
    pub deduplication: bool,
    pub consolidation: bool,
}

impl Default for ProviderCapabilities {
    fn default() -> Self {
        Self {
            semantic_search: true,
            full_text_search: true,
            entity_extraction: true,
            cross_agent: false,
            max_context_tokens: 6000,
            layer_aware: false,
            deduplication: false,
            consolidation: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryUnit {
    pub id: String,
    pub text: String,
    pub fact_type: String,
    pub context: Option<String>,
    pub occurred_at: Option<i64>,
    pub source_type: String,
    pub tags: Option<String>,
    pub importance_score: f64,
    pub semantic_score: f64,
    pub bm25_score: f64,
    pub temporal_score: f64,
    pub final_score: f64,
    pub memory_layer: Option<MemoryLayer>,
    pub expires_at: Option<i64>,
    pub access_count: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LayerAwareRetrieveRequest {
    pub query: String,
    pub agent_id: String,
    pub conversation_id: Option<String>,
    pub layers: Option<Vec<MemoryLayer>>,
    pub limit: usize,
}

#[async_trait]
pub trait MemoryProvider: Send + Sync {
    async fn store(&self, metadata: &MemoryMetadata, text: &str) -> Result<String, String>;
    async fn retrieve(&self, agent_id: &str, query: &str, conversation_id: Option<&str>, limit: usize) -> Result<Vec<MemoryUnit>, String>;
    async fn delete(&self, memory_id: &str) -> Result<(), String>;
    async fn update(&self, memory_id: &str, text: &str) -> Result<(), String>;
    async fn build_context(&self, agent_id: &str, conversation_id: &str, query: &str) -> Result<String, String>;
    async fn store_interaction(&self, agent_id: &str, conversation_id: Option<&str>, user_msg: &str, assistant_msg: &str) -> Result<(), String>;

    fn name(&self) -> &str;
    fn capabilities(&self) -> ProviderCapabilities;

    async fn retrieve_by_layers(&self, request: &LayerAwareRetrieveRequest) -> Result<Vec<MemoryUnit>, String> {
        let _ = request;
        self.retrieve(&request.query, &request.agent_id, request.conversation_id.as_deref(), request.limit).await
    }

    async fn consolidate(&self, agent_id: &str) -> Result<ConsolidationStats, String> {
        let _ = agent_id;
        Ok(ConsolidationStats::default())
    }

    async fn cleanup_expired(&self, agent_id: &str) -> Result<u64, String> {
        let _ = agent_id;
        Ok(0)
    }

    async fn get_layer_stats(&self, agent_id: &str) -> Result<Vec<LayerStatEntry>, String> {
        let _ = agent_id;
        Ok(Vec::new())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ConsolidationStats {
    pub promoted: usize,
    pub merged: usize,
    pub forgotten: usize,
    pub protected: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LayerStatEntry {
    pub layer: MemoryLayer,
    pub count: usize,
    pub avg_importance: f64,
}

pub struct MemoryManager {
    provider: Arc<dyn MemoryProvider>,
}

impl MemoryManager {
    pub fn new(provider: Arc<dyn MemoryProvider>) -> Self {
        Self { provider }
    }

    pub async fn build_memory_context(&self, agent_id: &str, conversation_id: &str, query: &str) -> Result<String, String> {
        self.provider.build_context(agent_id, conversation_id, query).await
    }

    pub async fn store_interaction(&self, agent_id: &str, conversation_id: Option<&str>, user_msg: &str, assistant_msg: &str) -> Result<(), String> {
        self.provider.store_interaction(agent_id, conversation_id, user_msg, assistant_msg).await
    }

    pub async fn store(&self, metadata: &MemoryMetadata, text: &str) -> Result<String, String> {
        self.provider.store(metadata, text).await
    }

    pub async fn retrieve(&self, agent_id: &str, query: &str, conversation_id: Option<&str>, limit: usize) -> Result<Vec<MemoryUnit>, String> {
        self.provider.retrieve(agent_id, query, conversation_id, limit).await
    }

    pub async fn retrieve_by_layers(&self, request: &LayerAwareRetrieveRequest) -> Result<Vec<MemoryUnit>, String> {
        self.provider.retrieve_by_layers(request).await
    }

    pub async fn consolidate(&self, agent_id: &str) -> Result<ConsolidationStats, String> {
        self.provider.consolidate(agent_id).await
    }

    pub async fn cleanup_expired(&self, agent_id: &str) -> Result<u64, String> {
        self.provider.cleanup_expired(agent_id).await
    }

    pub async fn get_layer_stats(&self, agent_id: &str) -> Result<Vec<LayerStatEntry>, String> {
        self.provider.get_layer_stats(agent_id).await
    }

    pub fn provider_name(&self) -> &str {
        self.provider.name()
    }
}
