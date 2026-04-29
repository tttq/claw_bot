// Claw Desktop - Agent 人物画像 Hook
// 提供单个 Agent 画像的获取/更新，以及全部画像列表的加载
import { useState, useEffect, useCallback } from 'react'
import { harnessPersonaUpdate, harnessPersonaGet, harnessPersonaList } from '../api/harness'

/** Agent 人物画像数据结构 */
export interface AgentPersona {
  agentId: string
  displayName: string
  personalityTraits: string[]
  communicationStyle: string
  expertiseDomain: string
  behaviorConstraints: string[]
  responseToneInstruction: string
  languagePreference: string
  createdAt: number
  updatedAt: number
}

/** 默认人物画像模板 */
const DEFAULT_PERSONA: AgentPersona = {
  agentId: '',
  displayName: '',
  personalityTraits: ['Professional', 'Helpful'],
  communicationStyle: 'friendly',
  expertiseDomain: 'General AI Assistant',
  behaviorConstraints: [],
  responseToneInstruction: '',
  languagePreference: 'zh-CN',
  createdAt: Date.now(),
  updatedAt: Date.now(),
}

/** 单个 Agent 画像 Hook：获取、更新指定 Agent 的人物画像 */
export function useAgentPersona(agentId?: string) {
  const [persona, setPersona] = useState<AgentPersona | null>(null)
  const [loading, setLoading] = useState(false)
  const [error, setError] = useState<string | null>(null)

  /** 从后端获取指定 Agent 的人物画像 */
  const fetchPersona = useCallback(async (id: string) => {
    setLoading(true)
    setError(null)
    try {
      const result = await harnessPersonaGet({ agent_id: id }) as unknown as AgentPersona
      setPersona(result)
    } catch (err) {
      setError(String(err))
      setPersona(null)
    } finally {
      setLoading(false)
    }
  }, [])

  /** 更新画像的单个字段 */
  const updateField = useCallback(async (id: string, field: string, value: string) => {
    try {
      await harnessPersonaUpdate({ agent_id: id, field, value })
      if (persona && persona.agentId === id) {
        setPersona(prev => prev ? { ...prev, [field === 'display_name' ? 'displayName' : field]: value, updatedAt: Date.now() } : null)
      }
      return true
    } catch (err) {
      setError(String(err))
      return false
    }
  }, [persona])

  useEffect(() => {
    if (agentId) {
      fetchPersona(agentId)
    }
  }, [agentId, fetchPersona])

  return {
    persona: persona || (agentId ? { ...DEFAULT_PERSONA, agentId: agentId!, displayName: agentId! } : null),
    loading,
    error,
    refetch: () => agentId && fetchPersona(agentId),
    updateField,
  }
}

/** 全部画像列表 Hook：加载所有 Agent 的人物画像 */
export function useAllPersonas() {
  const [personas, setPersonas] = useState<AgentPersona[]>([])
  const [loading, setLoading] = useState(false)

  const fetchAll = useCallback(async () => {
    setLoading(true)
    try {
      const result = await harnessPersonaList() as unknown as { personas?: AgentPersona[] }
      setPersonas(result.personas || [])
    } catch (err) {
      console.error('Failed to fetch personas:', err)
      setPersonas([])
    } finally {
      setLoading(false)
    }
  }, [])

  useEffect(() => { fetchAll() }, [fetchAll])

  return { personas, loading, refetch: fetchAll }
}
