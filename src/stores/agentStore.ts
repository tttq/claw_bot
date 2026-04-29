// Claw Desktop - Agent 状态管理模块
// 管理 AI Agent 列表及当前激活的 Agent
import { create } from 'zustand'

/** Agent 信息接口：描述一个 AI Agent 的基本元数据 */
export interface AgentInfo {
  id: string
  displayName: string
  description?: string
  purpose?: string
  scope?: string
}

/** Agent 状态管理接口：管理 Agent 列表和当前激活 Agent 的状态 */
interface AgentStore {
  agents: AgentInfo[]           // 所有已注册的 Agent 列表
  activeAgentId: string | null  // 当前激活的 Agent ID

  setAgents: (agents: AgentInfo[]) => void           // 设置 Agent 列表
  setActiveAgentId: (id: string | null) => void      // 设置当前激活的 Agent
  getActiveAgent: () => AgentInfo | undefined         // 获取当前激活的 Agent 信息
}

/** 创建 Agent 状态管理 Store，使用 Zustand 管理全局 Agent 状态 */
export const useAgentStore = create<AgentStore>((set, get) => ({
  agents: [],                   // 初始 Agent 列表为空
  activeAgentId: null,          // 初始无激活 Agent

  setAgents: (agents) => set({ agents }),
  setActiveAgentId: (id) => set({ activeAgentId: id }),

  /** 根据 ID 查找当前激活的 Agent */
  getActiveAgent: () => {
    const { agents, activeAgentId } = get()
    return agents.find((a) => a.id === activeAgentId)
  },
}))
