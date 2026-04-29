// Claw Desktop - 会话状态管理模块
// 管理聊天会话列表、消息、流式文本及多 Agent 消息等核心状态
import { create } from 'zustand'
import type { Conversation, Message } from '../types'
import type { MultiAgentMessageContent } from '../multiagent/types'

/** 工具执行详细记录（含输入参数和输出结果） */
export interface ToolExecutionDetail {
  toolName: string
  durationMs: number
  index: number
  total: number
  round?: number
  status: 'running' | 'completed'
  toolInput?: string
  toolResult?: string
}

/** 单个会话的运行时状态接口：包含消息列表、加载状态、流式文本、多Agent消息及工具执行记录 */
export interface ConversationState {
  messages: Message[]
  isLoading: boolean
  streamingText?: string
  thinkingText?: string
  multiAgentMessages: MultiAgentMessageContent[]
  toolExecutions: Array<{ toolName: string; durationMs: number; index: number; total: number }>
  toolExecutionDetails: ToolExecutionDetail[]
}

/** 会话状态管理接口：管理会话列表、激活会话及各会话的独立运行时状态 */
interface ConversationStore {
  conversations: Conversation[]                     // 所有会话列表
  activeConversationId: string | null               // 当前激活的会话 ID
  convState: Record<string, ConversationState>      // 各会话的运行时状态映射（key 为会话 ID）

  setConversations: (conversations: Conversation[]) => void                              // 批量设置会话列表
  addConversation: (conv: Conversation) => void                                          // 新增会话（同时初始化其状态）
  removeConversation: (id: string) => void                                               // 删除会话（同时清理其状态，若为激活会话则重置）
  updateConversation: (id: string, updates: Partial<Conversation>) => void               // 局部更新会话属性
  setActiveConversationId: (id: string | null) => void                                   // 设置当前激活的会话

  setConvState: (convId: string, state: Partial<ConversationState>) => void              // 局部更新指定会话的运行时状态
  initConvState: (convId: string, state?: Partial<ConversationState>) => void            // 初始化指定会话的运行时状态
  removeConvState: (convId: string) => void                                               // 移除指定会话的运行时状态
  resetAllConvState: () => void                                                            // 重置所有会话的运行时状态

  getActiveConv: () => ConversationState | null           // 获取当前激活会话的运行时状态
  getActiveConversation: () => Conversation | undefined   // 获取当前激活的会话对象
}

/** 会话运行时状态的默认值 */
const defaultConvState: ConversationState = {
  messages: [],
  isLoading: false,
  streamingText: undefined,
  thinkingText: undefined,
  multiAgentMessages: [],
  toolExecutions: [],
  toolExecutionDetails: [],
}

/** 创建会话状态管理 Store，使用 Zustand 管理全局会话列表及各会话的独立运行时状态 */
export const useConversationStore = create<ConversationStore>((set, get) => ({
  conversations: [],            // 初始会话列表为空
  activeConversationId: null,   // 初始无激活会话
  convState: {},                // 初始无会话运行时状态

  setConversations: (conversations) => set({ conversations }),

  /** 新增会话：插入列表头部，同时初始化其运行时状态 */
  addConversation: (conv) =>
    set((s) => ({
      conversations: [conv, ...s.conversations],
      convState: { ...s.convState, [conv.id]: { ...defaultConvState } },
    })),

  /** 删除会话：清理其运行时状态，若为激活会话则重置 */
  removeConversation: (id) =>
    set((s) => {
      const next = { ...s.convState }
      delete next[id]
      return {
        conversations: s.conversations.filter((c) => c.id !== id),
        activeConversationId: s.activeConversationId === id ? null : s.activeConversationId,
        convState: next,
      }
    }),

  /** 局部更新会话属性 */
  updateConversation: (id, updates) =>
    set((s) => ({
      conversations: s.conversations.map((c) => (c.id === id ? { ...c, ...updates } : c)),
    })),

  setActiveConversationId: (id) => set({ activeConversationId: id }),

  /** 局部更新指定会话的运行时状态（若不存在则使用默认值） */
  setConvState: (convId, partial) =>
    set((s) => ({
      convState: {
        ...s.convState,
        [convId]: { ...(s.convState[convId] || defaultConvState), ...partial },
      },
    })),

  /** 基于默认值初始化会话运行时状态，可覆盖部分字段 */
  initConvState: (convId, state) =>
    set((s) => ({
      convState: {
        ...s.convState,
        [convId]: { ...defaultConvState, ...state },
      },
    })),

  /** 移除指定会话的运行时状态 */
  removeConvState: (convId) =>
    set((s) => {
      const next = { ...s.convState }
      delete next[convId]
      return { convState: next }
    }),

  /** 清空所有会话运行时状态 */
  resetAllConvState: () => set({ convState: {} }),

  /** 获取当前激活会话的运行时状态 */
  getActiveConv: () => {
    const { activeConversationId, convState } = get()
    if (!activeConversationId) return null
    return convState[activeConversationId] ?? null
  },

  /** 根据 ID 查找当前激活的会话对象 */
  getActiveConversation: () => {
    const { conversations, activeConversationId } = get()
    return conversations.find((c) => c.id === activeConversationId)
  },
}))
