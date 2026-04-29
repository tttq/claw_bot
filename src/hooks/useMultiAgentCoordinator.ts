// Claw Desktop - 多 Agent 协调器 Hook
// 封装消息发送、多 Agent 协作执行、流式生成停止、用户确认等核心交互逻辑
// 支持：单 Agent 流式对话、@提及多 Agent 协作、附件处理、错误恢复
import { useCallback, useRef, useEffect } from 'react'
import { useTranslation } from 'react-i18next'
import { useConversationStore } from '../stores/conversationStore'
import { useAgentStore } from '../stores/agentStore'
import { useUIStore } from '../stores/uiStore'
import { debugLog } from '../utils/debugLog'
import { useStreamingStore } from '../stores/streamingStore'
import { createConversation, sendMessageStreaming, cancelStream } from '../api'
import { parseMentions, agentCoordinator, buildMentionDisplayText } from '../multiagent'
import { agentRegistry } from '../multiagent/agentRegistry'
import { MultiAgentSessionStatus } from '../multiagent/types'
import type { MultiAgentMessageContent, MentionedAgent, SubAgentStatus } from '../multiagent/types'
import type { Message } from '../types'

/** 多 Agent 协调器 Hook：管理消息发送和多 Agent 协作流程 */
export function useMultiAgentCoordinator() {
  const { t } = useTranslation()
  const { activeConversationId, setConvState, initConvState, convState, addConversation, setActiveConversationId, updateConversation } = useConversationStore()
  const { activeAgentId, agents } = useAgentStore()
  const { setActiveMultiAgentSession, setPendingConfirmation, setToast } = useUIStore()
  const { clearStreamingText } = useStreamingStore()

  const creatingConvRef = useRef(false)
  const sendingRefs = useRef<Set<string>>(new Set())
  const sendTimeoutRefs = useRef<Map<string, ReturnType<typeof setTimeout>>>(new Map())

  useEffect(() => {
    if (agents && agents.length > 0) {
      agentRegistry.registerCustomAgents(agents)
    }
  }, [agents])

  /** 发送消息：支持文本、附件、@提及多 Agent，自动创建会话 */
  const handleSendMessage = useCallback(
    async (
      content: string,
      attachments?: Array<{ name: string; type: string; dataUrl: string; size: number }>,
      mentionedAgentIds?: string[],
    ) => {
      if (!content.trim() && (!attachments || attachments.length === 0)) return
      if (!activeAgentId) return

      let finalContent = content.trim()
      const mediaAttachments: Array<{ data_url: string; media_type: string; name: string }> = []

      if (attachments && attachments.length > 0) {
        const attachmentInfo: string[] = []
        for (const a of attachments) {
          const isImage = a.type === 'image' || a.type.startsWith('image/')
          const isVideo = a.type.startsWith('video/')
          const isAudio = a.type.startsWith('audio/')
          attachmentInfo.push(
            `[Attachment: ${a.name} (${(a.size / 1024).toFixed(1)}KB, Type: ${a.type})${isImage ? ' [Image]' : isVideo ? ' [Video]' : isAudio ? ' [Audio]' : ''}]`,
          )

          if (a.dataUrl) {
            let mediaType = a.type
            if (a.type === 'image') mediaType = 'image/png'
            else if (a.type === 'file') {
              const ext = a.name.split('.').pop()?.toLowerCase() || ''
              const mimeMap: Record<string, string> = {
                mp4: 'video/mp4',
                webm: 'video/webm',
                mov: 'video/quicktime',
                avi: 'video/x-msvideo',
                mkv: 'video/x-matroska',
                mp3: 'audio/mpeg',
                wav: 'audio/wav',
                ogg: 'audio/ogg',
                flac: 'audio/flac',
                m4a: 'audio/mp4',
                pdf: 'application/pdf',
                doc: 'application/msword',
                docx: 'application/vnd.openxmlformats-officedocument.wordprocessingml.document',
                xls: 'application/vnd.ms-excel',
                xlsx: 'application/vnd.openxmlformats-officedocument.spreadsheetml.sheet',
                txt: 'text/plain',
                md: 'text/markdown',
                json: 'application/json',
                zip: 'application/zip',
                tar: 'application/x-tar',
              }
              mediaType = mimeMap[ext] || 'application/octet-stream'
            }
            mediaAttachments.push({ data_url: a.dataUrl, media_type: mediaType, name: a.name })
          }
        }
        finalContent = finalContent ? `${finalContent}\n\n${attachmentInfo.join('\n')}` : attachmentInfo.join('\n')
      }

      ;(async () => {
        let convId = activeConversationId

        if (!convId) {
          if (creatingConvRef.current) return
          creatingConvRef.current = true
          try {
            const conv = (await createConversation({ agentId: activeAgentId || undefined })) as import('../types').Conversation
            addConversation(conv)
            setActiveConversationId(conv.id)
            initConvState(conv.id)
            convId = conv.id
          } catch (e) {
            console.error('Failed to create conversation:', e)
            creatingConvRef.current = false
            return
          } finally {
            creatingConvRef.current = false
          }
        }

        if (sendingRefs.current.has(convId!)) return
        sendingRefs.current.add(convId!)

        const userMsg: Message = {
          id: crypto.randomUUID(),
          role: 'user',
          content: finalContent,
          timestamp: Date.now(),
        }
        const current = useConversationStore.getState().convState[convId!] || {
          messages: [],
          isLoading: true,
          multiAgentMessages: [],
          toolExecutions: [],
        }
        setConvState(convId!, {
          ...current,
          messages: [...(current.messages || []), userMsg],
          isLoading: true,
          streamingText: '',
        })

        const existingTimeout = sendTimeoutRefs.current.get(convId!)
        if (existingTimeout) clearTimeout(existingTimeout)
        sendTimeoutRefs.current.set(convId!, setTimeout(() => {
          const st = useConversationStore.getState().convState[convId!]
          if (st?.isLoading) {
            const timeoutMsg: Message = {
              id: crypto.randomUUID(),
              role: 'assistant',
              content: t('errors.requestTimeout'),
              timestamp: Date.now(),
              isError: true,
            }
            setConvState(convId!, {
              messages: [...(st.messages || []), timeoutMsg],
              isLoading: false,
              streamingText: undefined,
              multiAgentMessages: [],
              toolExecutions: [],
            })
            sendingRefs.current.delete(convId!)
            sendTimeoutRefs.current.delete(convId!)
          }
        }, 180000))

        try {
          const parsed = parseMentions(finalContent)
          const customMentions: MentionedAgent[] = (mentionedAgentIds || [])
            .filter((id) => !parsed.mentions.some((m) => m.agentId === id))
            .map((id) => {
              const agent = agents.find((a) => a.id === id)
              return { agentId: id, agentName: agent?.displayName || id, startIndex: -1, endIndex: -1 }
            })
          const allMentions = [...parsed.mentions, ...customMentions]

          if (allMentions.length > 0) {
            await executeMultiAgent(convId!, finalContent, { ...parsed, mentions: allMentions }, mentionedAgentIds)
          } else {
            debugLog(`[SendMsg] Starting streaming for conv=${convId?.slice(0, 16)}...`)
            await sendMessageStreaming({
              conversationId: convId,
              content: finalContent,
              ...(mediaAttachments.length > 0 ? { images: mediaAttachments } : {}),
            })
            debugLog(`[SendMsg] Streaming completed for conv=${convId?.slice(0, 16)}`)
          }

          updateConversation(convId!, { title: content.slice(0, 50), updatedAt: Date.now() })
          sendingRefs.current.delete(convId!)
        } catch (e) {
          console.error(`[SendMsg] Error for conv=${convId?.slice(0, 16)}:`, e)
          const errorContent = e instanceof Error ? e.message : String(e)
          const isTimeout = errorContent.includes('timeout') || errorContent.includes('Timeout')
          const errorMsg: Message = {
            id: crypto.randomUUID(),
            role: 'assistant',
            content: isTimeout ? t('errors.requestTimeout') : `Error: ${errorContent}`,
            timestamp: Date.now(),
            isError: true,
          }
          const existing = useConversationStore.getState().convState[convId!] || {
            messages: [],
            isLoading: true,
            multiAgentMessages: [],
            toolExecutions: [],
          }
          setConvState(convId!, {
            messages: [...existing.messages, errorMsg],
            isLoading: false,
            streamingText: undefined,
            multiAgentMessages: [],
            toolExecutions: [],
          })
          sendingRefs.current.delete(convId!)
          const tRef = sendTimeoutRefs.current.get(convId!)
          if (tRef) { clearTimeout(tRef); sendTimeoutRefs.current.delete(convId!) }
        }
      })()
    },
    [activeConversationId, activeAgentId, agents, setConvState, initConvState, addConversation, setActiveConversationId, updateConversation, t],
  )

  /** 执行多 Agent 协作会话：解析 @提及、创建子任务、聚合响应 */
  const executeMultiAgent = async (
    convId: string,
    content: string,
    parsed: ReturnType<typeof parseMentions>,
    _mentionedAgentIds?: string[],
  ) => {
    const sessionId = `ma-${Date.now()}-${crypto.randomUUID().slice(0, 8)}`
    setActiveMultiAgentSession(sessionId)

    const mentionsToUse = parsed.mentions.length > 0 ? parsed.mentions : []
    if (mentionsToUse.length === 0) {
      setToast(t('errors.noValidAgent'))
      setConvState(convId, { ...(useConversationStore.getState().convState[convId] || { messages: [], isLoading: false, multiAgentMessages: [], toolExecutions: [] }), isLoading: false })
      setActiveMultiAgentSession(null)
      return
    }

    const cleanContent = mentionsToUse.reduce((text, m) => text.replace(`@${m.agentName}`, '').replace(/\s+/g, ' ').trim(), content)

    const initialMultiMsg: MultiAgentMessageContent = {
      type: 'multi_agent',
      sessionId,
      mainResponse: '',
      subAgents: mentionsToUse.map((m) => ({
        taskId: `${sessionId}-${m.agentId}`,
        agentId: m.agentId,
        agentName: m.agentName,
        status: 'pending' as SubAgentStatus,
      })),
      status: 'planning' as MultiAgentSessionStatus,
      timestamp: Date.now(),
    }

    const current = useConversationStore.getState().convState[convId] || { messages: [], isLoading: true, multiAgentMessages: [], toolExecutions: [] }
    setConvState(convId, {
      ...current,
      multiAgentMessages: [...(current.multiAgentMessages || []), initialMultiMsg],
    })

    const updateMultiAgentMessage = (updater: (prev: MultiAgentMessageContent) => MultiAgentMessageContent) => {
      const currentState = useConversationStore.getState().convState[convId]
      if (!currentState) return
      setConvState(convId, {
        ...currentState,
        multiAgentMessages: currentState.multiAgentMessages.map((msg) =>
          msg.sessionId === sessionId ? updater(msg) : msg,
        ),
      })
    }

    try {
      const response = await agentCoordinator.executeMultiAgentSession(
        sessionId,
        convId,
        parsed.cleanText || content,
        mentionsToUse,
        activeAgentId || '',
        {
          onStatusChange: (_sid, status) => {
            updateMultiAgentMessage((prev) => ({ ...prev, status }))
          },
          onTaskUpdate: (taskId, task) => {
            updateMultiAgentMessage((prev) => ({
              ...prev,
              streamingAgentId: task.status === 'running' ? task.agentId : prev.streamingAgentId,
              streamingText: task.rawOutput || prev.streamingText,
              subAgents: prev.subAgents.map((sa) =>
                sa.taskId === taskId
                  ? {
                      ...sa,
                      status: task.status as SubAgentStatus,
                      result: task.result,
                      error: task.error,
                      durationMs: task.durationMs,
                      rawOutput: task.rawOutput,
                      streamingText: task.rawOutput,
                    }
                  : sa
              ),
            }))
          },
          onComplete: (aggregatedResponse) => {
            updateMultiAgentMessage((prev) => ({
              ...prev,
              mainResponse: aggregatedResponse.summary,
              status: MultiAgentSessionStatus.COMPLETED,
              streamingAgentId: undefined,
              streamingText: undefined,
              subAgents: aggregatedResponse.details
                ? Object.entries(aggregatedResponse.details).map(([agentId, detail]) => ({
                    taskId: `${sessionId}-${agentId}`,
                    agentId,
                    agentName: detail.agentName,
                    status: detail.status as SubAgentStatus,
                    result: detail.result,
                  }))
                : prev.subAgents,
            }))
          },
          onError: (error) => {
            setToast(t('errors.multiAgentError') + error)
            updateMultiAgentMessage((prev) => ({
              ...prev,
              status: MultiAgentSessionStatus.FAILED,
              mainResponse: prev.mainResponse || (t('errors.collaborationError') + error),
            }))
          },
        },
      )

      const assistantMsg: Message = {
        id: crypto.randomUUID(),
        role: 'assistant',
        content: '[MultiAgent] ' + buildMentionDisplayText(mentionsToUse) + '\n\n' + response.summary,
        timestamp: Date.now(),
      }

      const existing = useConversationStore.getState().convState[convId] || { messages: [], isLoading: false, multiAgentMessages: [], toolExecutions: [] }
      setConvState(convId, {
        ...existing,
        messages: [...existing.messages, assistantMsg],
        isLoading: false,
        streamingText: undefined,
      })
    } catch (e: any) {
      updateMultiAgentMessage((prev) => ({
        ...prev,
        status: MultiAgentSessionStatus.FAILED,
        mainResponse: prev.mainResponse || ('Execution failed: ' + (e.message || 'Unknown error')),
      }))
      setToast(t('errors.multiAgentFailed') + (e.message || 'Unknown error'))
    } finally {
      setConvState(convId, { ...(useConversationStore.getState().convState[convId] || { messages: [], isLoading: false, multiAgentMessages: [], toolExecutions: [] }), isLoading: false })
      setActiveMultiAgentSession(null)
    }
  }

  /** 停止当前流式生成：取消后端流、将已生成文本保存为消息 */
  const handleStopGeneration = useCallback(() => {
    if (activeConversationId) {
      cancelStream({ conversationId: activeConversationId }).catch((e) => { console.error(e) })
      const existing = useConversationStore.getState().convState[activeConversationId]
      if (!existing) return
      const finalText = existing.streamingText || ''
      const updatedMessages = finalText
        ? [
            ...existing.messages,
            { id: crypto.randomUUID(), role: 'assistant', content: finalText, timestamp: Date.now() } as Message,
          ]
        : existing.messages
      setConvState(activeConversationId, {
        messages: updatedMessages,
        isLoading: false,
        streamingText: undefined,
        toolExecutions: [],
        multiAgentMessages: [],
      })
      sendingRefs.current.delete(activeConversationId)
    }
  }, [activeConversationId, setConvState])

  /** 处理用户确认操作：将确认值作为新消息发送 */
  const handleConfirm = useCallback(
    (value: string) => {
      const pending = useUIStore.getState().pendingConfirmation
      if (!pending) return
      useUIStore.getState().setPendingConfirmation(null)
      handleSendMessage(value)
    },
    [handleSendMessage],
  )

  return {
    handleSendMessage,
    handleStopGeneration,
    handleConfirm,
    sendingRefs,
  }
}
