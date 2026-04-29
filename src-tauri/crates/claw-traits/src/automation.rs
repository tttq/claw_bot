// Claw Desktop - 自动化Trait - 定义UI自动化操作的统一接口
// 基于 Mano-P GUI-VLA 模型的纯视觉桌面操控
use async_trait::async_trait;
use std::sync::OnceLock;

/// 自动化操作执行器 Trait（由 claw-automatically 实现）
#[async_trait]
pub trait AutomationExecutor: Send + Sync {
    /// 执行自然语言自动化指令
    async fn execute_automation(&self, instruction: &str) -> Result<String, String>;
    /// 捕获屏幕截图（返回 base64 编码）
    async fn capture_screen(&self) -> Result<String, String>;
    /// 模拟鼠标点击
    async fn mouse_click(&self, x: f64, y: f64) -> Result<String, String>;
    /// 模拟鼠标双击
    async fn mouse_double_click(&self, x: f64, y: f64) -> Result<String, String>;
    /// 模拟右键点击
    async fn mouse_right_click(&self, x: f64, y: f64) -> Result<String, String>;
    /// 模拟键盘输入文本
    async fn keyboard_type(&self, text: &str) -> Result<String, String>;
    /// 模拟按键
    async fn keyboard_press(&self, key: &str) -> Result<String, String>;
    /// 列出已安装应用（Mano-P 模式下返回提示信息）
    async fn list_installed_apps(&self, filter: Option<&str>) -> Result<String, String>;
    /// 启动应用（Mano-P 模式下返回提示信息）
    async fn launch_application(&self, name: &str) -> Result<String, String>;
    /// OCR 识别屏幕内容
    async fn ocr_recognize_screen(&self, language: Option<&str>) -> Result<String, String>;
}

static EXECUTOR: OnceLock<Box<dyn AutomationExecutor>> = OnceLock::new();

/// 注册自动化执行器（在应用启动时调用一次）
pub fn set_executor(executor: impl AutomationExecutor + 'static) -> Result<(), String> {
    EXECUTOR
        .set(Box::new(executor))
        .map_err(|_| "Automation executor already set".to_string())
}

/// 检查执行器是否已注册
pub fn is_executor_registered() -> bool {
    EXECUTOR.get().is_some()
}

/// 获取执行器引用（内部使用）
pub fn get_executor() -> Option<&'static dyn AutomationExecutor> {
    EXECUTOR.get().map(|e| e.as_ref())
}
