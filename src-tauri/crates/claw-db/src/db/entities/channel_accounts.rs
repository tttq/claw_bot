// Claw Desktop - 渠道账号实体 - 渠道账号ORM实体
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "channel_accounts")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,

    pub channel_type: String,

    pub name: String,

    pub enabled: bool,

    pub config_json: serde_json::Value,

    pub encrypted_fields: Option<String>,

    pub dm_policy: Option<String>,

    pub group_policy: Option<String>,

    pub streaming_config: Option<String>,

    pub status: String,

    pub last_error: Option<String>,

    pub last_connected_at: Option<chrono::NaiveDateTime>,

    pub created_at: chrono::NaiveDateTime,

    pub updated_at: chrono::NaiveDateTime,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
