// Claw Desktop - Agent人设API
// 提供Agent人设的获取/保存/更新/删除/列表/提示词构建等HTTP接口
import { httpRequest } from '../ws/http'

/** 人设数据 — Agent的性格特质、专业领域、沟通风格 */
export interface PersonaData {
  agent_id: string
  personality_traits: string[]
  expertise_areas: string[]
  communication_style: string
  [key: string]: unknown
}

export async function personaGet(data?: unknown): Promise<PersonaData> {
  return httpRequest<PersonaData>('/api/persona/get', {
    method: 'POST',
    ...(data ? { body: JSON.stringify(data) } : {}),
  })
}

export async function personaSave(data: unknown): Promise<{ success: boolean }> {
  return httpRequest<{ success: boolean }>('/api/persona/save', {
    method: 'POST',
    body: JSON.stringify(data),
  })
}

export async function personaUpdateField(data: unknown): Promise<{ success: boolean }> {
  return httpRequest<{ success: boolean }>('/api/persona/update-field', {
    method: 'POST',
    body: JSON.stringify(data),
  })
}

export async function personaDelete(data?: unknown): Promise<{ success: boolean }> {
  return httpRequest<{ success: boolean }>('/api/persona/delete', {
    method: 'POST',
    ...(data ? { body: JSON.stringify(data) } : {}),
  })
}

export async function personaList(data?: unknown): Promise<{ personas: PersonaData[] }> {
  return httpRequest<{ personas: PersonaData[] }>('/api/persona/list', {
    method: 'POST',
    ...(data ? { body: JSON.stringify(data) } : {}),
  })
}

export async function personaBuildPrompt(data: unknown): Promise<{ prompt: string }> {
  return httpRequest<{ prompt: string }>('/api/persona/build-prompt', {
    method: 'POST',
    body: JSON.stringify(data),
  })
}
