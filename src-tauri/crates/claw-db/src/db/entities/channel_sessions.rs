// Claw Desktop - 渠道会话实体 - 渠道会话ORM实体
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "channel_sessions")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,

    pub internal_conversation_id: String,

    pub external_chat_id: String,

    pub channel_account_id: String,

    pub chat_type: String,

    pub thread_id: Option<String>,

    pub title: Option<String>,

    pub metadata: Option<serde_json::Value>,

    pub created_at: chrono::NaiveDateTime,

    pub last_active_at: chrono::NaiveDateTime,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
