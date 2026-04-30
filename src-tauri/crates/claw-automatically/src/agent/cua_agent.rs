// Claw Desktop - CUA（Computer Use Agent）桌面自动化Agent
// 实现观察-思考-行动循环：截图 → LLM推理决策 → 执行鼠标键盘操作，
// 支持点击/双击/右键/输入/快捷键/滚动/拖拽/启动应用等桌面交互动作
use crate::AutomaticallyConfig;
use crate::capture::screen;
use crate::error::{AutomaticallyError, Result};
use crate::input::{keyboard, mouse};
use crate::platform::{app_launcher, window};
use serde::{Deserialize, Serialize};
use std::time::Instant;

const MAX_AGENT_STEPS: usize = 50;
const STEP_DELAY_MS: u64 = 500;
const SCREENSHOT_AFTER_ACTION_DELAY_MS: u64 = 300;

/// CUA步骤执行结果 — 记录单步操作的状态和截图
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CuaStepResult {
    pub step: usize,
    pub action: String,
    pub success: bool,
    pub screenshot_before: Option<String>,
    pub screenshot_after: Option<String>,
    pub reasoning: Option<String>,
    pub error: Option<String>,
}

/// CUA完整执行结果 — 记录整个任务的多步执行过程
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CuaExecutionResult {
    pub instruction: String,
    pub success: bool,
    pub steps: Vec<CuaStepResult>,
    pub total_steps: usize,
    pub elapsed_ms: u64,
    pub final_screenshot: Option<String>,
    pub error: Option<String>,
}

/// CUA动作指令 — 描述一个桌面操作（点击/输入/快捷键/滚动/拖拽等）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CuaAction {
    #[serde(rename = "type")]
    pub action_type: String,
    pub x: Option<f64>,
    pub y: Option<f64>,
    pub text: Option<String>,
    pub keys: Option<Vec<String>>,
    pub direction: Option<String>,
    pub amount: Option<i32>,
    pub app_name: Option<String>,
    pub from_x: Option<f64>,
    pub from_y: Option<f64>,
    pub to_x: Option<f64>,
    pub to_y: Option<f64>,
    pub duration_ms: Option<u64>,
    pub reasoning: Option<String>,
}

/// CUA桌面自动化Agent — 实现"观察-思考-行动"循环控制桌面
pub struct CuaAgent {
    config: AutomaticallyConfig,
    max_steps: usize,
    skill: Option<super::automation_skill::AutomationSkill>,
}

impl CuaAgent {
    /// 创建CUA Agent实例 — 根据配置设置最大步骤数（上限50步），自动匹配技能
    pub fn new(config: AutomaticallyConfig) -> Self {
        let max_steps = config.max_action_steps.min(MAX_AGENT_STEPS).max(1);
        Self {
            config,
            max_steps,
            skill: None,
        }
    }

    /// 创建带技能的CUA Agent实例 — 根据指令自动匹配应用技能
    pub fn new_with_skill(config: AutomaticallyConfig, instruction: &str) -> Self {
        let max_steps = config.max_action_steps.min(MAX_AGENT_STEPS).max(1);
        let skill = super::automation_skill::match_skill(instruction);
        if let Some(ref s) = skill {
            log::info!(
                "[CuaAgent] Matched automation skill: {} for instruction: {}",
                s.app_name,
                instruction
            );
        }
        Self {
            config,
            max_steps,
            skill,
        }
    }

    /// 执行CUA任务 — 循环执行"截图→思考→行动"直到任务完成或达到最大步骤
    pub async fn execute(&self, instruction: &str) -> Result<CuaExecutionResult> {
        let start = Instant::now();
        log::info!(
            "[CuaAgent:execute] Starting CUA loop | instruction={}",
            instruction
        );

        let mut steps = Vec::new();
        let mut history_summary = String::new();
        let mut final_screenshot: Option<String> = None;
        let mut task_completed = false;
        let mut consecutive_think_failures = 0u32;
        const MAX_CONSECUTIVE_THINK_FAILURES: u32 = 3;

        for step in 0..self.max_steps {
            log::info!("[CuaAgent:execute] Step {}/{}", step + 1, self.max_steps);

            let screenshot_before = self.capture_screenshot_base64()?;

            let think_result = self
                .think(instruction, &history_summary, &screenshot_before)
                .await;

            let cua_action = match think_result {
                Ok(action) => {
                    consecutive_think_failures = 0;
                    action
                }
                Err(e) => {
                    consecutive_think_failures += 1;
                    log::error!(
                        "[CuaAgent:execute] Think failed at step {} (consecutive: {}): {}",
                        step + 1,
                        consecutive_think_failures,
                        e
                    );
                    steps.push(CuaStepResult {
                        step: step + 1,
                        action: "think".to_string(),
                        success: false,
                        screenshot_before: Some(screenshot_before),
                        screenshot_after: None,
                        reasoning: None,
                        error: Some(e.to_string()),
                    });
                    if consecutive_think_failures >= MAX_CONSECUTIVE_THINK_FAILURES {
                        log::error!(
                            "[CuaAgent:execute] Too many consecutive think failures, aborting"
                        );
                        break;
                    }
                    tokio::time::sleep(tokio::time::Duration::from_millis(1000)).await;
                    continue;
                }
            };

            let action_desc = self.describe_action(&cua_action);
            log::info!("[CuaAgent:execute] Action: {}", action_desc);

            if cua_action.action_type == "done" || cua_action.action_type == "complete" {
                log::info!(
                    "[CuaAgent:execute] Task completed by agent at step {}",
                    step + 1
                );
                steps.push(CuaStepResult {
                    step: step + 1,
                    action: action_desc,
                    success: true,
                    screenshot_before: Some(screenshot_before),
                    screenshot_after: None,
                    reasoning: cua_action.reasoning.clone(),
                    error: None,
                });
                task_completed = true;
                break;
            }

            if cua_action.action_type == "fail" {
                log::warn!(
                    "[CuaAgent:execute] Agent reported failure at step {}",
                    step + 1
                );
                steps.push(CuaStepResult {
                    step: step + 1,
                    action: action_desc,
                    success: false,
                    screenshot_before: Some(screenshot_before),
                    screenshot_after: None,
                    reasoning: cua_action.reasoning.clone(),
                    error: Some("Agent reported task as failed".to_string()),
                });
                break;
            }

            let act_result = self.act(&cua_action).await;

            tokio::time::sleep(tokio::time::Duration::from_millis(
                SCREENSHOT_AFTER_ACTION_DELAY_MS,
            ))
            .await;

            let screenshot_after = self.capture_screenshot_base64().ok();
            final_screenshot = screenshot_after.clone();

            let step_result = CuaStepResult {
                step: step + 1,
                action: action_desc.clone(),
                success: act_result.is_ok(),
                screenshot_before: Some(screenshot_before),
                screenshot_after,
                reasoning: cua_action.reasoning.clone(),
                error: act_result.err().map(|e| e.to_string()),
            };

            history_summary.push_str(&format!(
                "Step {}: {} - {}\n",
                step + 1,
                action_desc,
                if step_result.success { "OK" } else { "FAILED" }
            ));

            steps.push(step_result);

            tokio::time::sleep(tokio::time::Duration::from_millis(STEP_DELAY_MS)).await;
        }

        let elapsed_ms = start.elapsed().as_millis() as u64;
        let total_steps = steps.len();

        Ok(CuaExecutionResult {
            instruction: instruction.to_string(),
            success: task_completed,
            steps,
            total_steps,
            elapsed_ms,
            final_screenshot,
            error: if task_completed {
                None
            } else {
                Some("Task not completed within max steps".to_string())
            },
        })
    }

    /// 思考阶段 — 将截图和指令发送给LLM，解析返回的动作决策
    async fn think(
        &self,
        instruction: &str,
        history: &str,
        screenshot_base64: &str,
    ) -> Result<CuaAction> {
        let caller = claw_traits::llm_caller::get_llm_caller().ok_or_else(|| {
            AutomaticallyError::InferenceEngine("LlmCaller not registered".to_string())
        })?;

        let system_prompt = self.build_system_prompt();
        let user_message = self.build_user_message(instruction, history);

        let api_key = self.config.llm_api_key.as_deref().unwrap_or("");
        let base_url = self.config.llm_api_endpoint.clone();
        let model = self.config.llm_model.clone();
        let is_openai = !base_url.contains("anthropic");

        let response = caller
            .call_once_vision(
                api_key,
                &base_url,
                &model,
                &system_prompt,
                &user_message,
                screenshot_base64,
                is_openai,
            )
            .await
            .map_err(|e| {
                log::error!("[CuaAgent:think] LLM vision call failed: {}", e);
                AutomaticallyError::InferenceEngine(format!("LLM vision call failed: {}", e))
            })?;

        self.parse_action_response(&response)
    }

    /// 构建系统提示词 — 包含屏幕分辨率、可用动作类型、操作规则和技能知识
    fn build_system_prompt(&self) -> String {
        let (screen_w, screen_h) = window::get_screen_size().unwrap_or((1920, 1080));

        let mut prompt = format!(
            r#"You are a Computer Use Agent (CUA) that controls the desktop to complete user tasks.

You receive a screenshot of the current desktop and must decide the NEXT SINGLE ACTION to take.

SCREEN RESOLUTION: {screen_w}x{screen_h}
All coordinates are in pixels, with (0,0) at the top-left corner.

AVAILABLE ACTIONS (respond with exactly one JSON object per turn):

1. Click: {{"type": "click", "x": <number>, "y": <number>, "reasoning": "<why>"}}
2. Double Click: {{"type": "double_click", "x": <number>, "y": <number>, "reasoning": "<why>"}}
3. Right Click: {{"type": "right_click", "x": <number>, "y": <number>, "reasoning": "<why>"}}
4. Type Text: {{"type": "type", "text": "<text to type>", "reasoning": "<why>"}}
5. Press Key: {{"type": "key_press", "keys": ["Ctrl", "c"], "reasoning": "<why>"}}
6. Scroll: {{"type": "scroll", "direction": "up|down", "amount": <number>, "reasoning": "<why>"}}
7. Drag: {{"type": "drag", "from_x": <number>, "from_y": <number>, "to_x": <number>, "to_y": <number>, "reasoning": "<why>"}}
8. Open App: {{"type": "open_app", "app_name": "<app name>", "reasoning": "<why>"}}
9. Wait: {{"type": "wait", "duration_ms": <number>, "reasoning": "<why>"}}
10. Task Complete: {{"type": "done", "reasoning": "<why task is complete>"}}
11. Task Failed: {{"type": "fail", "reasoning": "<why task cannot be completed>"}}

STRATEGY FOR COMPLEX TASKS:

When the task requires multiple steps (e.g., "send a message to XXX on WeChat", "create a document in Word and save it"), follow this approach:

1. ASSESS THE CURRENT STATE: Before each action, carefully examine the screenshot to determine:
   - Is the target application already open? (check taskbar, window titles)
   - Is the application in the expected state? (logged in, on the right screen)
   - Are there any dialogs or popups that need handling first?

2. BREAK DOWN THE TASK: Think about the sequence of actions needed:
   - If app not open → open it first, then wait for it to load
   - If app needs login → handle login screen before proceeding
   - Navigate to the correct screen/section within the app
   - Perform the specific action (type, click, etc.)
   - Verify the result before declaring done

3. HANDLE UNCERTAINTY: If you cannot see the expected UI element:
   - Wait longer (use "wait" with 2000-3000ms for app loading)
   - Scroll to find the element
   - Look for alternative ways to achieve the same goal
   - If truly stuck after 3 attempts, report "fail"

COMMON WORKFLOW EXAMPLES:

- Send message on WeChat/DingTalk/Feishu:
  1. Check if app is open → if not, open_app
  2. Wait for app to load (1-3 seconds)
  3. Find and click search bar → type contact name → press Enter
  4. Click on the contact in search results
  5. Click on message input box → type message → press Enter to send
  6. Verify message was sent → done

- Open a file in an application:
  1. Open the application
  2. Wait for it to load
  3. Use Ctrl+O to open file dialog, or click File > Open
  4. Navigate to the file location
  5. Select and open the file

RULES:
- Analyze the screenshot carefully before acting
- Click on UI elements at their center coordinates
- For typing, first click on the input field, THEN type
- Use keyboard shortcuts when more efficient (e.g., Ctrl+S to save, Ctrl+F to find)
- After opening an app, ALWAYS wait at least 2000ms for it to load before interacting
- If a dialog/popup appears, handle it before continuing the main task
- After each action, verify the result on the next screenshot before proceeding
- If the task is complete, respond with "done"
- If the task is impossible, respond with "fail"
- Respond with ONLY the JSON object, no other text"#,
            screen_w = screen_w,
            screen_h = screen_h,
        );

        if let Some(ref skill) = self.skill {
            prompt.push_str(&super::automation_skill::format_skill_for_prompt(skill));
        }

        prompt
    }

    /// 构建用户消息 — 包含任务指令和历史操作
    fn build_user_message(&self, instruction: &str, history: &str) -> String {
        let mut msg = format!("TASK: {}\n\n", instruction);

        if !history.is_empty() {
            msg.push_str(&format!("PREVIOUS ACTIONS:\n{}\n\n", history));
        }

        msg.push_str("What is the next action based on the screenshot?");

        msg
    }

    /// 解析LLM返回的动作响应 — 清理markdown代码块并反序列化为CuaAction
    fn parse_action_response(&self, response: &str) -> Result<CuaAction> {
        let cleaned = response
            .trim()
            .trim_start_matches("```json")
            .trim_start_matches("```")
            .trim_end_matches("```")
            .trim();

        match serde_json::from_str::<CuaAction>(cleaned) {
            Ok(action) => Ok(action),
            Err(e) => {
                log::warn!(
                    "[CuaAgent:parse_action_response] Direct parse failed: {}, trying to extract JSON from text",
                    e
                );

                if let Some(start) = cleaned.find('{') {
                    if let Some(end) = cleaned.rfind('}') {
                        let json_str = &cleaned[start..=end];
                        match serde_json::from_str::<CuaAction>(json_str) {
                            Ok(action) => return Ok(action),
                            Err(e2) => {
                                log::error!(
                                    "[CuaAgent:parse_action_response] Extracted JSON also failed: {} | text: {}",
                                    e2,
                                    json_str
                                );
                            }
                        }
                    }
                }

                Err(AutomaticallyError::InferenceEngine(format!(
                    "Failed to parse action response: {} | raw: {}",
                    e, cleaned
                )))
            }
        }
    }

    /// 执行动作 — 根据动作类型分派到鼠标/键盘/应用启动等操作
    async fn act(&self, action: &CuaAction) -> Result<()> {
        match action.action_type.as_str() {
            "click" => {
                let x = action
                    .x
                    .ok_or_else(|| AutomaticallyError::Automation("click missing x".to_string()))?;
                let y = action
                    .y
                    .ok_or_else(|| AutomaticallyError::Automation("click missing y".to_string()))?;
                mouse::click(x, y).await
            }
            "double_click" => {
                let x = action.x.ok_or_else(|| {
                    AutomaticallyError::Automation("double_click missing x".to_string())
                })?;
                let y = action.y.ok_or_else(|| {
                    AutomaticallyError::Automation("double_click missing y".to_string())
                })?;
                mouse::double_click(x, y).await
            }
            "right_click" => {
                let x = action.x.ok_or_else(|| {
                    AutomaticallyError::Automation("right_click missing x".to_string())
                })?;
                let y = action.y.ok_or_else(|| {
                    AutomaticallyError::Automation("right_click missing y".to_string())
                })?;
                mouse::right_click(x, y).await
            }
            "type" => {
                let text = action.text.as_deref().ok_or_else(|| {
                    AutomaticallyError::Automation("type missing text".to_string())
                })?;
                keyboard::type_text(text).await
            }
            "key_press" | "hotkey" => {
                let keys = action.keys.as_ref().ok_or_else(|| {
                    AutomaticallyError::Automation("key_press missing keys".to_string())
                })?;
                self.execute_hotkey(keys).await
            }
            "scroll" => {
                let direction = action.direction.as_deref().unwrap_or("down");
                let amount = action.amount.unwrap_or(3);
                let scroll_amount = match direction {
                    "up" => amount,
                    "down" => -amount,
                    _ => amount,
                };
                let (x, y) = mouse::get_position().await?;
                mouse::scroll_at(x, y, scroll_amount).await
            }
            "drag" => {
                let from_x = action.from_x.ok_or_else(|| {
                    AutomaticallyError::Automation("drag missing from_x".to_string())
                })?;
                let from_y = action.from_y.ok_or_else(|| {
                    AutomaticallyError::Automation("drag missing from_y".to_string())
                })?;
                let to_x = action.to_x.ok_or_else(|| {
                    AutomaticallyError::Automation("drag missing to_x".to_string())
                })?;
                let to_y = action.to_y.ok_or_else(|| {
                    AutomaticallyError::Automation("drag missing to_y".to_string())
                })?;
                mouse::drag(from_x, from_y, to_x, to_y).await
            }
            "open_app" => {
                let app_name = action.app_name.as_deref().ok_or_else(|| {
                    AutomaticallyError::Automation("open_app missing app_name".to_string())
                })?;
                app_launcher::launch_application(app_name)
            }
            "wait" => {
                let duration_ms = action.duration_ms.unwrap_or(1000);
                tokio::time::sleep(tokio::time::Duration::from_millis(duration_ms)).await;
                Ok(())
            }
            "done" | "complete" | "fail" => Ok(()),
            _ => Err(AutomaticallyError::Automation(format!(
                "Unknown action type: {}",
                action.action_type
            ))),
        }
    }

    /// 执行快捷键组合 — 按下修饰键→按主键→释放修饰键
    async fn execute_hotkey(&self, keys: &[String]) -> Result<()> {
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

    /// 描述动作 — 生成人类可读的动作描述字符串
    fn describe_action(&self, action: &CuaAction) -> String {
        match action.action_type.as_str() {
            "click" => format!(
                "click({}, {})",
                action.x.unwrap_or(0.0),
                action.y.unwrap_or(0.0)
            ),
            "double_click" => format!(
                "double_click({}, {})",
                action.x.unwrap_or(0.0),
                action.y.unwrap_or(0.0)
            ),
            "right_click" => format!(
                "right_click({}, {})",
                action.x.unwrap_or(0.0),
                action.y.unwrap_or(0.0)
            ),
            "type" => format!("type('{}')", action.text.as_deref().unwrap_or("")),
            "key_press" | "hotkey" => format!("hotkey({:?})", action.keys),
            "scroll" => format!(
                "scroll({}, {})",
                action.direction.as_deref().unwrap_or("down"),
                action.amount.unwrap_or(3)
            ),
            "drag" => format!(
                "drag({},{} -> {},{})",
                action.from_x.unwrap_or(0.0),
                action.from_y.unwrap_or(0.0),
                action.to_x.unwrap_or(0.0),
                action.to_y.unwrap_or(0.0)
            ),
            "open_app" => format!("open_app('{}')", action.app_name.as_deref().unwrap_or("")),
            "wait" => format!("wait({}ms)", action.duration_ms.unwrap_or(1000)),
            "done" | "complete" => "TASK_COMPLETE".to_string(),
            "fail" => "TASK_FAILED".to_string(),
            _ => format!("unknown({})", action.action_type),
        }
    }

    /// 截取屏幕截图并转为Base64编码
    fn capture_screenshot_base64(&self) -> Result<String> {
        let frame = screen::capture_screen()?;
        Ok(frame.to_base64())
    }
}
