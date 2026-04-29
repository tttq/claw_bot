// Claw Desktop - 命令/工具管理API
// 提供工具注册/注销、技能加载、扩展扫描安装等HTTP接口
import { httpRequest } from '../ws/http'

/** 工具信息 — 名称、描述、启用状态 */
export interface ToolInfo {
  name: string
  description: string
  enabled: boolean
  [key: string]: unknown
}

export async function cmdListAllTools(): Promise<{ tools: ToolInfo[] }> {
  return httpRequest<{ tools: ToolInfo[] }>('/api/cmd/tools/list', { method: 'GET' })
}

export async function cmdRegisterTool(data: unknown): Promise<{ success: boolean }> {
  return httpRequest<{ success: boolean }>('/api/cmd/tools/register', {
    method: 'POST',
    body: JSON.stringify(data),
  })
}

export async function cmdUnregisterTool(data: unknown): Promise<{ success: boolean }> {
  return httpRequest<{ success: boolean }>('/api/cmd/tools/unregister', {
    method: 'POST',
    body: JSON.stringify(data),
  })
}

export async function cmdLoadSkillsFromDir(data?: unknown): Promise<{ loaded: number }> {
  return httpRequest<{ loaded: number }>('/api/cmd/skills/load', {
    method: 'POST',
    ...(data ? { body: JSON.stringify(data) } : {}),
  })
}

export async function cmdListLoadedSkills(): Promise<{ skills: string[] }> {
  return httpRequest<{ skills: string[] }>('/api/cmd/skills/list-loaded', { method: 'GET' })
}

export async function cmdScanExtensions(): Promise<{ extensions: Array<Record<string, unknown>> }> {
  return httpRequest<{ extensions: Array<Record<string, unknown>> }>('/api/cmd/extensions/scan', { method: 'GET' })
}

export async function cmdInstallExtension(data: unknown): Promise<{ success: boolean }> {
  return httpRequest<{ success: boolean }>('/api/cmd/extensions/install', {
    method: 'POST',
    body: JSON.stringify(data),
  })
}

export async function cmdUninstallExtension(data: unknown): Promise<{ success: boolean }> {
  return httpRequest<{ success: boolean }>('/api/cmd/extensions/uninstall', {
    method: 'POST',
    body: JSON.stringify(data),
  })
}
