// Claw Desktop - WebSocket 事件监听 Hook
// 监听后端推送的流式聊天事件（start/token/tool_execution/done/error），
// 并将事件数据映射到前端会话状态，驱动 UI 更新
import { useEffect, useCallback, useRef } from 'react'
import { useTranslation } from 'react-i18next'
import { wsOnEvent, wsClient } from '../ws/bridge'
import { useConversationStore } from '../stores/conversationStore'
import { useStreamingStore } from '../stores/streamingStore'
import { useUIStore } from '../stores/uiStore'
import { useWSStore } from '../stores/wsStore'
import type { Message } from '../types'
import type { ToolExecutionDetail } from '../stores/conversationStore'

/** 流式生成超时时间（3分钟） */
const STREAMING_TIMEOUT_MS = 180000
/** 流式空闲超时时间（45秒）：无 token/tool_execution 活动则强制结束流 */
const STALE_STREAM_TIMEOUT_MS = 45000

/** WebSocket 事件监听 Hook：接收后端流式事件并更新前端状态 */
export function useWebSocketEvents(scheduleStreamingRender: () => void) {
  const { t } = useTranslation()
  const { setConvState, initConvState, removeConvState } = useConversationStore()
  const { setStreamingText, clearStreamingText, rafIdRef, setRafId } = useStreamingStore()
  const { setPendingConfirmation, setToast } = useUIStore()
  const { setWsReady } = useWSStore()

  const loadingTimersRef = useRef<Map<string, number>>(new Map())
  const sendingRefs = useRef<Set<string>>(new Set())
  const staleStreamTimersRef = useRef<Map<string, number>>(new Map())

  /** 清除指定会话的加载超时定时器 */
  const clearLoadingTimer = useCallback((id: string) => {
    const existing = loadingTimersRef.current.get(id)
    if (existing) {
      clearTimeout(existing)
      loadingTimersRef.current.delete(id)
    }
  }, [])

  /** 清除指定会话的流式空闲检测定时器 */
  const clearStaleStreamTimer = useCallback((id: string) => {
    const existing = staleStreamTimersRef.current.get(id)
    if (existing) {
      clearTimeout(existing)
      staleStreamTimersRef.current.delete(id)
    }
  }, [])

  /** 设置流式空闲检测定时器：超时后强制结束流式生成并保存已生成文本 */
  const setStaleStreamTimeout = useCallback(
    (id: string) => {
      clearStaleStreamTimer(id)
      const timer = setTimeout(() => {
        const existing = useConversationStore.getState().convState[id]
        if (!existing?.isLoading) return
        console.warn(`[StaleStream] No activity for conv ${id.slice(0, 16)} in ${STALE_STREAM_TIMEOUT_MS / 1000}s, forcing stream end`)
        const finalText = existing.streamingText || ''
        const updatedMessages = finalText
          ? [
              ...existing.messages,
              { id: crypto.randomUUID(), role: 'assistant' as const, content: finalText, timestamp: Date.now() },
            ]
          : existing.messages
        setConvState(id, {
          messages: updatedMessages,
          isLoading: false,
          streamingText: undefined,
          thinkingText: undefined,
          toolExecutions: [],
          toolExecutionDetails: [],
          multiAgentMessages: [],
        })
        clearStreamingText(id)
        sendingRefs.current.delete(id)
        clearLoadingTimer(id)
      }, STALE_STREAM_TIMEOUT_MS)
      staleStreamTimersRef.current.set(id, timer)
    },
    [clearStaleStreamTimer, setConvState, clearStreamingText, clearLoadingTimer],
  )

  /** 设置加载超时定时器：超时后自动终止流式生成并显示错误 */
  const setLoadingTimeout = useCallback(
    (id: string) => {
      clearLoadingTimer(id)
      const timer = setTimeout(() => {
        console.warn(`[Streaming Timeout] Conversation ${id} exceeded ${STREAMING_TIMEOUT_MS / 1000}s timeout`)
        setConvState(id, {
          messages: [
            ...(useConversationStore.getState().convState[id]?.messages ?? []),
            {
              id: crypto.randomUUID(),
              role: 'assistant',
              content: t('errors.timeout'),
              timestamp: Date.now(),
              isError: true,
            } as Message,
          ],
          isLoading: false,
          streamingText: undefined,
          toolExecutions: [],
          multiAgentMessages: [],
        })
        sendingRefs.current.delete(id)
      }, STREAMING_TIMEOUT_MS)
      loadingTimersRef.current.set(id, timer)
    },
    [clearLoadingTimer, setConvState, t],
  )

  useEffect(() => {
    const unsub = wsOnEvent('send_message_streaming', (msg) => {
      const payload = (
        msg as {
          data: {
            type: string
            conversation_id?: string
            content?: string
            full_text?: string
            tool_name?: string
            duration_ms?: number
            tool_index?: number
            total_tools?: number
            round?: number
            tool_input?: string
            tool_result?: string
            tool_executions?: Array<{
              round: number
              tool_name: string
              tool_input: string
              tool_result: string
              duration_ms: number
            }>
            status?: string
          }
        }
      ).data
      if (!payload) return

      const convId = payload.conversation_id
      if (!convId) return

      if (payload.type === 'start') {
        clearLoadingTimer(convId)
        setLoadingTimeout(convId)
        setStaleStreamTimeout(convId)
        sendingRefs.current.add(convId)
        setStreamingText(convId, '')
        setConvState(convId, { isLoading: true, streamingText: '', thinkingText: '', toolExecutions: [], toolExecutionDetails: [] })
      } else if (payload.type === 'thinking' && payload.content) {
        setStaleStreamTimeout(convId)
        const existing = useConversationStore.getState().convState[convId]
        if (!existing) return
        const prevThinking = existing.thinkingText || ''
        setConvState(convId, { thinkingText: prevThinking + payload.content })
      } else if (payload.type === 'tool_execution' && payload.tool_name) {
        setStaleStreamTimeout(convId)
        const existing = useConversationStore.getState().convState[convId]
        if (!existing) return
        const newToolExec = {
          toolName: payload.tool_name!,
          durationMs: payload.duration_ms || 0,
          index: payload.tool_index || (existing.toolExecutions.length + 1),
          total: payload.total_tools || (existing.toolExecutions.length + 1),
        }

        const newDetail: ToolExecutionDetail = {
          toolName: payload.tool_name!,
          durationMs: payload.duration_ms || 0,
          index: payload.tool_index || (existing.toolExecutionDetails.length + 1),
          total: payload.total_tools || (existing.toolExecutionDetails.length + 1),
          round: payload.round,
          status: (payload.status === 'completed' ? 'completed' : 'running') as 'running' | 'completed',
          toolInput: payload.tool_input,
          toolResult: payload.tool_result,
        }

        if (payload.status === 'completed') {
          const updatedDetails = existing.toolExecutionDetails.map(d =>
            d.toolName === payload.tool_name && d.status === 'running' && d.round === payload.round
              ? { ...d, status: 'completed' as const, durationMs: payload.duration_ms || 0, toolResult: payload.tool_result }
              : d
          )
          const alreadyUpdated = updatedDetails.some(d => d.toolName === payload.tool_name && d.status === 'completed' && d.round === payload.round && d.durationMs === (payload.duration_ms || 0))
          if (!alreadyUpdated) {
            updatedDetails.push(newDetail)
          }
          setConvState(convId, { toolExecutions: [...existing.toolExecutions, newToolExec], toolExecutionDetails: updatedDetails })
        } else {
          setConvState(convId, { toolExecutions: [...existing.toolExecutions, newToolExec], toolExecutionDetails: [...existing.toolExecutionDetails, newDetail] })
        }
      } else if (payload.type === 'token' && payload.content) {
        const currentTexts = useStreamingStore.getState().streamingTextRef
        if (!currentTexts.hasOwnProperty(convId)) {
          console.warn(`[Streaming] Token event received before 'start' for conv ${convId}, auto-initializing`)
          setStreamingText(convId, '')
          clearLoadingTimer(convId)
          setLoadingTimeout(convId)
          setStaleStreamTimeout(convId)
          setConvState(convId, { isLoading: true, streamingText: '', thinkingText: '', toolExecutions: [], toolExecutionDetails: [] })
        }
        setStaleStreamTimeout(convId)
        const prev = useStreamingStore.getState().streamingTextRef[convId] || ''
        setStreamingText(convId, prev + payload.content)
        scheduleRendering()
      } else if (payload.type === 'done') {
        if (!sendingRefs.current.has(convId)) {
          return
        }
        clearLoadingTimer(convId)
        clearStaleStreamTimer(convId)
        const hadStreamingState = useStreamingStore.getState().streamingTextRef.hasOwnProperty(convId)
        clearStreamingText(convId)
        if (!hadStreamingState) {
          console.warn(`[Streaming] 'done' event for conv ${convId} without matching 'start', performing cleanup anyway`)
        }

        const remainingStreams = Object.keys(useStreamingStore.getState().streamingTextRef).length
        if (remainingStreams === 0 && useStreamingStore.getState().rafIdRef !== null) {
          cancelAnimationFrame(useStreamingStore.getState().rafIdRef!)
          useStreamingStore.getState().setRafId(null)
        } else if (remainingStreams > 0) {
          scheduleRendering()
        }

        const finalText = payload.full_text || ''
        const trimmedFinal = finalText.trim()
        const signalMarkers = {
          confirm: '[CONFIRM_REQUIRED]',
          input: '[INPUT_REQUIRED]',
          taskInProgress: '[TASK_IN_PROGRESS]',
          responseComplete: '[RESPONSE_COMPLETE]',
        }
        const hasConfirmMarker = trimmedFinal.includes(signalMarkers.confirm)
        const hasInputMarker = trimmedFinal.includes(signalMarkers.input)
        const hasTaskInProgressMarker = trimmedFinal.includes(signalMarkers.taskInProgress)
        const hasResponseCompleteMarker = trimmedFinal.includes(signalMarkers.responseComplete)

        let cleanText = trimmedFinal
        for (const marker of Object.values(signalMarkers)) {
          cleanText = cleanText.split(marker).join('').trim()
        }

        const hasAnySignal = hasConfirmMarker || hasInputMarker || hasTaskInProgressMarker || hasResponseCompleteMarker

        const existing = useConversationStore.getState().convState[convId] || {
          messages: [],
          isLoading: true,
          multiAgentMessages: [],
          toolExecutions: [],
          toolExecutionDetails: [],
          thinkingText: '',
        }
        const displayText = cleanText || finalText
        const finalThinkingText = existing.thinkingText || ''
        const finalToolDetails = existing.toolExecutionDetails || []

        const doneToolDetails: ToolExecutionDetail[] = (payload.tool_executions || []).map((te, i) => ({
          toolName: te.tool_name,
          durationMs: te.duration_ms,
          index: i + 1,
          total: payload.tool_executions!.length,
          round: te.round,
          status: 'completed' as const,
          toolInput: te.tool_input,
          toolResult: te.tool_result,
        }))

        const mergedToolDetails = finalToolDetails.length > 0 ? finalToolDetails : doneToolDetails

        const updatedMessages = displayText
          ? [
              ...existing.messages,
              {
                id: crypto.randomUUID(),
                role: 'assistant',
                content: displayText,
                timestamp: Date.now(),
                thinkingText: finalThinkingText || undefined,
                toolExecutionDetails: mergedToolDetails.length > 0 ? mergedToolDetails : undefined,
                signalStatus: hasConfirmMarker ? 'confirm_required' as const
                  : hasInputMarker ? 'input_required' as const
                  : hasTaskInProgressMarker ? 'task_in_progress' as const
                  : hasResponseCompleteMarker ? 'response_complete' as const
                  : hasAnySignal ? 'response_complete' as const
                  : 'response_complete' as const,
              } as Message,
            ]
          : existing.messages

        setConvState(convId, {
          messages: updatedMessages,
          isLoading: false,
          streamingText: undefined,
          thinkingText: undefined,
          toolExecutions: [],
          toolExecutionDetails: [],
          multiAgentMessages: [],
        })

        sendingRefs.current.delete(convId)

        if (hasConfirmMarker && cleanText.length > 5) {
          setPendingConfirmation({
            conversationId: convId,
            prompt: cleanText.slice(-200),
            options: [
              { label: t('errors.confirmYes'), value: t('panels.confirmOptions.yesContinue') },
              { label: t('errors.confirmNo'), value: t('panels.confirmOptions.noThanks') },
              { label: t('errors.confirmAnother'), value: t('panels.confirmOptions.tryAnother') },
            ],
          })
        }

        if (hasInputMarker) {
          setPendingConfirmation({
            conversationId: convId,
            prompt: cleanText.slice(-200),
            options: [
              { label: t('errors.confirmYes', 'Submit'), value: 'submit' },
              { label: t('errors.confirmNo', 'Cancel'), value: 'cancel' },
            ],
          })
        }
      } else if (payload.type === 'error') {
        clearLoadingTimer(convId)
        clearStaleStreamTimer(convId)
        clearStreamingText(convId)
        const errorText = payload.content || 'Unknown error'
        console.error(`[Streaming Error] Conversation ${convId}: ${errorText}`)
        const existing = useConversationStore.getState().convState[convId] || {
          messages: [],
          isLoading: true,
          multiAgentMessages: [],
          toolExecutions: [],
          toolExecutionDetails: [],
        }
        setConvState(convId, {
          messages: [
            ...existing.messages,
            {
              id: crypto.randomUUID(),
              role: 'assistant',
              content: `Error: ${errorText}`,
              timestamp: Date.now(),
              isError: true,
            } as Message,
          ],
          isLoading: false,
          streamingText: undefined,
          thinkingText: undefined,
          toolExecutions: [],
          toolExecutionDetails: [],
          multiAgentMessages: [],
        })
        sendingRefs.current.delete(convId)
      }
    })

    let scheduled = false
    const scheduleRendering = () => {
      if (scheduled) return
      scheduled = true
      requestAnimationFrame(() => {
        scheduled = false
        scheduleStreamingRender()
      })
    }

    return () => { unsub() }
  }, [t, scheduleStreamingRender, setConvState, initConvState, setStreamingText, clearStreamingText, clearLoadingTimer, setLoadingTimeout, setStaleStreamTimeout, clearStaleStreamTimer, setPendingConfirmation])

  /** 初始化认证：尝试恢复令牌或执行自动认证，然后建立 WebSocket 连接 */
  const initAuth = useCallback(async () => {
    const restored = wsClient.tryRestoreToken()
    if (!restored) {
      try {
        await wsClient.authenticate()
      } catch (e) {
        console.error('Auto-authentication failed:', e)
        setWsReady(true)
        return
      }
    }
    await wsClient.connect()
    setWsReady(true)
  }, [setWsReady])

  return { initAuth, sendingRefs }
}
