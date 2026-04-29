// Claw Desktop - WebSocket 客户端核心模块
// 封装 WebSocket 连接管理、RSA认证、自动重连(指数退避)、心跳保活(30s)、
// 请求-响应匹配、事件监听、降级模式、公钥混淆存储等全部逻辑
import { authHandshake, authValidate } from '../api/auth'
import { getEmbeddedPublicKey } from './publicKey'
import { httpRequest, HTTP_BASE_URL } from './http'
import { debugLog } from '../utils/debugLog'
import type { WsRequest, WsResponse, WsEvent } from './protocol'

/** 消息处理器类型 */
type MessageHandler = (response: WsResponse | WsEvent) => void
/** 连接状态变更处理器类型 */
type ConnectionHandler = () => void
/** 错误处理器类型 */
type ErrorHandler = (error: Event | Error) => void

/** WebSocket客户端 - 封装连接管理、RSA认证、自动重连、心跳保活等全部逻辑 */
class WsClient {
  private static readonly OBFUSCATE_PREFIX = 'qck_'           // 公钥混淆前缀
  private static readonly MAX_PENDING_REQUESTS = 50           // 最大待处理请求数
  private ws: WebSocket | null = null                         // WebSocket实例
  private url: string = ''                                    // 连接URL
  private token: string = ''                                  // 认证令牌
  private cachedPublicKey: string = ''                        // 缓存的RSA公钥
  private pendingRequests: Map<string, {                      // 待响应的请求映射
    resolve: (value: WsResponse) => void
    reject: (reason: string) => void
    timeout: ReturnType<typeof setTimeout>
  }> = new Map()
  private eventListeners: Map<string, Set<MessageHandler>> = new Map()  // 事件监听器
  private onConnectHandlers: Set<ConnectionHandler> = new Set()         // 连接成功处理器
  private onDisconnectHandlers: Set<ConnectionHandler> = new Set()      // 断开连接处理器
  private onErrorHandlers: Set<ErrorHandler> = new Set()                // 错误处理器
  private reconnectTimer: ReturnType<typeof setTimeout> | null = null   // 重连定时器
  private reconnectAttempts: number = 0                        // 当前重连次数
  private maxReconnectAttempts: number = 50                    // 最大重连次数
  private reconnectBaseDelay: number = 1000                    // 重连基础延迟(ms)
  private maxReconnectDelay: number = 30000                    // 重连最大延迟(ms)
  private heartbeatTimer: ReturnType<typeof setInterval> | null = null  // 心跳定时器
  private heartbeatInterval: number = 30000                    // 心跳间隔(ms)
  private readonly HEARTBEAT_TIMEOUT = 10000                   // 心跳超时(ms)
  private readonly maxHeartbeatMisses: number = 3              // 最大心跳丢失次数
  private heartbeatMissedCount: number = 0                     // 当前心跳丢失次数
  private isConnected: boolean = false                         // 是否已连接
  private isConnecting: boolean = false                        // 是否正在连接
  private intentionalClose: boolean = false                    // 是否主动关闭
  private degradationMode: boolean = false                     // 降级模式（仅使用HTTP）
  private connectionStartTime: number = 0                      // 连接开始时间
  private messageCount: number = 0                             // 消息计数

  /** 建立WebSocket连接，自动获取WS URL并完成握手 */
  async connect(): Promise<void> {
    if (this.isConnected) return
    if (this.isConnecting) {
      return new Promise(resolve => this.onConnectHandlers.add(resolve))
    }
    this.isConnecting = true
    this.intentionalClose = false

    try {
      let wsBaseUrl = 'ws://127.0.0.1:1421'
      try {
        const res = await fetch(`${HTTP_BASE_URL}/api/ws/url`, { headers: { Authorization: `Bearer ${this.token || ''}` } })
        if (res.ok) { const data = await res.json() as unknown as { data?: { url?: string } }; if (data?.data?.url) wsBaseUrl = data.data.url }
      } catch (e) {
        debugLog('[WS:Client] Failed to fetch WS URL:', e)
      }
      this.url = `${wsBaseUrl}/ws/events${this.token ? '?token=' + encodeURIComponent(this.token) : ''}`
    } catch (e) {
      this.isConnecting = false
      this.scheduleReconnect()
      return
    }

    if (!this.token) {
      this.isConnecting = false
      return
    }

    try {
      this.ws = new WebSocket(this.url)
    } catch (e) {
      this.isConnecting = false
      this.scheduleReconnect()
      return
    }

    const ws = this.ws!
    return new Promise((resolve, reject) => {
      const timeout = setTimeout(() => { reject(new Error('WebSocket connection timeout')) }, 15000)

      ws.onopen = () => {
        clearTimeout(timeout)
        this.isConnected = true
        this.isConnecting = false
        this.reconnectAttempts = 0
        this.heartbeatMissedCount = 0
        this.connectionStartTime = Date.now()
        this.messageCount = 0
        this.startHeartbeat()
        resolve()
        const handlers = [...this.onConnectHandlers]
        this.onConnectHandlers.clear()
        handlers.forEach(h => h())
      }

      ws.onmessage = (event) => {
        try {
          const msg = JSON.parse(event.data as string)

          if (msg.type === 'pong') {
            this.heartbeatMissedCount = 0
            return
          }

          this.messageCount++

          const typedMsg = msg as WsResponse | WsEvent

          if (typedMsg.type === 'response') {
            const pending = this.pendingRequests.get(typedMsg.id)
            if (pending) {
              clearTimeout(pending.timeout)
              this.pendingRequests.delete(typedMsg.id)
              pending.resolve(typedMsg as WsResponse)
            }
          } else if (typedMsg.type === 'stream') {
            const method = typedMsg.method
            const listeners = this.eventListeners.get(method)
            if (listeners) {
              listeners.forEach(h => h(typedMsg))
            }
            const wildcardListeners = this.eventListeners.get('*')
            if (wildcardListeners) {
              wildcardListeners.forEach(h => h(msg))
            }
          }
        } catch (e) {
          debugLog('[WS:Client] Failed to process message:', e)
        }
      }

      ws.onclose = () => {
        clearTimeout(timeout)
        this.isConnected = false
        this.isConnecting = false
        this.stopHeartbeat()
        this.rejectAllPending('Connection closed')
        debugLog('[WS:Client] Connection closed unexpectedly, will reconnect if needed')
        this.onDisconnectHandlers.forEach(h => h())
        if (!this.intentionalClose) {
          this.scheduleReconnect()
        }
      }

      ws.onerror = (event) => {
        clearTimeout(timeout)
        this.onErrorHandlers.forEach(h => h(event))
        reject(new Error('WebSocket connection error'))
      }
    })
  }

  /** 主动断开WebSocket连接 */
  disconnect(): void {
    this.intentionalClose = true
    this.cancelReconnect()
    this.stopHeartbeat()
    this.rejectAllPending('Client disconnected')
    if (this.ws) {
      this.ws.close()
      this.ws = null
    }
    this.isConnected = false
  }

  /** RSA认证 - 使用公钥加密会话密钥，获取访问令牌 */
  async authenticate(forceRefreshKey: boolean = false): Promise<void> {
    let publicKeyPem = getEmbeddedPublicKey()

    const doHandshake = async (pem: string) => {
      debugLog(`[WS:Auth] Using public key (${pem.length} chars)`)
      const sessionKey = crypto.getRandomValues(new Uint8Array(32))
      const encryptedBuffer = await this.encryptWithPublicKey(sessionKey, pem)
      const encryptedB64 = this.arrayBufferToBase64(encryptedBuffer)
      debugLog(`[WS:Auth] Session key encrypted (${encryptedB64.length} chars), calling handshake...`)
      const result = await authHandshake(encryptedB64) as { token: string; expiresAt: number }
      this.token = result.token
      localStorage.setItem('ws_token', result.token)
      localStorage.setItem('ws_token_expires', result.expiresAt.toString())
      debugLog(`[WS:Auth] Token obtained! expiresAt=${new Date(result.expiresAt * 1000).toLocaleString()}, prefix=${result.token.slice(0, 20)}...`)
      return result
    }

    try {
      await doHandshake(publicKeyPem)
    } catch (e) {
      const errMsg = (e as Error).message || ''
      if (errMsg.includes('401') || errMsg.includes('Decryption') || errMsg.includes('decryption')) {
        console.warn(`[WS:Auth] ⚠️ Embedded key mismatch, fetching fresh public key from server...`)
        try {
          const res = await httpRequest('/api/auth/public-key', { method: 'GET' })
          publicKeyPem = (res as unknown as { data?: string })?.data || (res as string) || ''
          if (!publicKeyPem?.includes('-----BEGIN')) throw new Error('Server returned invalid public key')
          debugLog(`[WS:Auth] Fetched server public key (${publicKeyPem.length} chars)`)
          await doHandshake(publicKeyPem)
        } catch (fetchErr) {
          throw new Error(`[WS:Auth] Both embedded and fetched key failed: ${(fetchErr as Error).message}`)
        }
      } else {
        throw e
      }
    }
  }

  /** 使令牌失效，清除本地存储 */
  invalidateToken(): void {
    this.token = ''
    localStorage.removeItem('ws_token')
    localStorage.removeItem('ws_token_expires')
  }

  /** 确保已认证，令牌过期时自动重新认证 */
  async ensureAuthenticated(): Promise<boolean> {
    const expires = localStorage.getItem('ws_token_expires')
    if (expires) {
      const expiresAt = parseInt(expires, 10)
      const bufferSec = 300
      if (expiresAt > Date.now() / 1000 + bufferSec) return true
      debugLog('[WS:Auth] Token expired, re-authenticating...')
    }
    try {
      await this.authenticate()
      return true
    } catch (e) {
      console.error('[WS:Auth] Re-authentication failed:', e)
      return false
    }
  }

  clearPublicKeyCache(): void {
    this.cachedPublicKey = ''
    localStorage.removeItem('ws_server_public_key')
  }

  private static obfuscate(plainText: string): string {
    const encoded = btoa(unescape(encodeURIComponent(plainText)))
    let obscured = ''
    for (let i = 0; i < encoded.length; i++) {
      obscured += String.fromCharCode(encoded.charCodeAt(i) ^ (((i * 7 + 13) & 0xFF)))
    }
    return WsClient.OBFUSCATE_PREFIX + btoa(obscured)
  }

  private static deobfuscate(obscured: string): string | null {
    if (!obscured) return null
    if (obscured.startsWith(WsClient.OBFUSCATE_PREFIX)) {
      try {
        const encodedB64 = obscured.slice(WsClient.OBFUSCATE_PREFIX.length)
        const obscuredBin = atob(encodedB64)
        let encoded = ''
        for (let i = 0; i < obscuredBin.length; i++) {
          encoded += String.fromCharCode(obscuredBin.charCodeAt(i) ^ (((i * 7 + 13) & 0xFF)))
        }
        return decodeURIComponent(escape(atob(encoded)))
      } catch { return null }
    }
    if (obscured.includes('-----BEGIN') && obscured.includes('PUBLIC KEY')) {
      debugLog(`[WS:Auth] Migrating plaintext key to obfuscated format`)
      return obscured
    }
    return null
  }

  async validateToken(): Promise<boolean> {
    if (!this.token) return false
    try {
      return await authValidate(this.token)
    } catch {
      return false
    }
  }

  logout(): void {
    this.token = ''
    this.cachedPublicKey = ''
    localStorage.removeItem('ws_token')
    localStorage.removeItem('ws_token_expires')
    localStorage.removeItem('ws_server_public_key')
    this.disconnect()
  }

  /** 发送WS请求并等待响应（超时120秒） */
  async request(method: string, params: Record<string, unknown> = {}): Promise<WsResponse> {
    if (this.degradationMode) {
      throw new Error(`[Degradation] WS disabled, use HTTP for: ${method}`)
    }

    if (!this.isConnected) {
      await this.connect()
    }

    if (!this.isConnected) {
      throw new Error(`Not connected, cannot send: ${method}`)
    }

    if (this.pendingRequests.size >= WsClient.MAX_PENDING_REQUESTS) {
      throw new Error(`Too many pending requests (${this.pendingRequests.size}/${WsClient.MAX_PENDING_REQUESTS}), method: ${method}`)
    }

    const id = crypto.randomUUID()
    const request: WsRequest = {
      id,
      type: 'request',
      method,
      params,
      token: this.token,
    }

    return new Promise<WsResponse>((resolve, reject) => {
      const timeout = setTimeout(() => {
        this.pendingRequests.delete(id)
        reject(new Error(`Request timeout: ${method}`))
      }, 120000)

      this.pendingRequests.set(id, { resolve, reject, timeout })

      try {
        this.ws!.send(JSON.stringify(request))
      } catch (e) {
        this.pendingRequests.delete(id)
        clearTimeout(timeout)
        reject(new Error(`Failed to send request: ${method}`))
      }
    })
  }

  /** 注册事件监听器，返回取消监听函数 */
  onEvent(method: string, handler: MessageHandler): () => void {
    if (!this.eventListeners.has(method)) {
      this.eventListeners.set(method, new Set())
    }
    this.eventListeners.get(method)!.add(handler)
    return () => {
      this.eventListeners.get(method)?.delete(handler)
    }
  }

  /** 注册连接成功回调，返回取消注册函数 */
  onConnect(handler: ConnectionHandler): () => void {
    this.onConnectHandlers.add(handler)
    return () => { this.onConnectHandlers.delete(handler) }
  }

  /** 注册断开连接回调，返回取消注册函数 */
  onDisconnect(handler: ConnectionHandler): () => void {
    this.onDisconnectHandlers.add(handler)
    return () => { this.onDisconnectHandlers.delete(handler) }
  }

  /** 注册错误回调，返回取消注册函数 */
  onError(handler: ErrorHandler): () => void {
    this.onErrorHandlers.add(handler)
    return () => { this.onErrorHandlers.delete(handler) }
  }

  /** 获取连接状态 */
  getConnectionState(): boolean {
    return this.isConnected
  }

  /** 启用降级模式（所有请求通过HTTP发送） */
  enableDegradationMode(): void {
    this.degradationMode = true
    console.warn('[WS] Degradation mode enabled - all requests via HTTP')
  }

  /** 禁用降级模式 */
  disableDegradationMode(): void {
    this.degradationMode = false
  }

  getDegradationState(): boolean {
    return this.degradationMode
  }

  /** 获取连接诊断信息（运行时间、消息数、待处理请求数等） */
  getConnectionDiagnostics(): {
    uptime: number
    messageCount: number
    pendingRequests: number
    eventListeners: number
    heartbeatMissedCount: number
    isConnected: boolean
    degradationMode: boolean
    maxPending: number
  } {
    return {
      uptime: this.connectionStartTime ? Date.now() - this.connectionStartTime : 0,
      messageCount: this.messageCount,
      pendingRequests: this.pendingRequests.size,
      eventListeners: Array.from(this.eventListeners.values())
        .reduce((sum, set) => sum + set.size, 0),
      heartbeatMissedCount: this.heartbeatMissedCount,
      isConnected: this.isConnected,
      degradationMode: this.degradationMode,
      maxPending: WsClient.MAX_PENDING_REQUESTS,
    }
  }

  /** 获取当前认证令牌 */
  getToken(): string {
    return this.token
  }

  /** 尝试从localStorage恢复令牌和公钥缓存 */
  tryRestoreToken(): boolean {
    const token = localStorage.getItem('ws_token')
    const expires = localStorage.getItem('ws_token_expires')
    if (token && expires) {
      const expiresAt = parseInt(expires, 10)
      if (expiresAt > Date.now() / 1000) {
        this.token = token
        const rawCachedKey = localStorage.getItem('ws_server_public_key')
        if (rawCachedKey) {
          const decoded = WsClient.deobfuscate(rawCachedKey)
          if (decoded) this.cachedPublicKey = decoded
        }
        return true
      }
      localStorage.removeItem('ws_token')
      localStorage.removeItem('ws_token_expires')
    }
    return false
  }

  /** 使用RSA公钥加密数据 */
  private async encryptWithPublicKey(data: Uint8Array, publicKeyPem: string): Promise<ArrayBuffer> {
    const pemBody = publicKeyPem
      .replace(/-----BEGIN RSA PUBLIC KEY-----/, '')
      .replace(/-----END RSA PUBLIC KEY-----/, '')
      .replace(/-----BEGIN PUBLIC KEY-----/, '')
      .replace(/-----END PUBLIC KEY-----/, '')
      .replace(/\s/g, '')

    const binaryStr = atob(pemBody)
    const bytes = new Uint8Array(binaryStr.length)
    for (let i = 0; i < binaryStr.length; i++) {
      bytes[i] = binaryStr.charCodeAt(i)
    }

    const keyData = bytes.buffer

    const key = await crypto.subtle.importKey(
      'spki',
      keyData,
      { name: 'RSA-OAEP', hash: 'SHA-256' },
      false,
      ['encrypt']
    )

    return crypto.subtle.encrypt(
      { name: 'RSA-OAEP' },
      key,
      data
    )
  }

  /** ArrayBuffer转Base64字符串 */
  private arrayBufferToBase64(buffer: ArrayBuffer): string {
    const bytes = new Uint8Array(buffer)
    let binary = ''
    for (let i = 0; i < bytes.byteLength; i++) {
      binary += String.fromCharCode(bytes[i])
    }
    return btoa(binary)
  }

  /** 启动心跳保活（每30秒发送ping，连续3次无pong则断开） */
  private startHeartbeat(): void {
    this.stopHeartbeat()
    this.heartbeatTimer = setInterval(() => {
      if (this.ws && this.ws.readyState === WebSocket.OPEN) {
        if (this.heartbeatMissedCount >= this.maxHeartbeatMisses) {
          console.warn(`[WS] Too many missed pongs (${this.heartbeatMissedCount}), closing connection`)
          this.ws.close(4000, 'Heartbeat timeout')
          return
        }
        this.ws.send(JSON.stringify({ type: 'ping' }))
        this.heartbeatMissedCount++
      }
    }, this.heartbeatInterval)
  }

  /** 停止心跳保活 */
  private stopHeartbeat(): void {
    if (this.heartbeatTimer) {
      clearInterval(this.heartbeatTimer)
      this.heartbeatTimer = null
    }
  }

  /** 调度重连（指数退避，最大延迟30秒，最多50次） */
  private scheduleReconnect(): void {
    if (this.intentionalClose) return
    if (this.reconnectAttempts >= this.maxReconnectAttempts) {
      debugLog('[WS:Client] Max reconnect attempts reached, giving up')
      return
    }

    const delay = Math.min(
      this.reconnectBaseDelay * Math.pow(2, this.reconnectAttempts),
      this.maxReconnectDelay
    ) + Math.random() * 1000

    this.reconnectAttempts++
    this.reconnectTimer = setTimeout(() => {
      this.connect()
    }, delay)
  }

  /** 取消重连定时器 */
  private cancelReconnect(): void {
    if (this.reconnectTimer) {
      clearTimeout(this.reconnectTimer)
      this.reconnectTimer = null
    }
    this.reconnectAttempts = 0
  }

  /** 拒绝所有待处理请求（连接断开时调用） */
  private rejectAllPending(reason: string): void {
    for (const [id, pending] of this.pendingRequests) {
      clearTimeout(pending.timeout)
      pending.reject(reason)
    }
    this.pendingRequests.clear()
  }
}

/** 全局WebSocket客户端单例 */
export const wsClient = new WsClient()
