// Claw Desktop - 隔离Agent（ISO）管理API
// 提供Agent的CRUD、配置读写、会话管理、工作区索引等HTTP接口
import { httpRequest } from '../ws/http'

/** Agent ID查询参数 */
export interface IsoAgentIdParams { agentId: string }
/** Agent创建参数 */
export interface IsoAgentCreateParams { displayName: string; description?: string; systemPrompt: string; category?: string; purpose?: string; scope?: string }
export interface IsoAgentRenameParams { agentId: string; newName: string }
export interface IsoSetConfigParams { agentId: string; key: string; value: string }
export interface IsoGetConfigParams { agentId: string; key?: string }
export interface IsoSetToolsConfigParams { agentId: string; config: unknown }
export interface IsoSetSkillsEnabledParams { agentId: string; enabled: string[] }
export interface IsoAgentUpdateConfigParams { agentId: string; systemPrompt?: string; purpose?: string; scope?: string; modelOverride?: string; maxTurns?: number; temperature?: number }
export interface IsoCreateSessionParams { agentId: string; conversationId?: string }
export interface IsoIndexWorkspaceParams { agentId: string; path?: string }

export interface IsoAgentInfo {
  id: string
  display_name: string
  description?: string
  category?: string
  created_at: number
  [key: string]: unknown
}

export interface IsoSessionInfo {
  id: string
  agent_id: string
  conversation_id: string
  created_at: number
  [key: string]: unknown
}

export async function isoAgentList(data?: { agentId?: string }): Promise<{ agents: IsoAgentInfo[] }> {
  return httpRequest<{ agents: IsoAgentInfo[] }>('/api/iso/agent-list', {
    method: 'POST',
    body: JSON.stringify(data || {}),
  })
}

export async function isoAgentCreate(data: IsoAgentCreateParams): Promise<{ success: boolean; agent_id?: string }> {
  return httpRequest<{ success: boolean; agent_id?: string }>('/api/iso/agent-create', {
    method: 'POST',
    body: JSON.stringify(data),
  })
}

export async function isoAgentGet(data: IsoAgentIdParams): Promise<IsoAgentInfo> {
  return httpRequest<IsoAgentInfo>('/api/iso/agent-get', {
    method: 'POST',
    body: JSON.stringify(data),
  })
}

export async function isoAgentRename(data: IsoAgentRenameParams): Promise<{ success: boolean }> {
  return httpRequest<{ success: boolean }>('/api/iso/agent-rename', {
    method: 'POST',
    body: JSON.stringify(data),
  })
}

export async function isoAgentDelete(data: IsoAgentIdParams): Promise<{ success: boolean }> {
  return httpRequest<{ success: boolean }>('/api/iso/agent-delete', {
    method: 'POST',
    body: JSON.stringify(data),
  })
}

export async function isoSetConfig(data: IsoSetConfigParams): Promise<{ success: boolean }> {
  return httpRequest<{ success: boolean }>('/api/iso/set-config', {
    method: 'POST',
    body: JSON.stringify(data),
  })
}

export async function isoGetConfig(data: IsoGetConfigParams): Promise<{ value: string }> {
  return httpRequest<{ value: string }>('/api/iso/get-config', {
    method: 'POST',
    body: JSON.stringify(data),
  })
}

export async function isoInitAgentDb(data?: IsoAgentIdParams): Promise<{ success: boolean }> {
  return httpRequest<{ success: boolean }>('/api/iso/init-agent-db', {
    method: 'POST',
    body: JSON.stringify(data || {}),
  })
}

export async function isoSetToolsConfig(data: IsoSetToolsConfigParams): Promise<{ success: boolean }> {
  return httpRequest<{ success: boolean }>('/api/iso/set-tools-config', {
    method: 'POST',
    body: JSON.stringify(data),
  })
}

export async function isoSetSkillsEnabled(data: IsoSetSkillsEnabledParams): Promise<{ success: boolean }> {
  return httpRequest<{ success: boolean }>('/api/iso/set-skills-enabled', {
    method: 'POST',
    body: JSON.stringify(data),
  })
}

export async function isoAgentUpdateConfig(data: IsoAgentUpdateConfigParams): Promise<{ success: boolean }> {
  return httpRequest<{ success: boolean }>('/api/iso/agent-update-config', {
    method: 'POST',
    body: JSON.stringify(data),
  })
}

export async function isoCreateSession(data: IsoCreateSessionParams): Promise<{ session_id: string; conversation_id: string }> {
  return httpRequest<{ session_id: string; conversation_id: string }>('/api/iso/create-session', {
    method: 'POST',
    body: JSON.stringify(data),
  })
}

export async function isoListSessions(data?: IsoAgentIdParams): Promise<{ sessions: IsoSessionInfo[] }> {
  return httpRequest<{ sessions: IsoSessionInfo[] }>('/api/iso/list-sessions', {
    method: 'POST',
    body: JSON.stringify(data || {}),
  })
}

export async function isoIndexWorkspace(data: IsoIndexWorkspaceParams): Promise<{ indexed: number }> {
  return httpRequest<{ indexed: number }>('/api/iso/index-workspace', {
    method: 'POST',
    body: JSON.stringify(data),
  })
}

export async function isoListWorkspace(data?: IsoAgentIdParams): Promise<{ files: Array<{ path: string; name: string }> }> {
  return httpRequest<{ files: Array<{ path: string; name: string }> }>('/api/iso/list-workspace', {
    method: 'POST',
    body: JSON.stringify(data || {}),
  })
}

export async function isoCleanup(data?: { daysThreshold?: number }): Promise<{ cleaned: number }> {
  return httpRequest<{ cleaned: number }>('/api/iso/cleanup', {
    method: 'POST',
    body: JSON.stringify(data || {}),
  })
}

export interface IsoGeneratePromptParams {
  displayName?: string
  category?: string
  purpose?: string
  scope?: string
  description?: string
  config: unknown
}

export async function isoGeneratePrompt(data: IsoGeneratePromptParams): Promise<{ prompt: string }> {
  return httpRequest<{ prompt: string }>('/api/iso/generate-prompt', {
    method: 'POST',
    body: JSON.stringify(data),
  })
}
