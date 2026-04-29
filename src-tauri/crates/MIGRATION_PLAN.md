# Claw-Core Crate 物理迁移计划

## 迁移进度总览

| Crate | 状态 | 迁移难度 | 优先级 |
|-------|------|---------|--------|
| ✅ claw-config | 已完成 | 低 | P0 |
| ✅ claw-db | 已完成 | 低 | P1 |
| ⏳ claw-rag | 待开始 | 中 | P2 |
| ⏳ claw-llm | 待开始 | 中 | P2 |
| ⏳ claw-harness | 待开始 | 低 | P3 |

---

## 依赖关系图

```
┌─────────────────────────────────────────┐
│              harness (顶层)              │
│         依赖: db, rag, llm              │
└──────────────┬──────────┬───────────────┘
               │          │
    ┌──────────▼──┐  ┌────▼──────────┐
    │   llm (核心)  │  │  rag (记忆)   │
    │ 依赖: db,rag │  │  依赖: db     │
    └──────┬───────┘  └──────┬────────┘
           │                 │
           └────────┬────────┘
                    ▼
              ┌──────────┐
              │  db (基础) │
              │   无依赖   │
              └──────────┘
```

**推荐迁移顺序**: `db → rag/llm → harness`

---

## 各模块详细分析

### 1. claw-db (数据库层) 🔄 进行中

**当前位置**: `claw-core/src/db/`

**子模块结构**:
```
db/
├── mod.rs              # 模块入口，导出 Database 结构体
├── database.rs         # Database impl (CRUD 操作)
├── db/
│   ├── mod.rs          # 双数据库架构 (主库 + Agent库)
│   ├── conn.rs         # 数据库连接管理 (OnceCell<DbConn>)
│   ├── entities/       # 通用实体 (conversations, messages, memory_units...)
│   └── agent_entities/ # Agent专属实体 (agents, agent_configs, agent_profiles)
└── channel_migration.rs # 通道数据迁移
```

**内部依赖**: 无循环依赖 ✅
**外部依赖**: sea-orm, uuid, chrono, dashmap, sqlx
**对其他模块依赖**: 无（纯基础设施层）

**迁移步骤**:
1. 创建 `crates/claw-db/Cargo.toml` (依赖 sea-orm, sqlx, uuid 等)
2. 复制 `db/` 目录到 `crates/claw-db/src/`
3. 在 `claw-core/src/db/mod.rs` 中添加 re-export 兼容层
4. 更新 `claw-core/Cargo.toml` 添加 `claw-db` 依赖
5. 验证编译通过

**预计工作量**: 30分钟

---

### 2. claw-rag (RAG 记忆系统) ⏳ 待开始

**当前位置**: `claw-core/src/rag/`

**子模块结构**:
```
rag/
├── rag.rs             # 核心: 向量化、检索、实体提取、记忆管理
├── local_embedder.rs  # ONNX 本地嵌入模型 (条件编译)
├── memory_provider.rs # 记忆提供者接口
└── builtin_provider.rs# 内置记忆实现
```

**内部依赖**: 无循环依赖 ✅
**外部依赖**: uuid, serde_json, chrono, sha2, ort (optional), tokenizers (optional)
**跨模块依赖**:
- ← `crate::db::database::Database` (强依赖)
- → 被 `crate::llm::llm.rs` 调用 (build_rag_context, store_interaction_to_rag)
- → 被 `crate::harness::persona.rs` 调用

**迁移难点**:
- 与 LLM 存在双向依赖（RAG 调用 LLM 嵌入，LLM 调用 RAG 构建上下文）
- 解决方案：通过 trait 对象解耦，或延迟初始化

**预计工作量**: 45分钟

---

### 3. claw-llm (LLM 交互核心) ⏳ 待开始

**当前位置**: `claw-core/src/llm/`

**子模块结构**:
```
llm/
├── llm.rs             # 核心入口: ChatResponse, send_chat_message
├── api_client.rs      # HTTP API 客户端 (Anthropic/OpenAI)
├── prompt_builder.rs  # Prompt 模板构建
├── tool_loop.rs       # 工具循环逻辑 (ReAct模式)
├── streaming.rs       # 流式响应处理
├── constants.rs       # 常量定义
└── types.rs           # LLM 专用类型
```

**内部依赖**: 子模块间有清晰的单向依赖链 ✅
**外部依赖**: reqwest, serde, lazy_static, dashmap, tokio, futures
**跨模块依赖**:
- ← `crate::db::*` (Agent 信息查询)
- ← `crate::rag::rag::*` (上下文构建、记忆存储)
- ← `crate::tools::*` (工具执行)
- → 被 `commands::send_message()` 调用

**迁移难点**:
- 依赖最多（db + rag + tools），是系统的"胶水层"
- tool_loop 中的 OnceLock 全局状态需要保留或重构

**预计工作量**: 60分钟

---

### 4. claw-harness (工程框架) ⏳ 待开始

**当前位置**: `claw-core/src/harness/`

**子模块结构**:
```
harness/
├── mod.rs              # 模块入口
├── harness/
│   ├── persona.rs      # Agent 行为模板
│   ├── observability.rs # 可观测性
│   ├── error_learning.rs # 错误学习
│   └── output_validation.rs # 输出验证
```

**内部依赖**: 无循环依赖 ✅
**外部依赖**: serde, serde_json, chrono
**跨模块依赖**:
- ← `crate::db::*` (Agent CRUD)
- ← `crate::rag::*` (用户画像)
- ← `crate::llm::*` (LLM 交互)

**迁移难度**: 低（纯消费层，不对外提供服务）

**预计工作量**: 20分钟

---

## 迁移原则

### ✅ 必须遵守
1. **向后兼容**: `claw-core` 必须保留 re-export 层，避免破坏性变更
2. **渐进式迁移**: 每次只迁移一个 crate，验证通过后再继续
3. **测试覆盖**: 每个 crate 迁移后必须 `cargo check` + `cargo test`
4. **文档更新**: 同步更新 `lib.rs` 和本文件的迁移状态

### ⚠️ 注意事项
1. **避免循环依赖**: 新 crate 不能反向依赖 `claw-core`
2. **OnceLock 处理**: 跨 crate 全局状态保留 OnceLock，不强制迁移到 State<T>
3. **Feature flags**: 条件编译 feature（如 onnx-embedding）必须正确传递
4. **Workspace 依赖**: 共享依赖通过 `workspace = true` 引用

### 🎯 成功标准
- [ ] 所有 5 个独立 crate 编译通过
- [ ] `cargo test --workspace` 全绿
- [ ] `pnpm tauri dev` 正常启动
- [ ] 0 warnings (允许 dead_code warning 用于未使用的 pub 接口)

---

## 下一步行动

**当前任务**: 完成 `claw-db` 物理迁移
**负责人**: AI Assistant
**截止时间**: 本次会话完成

**验收标准**:
- [x] `crates/claw-db/src/` 包含完整的 db/ 模块代码
- [x] `claw-core/src/db/mod.rs` 变为 re-export 层 (`pub use claw_db::*`)
- [x] `cargo check --workspace` 0 errors
- [x] 本文件状态更新为 ✅
