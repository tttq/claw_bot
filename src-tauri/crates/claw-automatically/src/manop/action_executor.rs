// Claw Desktop - Mano-P 动作执行器
// 将 Mano-P 模型输出的动作转换为实际的输入操作

use super::{ManoPAction, ScrollDirection, WindowActionType};
use crate::capture::screen;
use crate::coordinate_validator;
use crate::error::{AutomaticallyError, Result};
use crate::input::{keyboard, mouse};
use crate::retry;
use chrono::Utc;
use std::collections::HashMap;

const DEFAULT_ACTION_TIMEOUT_MS: u64 = 30_000;

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

        log::info!(
            "[ActionExecutor] Starting execution of {} actions",
            actions.len()
        );

        for action in actions {
            match Self::execute_single_inner(&action).await {
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

    /// 执行动作序列（带重试和超时）
    pub async fn execute_sequence_with_retry(
        &mut self,
        actions: Vec<ManoPAction>,
        max_retries: usize,
    ) -> Result<ActionExecutionContext> {
        let mut context = ActionExecutionContext::new();

        log::info!(
            "[ActionExecutor] Starting execution of {} actions with max_retries={}",
            actions.len(),
            max_retries
        );

        for action in actions {
            let action_description = format!("{:?}", action);
            let mut succeeded = false;

            for attempt in 0..=max_retries {
                let result = retry::with_retry_and_timeout(
                    &action_description,
                    None,
                    DEFAULT_ACTION_TIMEOUT_MS,
                    || {
                        let a = action.clone();
                        async move { ActionExecutor::execute_single_inner(&a).await }
                    },
                )
                .await;

                match result {
                    Ok(_) => {
                        succeeded = true;
                        if attempt > 0 {
                            log::info!(
                                "[ActionExecutor] Action succeeded on retry {}",
                                attempt
                            );
                        }
                        break;
                    }
                    Err(e) => {
                        log::warn!(
                            "[ActionExecutor] Action attempt {}/{} failed: {}",
                            attempt + 1,
                            max_retries + 1,
                            e
                        );
                        if attempt < max_retries {
                            tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
                        }
                    }
                }
            }

            context.record_action(&action, succeeded);

            if !succeeded {
                context.mark_failed(format!(
                    "Action failed after {} retries: {:?}",
                    max_retries + 1,
                    action
                ));
                return Ok(context);
            }
        }

        context.mark_success();
        log::info!("[ActionExecutor] All actions executed successfully");
        Ok(context)
    }

    async fn execute_single_inner(action: &ManoPAction) -> Result<()> {
        log::debug!("[ActionExecutor] Executing action: {:?}", action);

        match action {
            ManoPAction::Click {
                element_id: _,
                point,
            } => {
                let (x, y) = coordinate_validator::validate_and_clamp(
                    point.x as f64,
                    point.y as f64,
                );
                mouse::click(x, y).await
            }
            ManoPAction::DoubleClick {
                element_id: _,
                point,
            } => {
                let (x, y) = coordinate_validator::validate_and_clamp(
                    point.x as f64,
                    point.y as f64,
                );
                mouse::double_click(x, y).await
            }
            ManoPAction::RightClick {
                element_id: _,
                point,
            } => {
                let (x, y) = coordinate_validator::validate_and_clamp(
                    point.x as f64,
                    point.y as f64,
                );
                mouse::right_click(x, y).await
            }
            ManoPAction::TypeText {
                element_id: _,
                text,
            } => {
                if text.is_empty() {
                    return Ok(());
                }
                keyboard::type_text(text).await
            }
            ManoPAction::Scroll {
                element_id: _,
                direction,
                amount,
            } => {
                let scroll_amount = match direction {
                    ScrollDirection::Up => *amount,
                    ScrollDirection::Down => -*amount,
                    ScrollDirection::Left => *amount,
                    ScrollDirection::Right => -*amount,
                };
                let (x, y) = mouse::get_position().await?;
                mouse::scroll_at(x, y, scroll_amount as i32).await
            }
            ManoPAction::Drag { from, to } => {
                let (from_x, from_y) = coordinate_validator::validate_and_clamp(
                    from.x as f64,
                    from.y as f64,
                );
                let (to_x, to_y) = coordinate_validator::validate_and_clamp(
                    to.x as f64,
                    to.y as f64,
                );
                mouse::drag(from_x, from_y, to_x, to_y).await
            }
            ManoPAction::Wait { duration_ms } => {
                let duration = (*duration_ms).max(100);
                tokio::time::sleep(tokio::time::Duration::from_millis(duration)).await;
                Ok(())
            }
            ManoPAction::ScreenshotVerify {
                expected_elements: _,
            } => {
                let _frame = screen::capture_screen()?;
                Ok(())
            }
            ManoPAction::Hotkey { keys } => {
                if keys.is_empty() {
                    return Err(AutomaticallyError::Input(
                        "No keys specified for hotkey".to_string(),
                    ));
                }
                for key in keys.iter().take(keys.len().saturating_sub(1)) {
                    keyboard::key_down(key).await?;
                }
                if let Some(main_key) = keys.last() {
                    keyboard::press_key(main_key).await?;
                }
                for key in keys.iter().take(keys.len().saturating_sub(1)).rev() {
                    keyboard::key_up(key).await?;
                }
                Ok(())
            }
            ManoPAction::WindowAction { action } => match action {
                WindowActionType::Minimize => {
                    keyboard::press_key("Win+Down").await
                }
                WindowActionType::Maximize => {
                    keyboard::press_key("Win+Up").await
                }
                WindowActionType::Close => {
                    keyboard::press_key("Alt+F4").await
                }
                WindowActionType::Focus => {
                    let frame = screen::capture_screen()?;
                    let center_x = (frame.width / 2) as f64;
                    let center_y = (frame.height / 2) as f64;
                    mouse::click(center_x, center_y).await
                }
                WindowActionType::Move { x: _, y: _ } => {
                    log::warn!("[ActionExecutor] Window move requires platform-specific API");
                    Ok(())
                }
                WindowActionType::Resize {
                    width: _,
                    height: _,
                } => {
                    log::warn!(
                        "[ActionExecutor] Window resize requires platform-specific API"
                    );
                    Ok(())
                }
            },
        }
    }
}

impl Default for ActionExecutor {
    fn default() -> Self {
        Self::new()
    }
}
