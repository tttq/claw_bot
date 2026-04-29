// Claw Desktop - Agent画像实体 - Agent画像ORM实体
use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "agent_profiles")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub agent_id: String,
    pub profile_json: String,
    pub interaction_count: i64,
    pub last_updated_at: i64,
    pub created_at: i64,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
