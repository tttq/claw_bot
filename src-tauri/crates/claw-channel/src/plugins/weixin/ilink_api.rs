// Claw Desktop - iLink API - iLink平台接口调用
use serde::{Deserialize, Serialize};

const ILINK_BASE_URL: &str = "https://ilinkai.weixin.qq.com";
const ILINK_APP_ID: &str = "bot";
const CHANNEL_VERSION: &str = "2.2.0";

const EP_GET_UPDATES: &str = "ilink/bot/getupdates";
const EP_SEND_MESSAGE: &str = "ilink/bot/sendmessage";
const EP_SEND_TYPING: &str = "ilink/bot/sendtyping";
const EP_GET_CONFIG: &str = "ilink/bot/getconfig";
const EP_GET_UPLOAD_URL: &str = "ilink/bot/getuploadurl";
const EP_GET_BOT_QR: &str = "ilink/bot/get_bot_qrcode";
const EP_GET_QR_STATUS: &str = "ilink/bot/get_qrcode_status";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ILinkConfig {
    pub token: String,
    pub account_id: String,
    pub base_url: String,
    pub cdn_base_url: String,
}

impl Default for ILinkConfig {
    fn default() -> Self {
        Self {
            token: String::new(),
            account_id: String::new(),
            base_url: ILINK_BASE_URL.to_string(),
            cdn_base_url: "https://novac2c.cdn.weixin.qq.com/c2c".to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ILinkUpdate {
    pub msg_id: Option<String>,
    pub sender_id: Option<String>,
    pub chat_id: Option<String>,
    pub chat_type: Option<String>,
    pub content: Option<String>,
    pub context_token: Option<String>,
    pub item_list: Option<Vec<serde_json::Value>>,
    pub aes_key: Option<String>,
    pub encrypted_query_param: Option<String>,
    pub full_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ILinkQRCode {
    pub qrcode: Option<String>,
    pub qrcode_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ILinkQRStatus {
    pub status: Option<String>,
    pub ilink_bot_id: Option<String>,
    pub bot_token: Option<String>,
    pub baseurl: Option<String>,
    pub ilink_user_id: Option<String>,
}

pub struct ILinkClient {
    config: ILinkConfig,
    http: reqwest::Client,
}

impl ILinkClient {
    pub fn new(config: ILinkConfig) -> Self {
        let http = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(40))
            .build()
            .unwrap_or_default();
        Self { config, http }
    }

    fn build_headers(&self) -> reqwest::header::HeaderMap {
        let mut headers = reqwest::header::HeaderMap::new();
        headers.insert("Content-Type", "application/json".parse().unwrap());
        headers.insert("AuthorizationType", "ilink_bot_token".parse().unwrap());
        if !self.config.token.is_empty() {
            headers.insert("Authorization", format!("Bearer {}", self.config.token).parse().unwrap());
        }
        headers.insert("iLink-App-Id", ILINK_APP_ID.parse().unwrap());
        let client_version = (2 << 16) | (2 << 8) | 0;
        headers.insert("iLink-App-ClientVersion", client_version.to_string().parse().unwrap());
        headers
    }

    fn url(&self, endpoint: &str) -> String {
        format!("{}/{}", self.config.base_url.trim_end_matches('/'), endpoint.trim_start_matches('/'))
    }

    pub async fn get_updates(&self, sync_buf: &str, timeout_secs: u64) -> Result<(Vec<ILinkUpdate>, String), String> {
        let body = serde_json::json!({
            "base_info": {"channel_version": CHANNEL_VERSION},
            "get_updates_buf": sync_buf,
            "timeout": timeout_secs,
        });

        let resp = self.http
            .post(self.url(EP_GET_UPDATES))
            .headers(self.build_headers())
            .json(&body)
            .send()
            .await
            .map_err(|e| format!("get_updates request failed: {}", e))?;

        let data: serde_json::Value = resp.json().await.map_err(|e| format!("get_updates parse failed: {}", e))?;

        let errcode = data.get("errcode").and_then(|v| v.as_i64()).unwrap_or(0);
        if errcode != 0 {
            return Err(format!("iLink API error: errcode={}, msg={}", errcode,
                data.get("errmsg").and_then(|v| v.as_str()).unwrap_or("unknown")));
        }

        let new_buf = data.get("get_updates_buf")
            .and_then(|v| v.as_str())
            .unwrap_or(sync_buf)
            .to_string();

        let msgs = data.get("msgs")
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default();

        let updates: Vec<ILinkUpdate> = msgs.iter().filter_map(|m| {
            Some(ILinkUpdate {
                msg_id: m.get("msg_id").and_then(|v| v.as_str()).map(String::from),
                sender_id: m.get("sender_id").and_then(|v| v.as_str()).map(String::from),
                chat_id: m.get("chat_id").and_then(|v| v.as_str()).map(String::from),
                chat_type: m.get("chat_type").and_then(|v| v.as_str()).map(String::from),
                content: m.get("content").and_then(|v| v.as_str()).map(String::from),
                context_token: m.get("context_token").and_then(|v| v.as_str()).map(String::from),
                item_list: m.get("item_list").and_then(|v| v.as_array()).cloned(),
                aes_key: m.get("aes_key").and_then(|v| v.as_str()).map(String::from),
                encrypted_query_param: m.get("encrypted_query_param").and_then(|v| v.as_str()).map(String::from),
                full_url: m.get("full_url").and_then(|v| v.as_str()).map(String::from),
            })
        }).collect();

        Ok((updates, new_buf))
    }

    pub async fn send_message(&self, to_user_id: &str, text: &str, context_token: Option<&str>) -> Result<(), String> {
        let client_id = uuid::Uuid::new_v4().to_string();
        let item = serde_json::json!({
            "msg_type": 2,
            "message_state": 2,
            "text": text,
        });

        let mut body = serde_json::json!({
            "base_info": {"channel_version": CHANNEL_VERSION},
            "msg": {
                "to_user_id": to_user_id,
                "client_id": client_id,
                "message_type": 2,
                "message_state": 2,
                "item_list": [item],
            }
        });

        if let Some(token) = context_token {
            body["msg"]["context_token"] = serde_json::Value::String(token.to_string());
        }

        let resp = self.http
            .post(self.url(EP_SEND_MESSAGE))
            .headers(self.build_headers())
            .json(&body)
            .send()
            .await
            .map_err(|e| format!("send_message failed: {}", e))?;

        let data: serde_json::Value = resp.json().await.map_err(|e| format!("send_message parse failed: {}", e))?;
        let errcode = data.get("errcode").and_then(|v| v.as_i64()).unwrap_or(0);
        if errcode != 0 {
            return Err(format!("send_message API error: errcode={}", errcode));
        }

        Ok(())
    }

    pub async fn send_typing(&self, to_user_id: &str, typing_ticket: &str) -> Result<(), String> {
        let body = serde_json::json!({
            "base_info": {"channel_version": CHANNEL_VERSION},
            "to_user_id": to_user_id,
            "typing_ticket": typing_ticket,
        });

        let resp = self.http
            .post(self.url(EP_SEND_TYPING))
            .headers(self.build_headers())
            .json(&body)
            .send()
            .await
            .map_err(|e| format!("send_typing failed: {}", e))?;

        let data: serde_json::Value = resp.json().await.map_err(|e| format!("send_typing parse failed: {}", e))?;
        let errcode = data.get("errcode").and_then(|v| v.as_i64()).unwrap_or(0);
        if errcode != 0 {
            log::warn!("[WeChat] send_typing error: errcode={}", errcode);
        }
        Ok(())
    }

    pub async fn get_config(&self) -> Result<serde_json::Value, String> {
        let body = serde_json::json!({
            "base_info": {"channel_version": CHANNEL_VERSION},
        });

        let resp = self.http
            .post(self.url(EP_GET_CONFIG))
            .headers(self.build_headers())
            .json(&body)
            .send()
            .await
            .map_err(|e| format!("get_config failed: {}", e))?;

        resp.json().await.map_err(|e| format!("get_config parse failed: {}", e))
    }

    pub async fn get_bot_qrcode(&self) -> Result<ILinkQRCode, String> {
        let body = serde_json::json!({
            "base_info": {"channel_version": CHANNEL_VERSION},
            "bot_type": 3,
        });

        let resp = self.http
            .post(self.url(EP_GET_BOT_QR))
            .headers(self.build_headers())
            .json(&body)
            .send()
            .await
            .map_err(|e| format!("get_bot_qrcode failed: {}", e))?;

        let data: serde_json::Value = resp.json().await.map_err(|e| format!("get_bot_qrcode parse failed: {}", e))?;

        Ok(ILinkQRCode {
            qrcode: data.get("qrcode").and_then(|v| v.as_str()).map(String::from),
            qrcode_id: data.get("qrcode_id").and_then(|v| v.as_str()).map(String::from),
        })
    }

    pub async fn get_qrcode_status(&self, qrcode_id: &str) -> Result<ILinkQRStatus, String> {
        let body = serde_json::json!({
            "base_info": {"channel_version": CHANNEL_VERSION},
            "qrcode_id": qrcode_id,
        });

        let resp = self.http
            .post(self.url(EP_GET_QR_STATUS))
            .headers(self.build_headers())
            .json(&body)
            .send()
            .await
            .map_err(|e| format!("get_qrcode_status failed: {}", e))?;

        let data: serde_json::Value = resp.json().await.map_err(|e| format!("get_qrcode_status parse failed: {}", e))?;

        Ok(ILinkQRStatus {
            status: data.get("status").and_then(|v| v.as_str()).map(String::from),
            ilink_bot_id: data.get("ilink_bot_id").and_then(|v| v.as_str()).map(String::from),
            bot_token: data.get("bot_token").and_then(|v| v.as_str()).map(String::from),
            baseurl: data.get("baseurl").and_then(|v| v.as_str()).map(String::from),
            ilink_user_id: data.get("ilink_user_id").and_then(|v| v.as_str()).map(String::from),
        })
    }

    pub async fn get_upload_url(&self, file_key: &str, aes_key_b64: &str, file_size: i64, md5: &str) -> Result<serde_json::Value, String> {
        let body = serde_json::json!({
            "base_info": {"channel_version": CHANNEL_VERSION},
            "filekey": file_key,
            "aes_key": aes_key_b64,
            "file_size": file_size,
            "md5": md5,
        });

        let resp = self.http
            .post(self.url(EP_GET_UPLOAD_URL))
            .headers(self.build_headers())
            .json(&body)
            .send()
            .await
            .map_err(|e| format!("get_upload_url failed: {}", e))?;

        resp.json().await.map_err(|e| format!("get_upload_url parse failed: {}", e))
    }
}
