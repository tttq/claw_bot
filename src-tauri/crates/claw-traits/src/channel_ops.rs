// Claw Desktop - 渠道操作Trait - 定义渠道消息收发的统一接口
use async_trait::async_trait;
use serde_json::Value;
use std::sync::OnceLock;

/// Channel 操作抽象接口
/// 实现位于 claw-channel，注入到全局
#[async_trait]
pub trait ChannelOperations: Send + Sync {
    /// 发送消息到指定频道/用户
    async fn send_message(
        &self,
        account_id: &str,
        target_id: &str,
        chat_type: &str,
        text: &str,
    ) -> Result<SendResult, String>;

    /// 列出所有已配置的 Channel 账户
    async fn list_accounts(&self) -> Result<Value, String>;

    /// 获取账户状态
    async fn get_account_status(&self, account_id: &str) -> Result<Value, String>;

    /// 创建新账户配置
    async fn create_account(&self, config: Value) -> Result<String, String>;

    /// 测试连接
    async fn test_connection(&self, account_id: &str) -> Result<bool, String>;
}

/// 消息发送结果
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SendResult {
    pub success: bool,              // 是否发送成功
    pub message_id: Option<String>, // 消息ID
    pub error: Option<String>,      // 错误信息
}

// ===== 全局注入点 =====
static CHANNEL_OPS: OnceLock<Box<dyn ChannelOperations>> = OnceLock::new();

/// 注入ChannelOperations实现（在应用启动时调用一次）
pub fn set_channel_ops(ops: impl ChannelOperations + 'static) -> Result<(), String> {
    CHANNEL_OPS
        .set(Box::new(ops))
        .map_err(|_| "ChannelOperations already initialized".to_string())
}

/// 获取全局ChannelOperations实例
pub fn channel_ops() -> Option<&'static dyn ChannelOperations> {
    CHANNEL_OPS.get().map(|o| o.as_ref())
}
