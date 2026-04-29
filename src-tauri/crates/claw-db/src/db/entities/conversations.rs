// Claw Desktop - 会话实体 - 会话表ORM实体
use sea_orm::entity::prelude::*;

/// 会话实体：存储每个对话会话的元数据
/// 主键 id 为 UUID 字符串，与前端 conversation_id 一一对应
#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "conversations")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,
    pub title: String,
    pub created_at: i64,
    pub updated_at: i64,
    pub message_count: i64,
    pub metadata: Option<String>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
