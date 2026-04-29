// Claw Desktop - 工具调用 API
// 提供所有内置工具的 HTTP 调用接口：文件读写、Bash 执行、搜索、Web 操作、
// 任务管理、计划模式、标签系统等
import { httpRequest } from '../ws/http'
import type { ToolDefinition, TodoItem, TaskItem, CronJob } from '../types'

/** 文件读取参数 */
export interface ToolReadParams { path: string; offset?: number; limit?: number }
/** 文件编辑参数（搜索替换模式） */
export interface ToolEditParams { path: string; old_string: string; new_string: string; replace_all?: boolean }
/** 文件写入参数 */
export interface ToolWriteParams { path: string; content: string }
/** Bash 命令执行参数 */
export interface ToolBashParams { command: string; timeout?: number; cwd?: string }
/** 文件 glob 模式搜索参数 */
export interface ToolGlobParams { pattern: string; path?: string; excludePatterns?: string[] }
/** 文件内容 grep 搜索参数 */
export interface ToolGrepParams { pattern: string; path?: string; include?: string; output_mode?: string }
/** 网页抓取参数 */
export interface ToolWebFetchParams { url: string; prompt?: string; raw?: boolean }
/** 网页搜索参数 */
export interface ToolWebSearchParams { query: string; allowed_domains?: string[]; blocked_domains?: string[] }
/** 子 Agent 调用参数 */
export interface ToolAgentParams { agent_id: string; prompt: string }
/** 工作流执行参数 */
export interface ToolWorkflowParams { workflow: string; params?: Record<string, unknown> }
/** 技能调用参数 */
export interface ToolSkillParams { skill_name: string; arguments?: string }
/** 内容摘要参数 */
export interface ToolBriefParams { content: string; max_length?: number }
/** 配置操作参数 */
export interface ToolConfigParams { key: string; value?: string; action: 'get' | 'set' | 'delete' | 'list' }
/** Notebook 编辑参数 */
export interface ToolNotebookEditParams { notebook_path: string; old_content: string; new_content: string; cell_index?: number }
/** 用户提问参数 */
export interface ToolAskUserQuestionParams { question: string; header?: string; options?: Array<{ label: string; description: string }> }
/** 工具搜索参数 */
export interface ToolToolSearchParams { query: string }
/** 计划模式参数 */
export interface ToolPlanModeParams { plan?: string }
/** 标签操作参数 */
export interface ToolTagParams { name: string; color?: string }
/** 任务创建参数 */
export interface ToolTaskCreateParams { prompt: string; description?: string }
/** 任务更新参数 */
export interface ToolTaskUpdateParams { task_id: string; status?: string; result?: string }
/** 定时任务调度参数 */
export interface ToolScheduleCronParams { name: string; schedule: string; task: string; enabled?: boolean }

/** 工具执行结果 */
export interface ToolResult {
  output?: string                  // 执行输出
  error?: string                   // 错误信息
  success: boolean                 // 是否成功
}

/** 读取文件内容 */
export async function toolRead(data: ToolReadParams) {
  return httpRequest<ToolResult>('/api/tools/read', { method: 'POST', body: JSON.stringify(data) })
}

/** 编辑文件（搜索替换模式） */
export async function toolEdit(data: ToolEditParams) {
  return httpRequest<ToolResult>('/api/tools/edit', { method: 'POST', body: JSON.stringify(data) })
}

/** 写入文件 */
export async function toolWrite(data: ToolWriteParams) {
  return httpRequest<ToolResult>('/api/tools/write', { method: 'POST', body: JSON.stringify(data) })
}

/** 执行 Bash 命令 */
export async function toolBash(data: ToolBashParams) {
  return httpRequest<ToolResult>('/api/tools/bash', { method: 'POST', body: JSON.stringify(data) })
}

/** 取消正在执行的 Bash 命令 */
export async function toolBashCancel() {
  return httpRequest<{ cancelled: boolean }>('/api/tools/bash/cancel', { method: 'POST' })
}

/** Glob 模式搜索文件 */
export async function toolGlob(data: ToolGlobParams) {
  return httpRequest<ToolResult>('/api/tools/glob', { method: 'POST', body: JSON.stringify(data) })
}

/** Grep 搜索文件内容 */
export async function toolGrep(data: ToolGrepParams) {
  return httpRequest<ToolResult>('/api/tools/grep', { method: 'POST', body: JSON.stringify(data) })
}

/** 抓取网页内容 */
export async function toolWebFetch(data: ToolWebFetchParams) {
  return httpRequest<ToolResult>('/api/tools/web-fetch', { method: 'POST', body: JSON.stringify(data) })
}

/** 网页搜索 */
export async function toolWebSearch(data: ToolWebSearchParams) {
  return httpRequest<ToolResult>('/api/tools/web-search', { method: 'POST', body: JSON.stringify(data) })
}

/** 列出所有可用工具（可按 Agent 过滤） */
export async function toolListAll(data?: { agent_id?: string }) {
  return httpRequest<ToolDefinition[]>('/api/tools/list-all', {
    method: 'POST',
    ...(data ? { body: JSON.stringify(data) } : {}),
  })
}

/** 写入 Todo 列表 */
export async function toolTodoWrite(data: { todos: TodoItem[] }) {
  return httpRequest<ToolResult>('/api/tools/todo-write', { method: 'POST', body: JSON.stringify(data) })
}

/** 获取 Todo 列表 */
export async function toolTodoGet(data?: { agent_id?: string }) {
  return httpRequest<TodoItem[]>('/api/tools/todo-get', {
    method: 'POST',
    ...(data ? { body: JSON.stringify(data) } : {}),
  })
}

/** 创建后台任务 */
export async function toolTaskCreate(data: ToolTaskCreateParams) {
  return httpRequest<TaskItem>('/api/tools/task-create', { method: 'POST', body: JSON.stringify(data) })
}

/** 列出后台任务 */
export async function toolTaskList(data?: { agentId?: string; statusFilter?: string }) {
  return httpRequest<TaskItem[]>('/api/tools/task-list', {
    method: 'POST',
    ...(data ? { body: JSON.stringify(data) } : {}),
  })
}

/** 创建定时任务调度 */
export async function toolScheduleCron(data: ToolScheduleCronParams) {
  return httpRequest<ToolResult>('/api/tools/schedule-cron', { method: 'POST', body: JSON.stringify(data) })
}

/** 列出定时任务 */
export async function toolScheduleList(data?: { agent_id?: string }) {
  return httpRequest<CronJob[]>('/api/tools/schedule-list', {
    method: 'POST',
    ...(data ? { body: JSON.stringify(data) } : {}),
  })
}

/** 调用子 Agent 执行任务 */
export async function toolAgent(data: ToolAgentParams) {
  return httpRequest<ToolResult>('/api/tools/agent', { method: 'POST', body: JSON.stringify(data) })
}

/** 执行工作流 */
export async function toolWorkflow(data: ToolWorkflowParams) {
  return httpRequest<ToolResult>('/api/tools/workflow', { method: 'POST', body: JSON.stringify(data) })
}

/** 调用技能 */
export async function toolSkill(data: ToolSkillParams) {
  return httpRequest<ToolResult>('/api/tools/skill', { method: 'POST', body: JSON.stringify(data) })
}

/** 生成内容摘要 */
export async function toolBrief(data: ToolBriefParams) {
  return httpRequest<ToolResult>('/api/tools/brief', { method: 'POST', body: JSON.stringify(data) })
}

/** 读写配置项 */
export async function toolConfig(data: ToolConfigParams) {
  return httpRequest<ToolResult>('/api/tools/config', { method: 'POST', body: JSON.stringify(data) })
}

/** 编辑 Notebook 单元格 */
export async function toolNotebookEdit(data: ToolNotebookEditParams) {
  return httpRequest<ToolResult>('/api/tools/notebook-edit', { method: 'POST', body: JSON.stringify(data) })
}

/** 向用户提问（等待用户选择/输入） */
export async function toolAskUserQuestion(data: ToolAskUserQuestionParams) {
  return httpRequest<ToolResult>('/api/tools/ask-user-question', { method: 'POST', body: JSON.stringify(data) })
}

/** 搜索可用工具 */
export async function toolToolSearch(data: ToolToolSearchParams) {
  return httpRequest<ToolDefinition[]>('/api/tools/tool-search', { method: 'POST', body: JSON.stringify(data) })
}

/** 进入计划模式 */
export async function toolEnterPlanMode(data?: ToolPlanModeParams) {
  return httpRequest<ToolResult>('/api/tools/plan-mode/enter', {
    method: 'POST',
    ...(data ? { body: JSON.stringify(data) } : {}),
  })
}

/** 退出计划模式 */
export async function toolExitPlanMode(data?: ToolPlanModeParams) {
  return httpRequest<ToolResult>('/api/tools/plan-mode/exit', {
    method: 'POST',
    ...(data ? { body: JSON.stringify(data) } : {}),
  })
}

/** 获取计划模式状态 */
export async function toolGetPlanStatus() {
  return httpRequest<{ in_plan_mode: boolean; plan?: string }>('/api/tools/plan-mode/status', { method: 'GET' })
}

/** 添加标签 */
export async function toolTagAdd(data: ToolTagParams) {
  return httpRequest<ToolResult>('/api/tools/tag-add', { method: 'POST', body: JSON.stringify(data) })
}

/** 删除标签 */
export async function toolTagDelete(data: { name: string }) {
  return httpRequest<ToolResult>('/api/tools/tag-delete', { method: 'POST', body: JSON.stringify(data) })
}

/** 列出所有标签 */
export async function toolTagList() {
  return httpRequest<Array<{ name: string; color?: string }>>('/api/tools/tag-list', { method: 'POST' })
}

/** 获取单个后台任务详情 */
export async function toolTaskGet(data: { task_id: string }) {
  return httpRequest<TaskItem>('/api/tools/task-get', { method: 'POST', body: JSON.stringify(data) })
}

/** 更新后台任务状态 */
export async function toolTaskUpdate(data: ToolTaskUpdateParams) {
  return httpRequest<TaskItem>('/api/tools/task-update', { method: 'POST', body: JSON.stringify(data) })
}
