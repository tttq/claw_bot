// Claw Desktop - LLM调用器Trait - 定义单次LLM调用的统一接口
// 解决 claw-tools ↔ claw-llm 循环依赖：claw-tools 通过 trait 调用 LLM，无需直接依赖 claw-llm

use std::sync::OnceLock;

static LLM_CALLER: OnceLock<std::sync::Arc<dyn LlmCaller>> = OnceLock::new();

#[async_trait::async_trait]
pub trait LlmCaller: Send + Sync {
    async fn call_once(
        &self,
        api_key: &str,
        base_url: &str,
        model: &str,
        system_prompt: &str,
        user_message: &str,
        is_openai: bool,
    ) -> Result<String, String>;

    async fn call_once_vision(
        &self,
        api_key: &str,
        base_url: &str,
        model: &str,
        system_prompt: &str,
        user_message: &str,
        image_base64: &str,
        is_openai: bool,
    ) -> Result<String, String> {
        let _ = (
            api_key,
            base_url,
            model,
            system_prompt,
            user_message,
            image_base64,
            is_openai,
        );
        Err("Vision not supported by this LlmCaller implementation".to_string())
    }
}

pub fn set_llm_caller(caller: std::sync::Arc<dyn LlmCaller>) {
    let _ = LLM_CALLER.set(caller);
}

pub fn get_llm_caller() -> Option<std::sync::Arc<dyn LlmCaller>> {
    LLM_CALLER.get().cloned()
}

pub fn is_llm_caller_registered() -> bool {
    LLM_CALLER.get().is_some()
}
