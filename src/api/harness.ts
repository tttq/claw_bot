// Claw Desktop - Harness管理API
// 提供Agent人设管理、错误学习规则、交叉记忆检索、可观测性统计等HTTP接口
import { httpRequest } from '../ws/http'

/** Harness人设数据 — Agent的性格特质、专业领域、沟通风格 */
export interface HarnessPersonaData {
  agent_id: string
  personality_traits: string[]
  expertise_areas: string[]
  communication_style: string
  [key: string]: unknown
}

export interface ErrorRule {
  id: string
  agent_id: string
  pattern: string
  category: string
  cause: string
  fix: string
  trigger_count: number
  [key: string]: unknown
}

export interface ObservabilityStats {
  total_calls: number
  avg_latency_ms: number
  error_rate: number
  [key: string]: unknown
}

export interface ObservabilityEvent {
  id: string
  type: string
  timestamp: number
  data: Record<string, unknown>
  [key: string]: unknown
}

export async function harnessErrorTriggerHit(data: unknown): Promise<{ success: boolean }> {
  return httpRequest<{ success: boolean }>('/api/harness/error-trigger-hit', {
    method: 'POST',
    body: JSON.stringify(data),
  })
}

export async function harnessPersonaUpdate(data: unknown): Promise<{ success: boolean }> {
  return httpRequest<{ success: boolean }>('/api/harness/persona-update', {
    method: 'POST',
    body: JSON.stringify(data),
  })
}

export async function harnessErrorCapture(data: unknown): Promise<{ success: boolean; rule_id?: string }> {
  return httpRequest<{ success: boolean; rule_id?: string }>('/api/harness/error-capture', {
    method: 'POST',
    body: JSON.stringify(data),
  })
}

export async function harnessCrossMemoryRetrieve(data: unknown): Promise<{ count: number; results: unknown[] }> {
  return httpRequest<{ count: number; results: unknown[] }>('/api/harness/cross-memory/retrieve', {
    method: 'POST',
    body: JSON.stringify(data),
  })
}

export async function harnessCrossMemoryParseMentions(data: unknown): Promise<{ mentions: Array<{ agent_id: string; agent_name: string }> }> {
  return httpRequest<{ mentions: Array<{ agent_id: string; agent_name: string }> }>('/api/harness/cross-memory/parse-mentions', {
    method: 'POST',
    body: JSON.stringify(data),
  })
}

export async function harnessPersonaGet(data: unknown): Promise<HarnessPersonaData> {
  return httpRequest<HarnessPersonaData>('/api/persona/get', {
    method: 'POST',
    body: JSON.stringify(data),
  })
}

export async function harnessPersonaList(): Promise<{ count: number; personas: HarnessPersonaData[] }> {
  return httpRequest<{ count: number; personas: HarnessPersonaData[] }>('/api/persona/list', { method: 'POST' })
}

export async function harnessPersonaBuildEnhancedPrompt(data: unknown): Promise<{ prompt: string }> {
  return httpRequest<{ prompt: string }>('/api/persona/build-prompt', {
    method: 'POST',
    body: JSON.stringify(data),
  })
}

export async function harnessErrorBuildPromptSection(data: unknown): Promise<{ section: string }> {
  return httpRequest<{ section: string }>('/api/harness/error-build-prompt-section', {
    method: 'POST',
    body: JSON.stringify(data),
  })
}

export async function harnessErrorGetRules(data: unknown): Promise<{ rules: ErrorRule[] }> {
  return httpRequest<{ rules: ErrorRule[] }>('/api/harness/error-get-rules', {
    method: 'POST',
    body: JSON.stringify(data),
  })
}

export async function harnessObservabilityGetStats(): Promise<ObservabilityStats> {
  return httpRequest<ObservabilityStats>('/api/harness/observability/stats', { method: 'GET' })
}

export async function harnessObservabilityGetEvents(data?: unknown): Promise<{ events: ObservabilityEvent[] }> {
  return httpRequest<{ events: ObservabilityEvent[] }>('/api/harness/observability/events', {
    method: 'POST',
    ...(data ? { body: JSON.stringify(data) } : {}),
    headers: { 'Content-Type': 'application/json' },
  })
}
