// Claw Desktop - Agent向量实体 - Agent向量ORM实体
use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "agent_vectors")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = true)]
    pub id: i32,
    pub agent_id: String,
    pub chunk_index: i32,
    pub content_hash: String,
    pub vector: Vec<u8>,
    pub source_type: Option<String>,
    pub created_at: i64,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
