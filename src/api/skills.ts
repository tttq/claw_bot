// Claw Desktop - 技能管理API
// 提供技能列表/执行/安装、市场浏览、权限管理、遥测、MCP注册等HTTP接口
import { httpRequest } from '../ws/http'

/** 技能信息 — 名称、描述、版本、启用状态 */
export interface SkillInfo {
  name: string
  description: string
  version: string
  enabled: boolean
  [key: string]: unknown
}

export interface SkillExecuteResult {
  success: boolean
  output?: string
  error?: string
  [key: string]: unknown
}

export async function skillList(): Promise<{ skills: SkillInfo[] }> {
  return httpRequest<{ skills: SkillInfo[] }>('/api/skills/list', { method: 'GET' })
}

export async function skillExecute(params?: Record<string, unknown>): Promise<SkillExecuteResult> {
  const url = '/api/skills/execute'
  const searchParams = params
    ? '?' + new URLSearchParams(params as Record<string, string>).toString()
    : ''
  return httpRequest<SkillExecuteResult>(`${url}${searchParams}`, { method: 'GET' })
}

export async function skillInstall(data: unknown): Promise<{ success: boolean; skill_name?: string }> {
  return httpRequest<{ success: boolean; skill_name?: string }>('/api/skills/install', {
    method: 'POST',
    body: JSON.stringify(data),
  })
}

export async function skillMarketplaceList(data?: Record<string, unknown>): Promise<{ skills: SkillInfo[] }> {
  return httpRequest<{ skills: SkillInfo[] }>('/api/skills/marketplace', { method: 'GET' })
}

export async function skillMarketplaceFiles(data: { slug: string }): Promise<{ files: Array<{ name: string; content: string }> }> {
  return httpRequest<{ files: Array<{ name: string; content: string }> }>(`/api/skills/marketplace/${data.slug}/files`, { method: 'GET' })
}

export async function skillPermissionAdd(data: unknown): Promise<{ success: boolean }> {
  return httpRequest<{ success: boolean }>('/api/skills/permission/add', {
    method: 'POST',
    body: JSON.stringify(data),
  })
}

export async function skillPermissionRemove(data: unknown): Promise<{ success: boolean }> {
  return httpRequest<{ success: boolean }>('/api/skills/permission/remove', {
    method: 'POST',
    body: JSON.stringify(data),
  })
}

export async function skillPermissionsList(): Promise<{ permissions: Record<string, string[]> }> {
  return httpRequest<{ permissions: Record<string, string[]> }>('/api/skills/permission/list', { method: 'GET' })
}

export async function skillTelemetryList(data?: Record<string, unknown>): Promise<{ telemetry: Array<Record<string, unknown>> }> {
  return httpRequest<{ telemetry: Array<Record<string, unknown>> }>('/api/skills/telemetry/list', { method: 'GET' })
}

export async function skillTelemetryClear(): Promise<{ success: boolean }> {
  return httpRequest<{ success: boolean }>('/api/skills/telemetry/clear', { method: 'POST' })
}

export async function skillRegisterMcp(data: unknown): Promise<{ success: boolean; server_name?: string }> {
  return httpRequest<{ success: boolean; server_name?: string }>('/api/skills/register-mcp', {
    method: 'POST',
    body: JSON.stringify(data),
  })
}
