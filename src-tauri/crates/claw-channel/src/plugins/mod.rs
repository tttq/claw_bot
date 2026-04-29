// Claw Desktop - 渠道插件模块入口 - Discord/Telegram/微信适配器
pub mod telegram;
pub mod discord;
pub mod weixin;

pub use telegram::TelegramPlugin;
pub use discord::DiscordPlugin;
pub use weixin::WeixinPlugin;
