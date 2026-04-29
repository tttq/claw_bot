// Claw Desktop - 环境与代码API
// 提供环境变量查询、代码变更摘要、代码审查、快速模式切换、会话信息等HTTP接口
import { httpRequest } from '../ws/http'

/** 获取系统环境变量 */
export async function getEnvVariables(): Promise<{ variables: Record<string, string> }> {
  return httpRequest<{ variables: Record<string, string> }>('/api/env/variables', { method: 'GET' })
}

export async function getCodeChangesSummary(data: unknown): Promise<{ summary: string; files_changed: number }> {
  return httpRequest<{ summary: string; files_changed: number }>('/api/code/changes-summary', {
    method: 'POST',
    body: JSON.stringify(data),
  })
}

export async function runCodeReview(data: unknown): Promise<{ review: string; issues?: Array<Record<string, unknown>> }> {
  return httpRequest<{ review: string; issues?: Array<Record<string, unknown>> }>('/api/code/review', {
    method: 'POST',
    body: JSON.stringify(data),
  })
}

export async function toggleFastMode(data?: unknown): Promise<{ enabled: boolean }> {
  return httpRequest<{ enabled: boolean }>('/api/toggle-fast-mode', {
    method: 'POST',
    ...(data ? { body: JSON.stringify(data) } : {}),
  })
}

export async function getEnvSessionInfo(): Promise<{ agent_id: string; model: string; fast_mode: boolean }> {
  return httpRequest<{ agent_id: string; model: string; fast_mode: boolean }>('/api/env/session-info', { method: 'GET' })
}
