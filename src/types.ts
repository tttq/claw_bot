// Claw Desktop - TypeScript 类型定义
// 定义前后端数据交互的所有接口类型，与 Rust 端的 serde 结构体一一对应

/// 单条聊天消息
export interface Message {
  id: string
  role: 'user' | 'assistant' | 'system' | 'tool'
  content: string
  timestamp: number
  isError?: boolean
  thinkingText?: string
  toolExecutionDetails?: import('./stores/conversationStore').ToolExecutionDetail[]
  signalStatus?: 'response_complete' | 'input_required' | 'confirm_required' | 'task_in_progress'
}

/// 会话（对话）
export interface Conversation {
  id: string                          // 会话唯一标识（UUID）
  title: string                       // 会话标题（默认取首条消息前50字符）
  createdAt: number                   // 创建时间戳（毫秒）
  updatedAt: number                   // 最后更新时间戳（毫秒）
  messageCount?: number               // 消息数量（可选，用于列表显示）
  agentId?: string                    // 所属 Agent ID（可选，关联后自动填充）
}

/// 应用总配置（顶层），包含 7 个配置分组
export interface AppConfig {
  app: AppSettings         // 通用设置
  model: ModelSettings     // 模型设置
  api: ApiSettings         // API 设置
  ui: UiSettings           // UI 设置
  advanced: AdvancedSettings // 高级设置
  harness?: HarnessSettings // Harness Engineering 配置
  tools?: ToolSettings     // 工具启用/禁用配置
}

/// 工具类别启用/禁用配置
export interface ToolSettings {
  file_access: boolean     // 文件读取
  file_write: boolean      // 文件写入/编辑
  shell: boolean           // Shell 命令执行
  search: boolean          // 文件搜索 (Glob/Grep)
  web: boolean             // 网络工具 (WebFetch/WebSearch)
  git: boolean             // Git 版本控制
  browser: boolean         // 浏览器自动化
  automation: boolean      // UI 自动化 (屏幕捕获/OCR/鼠标键盘)
  agent: boolean           // Agent 编排工具
}

/// Harness Engineering 系统配置
export interface HarnessSettings {
  error_learning_enabled?: boolean    // 是否启用错误学习循环
  cross_memory_enabled?: boolean      // 是否启用交叉记忆
  validation_enabled?: boolean        // 是否启用输出验证引擎
  default_memory_visibility?: string  // 默认记忆可见性级别
  max_parallel_subtasks?: number      // 最大并行子任务数
  task_timeout_seconds?: number       // 任务执行超时秒数
  agents_md_auto_refresh?: boolean    // AGENTS.md 自动刷新
}

/// 通用应用设置
export interface AppSettings {
  language: string                  // 界面语言：zh-CN / en-US / ja-JP
  theme: 'dark' | 'light' | 'system'  // 主题模式
  auto_update: boolean             // 是否自动更新
  minimize_to_tray: boolean         // 最小化到系统托盘
  startup_behavior: 'normal' | 'maximized' | 'minimized'  // 启动窗口状态
}

/// 模型配置设置
export interface ModelSettings {
  default_model: string              // 默认模型名称
  provider: 'anthropic' | 'openai' | 'custom'  // AI 提供商
  custom_url: string                // 自定义模式：API 端点 URL
  custom_api_key: string            // 自定义模式：API 密钥
  custom_model_name: string         // 自定义模式：模型名称
  temperature: number               // 温度参数（0-2）
  max_tokens: number                // 最大生成 token 数
  top_p: number                     // Top P 参数（0-1）
  thinking_budget: number           // 扩展思考预算 token 数
  stream_mode: boolean              // 是否流式输出
  api_format: string                // API 格式：auto / anthropic / openai
}

/// API 连接设置
export interface ApiSettings {
  api_key: string                   // API 认证密钥
  base_url: string                  // API 基础 URL
  api_version: string               // API 版本号
  timeout_seconds: number            // 请求超时时间（秒）
  retry_count: number               // 失败重试次数
}

/// 界面外观设置
export interface UiSettings {
  font_size: number                 // 基础字号（px）
  font_family: string                // 字体族名称
  sidebar_width: number              // 侧边栏宽度（px）
  show_line_numbers: boolean         // 代码块是否显示行号
  code_theme: string                // 代码高亮主题名
  message_style: 'bubble' | 'plain' // 消息气泡风格
  show_tool_executions: boolean      // 是否在聊天中展示工具执行详情（默认 true）
}

/// 高级设置
export interface AdvancedSettings {
  data_dir: string                   // 应用数据目录路径
  log_level: 'trace' | 'debug' | 'info' | 'warn' | 'error'  // 日志级别
  max_conversation_history: number   // 单会话最大消息数
  auto_compact_tokens: number        // 自动压缩 token 阈值
  proxy_url: string                 // HTTP 代理地址
  enable_telemetry: boolean          // 是否启用遥测统计
}

// ==================== 工具系统类型定义 ====================

/// 工具定义（对应后端 ToolDefinition 结构）
export interface ToolDefinition {
  name: string                       // 工具名称（唯一标识符）
  description: string                // 工具功能描述
  input_schema: Record<string, any>  // JSON Schema 格式的参数定义
  parameters?: Record<string, unknown> // OpenAI格式的参数定义（可选）
  [key: string]: unknown             // 允许扩展字段
}

/// Agent 定义（AI 子代理配置）
export interface AgentConfig {
  id?: string                        // Agent 唯一 ID
  name?: string                      // 显示名称
  description?: string               // 功能描述
  systemPrompt?: string              // 系统提示词
  model?: string                     // 覆盖模型（可选，默认使用全局模型）
  provider?: string                  // AI 提供商（可选）
  apiKey?: string                    // API密钥（可选）
  customUrl?: string                 // 自定义URL（可选）
  tools?: string[]                   // 允许使用的工具列表（空数组=全部可用）
  maxTurns?: number                  // 最大对话轮次
  enabled?: boolean                  // 是否启用
  createdAt?: number                 // 创建时间
  updatedAt?: number                 // 更新时间
  [key: string]: unknown             // 允许扩展字段
}

/// Skill 定义（可安装/卸载的技能包）
export interface SkillDefinition {
  id?: string                        // Skill 唯一 ID
  name: string                       // 技能名称
  version?: string                   // 版本号
  description: string                // 功能描述
  author?: string                    // 作者
  category?: string                  // 分类（coding/search/web/agent/misc）
  tags?: string[]                    // 标签
  installed?: boolean                // 是否已安装
  configSchema?: Record<string, any> // 配置参数 Schema（可选）
  defaultConfig?: Record<string, any>// 默认配置值（可选）
  userConfig?: Record<string, any>   // 用户自定义配置（可选）
  commands?: SkillCommand[]          // 包含的命令列表（可选）
  [key: string]: unknown             // 允许扩展字段
}

/// Skill 内含命令
export interface SkillCommand {
  name: string                       // 命令名称
  description: string                // 命令描述
  params?: Record<string, any>       // 参数定义
}

/// Todo 项（任务跟踪）
export interface TodoItem {
  id?: string                        // 任务ID
  content: string                    // 任务内容
  status?: 'pending' | 'in_progress' | 'completed'  // 状态
  priority?: 'high' | 'medium' | 'low'  // 优先级
  completed?: boolean                // 是否完成
}

/// 后台任务
export interface TaskItem {
  id: string                         // 任务 ID
  prompt?: string                    // 任务描述
  title?: string                     // 任务标题
  status: 'pending' | 'running' | 'completed' | 'failed' | string  // 状态
  result?: string                    // 执行结果
  createdAt?: number                 // 创建时间
  [key: string]: unknown             // 允许扩展字段
}

/// 定时任务（Cron）
export interface CronJob {
  id?: string                        // 定时任务ID
  name?: string                      // 定时任务名称
  schedule: string                   // Cron 表达式
  task?: string                      // 要执行的任务
  command?: string                   // 要执行的命令
  enabled: boolean                   // 是否启用
  [key: string]: unknown             // 允许扩展字段
}

// ==================== Harness 人物画像类型定义 ====================

/// Agent 人物画像：定义 Agent 的性格、风格和专业特征
export interface AgentPersona {
  agent_id: string                   // 关联的 Agent ID
  display_name: string               // 显示名称
  personality_traits: string[]       // 性格特征标签：["严谨", "幽默", "耐心"]
  communication_style: CommunicationStyle  // 沟通风格
  expertise_domain: string           // 专业领域/知识背景
  behavior_constraints: string[]     // 行为约束列表
  response_tone_instruction: string  // 回复基调指令（注入 system prompt）
  language_preference: string        // 语言偏好："zh-CN"
  created_at: number                 // 创建时间戳
  updated_at: number                 // 更新时间戳
}

/// 沟通风格枚举
export enum CommunicationStyle {
  Formal = 'formal',           // 正式学术风
  Casual = 'casual',           // 轻松随意风
  Technical = 'technical',     // 技术专业风
  Friendly = 'friendly',       // 友好亲切风
  Concise = 'concise',         // 简洁高效风
  Educational = 'educational', // 教学引导风
}

/// Communication style display label mapping (i18n keys)
export const CommunicationStyleLabels: Record<CommunicationStyle, string> = {
  [CommunicationStyle.Formal]: 'communicationStyle.formal',
  [CommunicationStyle.Casual]: 'communicationStyle.casual',
  [CommunicationStyle.Technical]: 'communicationStyle.technical',
  [CommunicationStyle.Friendly]: 'communicationStyle.friendly',
  [CommunicationStyle.Concise]: 'communicationStyle.concise',
  [CommunicationStyle.Educational]: 'communicationStyle.educational',
}

// ==================== Model Provider Types ====================

export interface ModelProvider {
  id: string
  name: string
  category: string
  logo?: string
  description?: string
  defaultBaseUrl?: string
  supportsCustomModels?: boolean
  availableModels?: ModelProviderModel[]
  models?: ModelProviderModel[]
}

export interface ModelProviderModel {
  id: string
  name: string
  description?: string
  maxTokens?: number
  supportsVision?: boolean
}

// ==================== Skill Marketplace Types ====================

export interface MarketplaceSkill {
  slug: string
  name: string
  description: string
  version: string
  author: string
  category: string
  tags: string[]
  installed: boolean
  source: string
  files?: MarketplaceSkillFile[]
}

export interface MarketplaceSkillFile {
  path: string
  content: string
}

// ==================== Agent Types ====================

export interface IsoAgent {
  id: string
  name: string
  status: string
  config?: Record<string, unknown>
  recentMessages?: AgentMessage[]
}

export interface AgentMessage {
  role: string
  content: string
  timestamp?: number
}

export interface AgentWorkspaceFile {
  name: string
  path: string
  type?: 'file' | 'directory'
  is_dir?: boolean
  size?: number | string
}

export interface AgentSession {
  id: string
  status: string
  createdAt: number
  turnCount?: number                 // 对话轮次
  lastActive?: number                // 最后活跃时间戳
}

// ==================== Git Types ====================

export interface GitStatus {
  branch?: string
  items?: GitStatusItem[]
}

export interface GitStatusItem {
  file: string
  status: string
}

export interface GitCommit {
  hash: string
  message: string
  author: string
  date: string
}

export interface GitBranch {
  name: string
  current: boolean
}

export interface GitDiff {
  files_changed: GitDiffFile[]
}

export interface GitDiffFile {
  file: string
  lines: GitDiffLine[]
}

export interface GitDiffLine {
  content: string
  type: 'added' | 'removed' | 'context'
}

// ==================== 其他类型定义 ====================

export interface FsSkillInfo {
  name: string
  version?: string
  description?: string
  path: string
  files?: string[]
}

export interface SkillTelemetryEvent {
  skill_name: string
  execution_context: string
  invocation_trigger: string
  duration_ms: number
  query_depth: number
  source: string
  status: string
  timestamp: number
}

export interface SkillPermissionRule {
  tool_name: string
  rule_content: string
  behavior: 'allow' | 'deny' | 'ask'
}

// ==================== WebSocket Types ====================

export interface WsEvent {
  channel: string
  data: unknown
}

export type WsEventHandler = (data: unknown) => void

// ==================== Speech Recognition Types ====================

export interface SpeechRecognitionEvent {
  results: SpeechRecognitionResultList
}

export interface SpeechRecognitionResultList {
  [index: number]: SpeechRecognitionResult
  length: number
}

export interface SpeechRecognitionResult {
  [index: number]: SpeechRecognitionAlternative
  length: number
  isFinal: boolean
}

export interface SpeechRecognitionAlternative {
  transcript: string
  confidence: number
}

// ==================== Markdown Component Props Types ====================

export interface MarkdownComponentProps {
  className?: string
  children?: React.ReactNode
  href?: string
  [key: string]: unknown
}

// ==================== Code Review Types ====================

export interface CodeReviewChanges {
  files_changed: CodeReviewFile[]
}

export interface CodeReviewFile {
  file: string
  additions: number
  deletions: number
}
