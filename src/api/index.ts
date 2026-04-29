// Claw Desktop - API 模块统一导出入口
// 将所有子模块的 API 函数统一 re-export，供组件和 hooks 通过单路径引用
export * from './auth'           // 认证相关 API（握手、令牌验证）
export * from './config'         // 应用配置读写 API
export * from './conversations'  // 会话/消息/流式聊天 API
export * from './tools'          // 工具调用 API（文件读写、Bash、搜索等）
export * from './git'            // Git 操作 API（状态、提交、分支等）
export * from './skills'         // 技能管理 API（列表、安装、市场等）
export * from './agents'         // Agent 管理 API（增删改查、工作区文件操作）
export * from './iso'            // 隔离 Agent（ISO）管理 API
export * from './channels'       // 渠道管理 API（Discord/Telegram/微信等）
export * from './persona'        // Agent 人物画像 API
export * from './browser'        // 浏览器控制 API（CDP 协议操作）
export * from './memory'         // RAG 记忆系统 API（存储、检索、统计）
export * from './system'         // 系统级 API（导出/导入、健康检查、诊断）
export * from './env'            // 环境变量与代码审查 API
export * from './harness'        // Harness Engineering API（错误学习、交叉记忆、画像）
export * from './multiAgent'     // 多 Agent 协调 API
export * from './fsSkills'       // 文件系统技能管理 API（扫描、添加、移除）
export * from './cmd'            // 命令行工具/扩展管理 API
export * from './cron'           // 定时任务 API
export * from './hooks'          // Hook 钩子管理 API
export * from './weixin'         // 微信渠道 API（二维码登录）
export * from './automation'     // 桌面自动化 API（CUA、鼠标键盘、窗口管理）
