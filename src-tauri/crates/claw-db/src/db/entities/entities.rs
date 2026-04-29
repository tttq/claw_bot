// Claw Desktop - 实体表 - RAG实体ORM实体
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "entities")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,
    pub agent_id: String,
    #[sea_orm(column_type = "Text")]
    pub canonical_name: String,
    pub entity_type: String,
    pub metadata: Option<String>,
    pub first_seen: i64,
    pub last_seen: i64,
    pub mention_count: i32,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
