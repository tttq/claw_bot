# Claw WS - HTTP API 网关服务

> **定位**：项目的 HTTP API 服务层，提供 RESTful 接口供前端（Tauri/Web）调用后端功能

## 📌 核心作用

`claw-ws` 是项目的 **API网关和服务编排层**，负责：

1. **HTTP服务器** - 基于 Axum 框架提供 RESTful API 服务（端口 1421）
2. **认证鉴权** - JWT Token 认证、RSA 密钥交换、中间件拦截
3. **路由分发** - 18 个业务路由模块的统一注册和请求分发
4. **消息协议** - 定义 WebSocket/HTTP 统一的请求响应格式 (`WsRequest` / `WsResponse`)
5. **状态管理** - 全局应用状态 (`AppState`) 和配置管理

## 🏗️ 架构概览

```
┌─────────────────────────────────────────────────────────────┐
│                    客户端 (前端)                              │
│         Tauri Desktop / Web Browser                         │
└──────────────────────────┬──────────────────────────────────┘
                           │ HTTP REST API (Port 1421)
                           ▼
┌─────────────────────────────────────────────────────────────┐
│                    Axum HTTP Server                          │
│                                                             │
│  ┌─────────────────────────────────────────────────────┐   │
│  │              Middleware Chain                        │   │
│  │  CORS → Auth Middleware → Extension(AppState)       │   │
│  └─────────────────────────────────────────────────────┘   │
│                           │                                 │
│  ┌─────────────────────────────────────────────────────┐   │
│  │              Router Registry (18 routes)             │   │
│  ├─────────────────────────────────────────────────────┤   │
│  │ auth_routes        - 认证握手/登录                   │   │
│  │ config_routes      - 配置管理                       │   │
│  │ conversation_routes - 对话/聊天                     │   │
│  │ tool_routes        - 工具调用                       │   │
│  │ git_routes         - Git操作                        │   │
│  │ skill_routes       - 技能/Skill管理                 │   │
│  │ agent_routes       - Agent管理                      │   │
│  │ channel_routes     - 渠道管理(Telegram/Discord等)    │   │
│  │ persona_routes     - 人格/Persona设置               │   │
│  │ browser_routes     - 浏览器控制(CDP)                │   │
│  │ memory_routes      - 记忆系统(RAG)                  │   │
│  │ system_routes      - 系统信息/健康检查              │   │
│  │ system_agent_routes - 系统Agent                     │   │
│  │ multi_agent_routes - 多Agent协作                    │   │
│  │ fs_skill_routes    - 文件系统Skill                  │   │
│  │ iso_routes         - ISO镜像处理                    │   │
│  │ cmd_routes         - 命令行工具                      │   │
│  │ harness_routes     - Harness工程框架               │   │
│  └─────────────────────────────────────────────────────┘   │
│                           │                                 │
│  ┌─────────────────────────────────────────────────────┐   │
│  │              Core Services                           │   │
│  │  claw-core (配置/DB/LLM/RAG/Tools/Harness)          │   │
│  │  claw-channel (多渠道消息)                           │   │
│  └─────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────┘
```

## 📦 模块结构

### 核心模块

| 文件 | 功能 |
|-----|------|
| `lib.rs` | 主入口，模块导出 |
| `bootstrap.rs` | 服务启动初始化逻辑 |
| `commands.rs` | Tauri命令导出（auth相关命令） |

### WebSocket/协议层 (`ws/`)

| 模块 | 功能 |
|-----|------|
| `mod.rs` | WS模块入口 |
| `router.rs` | 路由分发器（已迁移到HTTP，保留兼容） |
| `router_registry.rs` | **HTTP路由注册中心**，合并所有18个路由模块 |
| `router_trait.rs` | `ClawRouter` trait 定义 |
| `server.rs` | HTTP服务器实例管理 |
| `app_state.rs` | 全局应用状态 `AppState` |
| `auth.rs` | JWT认证、Token验证 |
| `keygen.rs` | RSA密钥对生成 |
| `protocol.rs` | `WsRequest` / `WsResponse` 协议定义 |
| `response.rs` | 统一响应格式 `ApiResponse` |
| `middleware.rs` | HTTP中间件（认证拦截） |
| `agent_engine.rs` | Agent执行引擎 |
| `channel_handlers.rs` | 渠道消息处理器 |

### 路由模块 (`ws/routes/`)

| 路由模块 | 前缀 | 功能说明 |
|---------|------|---------|
| `auth_routes.rs` | `/api/auth` | 认证握手、JWT签发、公钥获取 |
| `config_routes.rs` | `/api/config` | 配置CRUD、导入导出 |
| `conversation_routes.rs` | `/api/conversations` | 对话管理、消息收发 |
| `tool_routes.rs` | `/api/tools` | 工具调用、列表、执行结果 |
| `git_routes.rs` | `/api/git` | Git仓库操作（clone/pull/commit/push） |
| `skill_routes.rs` | `/api/skills` | Skill安装、列表、市场 |
| `agent_routes.rs` | `/api/agents` | Agent CRUD、启停控制 |
| `channel_routes.rs` | `/api/channels` | 渠道账号管理、状态查询 |
| `persona_routes.rs` | `/api/personas` | 人格设置、切换 |
| `browser_routes.rs` | `/api/browser` | 浏览器CDP控制、标签页管理 |
| `memory_routes.rs` | `/api/memory` | RAG记忆增删查、向量搜索 |
| `system_routes.rs` | `/api/system` | 系统信息、健康检查、统计 |
| `system_agent_routes.rs` | `/api/system-agent` | 系统级Agent任务 |
| `multi_agent_routes.rs` | `/api/multi-agent` | 多Agent协作编排 |
| `fs_skill_routes.rs` | `/api/fs-skills` | 文件系统Skill扫描/加载 |
| `iso_routes.rs` | `/api/iso` | ISO镜像文件处理 |
| `cmd_routes.rs` | `/api/cmd` | Shell命令执行、工具列表 |
| `harness_routes.rs` | `/api/harness` | Harness工程框架API |

## 🔧 核心类型

### 应用状态

```rust
pub struct AppState {
    pub config: Arc<TokioMutex<AppConfig>>,  // 全局配置
}
```

### 统一响应格式

```rust
pub struct ApiResponse<T: Serialize> {
    pub success: bool,
    pub data: Option<T>,
    pub error: Option<String>,
    pub timestamp: DateTime<Utc>,
}
```

### 协议消息

```rust
pub struct WsRequest {
    pub id: String,           // 请求ID
    pub method: String,       // 方法名 (如 "auth_handshake")
    pub params: Value,        // 参数
    pub token: String,        // JWT Token
}

pub struct WsResponse {
    pub id: String,
    pub method: String,
    pub data: Option<Value>,
    pub error: Option<String>,
}
```

### 路由Trait

```rust
#[async_trait]
pub trait ClawRouter: Send + Sync {
    fn router(&self) -> Router;
}
```

## 🔐 认证流程

```
客户端                              Server (claw-ws)
  │                                    │
  ├── GET /api/auth/public-key ──────► │
  │                                    │ 返回 RSA 公钥
  ◄── { public_key: "..." } ──────────│
  │                                    │
  ├── POST /api/auth/handshake ──────► │
  │  { encrypted_secret: "..." }       │
  │                                    │ 解密 → 验证 → 签发JWT
  ◄── { token: "jwt..." } ────────────│
  │                                    │
  ├── GET /api/agents (Authorization: Bearer jwt) ──► │
  │                                    │ 验证JWT → 处理请求
  ◄── { data: [...] } ────────────────│
```

## 🚀 启动方式

```rust
use claw_ws::router_registry;

// 在 main.rs 中启动
let app_state = claw_ws::app_state::AppState::new(config.clone());
let state = std::sync::Arc::new(app_state);

match claw_ws::router_registry::start_http_server(state, 1421).await {
    Ok(port) => {
        log::info!("HTTP server started on port {}", port);
    }
    Err(e) => {
        log::error!("Failed to start HTTP server: {}", e);
    }
}
```

## 📊 API端点总览

### 认证类

| 方法 | 路径 | 说明 |
|-----|------|------|
| GET | `/api/auth/public-key` | 获取RSA公钥 |
| POST | `/api/auth/handshake` | 认证握手获取JWT |

### 配置类

| 方法 | 路径 | 说明 |
|-----|------|------|
| GET | `/api/config` | 获取当前配置 |
| PUT | `/api/config` | 更新配置 |
| POST | `/api/config/export` | 导出配置文件 |

### Agent类

| 方法 | 路径 | 说明 |
|-----|------|------|
| GET | `/api/agents` | 列出所有Agent |
| POST | `/api/agents` | 创建Agent |
| PUT | `/api/agents/:id` | 更新Agent |
| DELETE | `/api/agents/:id` | 删除Agent |

### 对话类

| 方法 | 路径 | 说明 |
|-----|------|------|
| GET | `/api/conversations` | 获取对话列表 |
| POST | `/api/conversations` | 发送消息 |
| GET | `/api/conversations/:id/history` | 获取历史记录 |

### 工具类

| 方法 | 路径 | 说明 |
|-----|------|------|
| GET | `/api/tools` | 列出所有可用工具 |
| POST | `/api/tools/:name/exec` | 执行指定工具 |
| GET | `/api/tools/:name/result` | 获取执行结果 |

*（更多端点请参考各routes文件）*

## 🔗 依赖关系

```
claw-ws
├── claw-core                 # 核心能力(配置/DB/LLM/RAG/Tools)
├── claw-channel              # 多渠道消息支持
│
├── axum                      # Web框架
├── tower-http                # CORS/中间件
├── tokio-tungstenite         # WebSocket (历史遗留)
├── jsonwebtoken              # JWT认证
├── rsa                       # RSA加密
├── reqwest                   # HTTP客户端
├── serde + serde_json        # 序列化
└── tauri                     # Tauri集成
```

## 📁 文件清单

```
src/
├── lib.rs                          # 主入口
├── bootstrap.rs                    # 服务初始化
├── commands.rs                     # Tauri命令导出 (auth)
│
├── ws/
│   ├── mod.rs                      # WS模块入口
│   ├── router.rs                   # 路由分发(兼容层)
│   ├── router_registry.rs          # ★ HTTP路由注册中心
│   ├── router_trait.rs             # ClawRouter trait
│   ├── server.rs                   # 服务器实例
│   ├── app_state.rs                # AppState定义
│   ├── auth.rs                     # JWT认证
│   ├── keygen.rs                   # RSA密钥生成
│   ├── protocol.rs                 # WsRequest/WsResponse
│   ├── response.rs                 # ApiResponse统一格式
│   ├── middleware.rs               # 认证中间件
│   ├── agent_engine.rs             # Agent引擎
│   └── channel_handlers.rs         # 渠道处理器
│
└── ws/routes/                      # 18个业务路由
    ├── auth_routes.rs              # 认证
    ├── config_routes.rs            # 配置
    ├── conversation_routes.rs      # 对话
    ├── tool_routes.rs              # 工具
    ├── git_routes.rs               # Git
    ├── skill_routes.rs             # Skill
    ├── agent_routes.rs             # Agent
    ├── channel_routes.rs           # 渠道
    ├── persona_routes.rs           # 人格
    ├── browser_routes.rs           # 浏览器
    ├── memory_routes.rs            # 记忆/RAG
    ├── system_routes.rs            # 系统
    ├── system_agent_routes.rs      # 系统Agent
    ├── multi_agent_routes.rs       # 多Agent
    ├── fs_skill_routes.rs          # 文件Skill
    ├── iso_routes.rs               # ISO镜像
    ├── cmd_routes.rs               # 命令行
    └── harness_routes.rs           # Harness
```

## 💡 设计特点

1. **双协议支持** - 原生WebSocket + 现代HTTP REST（已全面迁移到HTTP）
2. **模块化路由** - 每个业务域独立路由文件，实现 `ClawRouter` trait 即可注册
3. **安全认证** - RSA密钥交换 + JWT Token + 中间件拦截
4. **统一响应** - 所有API返回标准 `ApiResponse<T>` 格式
5. **状态共享** - 通过 Axum `Extension` 注入全局 `AppState`
6. **CORS支持** - 完整的跨域配置，方便前后端分离开发
7. **热更新** - 配置通过 `AppState` 动态更新，无需重启服务
