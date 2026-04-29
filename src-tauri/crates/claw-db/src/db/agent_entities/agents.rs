// Claw Desktop - Agent实体 - Agent表ORM实体
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "agents")]
#[serde(rename_all = "camelCase")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,
    pub display_name: String,
    pub description: Option<String>,
    pub purpose: Option<String>,
    pub scope: Option<String>,
    pub model_override: Option<String>,
    pub system_prompt: Option<String>,
    pub tools_config: Option<String>,
    pub skills_enabled: Option<String>,
    pub max_turns: i32,
    pub temperature: Option<f64>,
    pub workspace_path: Option<String>,
    pub is_active: i32,
    pub created_at: i64,
    pub updated_at: i64,
    pub conversation_count: i64,
    pub total_messages: i64,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
