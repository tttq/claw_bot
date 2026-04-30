// Claw Desktop - 自动化类型 - 核心数据类型定义
use serde::{Deserialize, Serialize};

/// 图像帧 — 存储原始RGB像素数据和尺寸信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageFrame {
    pub width: u32,
    pub height: u32,
    pub data: Vec<u8>,
    pub timestamp: Option<chrono::DateTime<chrono::Utc>>,
}

impl ImageFrame {
    /// 创建图像帧 — 记录当前时间戳
    pub fn new(width: u32, height: u32, data: Vec<u8>) -> Self {
        Self {
            width,
            height,
            data,
            timestamp: Some(chrono::Utc::now()),
        }
    }

    /// 转换为PNG格式字节
    pub fn to_png(&self) -> Result<Vec<u8>, String> {
        let img = image::RgbImage::from_raw(self.width, self.height, self.data.clone())
            .ok_or_else(|| "Failed to create image from raw data".to_string())?;
        let mut buf = std::io::Cursor::new(Vec::new());
        image::DynamicImage::ImageRgb8(img)
            .write_to(&mut buf, image::ImageFormat::Png)
            .map_err(|e| format!("PNG encode failed: {}", e))?;
        Ok(buf.into_inner())
    }

    /// 转换为Base64编码的PNG字符串
    pub fn to_base64(&self) -> String {
        let png_data = self.to_png().unwrap_or_default();
        base64::Engine::encode(&base64::engine::general_purpose::STANDARD, png_data)
    }
}

/// 二维坐标点
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Point {
    pub x: i32,
    pub y: i32,
}

impl Point {
    /// 创建坐标点
    pub fn new(x: i32, y: i32) -> Self {
        Self { x, y }
    }
}

/// 矩形边界框 — 用左上角和右下角坐标表示
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BoundingBox {
    pub x1: f64,
    pub y1: f64,
    pub x2: f64,
    pub y2: f64,
}

impl BoundingBox {
    /// 创建边界框
    pub fn new(x1: f64, y1: f64, x2: f64, y2: f64) -> Self {
        Self { x1, y1, x2, y2 }
    }

    /// 计算中心点坐标
    pub fn center(&self) -> Point {
        Point::new(
            ((self.x1 + self.x2) / 2.0) as i32,
            ((self.y1 + self.y2) / 2.0) as i32,
        )
    }

    /// 计算宽度
    pub fn width(&self) -> f64 {
        self.x2 - self.x1
    }

    /// 计算高度
    pub fn height(&self) -> f64 {
        self.y2 - self.y1
    }

    /// 计算面积
    pub fn area(&self) -> f64 {
        self.width() * self.height()
    }
}

/// UI元素 — 包含ID、文本、边界框、类型和置信度
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UiElement {
    pub id: String,
    pub text: String,
    pub bbox: BoundingBox,
    pub element_type: ElementType,
    pub confidence: f32,
}

/// UI元素类型枚举
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ElementType {
    Text,
    Button,
    InputField,
    Link,
    Icon,
    Unknown,
}

impl Default for ElementType {
    fn default() -> Self {
        ElementType::Unknown
    }
}

/// 操作指令 — 包含操作类型、目标坐标、文本和按键
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Operation {
    pub operation_type: String,
    pub target_x: f64,
    pub target_y: f64,
    pub text: Option<String>,
    pub keys: Option<Vec<String>>,
    pub reasoning: Option<String>,
}

/// 操作结果 — 记录操作是否成功及执行时间
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OperationResult {
    pub success: bool,
    pub operation: Operation,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

/// 重试策略 — 控制操作失败后的重试行为
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetryPolicy {
    pub max_attempts: u32,
    pub retry_delay_ms: u64,
    pub backoff_multiplier: f64,
}

impl Default for RetryPolicy {
    fn default() -> Self {
        Self {
            max_attempts: 3,
            retry_delay_ms: 1000,
            backoff_multiplier: 1.5,
        }
    }
}

/// 应用信息 — 记录已安装应用的名称、路径和元数据
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppInfo {
    pub name: String,
    pub executable_path: String,
    pub description: Option<String>,
    pub publisher: Option<String>,
    pub version: Option<String>,
    pub launch_command: Option<String>,
    #[serde(default)]
    pub app_source: String,
    #[serde(default)]
    pub keywords: Vec<String>,
}

impl AppInfo {
    /// 获取显示名称
    pub fn display_name(&self) -> &str {
        &self.name
    }

    /// 生成搜索文本 — 合并名称、发布者、描述和关键词
    pub fn search_text(&self) -> String {
        let mut text = self.name.to_lowercase();
        if let Some(ref pub_) = self.publisher {
            text.push(' ');
            text.push_str(&pub_.to_lowercase());
        }
        if let Some(ref desc) = self.description {
            text.push(' ');
            text.push_str(&desc.to_lowercase());
        }
        for kw in &self.keywords {
            text.push(' ');
            text.push_str(&kw.to_lowercase());
        }
        text
    }

    /// 检查是否匹配查询 — 包含匹配或模糊匹配
    pub fn matches_query(&self, query: &str) -> bool {
        let query_lower = query.to_lowercase();
        let search_text = self.search_text();
        if search_text.contains(&query_lower) {
            return true;
        }
        let name_lower = self.name.to_lowercase();
        if name_lower.contains(&query_lower) {
            return true;
        }
        if self.fuzzy_match(&query_lower) {
            return true;
        }
        false
    }

    /// 模糊匹配 — 按字符顺序匹配查询字符串
    fn fuzzy_match(&self, query: &str) -> bool {
        let name_lower = self.name.to_lowercase();
        let chars: Vec<char> = query.chars().collect();
        let mut ci = 0;
        for ch in name_lower.chars() {
            if ci < chars.len() && ch == chars[ci] {
                ci += 1;
            }
        }
        ci == chars.len() && !chars.is_empty()
    }

    /// 计算相关度分数 — 精确匹配100，前缀90，包含80，搜索文本60，模糊40，字符匹配20
    pub fn relevance_score(&self, query: &str) -> f64 {
        let query_lower = query.to_lowercase();
        let name_lower = self.name.to_lowercase();

        if name_lower == query_lower {
            return 100.0;
        }
        if name_lower.starts_with(&query_lower) {
            return 90.0;
        }
        if name_lower.contains(&query_lower) {
            return 80.0;
        }

        let search_text = self.search_text();
        if search_text.contains(&query_lower) {
            return 60.0;
        }

        if self.fuzzy_match(query) {
            return 40.0;
        }

        let query_chars: Vec<char> = query_lower.chars().collect();
        let mut matched = 0usize;
        for ch in search_text.chars() {
            if query_chars.contains(&ch) {
                matched += 1;
            }
        }
        if !query_chars.is_empty() {
            (matched as f64 / query_chars.len() as f64) * 20.0
        } else {
            0.0
        }
    }
}

/// 窗口信息 — 包含标题、进程ID、窗口ID和位置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WindowInfo {
    pub title: String,
    pub process_id: u32,
    pub window_id: u64,
    pub rect: Option<crate::platform::window::WindowRect>,
}
