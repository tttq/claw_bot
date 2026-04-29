// Claw Desktop - UI 状态管理模块
// 管理界面布局、面板显示、Toast 提示、Agent 配置及多 Agent 会话等 UI 状态
import { create } from 'zustand'

/** 主标签页类型 */
type TabType = 'chat' | '3d' | 'harness'

/** UI 状态管理接口：管理界面布局、面板可见性、提示消息及多Agent相关UI状态 */
interface UIStore {
  activeTab: TabType            // 当前激活的主标签页
  showSettings: boolean         // 设置面板是否可见
  showMemoryPanel: boolean      // 记忆（RAG）面板是否可见
  showBrowserPanel: boolean     // 浏览器面板是否可见
  showPerformanceMonitor: boolean // 性能监控面板是否可见
  toast: string | null          // Toast 提示消息内容
  configuringAgent: { agentId: string; agentName: string } | null  // 正在配置的 Agent 信息
  activeMultiAgentSession: string | null   // 当前激活的多 Agent 会话 ID
  pendingConfirmation: {                  // 待用户确认的操作
    conversationId: string                // 所属会话 ID
    prompt: string                        // 确认提示文本
    options: Array<{ label: string; value: string }>  // 可选操作项
  } | null

  setActiveTab: (tab: TabType) => void                                              // 切换主标签页
  setShowSettings: (show: boolean) => void                                          // 切换设置面板
  setShowMemoryPanel: (show: boolean) => void                                       // 切换记忆面板
  setShowBrowserPanel: (show: boolean) => void                                      // 切换浏览器面板
  setShowPerformanceMonitor: (show: boolean) => void                                // 切换性能监控面板
  setToast: (msg: string | null) => void                                            // 设置 Toast 提示消息
  setConfiguringAgent: (agent: { agentId: string; agentName: string } | null) => void  // 设置正在配置的 Agent
  setActiveMultiAgentSession: (sessionId: string | null) => void                    // 设置激活的多 Agent 会话
  setPendingConfirmation: (confirmation: UIStore['pendingConfirmation']) => void    // 设置待确认操作
}

/** 创建 UI 状态管理 Store，使用 Zustand 管理全局界面状态 */
export const useUIStore = create<UIStore>((set) => ({
  activeTab: 'chat',                    // 默认显示聊天标签页
  showSettings: false,                  // 默认隐藏设置面板
  showMemoryPanel: false,               // 默认隐藏记忆面板
  showBrowserPanel: false,              // 默认隐藏浏览器面板
  showPerformanceMonitor: false,        // 默认隐藏性能监控面板
  toast: null,                          // 默认无 Toast 提示
  configuringAgent: null,               // 默认无正在配置的 Agent
  activeMultiAgentSession: null,        // 默认无激活的多 Agent 会话
  pendingConfirmation: null,            // 默认无待确认操作

  setActiveTab: (tab) => set({ activeTab: tab }),
  setShowSettings: (show) => set({ showSettings: show }),
  setShowMemoryPanel: (show) => set({ showMemoryPanel: show }),
  setShowBrowserPanel: (show) => set({ showBrowserPanel: show }),
  setShowPerformanceMonitor: (show) => set({ showPerformanceMonitor: show }),
  setToast: (msg) => set({ toast: msg }),
  setConfiguringAgent: (agent) => set({ configuringAgent: agent }),
  setActiveMultiAgentSession: (sessionId) => set({ activeMultiAgentSession: sessionId }),
  setPendingConfirmation: (confirmation) => set({ pendingConfirmation: confirmation }),
}))
