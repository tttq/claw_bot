// Claw Desktop - WS 桥接模块
// 前端与后端 WebSocket/HTTP 通信的统一入口
// 提供：wsInvoke(请求-响应)、wsStreamInvoke(流式HTTP)、wsOnEvent(事件监听)、subscribe(频道订阅)
// 所有参数自动从 camelCase 转换为 snake_case，匹配后端 Rust 命名约定
import { wsClient } from './client'
import { httpStreamInvoke } from './http'
import { eventClient } from './eventClient'
import { debugLog } from '../utils/debugLog'
import type { WsResponse } from './protocol'

function toSnakeCase(obj: Record<string, unknown>): Record<string, unknown> {
  const result: Record<string, unknown> = {}
  for (const [key, value] of Object.entries(obj)) {
    const snakeKey = key.replace(/[A-Z]/g, letter => `_${letter.toLowerCase()}`)
    result[snakeKey] = value
  }
  return result
}

let isReauthenticating = false
let reauthPromise: Promise<void> | null = null

async function ensureAuthenticated(): Promise<void> {
  if (isReauthenticating && reauthPromise) {
    debugLog('[WS:Bridge] Re-auth already in progress, waiting...')
    return reauthPromise
  }

  debugLog('[WS:Bridge] Starting re-authentication flow...')
  isReauthenticating = true
  reauthPromise = (async () => {
    try {
      debugLog('[WS:Bridge] Invalidating expired token')
      wsClient.invalidateToken()
      debugLog('[WS:Bridge] Calling authenticate() with cached public key...')
      await wsClient.authenticate()
      if (!wsClient.getConnectionState()) {
        debugLog('[WS:Bridge] WebSocket not connected, connecting...')
        await wsClient.connect()
      }
      debugLog('[WS:Bridge] Re-authentication complete!')
    } finally {
      isReauthenticating = false
      reauthPromise = null
    }
  })()

  return reauthPromise
}

export async function wsInvoke<T = unknown>(method: string, params: Record<string, unknown> = {}): Promise<T> {
  const snakeParams = toSnakeCase(params)

  let response: WsResponse
  try {
    response = await wsClient.request(method, snakeParams)
  } catch (e) {
    const errMsg = e instanceof Error ? e.message : String(e)
    if (errMsg.includes('Unauthorized') || errMsg.includes('invalid or expired') || errMsg.includes('Not connected')) {
      console.warn(`[WS] Auth error on ${method}, re-authenticating...`)
      await ensureAuthenticated()
      response = await wsClient.request(method, snakeParams)
    } else {
      throw e
    }
  }

  if (!response.success) {
    throw new Error(response.error || `WS invoke failed: ${method}`)
  }

  return response.data as T
}

export async function wsStreamInvoke(
  method: string,
  params: Record<string, unknown>,
  options?: { signal?: AbortSignal; onEvent?: (e: any) => void; onError?: (e: Error) => void }
): Promise<void> {
  const snakeParams = toSnakeCase(params)

  if (method === 'send_message_streaming') {
    return httpStreamInvoke('/api/conversations/streaming', snakeParams, options)
  }

  return httpStreamInvoke(`/api/${method}`, snakeParams, options)
}

export function wsOnEvent(method: string, handler: (data: unknown) => void): () => void {
  return wsClient.onEvent(method, (msg) => {
    handler(msg)
  })
}

export function subscribe(channel: string, handler: (data: any) => void): () => void {
  return eventClient.subscribe(channel, handler)
}

export { wsClient } from './client'
export { eventClient } from './eventClient'
export { fetchStreamChat, type ChatEvent } from './http'
