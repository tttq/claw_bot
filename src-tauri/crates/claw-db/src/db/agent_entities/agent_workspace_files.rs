// Claw Desktop - Agent工作区文件实体 - 工作区文件ORM实体
use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "agent_workspace_files")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = true)]
    pub id: i32,
    pub agent_id: String,
    pub session_id: Option<String>,
    pub relative_path: String,
    pub full_path: String,
    pub file_size: i64,
    pub content_type: Option<String>,
    pub indexed_at: i64,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
