// Claw Desktop - WS协议 - 请求/响应/事件消息类型定义
use serde::{Deserialize, Serialize};

/// WebSocket请求消息结构
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct WsRequest {
    pub id: String,                          // 请求唯一ID
    #[serde(rename = "type")]
    pub msg_type: String,                    // 消息类型 (request/stream)
    pub method: String,                      // 调用方法名
    #[serde(default)]
    pub params: serde_json::Value,           // 请求参数
    #[serde(default)]
    pub token: String,                       // 认证Token
}

/// WebSocket响应消息结构
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct WsResponse {
    pub id: String,                          // 对应请求的ID
    #[serde(rename = "type")]
    pub msg_type: String,                    // 消息类型 (response)
    pub method: String,                      // 调用方法名
    pub success: bool,                       // 是否成功
    #[serde(default)]
    pub data: serde_json::Value,             // 响应数据
    #[serde(default)]
    pub error: String,                       // 错误信息
}

impl WsResponse {
    /// 创建成功响应
    pub fn ok(id: &str, method: &str, data: serde_json::Value) -> Self {
        Self {
            id: id.to_string(),
            msg_type: "response".to_string(),
            method: method.to_string(),
            success: true,
            data,
            error: String::new(),
        }
    }

    /// 创建错误响应
    pub fn err(id: &str, method: &str, error: &str) -> Self {
        Self {
            id: id.to_string(),
            msg_type: "response".to_string(),
            method: method.to_string(),
            success: false,
            data: serde_json::Value::Null,
            error: error.to_string(),
        }
    }
}

/// WebSocket流式事件消息结构
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WsEvent {
    pub id: String,                          // 事件ID
    #[serde(rename = "type")]
    pub msg_type: String,                    // 消息类型 (stream)
    pub method: String,                      // 关联的方法名
    pub event: String,                       // 事件名称
    #[serde(default)]
    pub data: serde_json::Value,             // 事件数据
}

impl WsEvent {
    /// 创建新的流式事件
    pub fn new(id: &str, method: &str, event: &str, data: serde_json::Value) -> Self {
        Self {
            id: id.to_string(),
            msg_type: "stream".to_string(),
            method: method.to_string(),
            event: event.to_string(),
            data,
        }
    }
}
