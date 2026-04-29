// Claw Desktop - Agent 状态可视化 Hook
// 为 3D 可视化组件提供 Agent 节点数据（位置、状态、协作关系），
// 合并内置 Agent 注册表和后端 Agent 数据，按分类计算 3D 布局位置
import { useState, useEffect, useCallback, useRef } from 'react'
import { agentRegistry } from '../multiagent/agentRegistry'
import { subAgentEngine } from '../multiagent/subAgentEngine'
import type { AgentRegistryEntry } from '../multiagent/types'
import { AgentCategory } from '../multiagent/types'
import type { MultiAgentMessageContent } from '../multiagent/types'

// ==================== Type Definitions ====================

/** Agent 3D visualization status */
export type AgentVisualStatus = 'idle' | 'running' | 'completed' | 'failed' | 'pending' | 'waiting_input' | 'timeout'

/** Agent 3D visualization data */
export interface AgentVisualData {
  id: string
  name: string
  category: AgentCategory
  icon: string
  status: AgentVisualStatus
  description: string
  capabilities: string[]
  position: [number, number, number]
  activeTaskCount: number
  isCollaborating: boolean
  collaboratingWith: string[]
  conversationCount: number
  currentTask?: string
}

/** Agent collaboration connection data */
export interface AgentConnection {
  fromId: string
  toId: string
  strength: number // 0-1 connection strength
}

// ==================== Constants ====================

/** Category-based layout positions for Agent nodes */
const CATEGORY_POSITIONS: Record<AgentCategory, [number, number, number]> = {
  [AgentCategory.SEARCH]: [-4, 2, 0],
  [AgentCategory.CODE]: [4, 2, 0],
  [AgentCategory.ANALYSIS]: [0, 0.5, 2],
  [AgentCategory.CREATIVE]: [-3, -2, -1],
  [AgentCategory.GENERAL]: [3, -2, -1],
  [AgentCategory.CUSTOM]: [0, -3.5, 1],
}

// ==================== Utility Functions ====================

/** Compute circular distribution positions for multiple Agents within the same category */
function computePositions(entries: AgentRegistryEntry[]): Map<string, [number, number, number]> {
  const positions = new Map<string, [number, number, number]>()

  const grouped = new Map<AgentCategory, AgentRegistryEntry[]>()
  for (const entry of entries) {
    const list = grouped.get(entry.category) || []
    list.push(entry)
    grouped.set(entry.category, list)
  }

  for (const [category, group] of grouped.entries()) {
    const center = CATEGORY_POSITIONS[category] || [0, 0, 0]
    if (group.length === 1) {
      positions.set(group[0].id, center)
      continue
    }

    const radius = Math.max(1.5, group.length * 0.6)
    for (let i = 0; i < group.length; i++) {
      const angle = (2 * Math.PI * i) / group.length - Math.PI / 2
      const x = center[0] + radius * Math.cos(angle)
      const y = center[1] + radius * Math.sin(angle)
      const z = center[2] + (i % 2 === 0 ? 0.5 : -0.5) // Slight Z offset for depth perception
      positions.set(group[i].id, [x, y, z])
    }
  }
  return positions
}

/** Merge runtime status from subAgentEngine + convState */
function mergeRuntimeStatus(
  taskStatuses: Array<{ agentId: string; status: string; description?: string; prompt?: string }>,
  messages: MultiAgentMessageContent[],
): Map<string, { status: string; taskCount: number; currentTask?: string; collaboratingWith: string[] }> {
  const map = new Map<string, { status: string; taskCount: number; currentTask?: string; collaboratingWith: string[] }>()

  for (const t of taskStatuses) {
    const existing = map.get(t.agentId) || { status: 'idle', taskCount: 0, currentTask: undefined, collaboratingWith: [] as string[] }
    existing.taskCount++
    const priority: Record<string, number> = { running: 5, waiting_input: 4, pending: 3, completed: 2, failed: 1 }
    if ((priority[t.status] || 0) > (priority[existing.status] || 0)) {
      existing.status = t.status
      existing.currentTask = t.description || t.prompt
    }
    map.set(t.agentId, existing)
  }

  for (const msg of messages) {
    const mainId = (msg as any).mainResponse?.agentId || ''
    if (!mainId) continue
    const existing = map.get(mainId) || { status: 'idle', taskCount: 0, currentTask: undefined, collaboratingWith: [] as string[] }
    if (msg.status === 'executing' || msg.status === 'planning') {
      existing.status = 'running'
      existing.currentTask = (msg as any).mainResponse?.summary || existing.currentTask
    }
    const collabIds = (msg.subAgents || []).map(s => s.agentId).filter(Boolean)
    for (const cid of collabIds) {
      if (!existing.collaboratingWith.includes(cid)) existing.collaboratingWith.push(cid)
    }
    map.set(mainId, existing)
  }

  return map
}

// ==================== Hook ====================

export function useAgentStatus({ agents, convState, isActive }: UseAgentStatusParams) {
  const [visualData, setVisualData] = useState<AgentVisualData[]>([])
  const [connections, setConnections] = useState<AgentConnection[]>([])
  const intervalRef = useRef<ReturnType<typeof setInterval> | null>(null)

  const computeVisualData = useCallback(() => {
    // 1. Get all built-in Agents from registry
    const registryEntries = agentRegistry.getAll()
    const positions = computePositions(registryEntries)

    // 2. Get runtime status
    const activeTasks = subAgentEngine.getActiveTasks()
    const allTasks = subAgentEngine.getAllTasks()

    // 3. Aggregate multiAgentMessages from all conversations
    const allMultiAgentMessages: MultiAgentMessageContent[] = []
    for (const conv of Object.values(convState)) {
      if (conv.multiAgentMessages?.length) {
        allMultiAgentMessages.push(...conv.multiAgentMessages)
      }
    }

    // 4. Merge runtime status
    const runtimeStatus = mergeRuntimeStatus(
      [...activeTasks, ...allTasks].map(t => ({
        agentId: t.agentId,
        status: t.status,
        description: t.description,
        prompt: t.prompt,
      })),
      allMultiAgentMessages,
    )

    // 5. Compute positions for backend Agents (non-built-in Agents grouped as CUSTOM)
    const safeAgents = Array.isArray(agents) ? agents : []
    const backendOnlyAgents = safeAgents.filter(
      a => !registryEntries.some(r => r.id === a.id),
    )
    const customStartIdx = registryEntries.filter(r => r.category === AgentCategory.CUSTOM).length
    backendOnlyAgents.forEach((agent, i) => {
      const center = CATEGORY_POSITIONS[AgentCategory.CUSTOM]
      const angle = (2 * Math.PI * (customStartIdx + i)) / Math.max(1, customStartIdx + backendOnlyAgents.length) - Math.PI / 2
      const radius = Math.max(1.5, (customStartIdx + backendOnlyAgents.length) * 0.6)
      positions.set(agent.id, [
        center[0] + radius * Math.cos(angle),
        center[1] + radius * Math.sin(angle),
        center[2] + (i % 2 === 0 ? 0.5 : -0.5),
      ])
    })

    // 6. Build AgentVisualData[]
    const result: AgentVisualData[] = []

    // Built-in Agents
    for (const entry of registryEntries) {
      const rt = runtimeStatus.get(entry.id)
      const backendAgent = safeAgents.find(a => a.id === entry.id)
      result.push({
        id: entry.id,
        name: entry.name,
        category: entry.category,
        icon: entry.icon,
        status: (rt?.status || 'idle') as AgentVisualStatus,
        description: entry.description,
        capabilities: entry.capabilities,
        position: positions.get(entry.id) || [0, 0, 0],
        activeTaskCount: rt?.taskCount || 0,
        isCollaborating: (rt?.collaboratingWith?.length || 0) > 0,
        collaboratingWith: rt?.collaboratingWith || [],
        conversationCount: 0,
        currentTask: rt?.currentTask,
      })
    }

    // Backend Agents (non-built-in)
    for (const agent of backendOnlyAgents) {
      const rt = runtimeStatus.get(agent.id)
      result.push({
        id: agent.id,
        name: agent.displayName,
        category: AgentCategory.CUSTOM,
        icon: '🤖',
        status: (rt?.status || 'idle') as AgentVisualStatus,
        description: agent.description || '',
        capabilities: [],
        position: positions.get(agent.id) || [0, 0, 0],
        activeTaskCount: rt?.taskCount || 0,
        isCollaborating: (rt?.collaboratingWith?.length || 0) > 0,
        collaboratingWith: rt?.collaboratingWith || [],
        conversationCount: 0,
        currentTask: rt?.currentTask,
      })
    }

    // 7. Build collaboration connections
    const connectionList: AgentConnection[] = []
    const seen = new Set<string>()
    for (const data of result) {
      if (data.isCollaborating && data.collaboratingWith.length > 0) {
        for (const targetId of data.collaboratingWith) {
          const key = [data.id, targetId].sort().join('-')
          if (!seen.has(key) && result.some(r => r.id === targetId)) {
            seen.add(key)
            connectionList.push({
              fromId: data.id,
              toId: targetId,
              strength: 0.6 + Math.random() * 0.4, // TODO: Replace with actual data
            })
          }
        }
      }
    }

    setVisualData(result)
    setConnections(connectionList)
  }, [agents, convState])

  // Polling logic
  useEffect(() => {
    if (isActive) {
      computeVisualData()
      intervalRef.current = setInterval(computeVisualData, POLL_INTERVAL_MS)
    }

    return () => {
      if (intervalRef.current) {
        clearInterval(intervalRef.current)
        intervalRef.current = null
      }
    }
  }, [isActive, computeVisualData])

  return { visualData, connections }
}

// ==================== Types ====================

interface UseAgentStatusParams {
  /** Backend-loaded Agent list */
  agents: Array<{ id: string; displayName: string; description?: string }>
  /** Conversation state (containing multiAgentMessages) */
  convState: Record<string, { multiAgentMessages: MultiAgentMessageContent[] }>
  /** Whether the hook is active (only poll when 3D tab is visible) */
  isActive: boolean
}

// Polling interval (ms)
const POLL_INTERVAL_MS = 2000
