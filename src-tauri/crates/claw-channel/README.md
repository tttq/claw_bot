# Claw Channel - 多渠道消息网关

> **定位**：统一的消息通道抽象层，支持多平台消息收发（Telegram、Discord等）

## 📌 核心作用

`claw-channel` 是项目的 **消息通道适配层**，负责：

1. **多平台接入** - 统一封装 Telegram、Discord、Slack 等聊天平台的API
2. **消息标准化** - 将不同平台的消息格式统一为 `InboundMessage` / `OutboundMessage`
3. **入站管道** - 消息接收、解析、加密解密、预处理流水线
4. **流式输出** - 支持流式响应、消息编辑等高级功能
5. **插件化架构** - 通过 Trait 实现新渠道的快速扩展

## 🏗️ 架构概览

```
┌─────────────────────────────────────────────────────────────┐
│                    外部聊天平台                               │
│  ┌──────────┐  ┌──────────┐  ┌──────────┐  ┌──────────┐   │
│  │ Telegram │  │ Discord  │  │  Slack   │  │ Custom   │   │
│  └────┬─────┘  └────┬─────┘  └────┬─────┘  └────┬─────┘   │
└───────┼─────────────┼─────────────┼─────────────┼──────────┘
        │             │             │             │
        ▼             ▼             ▼             ▼
┌─────────────────────────────────────────────────────────────┐
│                    Channel Registry                          │
│                  (渠道注册中心)                              │
└──────────────────────────┬──────────────────────────────────┘
                           │
                           ▼
┌─────────────────────────────────────────────────────────────┐
│                   InboundPipeline                           │
│              (入站处理管道)                                  │
│  接收 → 解密 → 解析 → 标准化 → ProcessedMessage            │
└──────────────────────────┬──────────────────────────────────┘
                           │
                           ▼
┌─────────────────────────────────────────────────────────────┐
│                    Claw Core / LLM                          │
│                   (核心处理层)                                │
└──────────────────────────┬──────────────────────────────────┘
                           │
                           ▼
┌─────────────────────────────────────────────────────────────┐
│                   Outbound (发送)                            │
│  OutboundMessage → 格式化 → 加密 → 平台API发送              │
└─────────────────────────────────────────────────────────────┘
```

## 📦 模块结构

| 模块 | 功能 | 关键依赖 |
|-----|------|---------|
| `types.rs` | 所有类型定义（ChannelId, MessageContent, InboundMessage, OutboundMessage等） | - |
| `traits.rs` | 渠道Trait定义（ChannelProvider） | `async-trait` |
| `config.rs` | 渠道配置管理 | - |
| `registry.rs` | 渠道注册中心，管理所有已注册的渠道实例 | - |
| `inbound.rs` | 入站消息处理管道 | - |
| `streaming.rs` | 流式响应支持（Partial/Block模式） | - |
| `encryption.rs` | 敏感信息加密存储（Token等） | `sha2`, `aes-gcm` |
| `bootstrap.rs` | 初始化和启动逻辑 | - |
| `error.rs` | 错误类型定义 | `thiserror` |
| `plugins/` | 具体平台实现 | - |
| `plugins/mod.rs` | 插件模块入口 | - |
| `plugins/telegram.rs` | Telegram Bot API 实现 | `teloxide` |
| `plugins/discord.rs` | Discord Bot API 实现 | `reqwest` |

## 🔧 核心类型

### 渠道标识

```rust
pub enum ChannelId {
    Telegram,
    Discord,
    Slack,
    WhatsApp,
    Signal,
    Custom(String),  // 支持自定义渠道
}
```

### 消息类型

```rust
// 入站消息（从平台收到）
pub struct InboundMessage {
    pub message_id: String,
    pub channel_id: ChannelId,       // 来源渠道
    pub account_id: String,          // 账号ID（支持多账号）
    pub sender_id: String,
    pub sender_name: Option<String>,
    pub chat_id: String,
    pub chat_type: ChatType,         // Direct/Group/Channel/Thread
    pub content: MessageContent,     // Text/Media/Poll
    pub timestamp: DateTime<Utc>,
    // ...更多字段
}

// 出站消息（发送到平台）
pub struct OutboundMessage {
    pub channel_id: ChannelId,
    pub account_id: String,
    pub target_id: String,
    pub target_chat_type: ChatType,
    pub content: MessageContent,
    pub options: OutboundOptions,    // silent, parse_mode, preview_url
}

// 消息内容
pub enum MessageContent {
    Text { text: String },
    Media { url: String, mime_type: String, caption: Option<String> },
    Poll { question: String, options: Vec<String> },
}
```

### 渠道能力声明

```rust
pub struct ChannelCapabilities {
    pub chat_types: Vec<ChatType>,       // 支持的聊天类型
    pub supports_polls: bool,            // 是否支持投票
    pub supports_reactions: bool,        // 是否支持表情反应
    pub supports_edit: bool,             // 是否支持编辑消息
    pub supports_unsend: bool,           // 是否支持撤回消息
    pub supports_media: bool,            // 是否支持媒体消息
    pub supports_threads: bool,          // 是否支持话题线程
    pub supports_streaming: bool,        // 是否支持流式输出
    pub max_message_length: Option<usize>,
    pub supported_parse_modes: Vec<ParseMode>,  // Markdown/Html/PlainText
}
```

### 流式传输配置

```rust
pub enum StreamingMode {
    Off,       // 关闭流式
    Partial,   // 部分更新（编辑消息模拟打字效果）
    Block,     // 分块发送
}

pub struct StreamingConfig {
    pub enabled: bool,
    pub mode: StreamingMode,
    pub chunk_size: Option<usize>,
    pub edit_delay_ms: Option<u64>,  // 编辑延迟(ms)
}
```

## 🔌 插件接口

```rust
#[async_trait]
pub trait ChannelProvider: Send + Sync {
    async fn start(&self) -> ChannelResult<()>;
    async fn stop(&self) -> ChannelResult<()>;
    async fn send_message(&self, msg: &OutboundMessage) -> ChannelResult<SendResult>;
    async fn get_status(&self) -> ChannelResult<ChannelStatus>;
    fn capabilities(&self) -> ChannelCapabilities;
    fn meta(&self) -> ChannelMeta;
}
```

## 🚀 使用方式

### 注册和使用渠道

```rust
use claw_channel::{ChannelRegistry, ChannelId, InboundMessage, OutboundMessage};

// 创建注册中心
let registry = ChannelRegistry::new();

// 注册Telegram渠道
registry.register(ChannelId::Telegram, telegram_provider).await;

// 发送消息
let outbound = OutboundMessage::new(
    ChannelId::Telegram,
    "account_123",
    "chat_456",
    ChatType::Direct,
    MessageContent::Text { text: "Hello!".to_string() },
);
let result = registry.send_message(&outbound).await?;
```

### 入站处理管道

```rust
use claw_channel::InboundPipeline;

let pipeline = InboundPipeline::new(registry, encryption_service);

// 处理入站消息
let processed = pipeline.process(inbound_message).await?;
// → 自动完成：解密 → 解析 → 标准化 → 返回 ProcessedMessage
```

## 📊 数据流

### 消息接收流程

```
平台推送 (Telegram/Discord/Webhook)
         ↓
[1] plugins/{channel}.rs      // 平台SDK接收原始消息
         ↓
[2] inbound::InboundPipeline  // 入站管道处理
    ├── encryption::decrypt() // 解密敏感字段(如有)
    ├── 类型转换               // 平台格式 → InboundMessage
    └── metadata enrichment   // 元数据丰富
         ↓
[3] registry::route()        // 路由到处理器
         ↓
[4] Claw Core / LLM          // 业务逻辑处理
```

### 消息发送流程

```
业务层生成回复内容
         ↓
[1] 构建 OutboundMessage     // 标准化出站消息
         ↓
[2] streaming模块            // 流式处理(如启用)
    ├── StreamingMode::Partial → 分段编辑消息
    └── StreamingMode::Block  → 分块发送
         ↓
[3] encryption::encrypt()    // 加密敏感字段(如有)
         ↓
[4] plugins/{channel}.rs     // 调用平台API发送
         ↓
[5] 返回 SendResult          // { success, message_id, error }
```

## 🔗 依赖关系

```
claw-channel
├── claw-core                # 配置、数据库、LLM等基础能力
├── teloxide                 # Telegram Bot SDK
├── reqwest                  # HTTP客户端(Discord等)
├── sea-orm                  # 数据库ORM(会话/消息日志)
├── sha2 + aes-gcm           # 加密(Token保护)
└── tokio + async-trait      # 异步运行时
```

## 🎯 支持的平台

| 平台 | 状态 | 特性 |
|-----|------|------|
| **Telegram** | ✅ 已实现 | 文本/媒体/投票/内联按钮/流式 |
| **Discord** | ✅ 已实现 | 文本/嵌入/附件/反应/线程 |
| **Slack** | 🔄 接口就绪 | Web API / Socket Mode |
| **WhatsApp** | 🔄 接口就绪 | Cloud API / Business API |
| **Signal** | 🔄 接口就绪 | Signal Protocol |
| **Custom** | ✅ 支持 | 通过 ChannelId::Custom 扩展 |

## 📁 文件清单

```
src/
├── lib.rs                    # 主入口，模块导出
├── types.rs                  # 所有类型定义 (ChannelId, Message, Config等)
├── traits.rs                 # ChannelProvider trait 定义
├── config.rs                 # 渠道配置
├── registry.rs               # 渠道注册中心
├── inbound.rs                # 入站消息处理管道
├── streaming.rs              # 流式响应支持
├── encryption.rs             # 敏感数据加密服务
├── bootstrap.rs              # 初始化引导
├── error.rs                  # 错误类型
│
└── plugins/
    ├── mod.rs                # 插件模块入口
    ├── telegram.rs           # Telegram Bot 实现 (teloxide)
    └── discord.rs            # Discord Bot 实现 (reqwest)
```

## 💡 设计特点

1. **统一抽象** - 不同平台的消息通过统一的 `InboundMessage` / `OutboundMessage` 类型交互
2. **插件化** - 新增渠道只需实现 `ChannelProvider` trait 并注册到 `ChannelRegistry`
3. **安全** - Token等敏感信息通过 AES-GCM 加密存储
4. **流式支持** - 支持多种流式输出模式，提升用户体验
5. **多账号** - 每个渠道支持多个独立账号实例
6. **能力声明** - 每个渠道声明自身能力，上层可根据能力做兼容处理
