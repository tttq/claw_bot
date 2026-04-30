// Claw Desktop - 本地嵌入器 - 本地文本向量化（ONNX Runtime）
pub use claw_types::common::EMBEDDING_DIM;

#[cfg(feature = "onnx-embedding")]
use std::path::Path;
#[cfg(feature = "onnx-embedding")]
use std::sync::OnceLock;

#[cfg(feature = "onnx-embedding")]
static EMBEDDER: OnceLock<std::sync::Mutex<LocalEmbedder>> = OnceLock::new();

/// 本地嵌入器 — 基于ONNX Runtime的本地文本向量化
///
/// 使用预训练的ONNX模型将文本转换为EMBEDDING_DIM维向量，
/// 支持批量嵌入和自动降级到特征哈希
#[cfg(feature = "onnx-embedding")]
pub struct LocalEmbedder {
    session: ort::session::Session,
    tokenizer: tokenizers::Tokenizer,
}

#[cfg(feature = "onnx-embedding")]
impl LocalEmbedder {
    /// 加载ONNX模型和分词器 — 从指定目录加载model.onnx和tokenizer.json
    pub fn new(model_dir: &Path) -> Result<Self, String> {
        let onnx_path = model_dir.join("model.onnx");
        if !onnx_path.exists() {
            return Err(format!("ONNX model not found at {:?}", onnx_path));
        }

        let tokenizer_path = model_dir.join("tokenizer.json");
        if !tokenizer_path.exists() {
            return Err(format!("Tokenizer not found at {:?}", tokenizer_path));
        }

        log::info!("[LocalEmbedder] Loading ONNX model from {:?}", onnx_path);
        let session = ort::session::Session::builder()
            .map_err(|e| format!("Failed to create ORT session builder: {}", e))?
            .commit_from_file(&onnx_path)
            .map_err(|e| format!("Failed to load ONNX model: {}", e))?;

        let mut t1 = tokenizers::Tokenizer::from_file(&tokenizer_path)
            .map_err(|e| format!("Failed to load tokenizer: {}", e))?;
        let t2 = t1.with_padding(None);
        let t3 = t2
            .with_truncation(Some(tokenizers::TruncationParams {
                max_length: 512,
                strategy: tokenizers::TruncationStrategy::LongestFirst,
                ..Default::default()
            }))
            .map_err(|e| format!("Failed to set truncation: {}", e))?;
        let _ = t3.add_special_tokens(&[
            tokenizers::AddedToken::from("[PAD]", true),
            tokenizers::AddedToken::from("[UNK]", true),
            tokenizers::AddedToken::from("[CLS]", true),
            tokenizers::AddedToken::from("[SEP]", true),
            tokenizers::AddedToken::from("[MASK]", true),
        ]);

        log::info!(
            "[LocalEmbedder] Model loaded successfully (dim={})",
            EMBEDDING_DIM
        );
        Ok(Self {
            session,
            tokenizer: t3.clone().into(),
        })
    }

    /// 嵌入单条文本 — 分词→ONNX推理→均值池化→L2归一化
    pub fn embed(&mut self, text: &str) -> Result<Vec<f32>, String> {
        if text.trim().is_empty() {
            return Ok(vec![0.0f32; EMBEDDING_DIM]);
        }

        let encoding = self
            .tokenizer
            .encode(text, true)
            .map_err(|e| format!("Tokenization failed: {}", e))?;

        let input_ids: Vec<i64> = encoding.get_ids().iter().map(|&id| id as i64).collect();
        let attention_mask: Vec<i64> = encoding
            .get_attention_mask()
            .iter()
            .map(|&m| m as i64)
            .collect();
        let token_type_ids: Vec<i64> = vec![0i64; input_ids.len()];
        let seq_len = input_ids.len();

        let input_ids_tensor =
            ort::value::Tensor::from_array((vec![1i64, seq_len as i64], input_ids))
                .map_err(|e| format!("Failed to create input_ids tensor: {}", e))?;
        let attention_mask_tensor =
            ort::value::Tensor::from_array((vec![1i64, seq_len as i64], attention_mask))
                .map_err(|e| format!("Failed to create attention_mask tensor: {}", e))?;
        let token_type_ids_tensor =
            ort::value::Tensor::from_array((vec![1i64, seq_len as i64], token_type_ids))
                .map_err(|e| format!("Failed to create token_type_ids tensor: {}", e))?;

        let inputs: std::collections::HashMap<String, ort::value::Value> = [
            ("input_ids".to_string(), input_ids_tensor.into()),
            ("attention_mask".to_string(), attention_mask_tensor.into()),
            ("token_type_ids".to_string(), token_type_ids_tensor.into()),
        ]
        .into();

        let outputs = self
            .session
            .run(inputs)
            .map_err(|e| format!("Inference failed: {}", e))?;

        let (shape, data_slice) = outputs[0]
            .try_extract_tensor::<f32>()
            .map_err(|e| format!("Failed to extract output tensor: {}", e))?;
        let hidden_size = shape[2];
        let data = data_slice.to_vec();

        let mut embedding = vec![0.0f32; hidden_size as usize];
        for i in 0..hidden_size as usize {
            let mut sum = 0.0f32;
            for j in 0..seq_len {
                sum += data[j * hidden_size as usize + i];
            }
            embedding[i] = sum / seq_len as f32;
        }

        let norm: f32 = embedding
            .iter()
            .map(|x| x * x)
            .sum::<f32>()
            .sqrt()
            .max(1e-6);
        for v in embedding.iter_mut() {
            *v /= norm;
        }

        Ok(embedding)
    }

    /// 批量嵌入文本 — 逐条调用embed方法
    pub fn embed_batch(&mut self, texts: &[String]) -> Result<Vec<Vec<f32>>, String> {
        texts.iter().map(|t| self.embed(t)).collect()
    }
}

/// 初始化全局嵌入器 — 加载ONNX模型到全局单例
#[cfg(feature = "onnx-embedding")]
pub fn init_embedder(model_dir: &Path) -> Result<(), String> {
    let embedder = LocalEmbedder::new(model_dir)?;
    EMBEDDER
        .set(std::sync::Mutex::new(embedder))
        .map_err(|_| "Embedder already initialized".to_string())?;
    Ok(())
}

/// 获取全局嵌入器实例 — 返回OnceLock中的静态引用
#[cfg(feature = "onnx-embedding")]
pub fn get_embedder() -> Option<&'static std::sync::Mutex<LocalEmbedder>> {
    EMBEDDER.get()
}

/// 文本嵌入降级方案 — 优先使用ONNX本地模型，失败时降级到特征哈希
pub fn embed_text_fallback(text: &str) -> Vec<f32> {
    #[cfg(feature = "onnx-embedding")]
    if let Some(embedder_mutex) = get_embedder() {
        if let Ok(mut embedder) = embedder_mutex.lock() {
            match embedder.embed(text) {
                Ok(vec) => return vec,
                Err(e) => log::warn!("[LocalEmbedder] Embed failed, using fallback: {}", e),
            }
        }
    }
    feature_hashing_fallback(text)
}

/// 特征哈希降级 — 基于TF和哈希的简单向量化
///
/// 将文本分词后，对每个词哈希到EMBEDDING_DIM维向量中，
/// 使用TF加权，最后L2归一化
fn feature_hashing_fallback(text: &str) -> Vec<f32> {
    use std::collections::HashMap;
    const FALLBACK_DIM: usize = EMBEDDING_DIM;
    let mut vec = vec![0.0f32; FALLBACK_DIM];
    if text.is_empty() {
        return vec;
    }

    let mut word_counts: HashMap<u64, u32> = HashMap::new();
    let mut total_words: u32 = 0;
    for word in text.split(|c: char| !c.is_alphanumeric()) {
        let w = word.to_lowercase();
        if w.len() < 2 {
            continue;
        }
        let hash = simple_hash(&w);
        *word_counts.entry(hash).or_insert(0) += 1;
        total_words += 1;
    }
    if total_words == 0 {
        return vec;
    }
    for (&hash, &count) in &word_counts {
        let idx = (hash as usize) % FALLBACK_DIM;
        let tf = (count as f32) / (total_words as f32).sqrt();
        vec[idx] += tf;
    }
    let norm: f32 = vec.iter().map(|x| x * x).sum::<f32>().sqrt().max(1e-6);
    for v in vec.iter_mut() {
        *v /= norm;
    }
    vec
}

/// 简单哈希函数 — 使用SHA256生成u64哈希值
fn simple_hash(s: &str) -> u64 {
    use sha2::Digest;
    let mut hasher = sha2::Sha256::new();
    hasher.update(s.as_bytes());
    let result = hasher.finalize();
    u64::from_le_bytes(result[0..8].try_into().unwrap_or([0u8; 8]))
}
