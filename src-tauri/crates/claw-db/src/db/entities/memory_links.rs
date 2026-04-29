// Claw Desktop - 记忆链接实体 - 记忆间关联ORM实体
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[allow(dead_code)]
#[sea_orm(table_name = "memory_links")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,
    pub from_unit_id: String,
    pub to_unit_id: String,
    pub link_type: String,
    pub weight: f64,
    pub created_at: i64,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
#[allow(dead_code)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
