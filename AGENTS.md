# AGENTS.md - Claw Desktop 强制规则

> 本文件定义了所有 AI Agent 在本项目工作时必须遵守的强制规则。违反这些规则将导致代码质量下降、构建失败或运行时错误。

---

## 1. 项目概览

- **项目名称**: Claw Desktop (qclaw-desktop)
- **项目类型**: Tauri 2.x 桌面应用
- **技术栈**: Rust (后端) + React 18 / TypeScript 5.6 (前端) + Tailwind CSS 3.4
- **定位**: AI Agent 工作台 — 多模型支持、工具循环、RAG 记忆、Skills 系统
- **Rust Edition**: 2024
- **包管理**: pnpm/npm (前端) / cargo (后端 workspace)

---

## 2. 项目结构约束

```
qclaw-desktop/
├── src-tauri/                    # Rust 后端 (Tauri)
│   ├── src/
│   │   ├── main.rs               # 程序入口 (禁止随意修改)
│   │   ├── lib.rs                # 主模块 (~2063行, 应用初始化+命令注册)
│   │   ├── config.rs             # 配置管理
│   │   ├── database.rs           # 数据库操作封装
│   │   ├── error.rs              # 错误处理
│   │   ├── encryption.rs         # 加密模块
│   │   ├── streaming.rs          # 流式处理
│   │   ├── bootstrap.rs          # 引导逻辑
│   │   ├── inbound.rs            # 入站消息处理
│   │   ├── registry.rs           # 注册中心
│   │   ├── traits.rs             # 共享 trait 定义
│   │   ├── types.rs              # 核心类型定义
│   │   ├── commands/             # Tauri 命令 (每个命令一个模块)
│   │   ├── llm/                  # LLM 交互核心
│   │   ├── rag/                  # RAG 记忆系统
│   │   ├── tools/                # 工具系统
│   │   │   └── tools/            # 具体工具实现
│   │   ├── ws/                   # WebSocket 服务
│   │   ├── db/                   # Sea-ORM 数据库层
│   │   ├── harness/              # Agent 管理系统
│   │   ├── config/               # 配置模块 (crate)
│   │   ├── plugins/              # 插件系统 (Discord/Telegram)
│   │   └── channel/              # 通道模块
│   ├── crates/                   # Workspace 内部 crate
│   │   ├── claw-config/          # 配置管理 crate
│   │   ├── claw-db/              # 数据库 crate
│   │   ├── claw-llm/             # LLM crate
│   │   ├── claw-rag/             # RAG crate
│   │   ├── claw-harness/         # Agent 管理 crate
│   │   ├── claw-tools/           # 工具系统 crate
│   │   ├── claw-ws/              # WebSocket crate
│   │   ├── claw-channel/         # 通道 crate
│   │   └── claw-desktop-lib/     # 桌面库 crate
│   ├── bundled-skills/           # 内置技能 (编译时嵌入, SKILL.md 格式)
│   ├── Cargo.toml                # Workspace 配置
│   └── tauri.conf.json           # Tauri 应用配置
├── src/                          # React 前端
│   ├── main.tsx                  # React 入口
│   ├── App.tsx                   # 根组件
│   ├── components/               # UI 组件
│   │   ├── layout/               # 布局组件
│   │   ├── chat/                 # 聊天相关组件
│   │   ├── settings/             # 设置组件
│   │   └── common/               # 通用组件
│   ├── hooks/                    # React Hooks (use*.ts)
│   ├── lib/                      # 工具函数库
│   │   ├── api.ts                # Tauri invoke 封装 (唯一 IPC 入口)
│   │   ├── utils.ts              # 通用工具函数
│   │   ├── constants.ts          # 常量定义
│   │   └── types.ts              # TypeScript 类型定义
│   ├── i18n/                     # 国际化
│   │   ├── config.ts             # i18next 配置
│   │   └── locales/              # 语言文件
│   │       ├── zh-CN.json        # 简体中文 (默认语言)
│   │       ├── zh-TW.json        # 繁体中文
│   │       └── en.json           # 英语
│   └── styles/globals.css        # 全局样式 (Tailwind)
├── public/                       # 静态资源
├── package.json                  # 前端依赖
├── tsconfig.json                 # TypeScript 配置 (strict: true)
├── vite.config.ts                # Vite 构建配置
├── tailwind.config.js            # Tailwind 配置
└── config.toml                   # 默认配置模板
```

---

## 3. Rust 后端规则

### 3.1 代码风格

- **Edition**: 使用 Rust 2024 edition
- **错误处理**: 统一使用 `Result<T, String>` 作为 Tauri command 返回类型
- **日志规范**: 必须使用结构化日志格式

```rust
// 正确的日志格式
log::info!("[ModuleName:FunctionName] Description | detail={}", value);
log::warn!("[ModuleName:FunctionName] Warning message");
log::error!("[ModuleName:FunctionName] Error: {}", error);

// 禁止的格式
log::info!("something happened");  // 缺少模块前缀
println!("...");                    // 禁止使用 println!
```

### 3.2 Tauri Command 规范

```rust
// 所有 command 必须遵循此签名模式
#[tauri::command]
async fn my_command(param: String, optional_param: Option<i32>) -> Result<serde_json::Value, String> {
    log::info!("[MyModule:my_command] Executing | param={}", param);
    
    // 业务逻辑使用 ? 操作符传播错误
    let result = do_something().await.map_err(|e| {
        log::error!("[MyModule:my_command] Failed: {}", e);
        e.to_string()
    })?;
    
    Ok(serde_json::to_value(result).unwrap_or(serde_json::Value::Null))
}
```

### 3.3 异步运行时

- 使用 `tokio` 作为异步运行时 (已在 workspace dependencies 中配置 full features)
- 后台任务必须通过 `tauri::async_runtime::spawn` 启动
- **禁止** 在 Tauri command 中阻塞主线程

```rust
// 正确: 使用 spawn 处理长时间运行的任务
tauri::async_runtime::spawn(async move {
    long_running_task().await;
});

// 错误: 阻塞 command
#[tauri::command]
async fn bad_command() -> Result<(), String> {
    std::thread::sleep(std::time::Duration::from_secs(10)); // 禁止!
    Ok(())
}
```

### 3.4 Workspace Crate 依赖

- 内部 crate 依赖通过 `workspace = true` 引用
- 新增内部 crate 必须在根 `Cargo.toml` 的 `[workspace.members]` 和 `[workspace.dependencies]` 中注册
- 共享依赖 (serde, tokio, anyhow 等) 统一在 workspace 层管理，**禁止**在各 crate 中单独声明版本

### 3.5 数据库规则

- ORM: Sea-ORM + SQLite (sqlx-sqlite driver)
- 数据库实体位于 `src-tauri/src/db/db/entities/`
- 所有数据库操作必须通过 Database 连接池
- **禁止**直接执行原始 SQL 字符串拼接 (SQL 注入风险)

### 3.6 关键常量约束

以下常量值**禁止修改**（除非经过架构评审）:

```rust
const MAX_TOOL_ROUNDS: usize = 15;              // 最大工具循环轮次
const TOTAL_LOOP_TIMEOUT_SECS: u64 = 180;       // 总超时时间 3分钟
const MAX_SAME_TOOL_CONSECUTIVE: usize = 4;     // 同一工具最大连续调用
const MAX_API_RETRIES: usize = 3;               // API 最大重试次数
const CONTEXT_OVERFLOW_MAX_RETRIES: usize = 2;  // 上下文溢出重试
const INCREMENTAL_SAVE_INTERVAL: usize = 3;     // 增量保存间隔
```

---

## 4. 前端规则

### 4.1 TypeScript 规范

- **strict mode 已启用** (`tsconfig.json`: `"strict": true`)
- `noUnusedLocals` 和 `noUnusedParameters` 为 `false` (允许未使用的变量)
- `jsx`: `react-jsx` (自动 runtime import)

```typescript
// 正确: 明确的类型定义
interface Conversation {
  id: string;
  title: string;
  createdAt: number;
}

// 正确: 使用 interface 或 type
type ToolCall = {
  id: string;
  name: string;
  input: Record<string, unknown>;
};
```

### 4.2 React 规范

- 函数组件 + Hooks (禁止 Class 组件)
- 组件文件名: PascalCase (如 `ChatWindow.tsx`)
- Hook 文件名: use* 前缀 (如 `useChat.ts`)
- 工具函数文件名: camelCase (如 `api.ts`, `utils.ts`)

```typescript
// 正确: 函数组件 + 解构 hooks
function ChatWindow({ conversationId }: { conversationId: string }) {
  const [messages, setMessages] = useState<Message[]>([]);
  const { streamingText, send } = useChat(conversationId);
  
  return <div>...</div>;
}

// 禁止: Class 组件
class OldComponent extends React.Component { ... }  // 禁止
```

### 4.3 样式规则

- **仅使用 Tailwind CSS** (禁止引入其他 CSS-in-JS 库)
- 全局样式仅在 `src/styles/globals.css` 中定义
- 组件优先使用 Tailwind utility classes

```tsx
// 正确: Tailwind utility classes
<div className="flex items-center gap-4 p-4 bg-gray-900 rounded-lg">
  <span className="text-sm font-medium text-white">内容</span>
</div>

// 禁止: 内联 style 对象 (除非动态计算值)
<div style={{ color: 'red', padding: '10px' }}>  // 避免
```

### 4.4 IPC 通信规则

- **所有 Tauri invoke 调用必须封装在 `src/lib/api.ts` 中**
- **禁止**在组件中直接调用 `invoke()`
- 流式事件监听必须在 `useEffect` 中注册并在 cleanup 中取消

```typescript
// 正确: 封装在 api.ts
// src/lib/api.ts
export async function getConfig(): Promise<AppConfig> {
  return invoke<AppConfig>('get_config');
}

export async function sendMessage(conversationId: string, content: string): Promise<ChatResponse> {
  return invoke<ChatResponse>('send_message', { conversationId, content });
}

// 正确: 事件监听带清理
useEffect(() => {
  let unlisten: (() => void) | undefined;
  
  listen<string>('chat-stream-token', (event) => {
    handleToken(event.payload);
  }).then((fn) => { unlisten = fn; });
  
  return () => { unlisten?.(); };  // 必须清理
}, []);
```

### 4.5 i18n 国际化规则

- **所有用户可见文本必须使用 i18n**
- 翻译 key 命名: 点分分层 (如 `chat.send`, `settings.model.title`)
- 支持的语言: `zh-CN`(默认), `zh-TW`, `en`
- 翻译文件位置: `src/i18n/locales/{lang}.json`
- i18n 配置: `src/i18n/config.ts`
- 使用 react-i18next 的 `useTranslation` hook

```typescript
// 正确: 使用 t() 函数
const { t } = useTranslation();
<button>{t('chat.send')}</button>
<span>{t('settings.model.title')}</span>

// 禁止: 硬编码中文/英文文本
<button>发送</button>        // 禁止!
<button>Send</button>       // 禁止!
```

---

## 5. 技能系统 (Skills) 规则

### 5.1 SKILL.md 文件格式

内置技能位于 `src-tauri/bundled-skills/{skill_name}/SKILL.md`，必须遵循以下格式：

```markdown
---
name: skill_name
description: 技能描述
when_to_use: 使用场景说明
allowed-tools: ["Tool1", "Tool2"]
argument_hint: <参数提示>
user_invocable: true
version: 1.0.0
model: claude-sonnet-4
effort: medium
---

# Skill Name

技能详细说明...
```

### 5.2 技能加载顺序

1. 内置技能 (bundled-skills/, 编译时 embed)
2. 用户自定义技能 (`~/.claw-desktop/skills/*.md`)
3. 同名技能: 内置优先 (用户自定义不会覆盖内置)

---

## 6. Git 规则

### 6.1 提交信息规范

使用约定式提交 (Conventional Commits):

```
<type>(<scope>): <subject>

<body>
```

**Type**: feat | fix | docs | style | refactor | perf | test | build | ci | chore | revert

**Scope**: rust | frontend | i18n | db | tools | llm | rag | ws | config | harness

示例:
```
feat(tools): add docker container management tool

fix(rag): resolve memory unit embedding dimension mismatch

i18n: add zh-TW translations for settings panel
```

---

## 7. 构建与开发命令

```bash
# 开发模式 (前端热重载 + Tauri 窗口)
pnpm tauri dev

# 仅前端开发服务器
pnpm dev

# 前端生产构建
pnpm build

# Rust 编译检查
cd src-tauri && cargo check

# 完整生产构建
pnpm tauri build

# 运行测试
cd src-tauri && cargo test

# 日志调试
RUST_LOG=debug pnpm tauri dev
RUST_LOG=claw_desktop::llm=debug,claw_desktop::tools=trace pnpm tauri dev
```

---

## 8. 安全规则

### 8.1 禁止事项

- **禁止**在代码中硬编码 API Key、密码、Token 等敏感信息
- **禁止**将密钥提交到版本控制
- **禁止**在日志中输出完整 API Key 或 Token (可输出前4位***后4位)
- **禁止**使用 `shell_tools` 执行未经验证的用户输入
- **禁止**在 SQL 查询中使用字符串拼接 (必须参数化查询)

### 8.2 敏感信息处理

```rust
// 正确: 脱敏日志
log::info!("[LLM] Using API key: {}...{}", &key[..4], &key[key.len()-4..]);

// 错误: 泄露密钥
log::info!("[LLM] API key: {}", api_key);  // 禁止!
```

---

## 9. 错误处理规则

### 9.1 LLM 错误分类

Agent 循环中的错误必须正确分类以触发对应的恢复策略:

| 错误类型 | HTTP 状态码 | 恢复策略 |
|---------|------------|---------|
| RateLimit | 429 | 指数退避重试 (最多3次) |
| AuthError | 401/403 | 不重试，返回友好提示 |
| ServerError | 5xx | 固定间隔重试 (最多2次) |
| ContextOverflow | N/A | 触发 RAG 历史压缩 |
| Timeout | N/A | 渐进延迟重试 + 可能压缩 |
| NetworkError | N/A | 指数退避重试 (最多3次) |

### 9.2 前端错误处理

```typescript
// 所有 IPC 调用必须有 try-catch
try {
  const result = await sendMessage(id, content);
} catch (error) {
  const errMsg = error instanceof Error ? error.message : String(error);
  showToast(`操作失败: ${errMsg}`, 'error');
  logErrorToRemote(errMsg);  // 可选: 远程上报
}
```

---

## 10. 性能约束

### 10.1 并发控制

- 最大并发流式请求: **10** (Semaphore 控制)
- WebSocket 连接: RSA 公私钥认证
- 数据库连接池: 由 Sea-ORM 自动管理

### 10.2 前端性能

- 长列表必须使用虚拟滚动 (消息列表 >100 条时)
- 大文本渲染需分段处理 (流式文本)
- Three.js 3D 背景需控制粒子数量 (<2000)

---

## 11. 依赖管理规则

### 11.1 前端依赖

- 安装依赖使用 `pnpm install`
- **禁止**直接修改 `package-lock.json` 或 `node_modules`
- 新增依赖前确认: 项目是否已有类似功能? 版本兼容性?
- 生产依赖 vs 开发依赖严格区分

### 11.2 后端依赖

- 所有共享依赖在 `Cargo.toml` workspace 层声明
- 新增外部 crate 需评估: 维护状态、许可证、体积影响
- 更新依赖前检查 breaking changes

---

## 12. 文件命名与组织

| 类别 | 命名规范 | 示例 |
|------|---------|------|
| React 组件 | PascalCase.tsx | `ChatWindow.tsx` |
| Hook | camelCase.ts (use 前缀) | `useChat.ts` |
| 工具函数 | camelCase.ts | `api.ts`, `utils.ts` |
| 类型定义 | camelCase.ts | `types.ts`, `constants.ts` |
| Rust 模块 | snake_case.rs | `tool_registry.rs` |
| Rust 结构体/Enum | PascalCase | `MemoryUnitModel` |
| Rust 函数/方法 | snake_case | `hybrid_retrieve()` |
| 技能目录 | kebab-case | `coding-agent/S` |
| Locale 文件 | 语言代码.json | `zh-CN.json` |

---

## 13. 禁止行为清单

1. **禁止** 修改 `main.rs` 入口文件 (除非必要且经过 review)
2. **禁止** 在前端代码中直接调用 `invoke()` (必须通过 `lib/api.ts`)
3. **禁止** 硬编码用户可见文本 (必须走 i18n)
4. **禁止** 在 Rust 代码中使用 `println!` / `eprintln!` (必须用 `log::` 宏)
5. **禁止** 在 Tauri command 中执行阻塞操作
6. **禁止** SQL 字符串拼接 (必须参数化)
7. **禁止** 提交敏感信息到仓库
8. **禁止** 修改 `MAX_TOOL_ROUNDS` 等关键常量未经评审
9. **禁止** 引入未在 workspace 中声明的依赖版本
10. **禁止** 绕过错误处理 (空的 `catch` 块或不必要的 `unwrap()`)
