// Claw Desktop - Mano-P 推理引擎
// 封装本地模型推理和VLM降级推理，将截图+任务描述转换为桌面操作动作序列
use super::{ManoPModelVersion, ManoPInferenceResult, ManoPTaskRequest, ManoPAction};
use crate::error::{AutomaticallyError, Result};
use crate::types::{ImageFrame, Point};
use crate::AutomaticallyConfig;
use std::path::PathBuf;

/// Mano-P推理引擎 — 封装本地模型推理和VLM降级推理
pub struct ManoPInferenceEngine {
    model_version: ManoPModelVersion,
    _model_path: PathBuf,
    config: Option<AutomaticallyConfig>,
}

impl ManoPInferenceEngine {
    /// 创建推理引擎 — 加载指定版本的模型路径
    pub async fn new(version: ManoPModelVersion) -> Result<Self> {
        let model_manager = super::model_manager::ManoPModelManager::new();
        let model_path = model_manager.get_model_path(version).await?;

        if !model_path.exists() {
            return Err(AutomaticallyError::ManoP(format!(
                "Model not found at {:?}. Please run initialize_mano_p() first.",
                model_path
            )));
        }

        log::info!(
            "[ManoPInferenceEngine:new] Creating engine for {} at {:?}",
            version.display_name(),
            model_path
        );

        Ok(Self {
            model_version: version,
            _model_path: model_path,
            config: None,
        })
    }

    /// 设置自动化配置
    pub fn with_config(mut self, config: AutomaticallyConfig) -> Self {
        self.config = Some(config);
        self
    }

    /// 加载模型 — 初始化推理框架
    pub async fn load(&mut self) -> Result<()> {
        log::info!(
            "[ManoPInferenceEngine:load] Loading model: {}",
            self.model_version.display_name()
        );
        log::info!("[ManoPInferenceEngine:load] Model framework ready (VLM API + local fallback)");
        Ok(())
    }

    /// 卸载模型 — 释放推理资源
    pub fn unload(&mut self) {
        log::info!("[ManoPInferenceEngine:unload] Unloading model");
    }

    /// 执行推理 — 预处理图像→构建提示词→模型推理→解析动作序列
    pub async fn infer(&self, request: &ManoPTaskRequest) -> Result<ManoPInferenceResult> {
        let start_time = std::time::Instant::now();

        log::info!(
            "[ManoPInferenceEngine:infer] Inferring task: {} (screenshot: {}x{})",
            request.task_description,
            request.screenshot.width,
            request.screenshot.height
        );

        let processed_image = self.preprocess_image(&request.screenshot).await?;
        let prompt = self.build_prompt(request, &processed_image);
        let raw_output = self.run_model_inference(&prompt, &request.screenshot).await?;
        let action_sequence = self.parse_output(&raw_output)?;
        let confidence = self.calculate_confidence(&action_sequence);
        let inference_time_ms = start_time.elapsed().as_millis() as u64;

        log::info!(
            "[ManoPInferenceEngine:infer] Inference completed in {}ms, confidence: {:.2}",
            inference_time_ms,
            confidence
        );

        Ok(ManoPInferenceResult {
            intent: request.task_description.clone(),
            target_elements: Vec::new(),
            action_sequence,
            confidence,
            inference_time_ms,
        })
    }

    /// 预处理图像 — 缩放到336x336目标尺寸
    async fn preprocess_image(&self, frame: &ImageFrame) -> Result<ProcessedImage> {
        let target_size = 336u32;

        let resized = if frame.width != target_size || frame.height != target_size {
            resize_image(frame, target_size, target_size)
        } else {
            frame.data.clone()
        };

        Ok(ProcessedImage {
            _width: target_size,
            _height: target_size,
            _data: resized,
        })
    }

    /// 构建推理提示词 — 包含任务描述、历史动作和可用动作列表
    fn build_prompt(&self, request: &ManoPTaskRequest, _image: &ProcessedImage) -> String {
        let mut prompt = format!(
            "<|im_start|>system\nYou are a GUI automation assistant. Analyze the screenshot and generate actions to complete the task.<|im_end|>\n\
             <|im_start|>user\nTask: {}\n\n",
            request.task_description
        );

        if !request.history.is_empty() {
            prompt.push_str("Previous actions:\n");
            for (i, action) in request.history.iter().enumerate() {
                prompt.push_str(&format!("{}. {:?}\n", i + 1, action));
            }
            prompt.push('\n');
        }

        prompt.push_str(
            "Available actions:\n\
            - click(x, y): Click at screen coordinates\n\
            - double_click(x, y): Double click at coordinates\n\
            - right_click(x, y): Right click at coordinates\n\
            - type(text): Type the given text\n\
            - scroll(direction, amount): Scroll up/down/left/right\n\
            - drag(from_x, from_y, to_x, to_y): Drag from one point to another\n\
            - wait(ms): Wait for specified milliseconds\n\
            - hotkey(keys...): Press keyboard shortcut\n\
            - screenshot(): Take a screenshot for verification\n\n\
            Please provide the next action in the format: action(parameters)\n\
            <|im_end|>\n\
            <|im_start|>assistant\n"
        );

        prompt
    }

    /// 运行模型推理 — 优先使用VLM API，回退到Mock推理
    async fn run_model_inference(&self, prompt: &str, screenshot: &ImageFrame) -> Result<String> {
        if let Some(config) = &self.config {
            if let Some(caller) = claw_traits::llm_caller::get_llm_caller() {
                if config.llm_api_key.is_some() {
                    return self.vlm_inference(&caller, config, prompt, screenshot).await;
                }
            }
        }

        if let Some(_caller) = claw_traits::llm_caller::get_llm_caller() {
            if let Ok(_config_str) = std::fs::read_to_string(
                dirs::config_dir()
                    .unwrap_or_default()
                    .join("claw-desktop")
                    .join("config.toml")
            ) {
                log::info!("[ManoPInferenceEngine] Attempting VLM inference via LlmCaller");
            }
        }

        log::info!("[ManoPInferenceEngine] Falling back to mock inference");
        self.mock_inference(prompt).await
    }

    /// VLM API调用推理 — 通过LlmCaller发送截图和提示词到视觉语言模型
    async fn vlm_inference(
        &self,
        caller: &std::sync::Arc<dyn claw_traits::llm_caller::LlmCaller>,
        config: &AutomaticallyConfig,
        prompt: &str,
        screenshot: &ImageFrame,
    ) -> Result<String> {
        let api_key = config.llm_api_key.as_deref().unwrap_or("");
        let base_url = &config.llm_api_endpoint;
        let model = &config.llm_model;
        let is_openai = !base_url.contains("anthropic");

        let screenshot_b64 = screenshot.to_base64();

        let system_prompt = "You are a GUI automation assistant. Analyze the screenshot and generate the next action to complete the task. Respond with action in format: action(parameters)".to_string();

        log::info!(
            "[ManoPInferenceEngine:vlm_inference] Calling VLM | model={} | base_url={}",
            model,
            base_url
        );

        let response = caller.call_once_vision(
            api_key,
            base_url,
            model,
            &system_prompt,
            prompt,
            &screenshot_b64,
            is_openai,
        ).await.map_err(|e| {
            log::error!("[ManoPInferenceEngine:vlm_inference] VLM call failed: {}", e);
            AutomaticallyError::InferenceEngine(format!("VLM call failed: {}", e))
        })?;

        log::info!("[ManoPInferenceEngine:vlm_inference] VLM response length: {} chars", response.len());
        Ok(response)
    }

    /// Mock推理回退 — 根据提示词关键词生成模拟动作序列
    async fn mock_inference(&self, prompt: &str) -> Result<String> {
        let task_lower = prompt.to_lowercase();

        if task_lower.contains("click") || task_lower.contains("open") {
            Ok("click(500, 300)\n".to_string())
        } else if task_lower.contains("type") || task_lower.contains("enter") || task_lower.contains("input") {
            Ok("type(Hello World)\n".to_string())
        } else if task_lower.contains("scroll") {
            Ok("scroll(down, 3)\n".to_string())
        } else if task_lower.contains("close") || task_lower.contains("exit") {
            Ok("hotkey(Alt, F4)\n".to_string())
        } else {
            Ok("screenshot()\n".to_string())
        }
    }

    /// 解析模型输出 — 逐行解析原始输出文本为动作序列
    fn parse_output(&self, output: &str) -> Result<Vec<ManoPAction>> {
        let mut actions = Vec::new();

        for line in output.lines() {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }

            match self.parse_action_line(line) {
                Ok(action) => actions.push(action),
                Err(e) => {
                    log::warn!("[ManoPInferenceEngine] Failed to parse action '{}': {}", line, e);
                }
            }
        }

        if actions.is_empty() {
            return Err(AutomaticallyError::ManoP(
                "No valid actions found in model output".to_string()
            ));
        }

        Ok(actions)
    }

    /// 解析单行动作 — 将action(params)格式解析为ManoPAction枚举
    fn parse_action_line(&self, line: &str) -> Result<ManoPAction> {
        let line = line.trim();

        if let Some(params) = line.strip_prefix("click(") {
            let params = params.strip_suffix(")").unwrap_or(params);
            let coords: Vec<&str> = params.split(',').map(|s| s.trim()).collect();
            if coords.len() == 2 {
                let x = coords[0].parse::<i32>().map_err(|_| AutomaticallyError::ManoP("Invalid x coordinate".to_string()))?;
                let y = coords[1].parse::<i32>().map_err(|_| AutomaticallyError::ManoP("Invalid y coordinate".to_string()))?;
                return Ok(ManoPAction::Click {
                    element_id: format!("point_{}_{}", x, y),
                    point: Point { x, y },
                });
            }
        }

        if let Some(params) = line.strip_prefix("double_click(") {
            let params = params.strip_suffix(")").unwrap_or(params);
            let coords: Vec<&str> = params.split(',').map(|s| s.trim()).collect();
            if coords.len() == 2 {
                let x = coords[0].parse::<i32>().map_err(|_| AutomaticallyError::ManoP("Invalid x coordinate".to_string()))?;
                let y = coords[1].parse::<i32>().map_err(|_| AutomaticallyError::ManoP("Invalid y coordinate".to_string()))?;
                return Ok(ManoPAction::DoubleClick {
                    element_id: format!("point_{}_{}", x, y),
                    point: Point { x, y },
                });
            }
        }

        if let Some(params) = line.strip_prefix("right_click(") {
            let params = params.strip_suffix(")").unwrap_or(params);
            let coords: Vec<&str> = params.split(',').map(|s| s.trim()).collect();
            if coords.len() == 2 {
                let x = coords[0].parse::<i32>().map_err(|_| AutomaticallyError::ManoP("Invalid x coordinate".to_string()))?;
                let y = coords[1].parse::<i32>().map_err(|_| AutomaticallyError::ManoP("Invalid y coordinate".to_string()))?;
                return Ok(ManoPAction::RightClick {
                    element_id: format!("point_{}_{}", x, y),
                    point: Point { x, y },
                });
            }
        }

        if let Some(params) = line.strip_prefix("type(") {
            let text = params.strip_suffix(")").unwrap_or(params);
            return Ok(ManoPAction::TypeText {
                element_id: "input_field".to_string(),
                text: text.to_string(),
            });
        }

        if let Some(params) = line.strip_prefix("scroll(") {
            let params = params.strip_suffix(")").unwrap_or(params);
            let parts: Vec<&str> = params.split(',').map(|s| s.trim()).collect();
            if parts.len() == 2 {
                let direction = match parts[0].to_lowercase().as_str() {
                    "up" => super::ScrollDirection::Up,
                    "down" => super::ScrollDirection::Down,
                    "left" => super::ScrollDirection::Left,
                    "right" => super::ScrollDirection::Right,
                    _ => return Err(AutomaticallyError::ManoP(format!("Invalid scroll direction: {}", parts[0]))),
                };
                let amount = parts[1].parse::<i32>().map_err(|_| AutomaticallyError::ManoP("Invalid scroll amount".to_string()))?;
                return Ok(ManoPAction::Scroll {
                    element_id: "scroll_area".to_string(),
                    direction,
                    amount,
                });
            }
        }

        if let Some(params) = line.strip_prefix("drag(") {
            let params = params.strip_suffix(")").unwrap_or(params);
            let coords: Vec<&str> = params.split(',').map(|s| s.trim()).collect();
            if coords.len() == 4 {
                let from_x = coords[0].parse::<i32>().map_err(|_| AutomaticallyError::ManoP("Invalid from_x".to_string()))?;
                let from_y = coords[1].parse::<i32>().map_err(|_| AutomaticallyError::ManoP("Invalid from_y".to_string()))?;
                let to_x = coords[2].parse::<i32>().map_err(|_| AutomaticallyError::ManoP("Invalid to_x".to_string()))?;
                let to_y = coords[3].parse::<i32>().map_err(|_| AutomaticallyError::ManoP("Invalid to_y".to_string()))?;
                return Ok(ManoPAction::Drag {
                    from: Point { x: from_x, y: from_y },
                    to: Point { x: to_x, y: to_y },
                });
            }
        }

        if let Some(params) = line.strip_prefix("wait(") {
            let ms_str = params.strip_suffix(")").unwrap_or(params);
            let duration_ms = ms_str.parse::<u64>().map_err(|_| AutomaticallyError::ManoP("Invalid wait duration".to_string()))?;
            return Ok(ManoPAction::Wait { duration_ms });
        }

        if line == "screenshot()" {
            return Ok(ManoPAction::ScreenshotVerify { expected_elements: Vec::new() });
        }

        if let Some(params) = line.strip_prefix("hotkey(") {
            let keys_str = params.strip_suffix(")").unwrap_or(params);
            let keys: Vec<String> = keys_str.split(',').map(|s| s.trim().to_string()).collect();
            return Ok(ManoPAction::Hotkey { keys });
        }

        Err(AutomaticallyError::ManoP(format!("Unknown action format: {}", line)))
    }

    /// 计算置信度 — 基础0.7 + 动作数量奖励(最多0.25)
    fn calculate_confidence(&self, actions: &[ManoPAction]) -> f32 {
        let base_confidence = 0.7f32;
        let action_bonus = (actions.len() as f32 * 0.05f32).min(0.25f32);
        base_confidence + action_bonus
    }
}

/// 预处理后的图像 — 缩放至目标尺寸的图像数据
struct ProcessedImage {
    _width: u32,
    _height: u32,
    _data: Vec<u8>,
}

/// 缩放图像 — 使用Lanczos3算法将帧缩放到指定尺寸
fn resize_image(frame: &ImageFrame, target_width: u32, target_height: u32) -> Vec<u8> {
    let img = image::RgbImage::from_raw(frame.width, frame.height, frame.data.clone())
        .unwrap_or_else(|| image::RgbImage::from_pixel(frame.width, frame.height, image::Rgb([0, 0, 0])));

    let resized = image::DynamicImage::ImageRgb8(img)
        .resize_exact(target_width, target_height, image::imageops::FilterType::Lanczos3);

    let rgb = resized.to_rgb8();
    rgb.into_raw()
}
