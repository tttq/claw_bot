// Claw Desktop - 渠道流式 - 渠道消息的流式输出
use crate::error::ChannelResult;
use crate::types::*;
use std::sync::Arc;
use std::time::Duration;
use tokio::time::sleep;

pub struct StreamingController {
    config: StreamingConfig,
}

impl StreamingController {
    pub fn new(config: StreamingConfig) -> Self {
        Self { config }
    }

    pub async fn process_stream(
        &self,
        full_text: String,
        on_chunk: Arc<dyn Fn(String) + Send + Sync>,
    ) -> ChannelResult<String> {
        if !self.config.enabled || self.config.mode == StreamingMode::Off {
            on_chunk(full_text.clone());
            return Ok(full_text);
        }

        match self.config.mode {
            StreamingMode::Partial => self.stream_by_sentence(full_text, on_chunk).await,
            StreamingMode::Block => self.stream_by_block(full_text, on_chunk).await,
            StreamingMode::Off => {
                on_chunk(full_text.clone());
                Ok(full_text)
            }
        }
    }

    async fn stream_by_sentence(
        &self,
        text: String,
        on_chunk: Arc<dyn Fn(String) + Send + Sync>,
    ) -> ChannelResult<String> {
        let chunk_size = self.config.chunk_size.unwrap_or(100);

        let mut accumulated = String::new();
        let chars: Vec<char> = text.chars().collect();
        let mut i = 0;

        while i < chars.len() {
            let end = (i + chunk_size).min(chars.len());
            let chunk: String = chars[i..end].iter().collect();
            accumulated.push_str(&chunk);

            on_chunk(accumulated.clone());

            if let Some(d) = self.config.edit_delay_ms {
                sleep(Duration::from_millis(d)).await;
            }

            i = end;
        }

        Ok(accumulated)
    }

    async fn stream_by_block(
        &self,
        text: String,
        on_chunk: Arc<dyn Fn(String) + Send + Sync>,
    ) -> ChannelResult<String> {
        let delimiter = "\n\n";
        let blocks: Vec<&str> = text.split(delimiter).collect();
        let mut accumulated = String::new();

        for (i, block) in blocks.iter().enumerate() {
            if i > 0 {
                accumulated.push_str(delimiter);
            }
            accumulated.push_str(block);

            on_chunk(accumulated.clone());

            if let Some(d) = self.config.edit_delay_ms {
                sleep(Duration::from_millis(d)).await;
            }
        }

        Ok(accumulated)
    }
}
