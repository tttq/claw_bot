// Claw Desktop - 文件系统技能API
// 提供技能文件扫描、列表、增删、重载、源码读写等HTTP接口
import { httpRequest } from '../ws/http'

/** 文件系统技能信息 — 名称、路径、启用状态 */
export interface FsSkillInfo {
  name: string
  path: string
  enabled: boolean
  [key: string]: unknown
}

export async function fsSkillScan(): Promise<{ scanned: number }> {
  return httpRequest<{ scanned: number }>('/api/fs-skills/scan', { method: 'GET' })
}

export async function fsSkillList(): Promise<{ skills: FsSkillInfo[] }> {
  return httpRequest<{ skills: FsSkillInfo[] }>('/api/fs-skills/list', { method: 'GET' })
}

export async function fsSkillAdd(data: unknown): Promise<{ success: boolean; skill_name?: string }> {
  return httpRequest<{ success: boolean; skill_name?: string }>('/api/fs-skills/add', {
    method: 'POST',
    body: JSON.stringify(data),
  })
}

export async function fsSkillRemove(data: unknown): Promise<{ success: boolean }> {
  return httpRequest<{ success: boolean }>('/api/fs-skills/remove', {
    method: 'POST',
    body: JSON.stringify(data),
  })
}

export async function fsSkillReload(): Promise<{ reloaded: number }> {
  return httpRequest<{ reloaded: number }>('/api/fs-skills/reload', { method: 'GET' })
}

export async function fsSkillReadSource(data: unknown): Promise<{ source: string; name: string }> {
  return httpRequest<{ source: string; name: string }>('/api/fs-skills/read-source', {
    method: 'POST',
    body: JSON.stringify(data),
  })
}

export async function fsSkillUpdateSource(data: unknown): Promise<{ success: boolean }> {
  return httpRequest<{ success: boolean }>('/api/fs-skills/update-source', {
    method: 'POST',
    body: JSON.stringify(data),
  })
}

export async function fsSkillsDirPath(): Promise<{ path: string }> {
  return httpRequest<{ path: string }>('/api/fs-skills/dir-path', { method: 'GET' })
}
