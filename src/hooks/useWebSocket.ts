// Claw Desktop - WebSocket 连接与认证 Hook
// 提供 WebSocket 连接状态管理、事件监听、认证/登出等功能
import { useState, useEffect, useCallback, useRef } from 'react'
import { wsClient } from '../ws/client'
import type { WsEvent } from '../ws/protocol'

/** WebSocket 连接状态类型 */
export type ConnectionState = 'disconnected' | 'connecting' | 'connected' | 'reconnecting'

/** WebSocket 连接管理 Hook：监听连接/断开事件，提供手动连接/断开操作 */
export function useWebSocket() {
  const [state, setState] = useState<ConnectionState>(
    wsClient.getConnectionState() ? 'connected' : 'disconnected'
  )

  useEffect(() => {
    const unsubConnect = wsClient.onConnect(() => setState('connected'))
    const unsubDisconnect = wsClient.onDisconnect(() => setState('disconnected'))

    return () => {
      unsubConnect()
      unsubDisconnect()
    }
  }, [])

  const connect = useCallback(async () => {
    setState('connecting')
    await wsClient.connect()
  }, [])

  const disconnect = useCallback(() => {
    wsClient.disconnect()
    setState('disconnected')
  }, [])

  return { state, connect, disconnect, isConnected: state === 'connected' }
}

/** WebSocket 单事件监听 Hook：注册指定方法的事件处理器，组件卸载时自动取消 */
export function useWsEvent(method: string, handler: (event: WsEvent) => void) {
  const handlerRef = useRef(handler)
  handlerRef.current = handler

  useEffect(() => {
    const unsub = wsClient.onEvent(method, (msg) => {
      handlerRef.current(msg as WsEvent)
    })
    return unsub
  }, [method])
}

/** 认证管理 Hook：提供 RSA 认证、令牌恢复、登出、令牌验证等操作 */
export function useAuth() {
  const [isAuthenticated, setIsAuthenticated] = useState(() => {
    return wsClient.tryRestoreToken()
  })
  const [isLoading, setIsLoading] = useState(false)
  const [error, setError] = useState<string | null>(null)

  const authenticate = useCallback(async () => {
    setIsLoading(true)
    setError(null)
    try {
      await wsClient.authenticate()
      setIsAuthenticated(true)
      await wsClient.connect()
    } catch (e) {
      const msg = e instanceof Error ? e.message : 'Authentication failed'
      setError(msg)
      setIsAuthenticated(false)
      throw e
    } finally {
      setIsLoading(false)
    }
  }, [])

  const logout = useCallback(() => {
    wsClient.logout()
    setIsAuthenticated(false)
  }, [])

  const checkAuth = useCallback(async () => {
    const valid = await wsClient.validateToken()
    setIsAuthenticated(valid)
    return valid
  }, [])

  return { isAuthenticated, isLoading, error, authenticate, logout, checkAuth }
}
