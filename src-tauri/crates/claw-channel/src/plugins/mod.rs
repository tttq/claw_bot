// Claw Desktop - 渠道插件模块入口 - Discord/Telegram/微信适配器
pub mod discord;
pub mod telegram;
pub mod weixin;

pub use discord::DiscordPlugin;
pub use telegram::TelegramPlugin;
pub use weixin::WeixinPlugin;
