// Claw Desktop - Mano-P 动作执行器
// 将 Mano-P 模型输出的动作转换为实际的输入操作

use super::{ManoPAction, ScrollDirection, WindowActionType};
use crate::error::{AutomaticallyError, Result};
use crate::input::{mouse, keyboard};
use crate::capture::screen;
use chrono::Utc;
use std::collections::HashMap;

/// 动作执行上下文
#[derive(Debug, Clone)]
pub struct ActionExecutionContext {
    /// 当前执行步数
    pub step_count: usize,
    /// 是否成功
    pub success: bool,
    /// 错误信息
    pub error_message: Option<String>,
    /// 执行历史
    pub history: Vec<ActionRecord>,
    /// 变量存储 (用于动作间传递数据)
    pub variables: HashMap<String, String>,
}

impl ActionExecutionContext {
    /// 创建动作执行上下文 — 初始化空历史和变量存储
    pub fn new() -> Self {
        Self {
            step_count: 0,
            success: false,
            error_message: None,
            history: Vec::new(),
            variables: HashMap::new(),
        }
    }

    /// 记录动作执行结果 — 追加到历史记录并递增步数
    pub fn record_action(&mut self, action: &ManoPAction, result: bool) {
        self.history.push(ActionRecord {
            step: self.step_count,
            action: format!("{:?}", action),
            result,
            timestamp: Utc::now(),
        });
        self.step_count += 1;
    }

    /// 标记执行成功
    pub fn mark_success(&mut self) {
        self.success = true;
    }

    /// 标记执行失败 — 记录错误信息
    pub fn mark_failed(&mut self, error: String) {
        self.success = false;
        self.error_message = Some(error);
    }
}

impl Default for ActionExecutionContext {
    fn default() -> Self {
        Self::new()
    }
}

/// 动作执行记录 — 记录单步动作的执行结果和时间戳
#[derive(Debug, Clone)]
pub struct ActionRecord {
    pub step: usize,
    pub action: String,
    pub result: bool,
    pub timestamp: chrono::DateTime<Utc>,
}

/// 动作执行器
pub struct ActionExecutor;

impl ActionExecutor {
    /// 创建动作执行器
    pub fn new() -> Self {
        Self
    }

    /// 执行动作序列
    pub async fn execute_sequence(
        &mut self,
        actions: Vec<ManoPAction>,
    ) -> Result<ActionExecutionContext> {
        let mut context = ActionExecutionContext::new();

        log::info!("[ActionExecutor] Starting execution of {} actions", actions.len());

        for action in actions {
            match self.execute_single(&action).await {
                Ok(_) => {
                    context.record_action(&action, true);
                }
                Err(e) => {
                    log::error!("[ActionExecutor] Action failed: {}", e);
                    context.record_action(&action, false);
                    context.mark_failed(e.to_string());
                    return Ok(context);
                }
            }
        }

        context.mark_success();
        log::info!("[ActionExecutor] All actions executed successfully");
        Ok(context)
    }

    /// 执行单个动作
    async fn execute_single(&self, action: &ManoPAction) -> Result<()> {
        log::debug!("[ActionExecutor] Executing action: {:?}", action);

        match action {
            ManoPAction::Click { element_id: _, point } => {
                self.execute_click(point.x as f64, point.y as f64).await
            }
            ManoPAction::DoubleClick { element_id: _, point } => {
                self.execute_double_click(point.x as f64, point.y as f64).await
            }
            ManoPAction::RightClick { element_id: _, point } => {
                self.execute_right_click(point.x as f64, point.y as f64).await
            }
            ManoPAction::TypeText { element_id: _, text } => {
                self.execute_type_text(text).await
            }
            ManoPAction::Scroll { element_id: _, direction, amount } => {
                self.execute_scroll(direction, *amount).await
            }
            ManoPAction::Drag { from, to } => {
                self.execute_drag(from.x as f64, from.y as f64, to.x as f64, to.y as f64).await
            }
            ManoPAction::Wait { duration_ms } => {
                self.execute_wait(*duration_ms).await
            }
            ManoPAction::ScreenshotVerify { expected_elements: _ } => {
                self.execute_screenshot_verify().await
            }
            ManoPAction::Hotkey { keys } => {
                self.execute_hotkey(keys).await
            }
            ManoPAction::WindowAction { action } => {
                self.execute_window_action(action).await
            }
        }
    }

    /// 执行点击
    async fn execute_click(&self, x: f64, y: f64) -> Result<()> {
        log::info!("[ActionExecutor] Clicking at ({}, {})", x, y);
        mouse::click(x, y).await
    }

    /// 执行双击
    async fn execute_double_click(&self, x: f64, y: f64) -> Result<()> {
        log::info!("[ActionExecutor] Double-clicking at ({}, {})", x, y);
        mouse::double_click(x, y).await
    }

    /// 执行右键点击
    async fn execute_right_click(&self, x: f64, y: f64) -> Result<()> {
        log::info!("[ActionExecutor] Right-clicking at ({}, {})", x, y);
        mouse::right_click(x, y).await
    }

    /// 执行文本输入
    async fn execute_type_text(&self, text: &str) -> Result<()> {
        log::info!("[ActionExecutor] Typing text: {}", text);
        keyboard::type_text(text).await
    }

    /// 执行滚动
    async fn execute_scroll(&self, direction: &ScrollDirection, amount: i32) -> Result<()> {
        log::info!("[ActionExecutor] Scrolling {:?} by {}", direction, amount);
        // 将滚动转换为鼠标滚轮操作
        // 正值为向上/向左，负值为向下/向右
        let scroll_amount = match direction {
            ScrollDirection::Up => amount,
            ScrollDirection::Down => -amount,
            ScrollDirection::Left => amount,
            ScrollDirection::Right => -amount,
        };

        // 获取当前鼠标位置
        let (x, y) = mouse::get_position().await?;

        // 执行滚动
        mouse::scroll_at(x, y, scroll_amount as i32).await
    }

    /// 执行拖拽
    async fn execute_drag(&self, from_x: f64, from_y: f64, to_x: f64, to_y: f64) -> Result<()> {
        log::info!("[ActionExecutor] Dragging from ({}, {}) to ({}, {})", from_x, from_y, to_x, to_y);
        mouse::drag(from_x, from_y, to_x, to_y).await
    }

    /// 执行等待
    async fn execute_wait(&self, duration_ms: u64) -> Result<()> {
        log::info!("[ActionExecutor] Waiting for {}ms", duration_ms);
        tokio::time::sleep(tokio::time::Duration::from_millis(duration_ms)).await;
        Ok(())
    }

    /// 执行截图验证
    async fn execute_screenshot_verify(&self) -> Result<()> {
        log::info!("[ActionExecutor] Taking screenshot for verification");
        let _frame = screen::capture_screen()?;
        // 这里可以添加与预期状态的对比逻辑
        Ok(())
    }

    /// 执行快捷键
    async fn execute_hotkey(&self, keys: &[String]) -> Result<()> {
        log::info!("[ActionExecutor] Executing hotkey: {:?}", keys);
        if keys.is_empty() {
            return Err(AutomaticallyError::Input("No keys specified for hotkey".to_string()));
        }

        // 按下所有修饰键
        for key in keys.iter().take(keys.len().saturating_sub(1)) {
            keyboard::key_down(key).await?;
        }

        // 按下并释放主键
        if let Some(main_key) = keys.last() {
            keyboard::press_key(main_key).await?;
        }

        // 释放所有修饰键
        for key in keys.iter().take(keys.len().saturating_sub(1)).rev() {
            keyboard::key_up(key).await?;
        }

        Ok(())
    }

    /// 执行窗口操作
    async fn execute_window_action(&self, action: &WindowActionType) -> Result<()> {
        log::info!("[ActionExecutor] Executing window action: {:?}", action);

        match action {
            WindowActionType::Minimize => {
                // Windows: Win+Down
                keyboard::press_key("Win+Down").await
            }
            WindowActionType::Maximize => {
                // Windows: Win+Up
                keyboard::press_key("Win+Up").await
            }
            WindowActionType::Close => {
                // Alt+F4
                keyboard::press_key("Alt+F4").await
            }
            WindowActionType::Focus => {
                // 点击屏幕中心获取焦点
                let frame = screen::capture_screen()?;
                let center_x = (frame.width / 2) as f64;
                let center_y = (frame.height / 2) as f64;
                mouse::click(center_x, center_y).await
            }
            WindowActionType::Move { x: _, y: _ } => {
                // 窗口移动需要平台特定的实现
                log::warn!("[ActionExecutor] Window move not fully implemented");
                Ok(())
            }
            WindowActionType::Resize { width: _, height: _ } => {
                // 窗口调整大小需要平台特定的实现
                log::warn!("[ActionExecutor] Window resize not fully implemented");
                Ok(())
            }
        }
    }
}

impl Default for ActionExecutor {
    fn default() -> Self {
        Self::new()
    }
}
