// Claw Desktop - WS响应 - 统一响应构建工具
use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde::{Deserialize, Serialize};

/// 统一API响应结构 - 所有HTTP接口返回此格式
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApiResponse<T: Serialize> {
    pub success: bool, // 是否成功
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<T>, // 响应数据
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>, // 错误信息
}

impl<T: Serialize> ApiResponse<T> {
    /// 创建成功响应
    pub fn ok(data: T) -> Self {
        Self {
            success: true,
            data: Some(data),
            error: None,
        }
    }

    /// 创建错误响应
    pub fn err(msg: &str) -> Self {
        Self {
            success: false,
            data: None,
            error: Some(msg.to_string()),
        }
    }

    /// 创建带HTTP状态码的错误响应
    pub fn err_with_status(status: StatusCode, msg: &str) -> (StatusCode, Self) {
        (status, Self::err(msg))
    }
}

impl<T: Serialize + 'static> IntoResponse for ApiResponse<T> {
    fn into_response(self) -> Response {
        let status = if self.success {
            StatusCode::OK
        } else {
            StatusCode::BAD_REQUEST
        };
        (status, Json(self)).into_response()
    }
}
