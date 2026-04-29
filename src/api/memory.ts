// Claw Desktop - 记忆系统API
// 提供记忆存储/检索/删除/导出、实体列表、统计、用户画像、压缩等HTTP接口
import { httpRequest } from '../ws/http'

/** 记忆统计 — 总记忆数、实体数、向量维度、事实类型分布 */
export interface MemoryStats {
  total_memories: number
  total_entities: number
  vector_dimension: number
  fact_types: { world: number; experience: number; observation: number }
  search_methods: string[]
  [key: string]: unknown
}

export interface MemoryEntity {
  id: string
  name: string
  type: string
  mention_count: number
  first_seen: number
  last_seen: number
}

export interface MemoryEntitiesResponse {
  entities: MemoryEntity[]
}

export interface MemoryRetrieveResult {
  id: string
  text: string
  score: number
  fact_type: string
  agent_id: string
  created_at: number
  [key: string]: unknown
}

export interface MemoryExportData {
  agent_id: string
  units: unknown[]
  entities: unknown[]
  exported_at: number
}

export interface UserProfile {
  agent_id: string
  preferences: Record<string, unknown>
  expertise_areas: string[]
  communication_style: string
  updated_at: number
  [key: string]: unknown
}

export async function memoryStore(data: unknown): Promise<{ success: boolean; id?: string }> {
  return httpRequest<{ success: boolean; id?: string }>('/api/memory/store', {
    method: 'POST',
    body: JSON.stringify(data),
  })
}

export async function memoryRetrieve(data: { query: string; agent_id?: string; limit?: number }): Promise<{ results: MemoryRetrieveResult[] }> {
  return httpRequest<{ results: MemoryRetrieveResult[] }>('/api/memory/retrieve', {
    method: 'POST',
    body: JSON.stringify(data),
  })
}

export async function memoryListEntities(agentId: string): Promise<MemoryEntitiesResponse> {
  return httpRequest<MemoryEntitiesResponse>(`/api/memory/entities/${agentId}`, { method: 'GET' })
}

export async function memoryStats(agentId: string): Promise<MemoryStats> {
  return httpRequest<MemoryStats>(`/api/memory/stats/${agentId}`, { method: 'GET' })
}

export async function memoryDelete(unitId: string): Promise<{ success: boolean }> {
  return httpRequest<{ success: boolean }>(`/api/memory/${unitId}`, { method: 'DELETE' })
}

export async function memoryExport(agentId: string): Promise<MemoryExportData> {
  return httpRequest<MemoryExportData>(`/api/memory/export/${agentId}`, { method: 'GET' })
}

export async function getUserProfile(): Promise<UserProfile> {
  return httpRequest<UserProfile>('/api/memory/user-profile', { method: 'GET' })
}

export async function getCompactionThreshold(): Promise<{ threshold: number }> {
  return httpRequest<{ threshold: number }>('/api/memory/compaction-threshold', { method: 'GET' })
}

export async function compactAllMemories(): Promise<{ success: boolean }> {
  return httpRequest<{ success: boolean }>('/api/memory/compact-all', { method: 'POST' })
}

export async function getGlobalMemoryStats(): Promise<MemoryStats> {
  return httpRequest<MemoryStats>('/api/memory/stats', { method: 'GET' })
}
