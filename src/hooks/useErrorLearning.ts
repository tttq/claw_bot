// Claw Desktop - 错误学习 Hook
// 提供 Harness 错误学习系统的前端接口：获取规则、捕获错误、触发命中、构建提示词片段
import { useState, useEffect, useCallback } from 'react'
import { harnessErrorTriggerHit, harnessErrorCapture, harnessErrorBuildPromptSection, harnessErrorGetRules } from '../api/harness'
import type { AvoidanceRule, ErrorCategory } from '../multiagent/errorLearning'

/** 错误统计信息 */
export interface ErrorStats {
  totalRules: number               // 规则总数
  byCategory: Record<string, number> // 按分类统计
  activeRules: number              // 活跃规则数
  deprecatedRules: number          // 已废弃规则数
}

/** 错误学习 Hook：管理错误规则的获取、捕获、触发和提示词构建 */
export function useErrorLearning(agentId?: string) {
  const [rules, setRules] = useState<AvoidanceRule[]>([])
  const [loading, setLoading] = useState(false)
  const [promptSection, setPromptSection] = useState<string>('')
  const [lastCaptureResult, setLastCaptureResult] = useState<{ success: boolean; ruleId?: string } | null>(null)

  /** 从后端获取指定 Agent 的错误规避规则 */
  const fetchRules = useCallback(async (id: string) => {
    setLoading(true)
    try {
      const result = await harnessErrorGetRules({ agent_id: id }) as unknown as { rules?: AvoidanceRule[] }
      setRules(result.rules || [])
    } catch (err) {
      console.error('Failed to fetch error rules:', err)
      setRules([])
    } finally {
      setLoading(false)
    }
  }, [])

  /** 捕获错误并生成规避规则 */
  const captureError = useCallback(async (
    id: string,
    category: ErrorCategory,
    errorMessage: string,
    userInputSnippet?: string,
    contextSnapshot?: string
  ) => {
    try {
      const result = await harnessErrorCapture({
        agent_id: id,
        category,
        error_message: errorMessage,
        user_input_snippet: userInputSnippet,
        context_snapshot: contextSnapshot,
      }) as { success: boolean; ruleId?: string }

      setLastCaptureResult(result)
      if (result.success && id === agentId) {
        await fetchRules(id)
      }

      return result
    } catch (err) {
      console.error('Failed to capture error:', err)
      setLastCaptureResult({ success: false })
      return { success: false }
    }
  }, [agentId, fetchRules])

  /** 记录规则触发命中（更新触发计数和最后触发时间） */
  const triggerHit = useCallback(async (ruleId: string) => {
    try {
      await harnessErrorTriggerHit({ rule_id: ruleId })
      setRules(prev => prev.map(r =>
        r.id === ruleId ? { ...r, triggerCount: r.triggerCount + 1, lastTriggeredAt: Date.now() } : r
      ))
    } catch (err) {
      console.error('Failed to trigger rule hit:', err)
    }
  }, [])

  /** 构建错误学习提示词片段（注入 system prompt） */
  const buildPromptSection = useCallback(async (id: string) => {
    try {
      const section = await harnessErrorBuildPromptSection({ agent_id: id }) as unknown as string
      setPromptSection(section || '')
      return section || ''
    } catch (err) {
      console.error('Failed to build prompt section:', err)
      return ''
    }
  }, [])

  /** 计算错误规则统计信息 */
  const getStats = useCallback((): ErrorStats => {
    const byCategory: Record<string, number> = {}
    let active = 0
    let deprecated = 0

    for (const rule of rules) {
      byCategory[rule.category] = (byCategory[rule.category] || 0) + 1
      if (rule.isDeprecated) deprecated++
      else active++
    }

    return { totalRules: rules.length, byCategory, activeRules: active, deprecatedRules: deprecated }
  }, [rules])

  useEffect(() => {
    if (agentId) {
      fetchRules(agentId)
      buildPromptSection(agentId)
    }
  }, [agentId, fetchRules, buildPromptSection])

  return {
    rules,
    loading,
    promptSection,
    lastCaptureResult,
    stats: getStats(),
    refetch: () => agentId && fetchRules(agentId),
    captureError,
    triggerHit,
    buildPromptSection,
  }
}
