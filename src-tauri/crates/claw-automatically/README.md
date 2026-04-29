# Claw Automatically - 智能自动化引擎

> **定位**：电脑操作自动化、屏幕理解、多媒体处理的智能执行引擎

## 📌 核心作用

`claw-automatically` 是项目的 **UI自动化和视觉理解层**，负责：

1. **屏幕捕获与理解** - 截图、OCR文字识别、UI元素空间定位
2. **输入模拟** - 鼠标点击/键盘输入的跨平台模拟（Windows/Linux/macOS）
3. **多媒体处理** - 图片OCR、视频帧提取与逐帧分析、文件解析
4. **智能决策** - 基于LLM的自动化任务编排和执行

## 🏗️ 架构概览

```
┌─────────────────────────────────────────────────────────────┐
│                    用户请求 / 触发                            │
└──────────────────────────┬──────────────────────────────────┘
                           │
                           ▼
┌─────────────────────────────────────────────────────────────┐
│                  AutomaticallyEngine                         │
│                                                             │
│  ┌──────────┐  ┌──────────┐  ┌──────────┐  ┌──────────┐   │
│  │ Capture  │  │   OCR    │  │ Spatial  │  │   LLM    │   │
│  │ 屏幕捕获  │→│ 文字识别  │→│ 空间定位  │→│ 决策分析  │   │
│  └──────────┘  └──────────┘  └──────────┘  └────┬─────┘   │
│                                              │             │
│  ┌──────────┐  ┌──────────┐  ┌──────────┐     ▼             │
│  │  Input   │  │   File   │  │Automation│  ┌──────────┐    │
│  │ 输入模拟  │  │ 文件处理  │  │ 决策引擎  │  │ Verifier │    │
│  └──────────┘  └──────────┘  └──────────┘  └──────────┘    │
└─────────────────────────────────────────────────────────────┘
```

## 📦 模块结构

| 模块 | 功能 | 关键依赖 |
|-----|------|---------|
| `capture/screen.rs` | 屏幕截图 | `image` crate |
| `capture/video_processor.rs` | 视频解码/帧提取 | `ffmpeg-next` |
| `capture/image_decoder.rs` | 图片解码 | `image` crate |
| `ocr/engine.rs` | Tesseract OCR识别 | `ocr-rs` |
| `ocr/preprocessor.rs` | 图像预处理（灰度/二值化） | - |
| `spatial/cluster.rs` | OCR结果空间聚类 | - |
| `spatial/element_binding.rs` | UI元素类型绑定 | - |
| `spatial/output_formatter.rs` | 屏幕数据格式化输出 | - |
| `input/mouse.rs` | 鼠标操作模拟 | 平台API (Win32/X11/CoreGraphics) |
| `input/keyboard.rs` | 键盘操作模拟 | 平台API |
| `llm/client.rs` | LLM API客户端 | `reqwest` |
| `llm/core_adapter.rs` | Claw Core LLM适配器 | `claw-core` (可选) |
| `llm/prompt_builder.rs` | 提示词构建 | - |
| `llm/response_parser.rs` | LLM响应解析为操作指令 | - |
| `automation/decision_engine.rs` | 多步骤决策引擎 | - |
| `automation/task_executor.rs` | 任务执行编排 | - |
| `automation/verifier.rs` | 操作结果验证 | - |
| `file_handler/text_processor.rs` | 文本文件分块处理 | - |
| `file_handler/archive_extractor.rs` | 压缩包解压处理 | `zip` crate |
| `cache/session_manager.rs` | 会话缓存管理 | `dashmap` |
| `cache/cleanup.rs` | 缓存清理 | - |
| `config_integration.rs` | Claw Core配置集成 | `claw-core` (可选) |

## 🔧 核心类型

```rust
// 引擎配置
pub struct AutomaticallyConfig {
    pub ocr_language: String,           // OCR语言 ("chi_sim+eng")
    pub llm_api_endpoint: String,       // LLM API地址
    pub llm_api_key: Option<String>,    // API密钥
    pub llm_model: String,              // 模型名称
    pub screen_capture_fps: u32,        // 截图帧率
    pub session_ttl_seconds: i64,       // 会话TTL
}

// 屏幕图像帧
pub struct ImageFrame { width, height, data, timestamp }

// OCR识别结果
pub struct OcrResult { text, confidence, bbox, line_number }

// UI元素
pub struct UiElement { id, text, bbox, element_type, ... }

// 结构化屏幕数据
pub struct StructuredScreenData { screen_size, regions, timestamp }

// 执行操作指令
pub struct Operation {
    pub operation_type: String,     // "click"/"type"/"key_combination"
    pub target_x: f64,
    pub target_y: f64,
    pub text: Option<String>,
    pub keys: Option<Vec<String>>,
}

// 操作结果
pub struct OperationResult { success, operation, timestamp }

// 视频处理摘要
pub struct VideoSummary { file_path, duration, frames, extracted_text }
```

## 🚀 使用方式

### 基本自动化流程

```rust
use claw_automatically::{AutomaticallyEngine, AutomaticallyConfig};

let config = AutomaticallyConfig::default();
let engine = AutomaticallyEngine::new(config);

// 执行指令："点击登录按钮"
let result = engine.execute_instruction("点击登录按钮").await?;
// → 自动完成：截图 → OCR → 定位 → LLM决策 → 点击
```

### Tauri命令接口

| 命令 | 说明 |
|-----|------|
| `init_automatically_engine` | 初始化引擎(默认配置) |
| `execute_automation_instruction` | 执行自动化指令 |
| `capture_screen` | 截屏返回Base64图片 |
| `ocr_recognize_screen` | OCR识别屏幕文字 |
| `mouse_click(x, y)` | 模拟鼠标点击 |
| `keyboard_type(text)` | 模拟键盘输入 |

### Feature Flags

| Feature | 说明 |
|---------|------|
| `core-integration` | 启用Claw Core LLM集成，使用统一的LLM和配置 |
| (默认) | 独立运行，通过外部LLM API |

## 🎯 适用场景

### ✅ 应该使用 claw-automatically 的场景：

- **RPA自动化** - 表单填写、批量操作、重复性任务
- **屏幕理解** - "截图看看"、"识别这个界面"
- **UI交互** - "点击那个按钮"、"帮我输入xxx"
- **图片处理** - 发送图片进行OCR识别或视觉问答
- **视频分析** - 视频字幕提取、内容总结
- **文件处理** - 文档阅读、压缩包解析

### ❌ 不需要的场景：

- 纯文本对话 → 直接走 LLM
- 知识问答 → 直接走 LLM
- 代码生成 → 直接走 LLM

## 📊 数据流

```
用户指令："帮我在XX网站注册账号"
         ↓
[1] capture::screen::capture_screen()      // 截取当前屏幕
         ↓
[2] ocr::preprocessor::preprocess_frame()  // 图像预处理
         ↓
[3] ocr::engine::recognize()               // Tesseract OCR识别
         ↓
[4] spatial::process_ocr_results()         // 空间聚类+元素绑定
         ↓
[5] llm::prompt_builder::build_*()         // 构建LLM提示
         ↓
[6] llm::client::call_llm()                // 调用大模型决策
         ↓
[7] llm::response_parser::parse_*()        // 解析为Operation
         ↓
[8] input::mouse/keyboard::*()             // 执行鼠标/键盘操作
         ↓
[9] automation::verifier                   // 验证结果（可选）
```

## 🔗 依赖关系

```
claw-automatically
├── claw-core (可选，feature: core-integration)
│   ├── 统一LLM调用
│   └── 配置共享
├── ffmpeg-next (视频处理)
├── ocr-rs (Tesseract OCR)
├── image (图像编解码)
├── reqwest (HTTP客户端)
└── 平台特定:
    ├── Windows: windows-rs (Win32 API)
    ├── Linux: x11rb (X11协议)
    └── macOS: core-graphics + core-foundation
```

## 📁 文件清单

```
src/
├── lib.rs                    # 主入口，AutomaticallyEngine定义
├── commands.rs               # Tauri命令接口 (12个命令)
├── types.rs                  # 所有共享类型定义
├── error.rs                  # 错误类型
├── config_integration.rs     # Core配置集成
│
├── capture/
│   ├── mod.rs
│   ├── screen.rs            # 屏幕捕获实现
│   ├── video_processor.rs   # FFmpeg视频处理
│   └── image_decoder.rs     # 图片解码
│
├── ocr/
│   ├── mod.rs
│   ├── engine.rs            # Tesseract OCR引擎
│   └── preprocessor.rs      # 图像预处理
│
├── spatial/
│   ├── mod.rs
│   ├── cluster.rs           # 空间聚类算法
│   ├── element_binding.rs   # 元素绑定
│   └── output_formatter.rs  # 输出格式化
│
├── input/
│   ├── mod.rs
│   ├── mouse.rs             # 鼠标操作
│   └── keyboard.rs          # 键盘操作
│
├── llm/
│   ├── mod.rs
│   ├── client.rs            # LLM客户端
│   ├── core_adapter.rs      # Core LLM适配器
│   ├── prompt_builder.rs    # 提示构建
│   └── response_parser.rs   # 响应解析
│
├── automation/
│   ├── mod.rs
│   ├── decision_engine.rs   # 决策引擎
│   ├── task_executor.rs     # 任务执行器
│   └── verifier.rs          # 结果验证器
│
├── file_handler/
│   ├── mod.rs
│   ├── text_processor.rs    # 文本处理
│   └── archive_extractor.rs # 压缩包解压
│
└── cache/
    ├── mod.rs
    ├── session_manager.rs   # 会话管理
    └── cleanup.rs           # 缓存清理
```
