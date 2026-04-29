// Claw Desktop - 单元实体关联 - 记忆单元与实体关联ORM实体
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "unit_entities")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub unit_id: String,
    #[sea_orm(primary_key)]
    pub entity_id: String,
    pub role: Option<String>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
