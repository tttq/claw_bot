// Claw Desktop - 事件订阅客户端模块
// 基于 WebSocket 的发布/订阅事件客户端，支持频道订阅、自动重连、通配符'*'订阅
import { debugLog } from '../utils/debugLog'

export type EventHandler = (data: unknown) => void

class EventClient {
  private ws: WebSocket | null = null
  private url: string = ''
  private token: string = ''
  private subscriptions: Map<string, Set<EventHandler>> = new Map()
  private reconnectTimer: ReturnType<typeof setTimeout> | null = null
  private isConnected: boolean = false

  async connect(url?: string, token?: string): Promise<void> {
    this.url = url || 'ws://127.0.0.1:1421/ws/events'
    this.token = token || localStorage.getItem('ws_token') || ''

    return new Promise((resolve, reject) => {
      this.ws = new WebSocket(this.url)

      this.ws.onopen = () => {
        this.isConnected = true
        debugLog('[EventClient] Connected to event server')

        this.ws!.send(JSON.stringify({
          type: 'subscribe',
          token: this.token,
          channels: Array.from(this.subscriptions.keys()),
        }))

        resolve()
      }

      this.ws.onmessage = (event) => {
        try {
          const msg = JSON.parse(event.data as string)
          this.handleEvent(msg)
        } catch {
          // Ignore non-JSON messages
        }
      }

      this.ws.onclose = () => {
        this.isConnected = false
        debugLog('[EventClient] Disconnected')
        this.scheduleReconnect()
      }

      this.ws.onerror = () => {
        reject(new Error('WebSocket connection error'))
      }

      setTimeout(() => reject(new Error('Connection timeout')), 10000)
    })
  }

  subscribe(channel: string, handler: EventHandler): () => void {
    if (!this.subscriptions.has(channel)) {
      this.subscriptions.set(channel, new Set())
    }
    this.subscriptions.get(channel)!.add(handler)

    if (this.isConnected && this.ws) {
      this.ws.send(JSON.stringify({
        type: 'subscribe',
        channel,
      }))
    }

    return () => {
      this.subscriptions.get(channel)?.delete(handler)
      if (this.isConnected && this.ws) {
        this.ws.send(JSON.stringify({ type: 'unsubscribe', channel }))
      }
    }
  }

  private handleEvent(msg: { channel?: string; event?: string; data?: unknown }) {
    const channel = msg.channel || msg.event || '*'
    const handlers = this.subscriptions.get(channel)
    if (handlers) {
      handlers.forEach(h => h(msg.data))
    }
    const wildcardHandlers = this.subscriptions.get('*')
    if (wildcardHandlers) {
      wildcardHandlers.forEach(h => h(msg))
    }
  }

  disconnect(): void {
    if (this.reconnectTimer) {
      clearTimeout(this.reconnectTimer)
    }
    if (this.ws) {
      this.ws.close()
      this.ws = null
    }
    this.isConnected = false
  }

  get connected(): boolean {
    return this.isConnected
  }

  private scheduleReconnect(): void {
    this.reconnectTimer = setTimeout(() => {
      if (this.url && this.token) {
        this.connect().catch((e) => { console.error(e) })
      }
    }, 5000)
  }
}

export const eventClient = new EventClient()
