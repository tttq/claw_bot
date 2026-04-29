// Claw Desktop - HTTP 通信模块
// 封装与后端的 HTTP 通信：通用请求(401自动重认证)、流式SSE请求、聊天流式AsyncGenerator、便捷GET/POST
const HTTP_BASE_URL = ''

interface HttpResponse<T = unknown> {
  success: boolean
  data: T
  error?: string
}

let isReauthing = false
let reauthPromise: Promise<void> | null = null

async function doHandshake(pem: string): Promise<void> {
  const sessionKey = crypto.getRandomValues(new Uint8Array(32))
  const pemClean = pem.replace(/-----[A-Z ]+-----/g, '').replace(/\s/g, '')
  const pemBinary = atob(pemClean)
  const pemBytes = new Uint8Array(pemBinary.length)
  for (let i = 0; i < pemBinary.length; i++) { pemBytes[i] = pemBinary.charCodeAt(i) }
  const key = await crypto.subtle.importKey(
    'spki',
    pemBytes.buffer,
    { name: 'RSA-OAEP', hash: 'SHA-256' },
    false,
    ['encrypt']
  )
  const encrypted = await crypto.subtle.encrypt({ name: 'RSA-OAEP' }, key, sessionKey)
  const encryptedArray = new Uint8Array(encrypted)
  let b64 = ''
  const chunkSize = 8192
  for (let i = 0; i < encryptedArray.length; i += chunkSize) {
    const chunk = encryptedArray.subarray(i, i + chunkSize)
    b64 += String.fromCharCode.apply(null, Array.from(chunk))
  }
  b64 = btoa(b64)
  const res = await fetch(`${HTTP_BASE_URL}/api/auth/handshake`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ encryptedSessionKey: b64 })
  })
  if (!res.ok) throw new Error(`Handshake failed: ${res.status}`)
  const result = await res.json()
  if (!result.success) throw new Error(result.error)
  localStorage.setItem('ws_token', result.data.token)
  localStorage.setItem('ws_token_expires', String(result.data.expiresAt))
}

async function fetchServerPublicKey(): Promise<string> {
  const res = await fetch(`${HTTP_BASE_URL}/api/auth/public-key`, { method: 'GET' })
  if (!res.ok) throw new Error(`Failed to fetch public key: ${res.status}`)
  const result = await res.json()
  const pem = result.data || result
  if (typeof pem !== 'string' || !pem.includes('-----BEGIN')) {
    throw new Error('Server returned invalid public key')
  }
  return pem
}

async function reauthenticate(): Promise<void> {
  if (isReauthing) return reauthPromise || Promise.reject(new Error('Reauth already in progress'))
  isReauthing = true
  reauthPromise = (async () => {
    try {
      const { getEmbeddedPublicKey } = await import('./publicKey')
      const pem = getEmbeddedPublicKey()
      try {
        await doHandshake(pem)
        return
      } catch (e) {
        const errMsg = e instanceof Error ? e.message : String(e)
        if (errMsg.includes('401') || errMsg.includes('Handshake failed') || errMsg.includes('Decryption')) {
          console.warn('[HTTP:reauthenticate] Embedded key mismatch, fetching fresh public key from server...')
          const serverPem = await fetchServerPublicKey()
          await doHandshake(serverPem)
          return
        }
        throw e
      }
    } finally {
      isReauthing = false
      reauthPromise = null
    }
  })()
  return reauthPromise
}

async function httpRequest<T>(
  endpoint: string,
  options: RequestInit & { stream?: boolean } = {},
  _retryCount: number = 0
): Promise<T> {
  let token = localStorage.getItem('ws_token')

  const response = await fetch(`${HTTP_BASE_URL}${endpoint}`, {
    ...options,
    headers: {
      'Content-Type': 'application/json',
      ...(token ? { Authorization: `Bearer ${token}` } : {}),
      ...options.headers,
    },
  })

  if (response.status === 401 && _retryCount === 0 && !endpoint.includes('/auth/')) {
    console.warn('[HTTP] 401 received, attempting re-authentication...')
    localStorage.removeItem('ws_token')
    try {
      await reauthenticate()
      token = localStorage.getItem('ws_token') || ''
      return httpRequest(endpoint, options, 1)
    } catch (e) {
      console.error('[HTTP] Re-authentication failed:', e)
      throw new Error(`Authentication failed: ${e instanceof Error ? e.message : String(e)}`)
    }
  }

  if (!response.ok) {
    const errorBody = await response.text().catch(() => '')
    throw new Error(`HTTP ${response.status}: ${errorBody || response.statusText}`)
  }

  if (options.stream) {
    return response as unknown as T
  }

  const result: HttpResponse<T> = await response.json()
  if (!result.success && result.data === undefined) {
    throw new Error(result.error || 'Request failed')
  }
  return result.data as T
}

export { httpRequest, HTTP_BASE_URL }

export interface StreamOptions {
  signal?: AbortSignal
  onEvent?: (event: Record<string, unknown>) => void
  onError?: (error: Error) => void
}

export async function httpStreamInvoke(
  url: string,
  params: Record<string, unknown> = {},
  options: StreamOptions = {}
): Promise<void> {
  const token = localStorage.getItem('ws_token')

  const response = await fetch(`${HTTP_BASE_URL}${url}`, {
    method: 'POST',
    headers: {
      'Content-Type': 'application/json',
      ...(token ? { Authorization: `Bearer ${token}` } : {}),
    },
    body: JSON.stringify(params),
    signal: options.signal,
  })

  if (!response.ok) {
    const err = new Error(`Stream error: ${response.status}`)
    options.onError?.(err)
    throw err
  }

  if (!response.body) {
    const err = new Error('Response body is null')
    options.onError?.(err)
    throw err
  }

  const reader = response.body.getReader()
  const decoder = new TextDecoder()
  let buffer = ''

  try {
    while (true) {
      const { done, value } = await reader.read()
      if (done) break

      buffer += decoder.decode(value, { stream: true })
      const lines = buffer.split('\n')
      buffer = lines.pop() || ''

      for (const line of lines) {
        const trimmed = line.trim()
        if (!trimmed) continue

        const jsonStr = trimmed.startsWith('data: ') ? trimmed.slice(6) : trimmed
        try {
          const event = JSON.parse(jsonStr)
          options.onEvent?.(event)
        } catch {
        }
      }
    }
  } finally {
    reader.releaseLock()
  }
}

export interface ChatEvent {
  type: 'session_start' | 'connected'
      | 'thinking' | 'chunk'
      | 'tool_call' | 'tool_result'
      | 'chunk_reset' | 'done' | 'error'
  content?: string
  name?: string
  args?: Record<string, unknown>
  output?: string
  full_response?: string
  message?: string
  code?: string
}

export async function* fetchStreamChat(
  conversationId: string,
  message: string,
  options?: { signal?: AbortSignal }
): AsyncGenerator<ChatEvent> {
  const token = localStorage.getItem('ws_token')

  const response = await fetch(`${HTTP_BASE_URL}/api/chat/stream`, {
    method: 'POST',
    headers: {
      'Content-Type': 'application/json',
      ...(token ? { Authorization: `Bearer ${token}` } : {}),
    },
    body: JSON.stringify({ conversationId: conversationId, content: message }),
    signal: options?.signal,
  })

  if (!response.ok) {
    const errorBody = await response.text().catch(() => '')
    throw new Error(`Chat error ${response.status}: ${errorBody}`)
  }

  if (!response.body) {
    throw new Error('Response body is null')
  }

  const reader = response.body.getReader()
  const decoder = new TextDecoder()
  let buffer = ''

  try {
    while (true) {
      const { done, value } = await reader.read()
      if (done) break

      buffer += decoder.decode(value, { stream: true })

      const lines = buffer.split('\n')
      buffer = lines.pop() || ''

      for (const line of lines) {
        const trimmed = line.trim()
        if (!trimmed || !trimmed.startsWith('data: ')) continue

        const jsonStr = trimmed.slice(6)
        try {
          yield JSON.parse(jsonStr) as ChatEvent
        } catch {
        }
      }
    }
  } finally {
    reader.releaseLock()
  }
}

export function httpGet<T>(endpoint: string): Promise<T> {
  return httpRequest<T>(endpoint, { method: 'GET' })
}

export function httpPost<T>(endpoint: string, data?: unknown): Promise<T> {
  return httpRequest<T>(endpoint, {
    method: 'POST',
    body: data ? JSON.stringify(data) : undefined,
  })
}
