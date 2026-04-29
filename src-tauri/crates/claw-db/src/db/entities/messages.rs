// Claw Desktop - 消息实体 - 消息表ORM实体
use sea_orm::entity::prelude::*;

/// 消息实体：存储对话中的每条消息（user/assistant/system/tool 角色）
/// 属于某个会话(conversation_id)，可选存 token_count 和向量嵌入
#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "messages")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,
    pub conversation_id: String,
    pub role: String,
    pub content: String,
    pub timestamp: i64,
    pub token_count: Option<i32>,
    pub embedding: Option<Vec<u8>>,
    pub is_error: i32,
    pub model: Option<String>,
    pub metadata: Option<String>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
