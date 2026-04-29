// Claw Desktop - 记忆单元实体 - RAG记忆存储ORM实体
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "memory_units")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,
    pub agent_id: String,
    pub conversation_id: Option<String>,
    #[sea_orm(column_type = "Text")]
    pub text: String,
    #[sea_orm(column_type = "Blob")]
    pub embedding: Vec<u8>,
    pub fact_type: String,
    pub context: Option<String>,
    pub occurred_at: Option<i64>,
    pub mentioned_at: Option<i64>,
    pub source_type: String,
    pub metadata: Option<String>,
    pub tags: Option<String>,
    pub importance_score: f64,
    pub access_count: i32,
    pub memory_layer: Option<String>,
    pub expires_at: Option<i64>,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
