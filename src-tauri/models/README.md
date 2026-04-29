# Embedding Model Files

This directory contains ONNX embedding model files that will be bundled into the application package.

## Required Files

- `model.onnx` - ONNX model file (e.g., `paraphrase-multilingual-MiniLM-L12-v2.onnx`)
- `tokenizer.json` - Tokenizer configuration
- (optional) `config.json` - Model configuration

## Download Instructions

Download from HuggingFace:
```bash
# Option A: Multilingual (recommended, ~470MB)
wget https://huggingface.co/sentence-transformers/paraphrase-multilingual-MiniLM-L12-v2/resolve/main/model.onnx -O model.onnx
wget https://huggingface.co/sentence-transformers/paraphrase-multilingual-MiniLM-L12-v2/resolve/main/tokenizer.json

# Option B: Chinese optimized (~90MB)
wget https://huggingface.co/nomic-ai/nomic-embed-text-v1.5/resolve/main/onnx/model.onnx -O model.onnx
wget https://huggingface.co/nomic-ai/nomic-embed-text-v1.5/resolve/main/tokenizer.json
```

## Notes

- If model files are missing at runtime, the system will fall back to Feature Hashing (128-dim) for compatibility.
- The LocalEmbedder module automatically detects and uses available models.
