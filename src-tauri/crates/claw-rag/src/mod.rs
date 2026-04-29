// Claw Desktop - RAG模块入口
// Claw Core - 检索增强生成记忆系统 v2 (Hindsight-inspired)
// 功能：文本向量化、多路融合检索、实体提取、记忆管理、压缩
// 向量化：LocalEmbedder (ONNX 384维) > Feature Hashing Fallback (128维)

pub mod rag;
pub mod local_embedder;
pub mod memory_provider;
pub mod builtin_provider;

pub use rag::*;
pub use local_embedder::*;
pub use memory_provider::*;
pub use builtin_provider::*;
