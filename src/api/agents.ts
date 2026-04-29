// Claw Desktop - Agent 管理 API
// 提供 Agent 的增删改查、工作区文件操作、重载等 HTTP 接口封装
import { httpRequest } from '../ws/http'

export interface AgentCreateRequest {
  name: string
  description?: string
  systemPrompt: string
  modelOverride?: string
  maxTurns?: number
  temperature?: number
  enabled?: boolean
}

export interface AgentInfo {
  id: string
  name: string
  displayName?: string
  description?: string
  systemPrompt: string
  modelOverride?: string
  maxTurns?: number
  temperature?: number
  isActive: boolean
  conversationCount?: number
  totalMessages?: number
  createdAt: number
  updatedAt: number
}

export async function agentList() {
  return httpRequest<AgentInfo[]>('/api/agents', { method: 'GET' })
}

export async function agentCreate(data: AgentCreateRequest) {
  return httpRequest<AgentInfo>('/api/agents', {
    method: 'POST',
    body: JSON.stringify(data),
  })
}

export async function agentRemove(data: { id: string }) {
  return httpRequest<{ deleted: boolean }>(`/api/agents/${data.id}`, { method: 'DELETE' })
}

export async function agentGet(data: { id: string }) {
  return httpRequest<AgentInfo>(`/api/agents/${data.id}`, { method: 'GET' })
}

export async function agentListWorkspace(data: { id: string }) {
  return httpRequest<{ files: Array<{ path: string; size: number; modified: number }> }>(`/api/agents/${data.id}/workspace`, { method: 'GET' })
}

export async function agentWriteFile(data: { id: string; relPath: string; content: string }) {
  return httpRequest<{ written: boolean }>(`/api/agents/${data.id}/write-file`, {
    method: 'POST',
    body: JSON.stringify(data),
  })
}

export async function agentReadFile(data: { id: string; relPath: string }) {
  return httpRequest<{ content: string }>(`/api/agents/${data.id}/read-file`, {
    method: 'POST',
    body: JSON.stringify(data),
  })
}

export async function agentDeleteFile(data: { id: string; relPath: string }) {
  return httpRequest<{ deleted: boolean }>(`/api/agents/${data.id}/delete-file`, {
    method: 'POST',
    body: JSON.stringify(data),
  })
}

export async function agentReload() {
  return httpRequest<{ count: number }>('/api/agents/reload', { method: 'GET' })
}

export async function agentsDirPath() {
  return httpRequest<{ path: string }>('/api/agents/dir-path', { method: 'GET' })
}
