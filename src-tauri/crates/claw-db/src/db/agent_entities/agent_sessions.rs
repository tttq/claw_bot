// Claw Desktop - Agent会话实体 - Agent会话ORM实体
use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "agent_sessions")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,
    pub agent_id: String,
    pub conversation_id: Option<String>,
    pub status: String,
    pub turn_count: i32,
    pub total_tokens_used: f64,
    pub started_at: i64,
    pub last_active: Option<i64>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
