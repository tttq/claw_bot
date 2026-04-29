// Claw Desktop - 多Agent类型定义模块
// 定义子Agent状态、执行模式、Agent分类、多Agent会话、任务、上下文、注册表等核心类型
import type { AgentConfig } from '../types'

export enum SubAgentStatus {
  PENDING = 'pending',
  RUNNING = 'running',
  COMPLETED = 'completed',
  FAILED = 'failed',
  TIMEOUT = 'timeout',
  WAITING_INPUT = 'waiting_input',
}

export enum ExecutionMode {
  PARALLEL = 'parallel',
  SEQUENTIAL = 'sequential',
}

export interface MentionedAgent {
  agentId: string
  agentName: string
  startIndex: number
  endIndex: number
}

export interface ParsedMentions {
  rawText: string
  mentions: MentionedAgent[]
  cleanText: string
}

export interface SubAgentTask {
  id: string
  agentId: string
  agentName: string
  description: string
  prompt: string
  status: SubAgentStatus
  result?: string
  error?: string
  startTime?: number
  endTime?: number
  durationMs?: number
  dependsOn?: string[]
  contextShared: boolean
  rawOutput?: string
  retryCount: number
}

export interface MultiAgentContext {
  conversationId: string
  originalQuery: string
  mentionedAgents: MentionedAgent[]
  mainAgentId: string
  executionMode: ExecutionMode
  tasks: SubAgentTask[]
  aggregatedResult?: string
  status: MultiAgentSessionStatus
  createdAt: number
  updatedAt: number
  error?: string
}

export enum MultiAgentSessionStatus {
  IDLE = 'idle',
  PLANNING = 'planning',
  EXECUTING = 'executing',
  AGGREGATING = 'aggregating',
  COMPLETED = 'completed',
  FAILED = 'failed',
  WAITING_INPUT = 'waiting_input',
}

export interface AgentRegistryEntry extends AgentConfig {
  id: string
  name: string
  description: string
  mentionable: boolean
  category: AgentCategory
  icon: string
  capabilities: string[]
  dependencies?: string[]
  timeoutMs: number
  maxRetries: number
}

export enum AgentCategory {
  GENERAL = 'general',
  SEARCH = 'search',
  CODE = 'code',
  ANALYSIS = 'analysis',
  CREATIVE = 'creative',
  CUSTOM = 'custom',
}

export interface CoordinationMessage {
  id: string
  fromAgentId: string
  toAgentId: string | '*'
  type: 'task_request' | 'task_result' | 'info_share' | 'error_report' | 'status_update' | 'input_request'
  payload: Record<string, any>
  timestamp: number
}

export interface AggregatedResponse {
  summary: string
  details: Record<string, { agentName: string; result: string; status: SubAgentStatus }>
  conflicts: ConflictResolution[]
  suggestions?: string[]
}

export interface ConflictResolution {
  type: 'priority' | 'merge' | 'user_decision'
  agents: string[]
  resolution: string
  reason: string
}

export interface MultiAgentMessageContent {
  type: 'multi_agent'
  sessionId: string
  mainResponse: string
  subAgents: SubAgentResult[]
  status: MultiAgentSessionStatus
  timestamp: number
  streamingAgentId?: string
  streamingText?: string
}

export interface SubAgentResult {
  taskId: string
  agentId: string
  agentName: string
  status: SubAgentStatus
  result?: string
  error?: string
  durationMs?: number
  rawOutput?: string
  streamingText?: string
}
