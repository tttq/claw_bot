// Claw Desktop - 渠道消息日志 - 渠道消息记录ORM实体
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "channel_message_log")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,

    pub message_id: String,

    pub channel_account_id: String,

    pub direction: String,

    pub content_summary: Option<String>,

    pub full_content: Option<String>,

    pub sender_id: Option<String>,

    pub target_id: Option<String>,

    pub chat_type: Option<String>,

    pub metadata: Option<serde_json::Value>,

    pub timestamp: chrono::NaiveDateTime,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
