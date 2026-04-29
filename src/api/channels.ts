// Claw Desktop - 渠道管理API
// 提供渠道账号的CRUD、连接状态查询、启用/禁用、测试连接、发送消息等HTTP接口
import { httpRequest } from '../ws/http'

/** 渠道账号 — ID、类型、名称、启用状态 */
export interface ChannelAccount {
  id: string
  channel_type: string
  name: string
  enabled: boolean
  [key: string]: unknown
}

export interface ChannelStatusInfo {
  account_id: string
  connected: boolean
  error?: string
  [key: string]: unknown
}

export async function channelList(): Promise<{ accounts: ChannelAccount[] }> {
  return httpRequest<{ accounts: ChannelAccount[] }>('/api/channels', { method: 'GET' })
}

export async function channelStatus(): Promise<{ statuses: ChannelStatusInfo[] }> {
  return httpRequest<{ statuses: ChannelStatusInfo[] }>('/api/channels/status', { method: 'GET' })
}

export async function channelCreateAccount(data: unknown): Promise<{ success: boolean; account_id?: string }> {
  return httpRequest<{ success: boolean; account_id?: string }>('/api/channels/account', {
    method: 'POST',
    body: JSON.stringify(data),
  })
}

export async function channelUpdateAccount(data: unknown): Promise<{ success: boolean }> {
  return httpRequest<{ success: boolean }>('/api/channels/account/update', {
    method: 'POST',
    body: JSON.stringify(data),
  })
}

export async function channelDeleteAccount(data: unknown): Promise<{ success: boolean }> {
  return httpRequest<{ success: boolean }>('/api/channels/account/delete', {
    method: 'POST',
    body: JSON.stringify(data),
  })
}

export async function channelToggle(data: unknown): Promise<{ success: boolean; enabled?: boolean }> {
  return httpRequest<{ success: boolean; enabled?: boolean }>('/api/channels/toggle', {
    method: 'POST',
    body: JSON.stringify(data),
  })
}

export async function channelTestConnection(data: unknown): Promise<{ success: boolean; error?: string }> {
  return httpRequest<{ success: boolean; error?: string }>('/api/channels/test', {
    method: 'POST',
    body: JSON.stringify(data),
  })
}

export async function channelSendMessage(data: unknown): Promise<{ success: boolean; message_id?: string }> {
  return httpRequest<{ success: boolean; message_id?: string }>('/api/channels/send-message', {
    method: 'POST',
    body: JSON.stringify(data),
  })
}

export async function channelGetSchema(data?: unknown): Promise<{ schema: Record<string, unknown> }> {
  return httpRequest<{ schema: Record<string, unknown> }>('/api/channels/schema', {
    method: 'POST',
    ...(data ? { body: JSON.stringify(data) } : {}),
  })
}
