// Claw Desktop - 实体共现 - 实体共现统计ORM实体
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[allow(dead_code)]
#[sea_orm(table_name = "entity_cooccurrences")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub entity_id_1: String,
    #[sea_orm(primary_key)]
    pub entity_id_2: String,
    pub cooccurrence_count: i32,
    pub last_cooccurred: i64,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
#[allow(dead_code)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
