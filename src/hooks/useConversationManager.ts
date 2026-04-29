// Claw Desktop - 会话管理 Hook
// 封装会话的增删改查、配置加载、Agent 列表加载等核心业务逻辑
import { useCallback, useRef } from 'react'
import { useTranslation } from 'react-i18next'
import { useConversationStore } from '../stores/conversationStore'
import { useAgentStore } from '../stores/agentStore'
import { useConfigStore } from '../stores/configStore'
import { useUIStore } from '../stores/uiStore'
import { useStreamingStore } from '../stores/streamingStore'
import {
  listConversations,
  createConversation,
  deleteConversation,
  renameConversation,
  ragCompact,
  getMessages,
  getConfig,
  agentList,
} from '../api'
import type { Conversation, Message, AppConfig } from '../types'
import type { AgentInfo } from '../stores/agentStore'

/** 会话管理 Hook：提供会话生命周期管理和 API 调用封装 */
export function useConversationManager() {
  const { t } = useTranslation()
  const {
    conversations,                   // 会话列表
    activeConversationId,            // 当前活跃会话 ID
    convState,                       // 各会话状态映射
    setConversations,                // 设置会话列表
    addConversation,                 // 添加会话
    removeConversation,              // 移除会话
    updateConversation,              // 更新会话属性
    setActiveConversationId,         // 设置活跃会话 ID
    setConvState,                    // 设置会话状态
    initConvState,                   // 初始化会话状态
    removeConvState,                 // 移除会话状态
    getActiveConv,                   // 获取当前活跃会话状态
  } = useConversationStore()
  const { activeAgentId } = useAgentStore()
  const { setConfig } = useConfigStore()
  const { setToast } = useUIStore()
  const { clearStreamingText } = useStreamingStore()

  const sendingRefs = useRef<Set<string>>(new Set())                    // 正在发送消息的会话 ID 集合（防重复发送）
  const creatingConvRef = useRef(false)                                 // 正在创建会话的标志（防重复创建）
  const convStateRef = useRef<Record<string, typeof convState[string]>>({}) // 会话状态引用（用于闭包中获取最新值）
  const loadingTimersRef = useRef<Map<string, number>>(new Map())       // 加载超时定时器映射

  convStateRef.current = convState

  /** 从后端加载应用配置 */
  const loadConfig = useCallback(async () => {
    try {
      const cfg = await getConfig()
      setConfig(cfg)
    } catch (e) {
      console.error('Failed to load config:', e)
    }
  }, [setConfig])

  /** 从后端加载会话列表 */
  const loadConversations = useCallback(async () => {
    try {
      const convs = (await listConversations()) as Conversation[]
      setConversations(convs)
    } catch (e) {
      console.error('Failed to load conversations:', e)
    }
  }, [setConversations])

  /** 创建新会话：先压缩当前会话，再创建并切换到新会话 */
  const handleNewConversation = useCallback(async () => {
    try {
      if (activeAgentId && activeConversationId) {
        const currentConv = convStateRef.current[activeConversationId]
        if (currentConv && currentConv.messages.length > 0) {
          ragCompact({ conversationId: activeConversationId }).catch((e) => { console.error(e) })
        }
      }
      const conv = (await createConversation({ agentId: activeAgentId || undefined })) as Conversation
      addConversation(conv)
      setActiveConversationId(conv.id)
      initConvState(conv.id)
    } catch (e) {
      console.error('Failed to create conversation:', e)
    }
  }, [activeAgentId, activeConversationId, convState, addConversation, setActiveConversationId, initConvState])

  /** 选择会话：切换活跃会话并加载其消息历史 */
  const handleSelectConversation = useCallback(
    async (id: string) => {
      setActiveConversationId(id)
      const currentState = convStateRef.current[id]
      if (currentState?.isLoading) return
      try {
        const msgs = (await getMessages({ conversationId: id })) as Message[]
        initConvState(id, { messages: msgs, isLoading: false })
        const conv = conversations.find((c) => c.id === id)
        if (conv?.agentId && conv.agentId !== activeAgentId) {
          useAgentStore.getState().setActiveAgentId(conv.agentId)
        }
      } catch (e) {
        console.error('Failed to load messages:', e)
        initConvState(id, { messages: [], isLoading: false })
      }
    },
    [activeAgentId, conversations, setActiveConversationId, initConvState],
  )

  /** 删除会话：取消流式生成、删除后端数据、清理前端状态 */
  const handleDeleteConversation = useCallback(
    async (id: string) => {
      try {
        const { cancelStream } = await import('../api')
        cancelStream({ conversationId: id }).catch((e) => { console.error(e) })
        await deleteConversation({ conversationId: id })
        removeConversation(id)
        removeConvState(id)
        clearStreamingText(id)
        sendingRefs.current.delete(id)
      } catch (e) {
        console.error('Failed to delete:', e)
      }
    },
    [removeConversation, removeConvState, clearStreamingText],
  )

  /** 重命名会话标题 */
  const handleRenameConversation = useCallback(async (id: string, newTitle: string) => {
    if (!newTitle.trim()) return
    try {
      await renameConversation(id, { newTitle: newTitle.trim() })
      updateConversation(id, { title: newTitle.trim() })
    } catch (e) {
      console.error('Failed to rename:', e)
    }
  }, [updateConversation])

  /** 显示 Toast 提示消息（3秒后自动消失） */
  const showToastMsg = useCallback(
    (msg: string) => {
      setToast(msg)
      setTimeout(() => setToast(null), 3000)
    },
    [setToast],
  )

  /** 从后端加载 Agent 列表 */
  const loadAgents = useCallback(async () => {
    try {
      const agents = (await agentList()) as AgentInfo[]
      useAgentStore.getState().setAgents(agents)
    } catch (e) {
      console.error('Failed to load agents:', e)
    }
  }, [])

  return {
    conversations,
    activeConversationId,
    convState,
    convStateRef,
    sendingRefs,
    creatingConvRef,
    loadingTimersRef,
    getActiveConv,
    loadConfig,
    loadConversations,
    loadAgents,
    handleNewConversation,
    handleSelectConversation,
    handleDeleteConversation,
    handleRenameConversation,
    showToastMsg,
    setConvState,
    initConvState,
    removeConvState,
    clearStreamingText,
  }
}
