// Claw Desktop - RAG记忆库 - 提供记忆存储、检索、压缩等RAG能力
// 功能：文本向量化、多路融合检索、实体提取、记忆管理、压缩
// 四层架构：工作记忆(短期) → 情景记忆(中期) → 语义记忆(长期) → 程序记忆(核心)
// ✅ Phase 2 物理迁移完成 — 从 claw-core/src/rag/ 迁移至此

pub mod builtin_provider;
pub mod local_embedder;
pub mod memory_layers;
pub mod memory_pipeline;
pub mod memory_provider;
pub mod rag;
