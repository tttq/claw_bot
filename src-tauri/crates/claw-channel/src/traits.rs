// Claw Desktop - 渠道Trait - 定义渠道适配器的统一接口
use async_trait::async_trait;
use crate::error::ChannelResult;
use crate::types::*;
use crate::config::ChannelAccountConfig;

// ====== Channel Plugin 核心 Trait ======

/// 渠道插件Trait - 所有渠道适配器必须实现此接口
/// 定义了渠道的生命周期：初始化→启动→运行→停止
#[async_trait]
pub trait ChannelPlugin: Send + Sync {
    /// 获取渠道元数据（名称、描述、版本等）
    fn meta(&self) -> &ChannelMeta;

    /// 获取渠道能力声明（支持的聊天类型、功能等）
    fn capabilities(&self) -> &ChannelCapabilities;

    /// 初始化渠道插件（加载配置、建立连接）
    async fn initialize(&self, account_config: &ChannelAccountConfig) -> ChannelResult<()>;

    /// 启动指定账号的消息监听
    async fn start(&self, account_id: &str) -> ChannelResult<()>;

    /// 停止指定账号的消息监听
    async fn stop(&self, account_id: &str) -> ChannelResult<()>;

    /// 获取指定账号的连接状态
    async fn status(&self, account_id: &str) -> ChannelResult<ChannelStatus>;
}

// ====== 入站消息处理 Trait ======

/// 入站消息处理器Trait - 处理从外部渠道接收到的消息
#[async_trait]
pub trait InboundHandler: Send + Sync {
    /// 处理入站消息
    async fn handle_inbound(&self, message: InboundMessage) -> ChannelResult<()>;
}

// ====== 出站消息发送 Trait ======

/// 出站消息发送器Trait - 向外部渠道发送消息
#[async_trait]
pub trait OutboundSender: Send + Sync {
    /// 发送文本消息
    async fn send_text(&self, msg: &OutboundMessage) -> ChannelResult<SendResult>;

    /// 发送媒体消息
    async fn send_media(&self, msg: &OutboundMessage) -> ChannelResult<SendResult>;

    /// 流式发送文本（边生成边发送，模拟打字效果）
    async fn stream_text(
        &self,
        msg: &OutboundMessage,
        on_token: Box<dyn Fn(String) + Send + Sync>,
    ) -> ChannelResult<SendResult>;
}

// ====== 配置验证 Trait ======

/// 配置验证器Trait - 验证渠道配置的合法性
#[async_trait]
pub trait ConfigValidator: Send + Sync {
    /// 验证配置值是否合法
    async fn validate_config(&self, config: &serde_json::Value) -> ChannelResult<()>;

    /// 获取配置字段的元数据（用于前端动态生成配置表单）
    fn config_schema(&self) -> Vec<ConfigFieldMeta>;
}
