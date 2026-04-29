// Claw Desktop - 错误学习服务模块
// 实现Harness错误学习系统：捕获错误→生成规避规则→触发命中→构建提示词片段
// 支持错误分类（API/工具/逻辑/上下文）、规则缓存、统计计算
import {
  harnessErrorTriggerHit,
  harnessErrorCapture,
  harnessErrorGetRules,
  harnessErrorBuildPromptSection,
} from '../api/harness'

export interface AvoidanceRule {
  id: string
  agentId: string
  pattern: string
  category: string
  cause: string
  fix: string
  triggerCount: number
  lastTriggeredAt: number
  createdAt: number
  expiresAt?: number | null
  isDeprecated: boolean
}

export type ErrorCategory = 'api' | 'tool' | 'logic' | 'context' | 'validation' | 'other'

export interface ErrorCaptureResult {
  success: boolean
  ruleId?: string
}

interface ErrorLearningOptions {
  autoCapture?: boolean
  autoReportToBackend?: boolean
}

class ErrorLearningService {
  private options: ErrorLearningOptions
  private ruleCache: Map<string, AvoidanceRule[]> = new Map()

  constructor(options: ErrorLearningOptions = {}) {
    this.options = { autoCapture: true, autoReportToBackend: true, ...options }
  }

  async captureError(
    agentId: string,
    category: ErrorCategory,
    errorMessage: string,
    userInputSnippet?: string,
    contextSnapshot?: string
  ): Promise<ErrorCaptureResult | null> {
    if (!this.options.autoReportToBackend) {
      console.warn(`[ErrorLearning] [${agentId}] ${category}: ${errorMessage}`)
      return null
    }

    try {
      const result = await harnessErrorCapture({
        agent_id: agentId,
        category,
        error_message: errorMessage,
        user_input_snippet: userInputSnippet,
        context_snapshot: contextSnapshot,
      }) as ErrorCaptureResult

      if (result.success && result.ruleId) {
        console.info(`[ErrorLearning] Rule generated for agent '${agentId}': ${result.ruleId}`)
        this.invalidateCache(agentId)
      }

      return result
    } catch (error) {
      console.error('[ErrorLearning] Failed to capture error:', error)
      return null
    }
  }

  async getRules(agentId: string): Promise<void> {
    if (this.ruleCache.has(agentId)) return

    try {
      const result = await harnessErrorGetRules({
        agent_id: agentId,
      }) as unknown as { rules: AvoidanceRule[]; count: number }

      this.ruleCache.set(agentId, result.rules || [])
    } catch (error) {
      console.warn(`[ErrorLearning] Failed to load rules for agent '${agentId}':`, error)
      this.ruleCache.set(agentId, [])
    }
  }

  getCachedRules(agentId: string): AvoidanceRule[] {
    return this.ruleCache.get(agentId) || []
  }

  async buildPromptSection(agentId: string): Promise<string> {
    try {
      const section = await harnessErrorBuildPromptSection({
        agent_id: agentId,
      }) as unknown as string
      return section || ''
    } catch (error) {
      console.warn('[ErrorLearning] Failed to build prompt section:', error)
      return ''
    }
  }

  async triggerRuleHit(ruleId: string): Promise<void> {
    try {
      await harnessErrorTriggerHit({ rule_id: ruleId })
    } catch (error) {
      console.warn('[ErrorLearning] Failed to trigger rule hit:', error)
    }
  }

  invalidateCache(agentId: string) {
    this.ruleCache.delete(agentId)
  }

  clearAllCache() {
    this.ruleCache.clear()
  }

  categorizeError(error: unknown): ErrorCategory {
    const msg = String(error).toLowerCase()

    if (msg.includes('timeout') || msg.includes('rate_limit') || msg.includes('429') || msg.includes('auth')) {
      return 'api'
    }
    if (msg.includes('file not found') || msg.includes('permission') || msg.includes('ENOENT')) {
      return 'tool'
    }
    if (msg.includes('format') || msg.includes('parse') || msg.includes('invalid')) {
      return 'logic'
    }
    if (msg.includes('token') || msg.includes('context') || msg.includes('exceeds')) {
      return 'context'
    }

    return 'other'
  }

  extractUserInputSnippet(fullInput: string, maxLength: number = 200): string {
    if (fullInput.length <= maxLength) return fullInput
    return `${fullInput.slice(0, maxLength)}...`
  }
}

export const errorLearningService = new ErrorLearningService()
export default ErrorLearningService
