// Claw Desktop - 系统管理API
// 提供数据导入导出、会话信息、数据库统计、用量统计、队列状态、健康检查、诊断等HTTP接口
import { httpRequest } from '../ws/http'

/** 会话信息 — 当前Agent ID、模型、运行时长 */
export interface SessionInfo {
  agent_id: string
  model: string
  uptime_secs: number
  [key: string]: unknown
}

export interface DbStats {
  tables: Record<string, number>
  db_size_bytes: number
  [key: string]: unknown
}

export interface UsageStats {
  total_tokens: number
  total_api_calls: number
  [key: string]: unknown
}

export interface QueueStats {
  waiting: number
  running: number
  completed: number
  [key: string]: unknown
}

export interface SystemHealth {
  status: string
  components: Record<string, { status: string; latency_ms?: number }>
  [key: string]: unknown
}

export interface DoctorCheckResult {
  checks: Array<{ name: string; status: string; message?: string }>
  [key: string]: unknown
}

export async function exportDataToPath(data: unknown): Promise<{ success: boolean; path?: string }> {
  return httpRequest<{ success: boolean; path?: string }>('/api/system/export', {
    method: 'POST',
    body: JSON.stringify(data),
  })
}

export async function importData(data: unknown): Promise<{ success: boolean }> {
  return httpRequest<{ success: boolean }>('/api/system/import', {
    method: 'POST',
    body: JSON.stringify(data),
  })
}

export async function getSessionInfo(): Promise<SessionInfo> {
  return httpRequest<SessionInfo>('/api/system/session-info', { method: 'GET' })
}

export async function getDbStats(): Promise<DbStats> {
  return httpRequest<DbStats>('/api/system/db-stats', { method: 'GET' })
}

export async function getUsageStats(): Promise<UsageStats> {
  return httpRequest<UsageStats>('/api/system/usage-stats', { method: 'GET' })
}

export async function getQueueStats(): Promise<QueueStats> {
  return httpRequest<QueueStats>('/api/system/queue-stats', { method: 'GET' })
}

export async function getSystemHealth(): Promise<SystemHealth> {
  return httpRequest<SystemHealth>('/api/system/health', { method: 'GET' })
}

export async function runDoctorCheck(): Promise<DoctorCheckResult> {
  return httpRequest<DoctorCheckResult>('/api/system/doctor', { method: 'GET' })
}

export async function testConnection(data: unknown): Promise<{ success: boolean; latency_ms?: number; error?: string }> {
  return httpRequest<{ success: boolean; latency_ms?: number; error?: string }>('/api/system/test-connection', {
    method: 'POST',
    body: JSON.stringify(data),
  })
}

export async function logout(): Promise<{ success: boolean }> {
  return httpRequest<{ success: boolean }>('/api/system/logout', { method: 'POST' })
}

export async function exportWithDialog(): Promise<{ success: boolean; path?: string }> {
  return httpRequest<{ success: boolean; path?: string }>('/api/system/export-with-dialog', { method: 'POST' })
}
