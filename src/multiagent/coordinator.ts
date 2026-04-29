// Claw Desktop - 多Agent协调器
// 管理多Agent协作会话的完整生命周期：任务分解、执行模式决策（并行/串行）、
// 上下文共享注入、交叉记忆检索、死循环检测与重试、结果聚合与冲突检测
import type {
  MultiAgentContext,
  SubAgentTask,
  SubAgentStatus,
  SubAgentResult,
  MentionedAgent,
  ExecutionMode,
  AggregatedResponse,
  ConflictResolution,
  MultiAgentSessionStatus,
} from './types'
import {
  SubAgentStatus as SAS,
  ExecutionMode as EM,
  MultiAgentSessionStatus as MASS,
} from './types'
import { subAgentEngine, type TaskUpdateCallback, type TaskCompleteCallback, type ErrorCallback } from './subAgentEngine'
import { agentRegistry } from './agentRegistry'
import { crossMemoryService } from './crossMemory'
import { errorLearningService } from './errorLearning'

/** 协调器回调函数集合 — 会话状态变更、任务更新、完成、错误通知 */
interface CoordinatorCallbacks {
  onStatusChange?: (sessionId: string, status: MultiAgentSessionStatus) => void
  onTaskUpdate?: (taskId: string, task: SubAgentTask) => void
  onComplete?: (response: AggregatedResponse) => void
  onError?: (error: string) => void
}

/** 协调器配置选项 — 超时、并发数、自动重试、上下文共享、交叉记忆、死循环检测 */
interface CoordinatorOptions {
  defaultTimeoutMs?: number
  maxConcurrentTasks?: number
  enableAutoRetry?: boolean
  contextSharing?: boolean
  enableCrossMemory?: boolean
  enableErrorLearning?: boolean
  maxRetryOnDeadloop?: number
  deadloopDetectionMs?: number
}

/** 多Agent协调器 — 管理协作会话的创建、任务分解、执行调度和结果聚合 */
class AgentCoordinator {
  private sessions: Map<string, MultiAgentContext> = new Map()
  private options: Required<CoordinatorOptions>

  constructor(options: CoordinatorOptions = {}) {
    this.options = {
      defaultTimeoutMs: options.defaultTimeoutMs ?? 60000,
      maxConcurrentTasks: options.maxConcurrentTasks ?? 5,
      enableAutoRetry: options.enableAutoRetry ?? true,
      contextSharing: options.contextSharing ?? true,
      enableCrossMemory: options.enableCrossMemory ?? true,
      enableErrorLearning: options.enableErrorLearning ?? true,
      maxRetryOnDeadloop: options.maxRetryOnDeadloop ?? 2,
      deadloopDetectionMs: options.deadloopDetectionMs ?? 30000,
    }
  }

  /** 执行多Agent协作会话：规划 → 执行 → 聚合，支持回调和错误恢复 */
  async executeMultiAgentSession(
    sessionId: string,
    conversationId: string,
    originalQuery: string,
    mentionedAgents: MentionedAgent[],
    mainAgentId: string,
    callbacks?: CoordinatorCallbacks
  ): Promise<AggregatedResponse> {
    const session = this.createSession(sessionId, conversationId, originalQuery, mentionedAgents, mainAgentId)

    try {
      callbacks?.onStatusChange?.(sessionId, MASS.PLANNING)

      const tasks = this.decomposeTask(session)
      session.tasks = tasks

      const executionMode = this.determineExecutionMode(tasks)
      session.executionMode = executionMode
      session.status = MASS.EXECUTING
      callbacks?.onStatusChange?.(sessionId, MASS.EXECUTING)

      await this.executeTasks(session, executionMode, callbacks)

      callbacks?.onStatusChange?.(sessionId, MASS.AGGREGATING)
      const response = this.aggregateResults(session)

      session.aggregatedResult = response.summary
      session.status = MASS.COMPLETED
      callbacks?.onStatusChange?.(sessionId, MASS.COMPLETED)
      callbacks?.onComplete?.(response)

      return response
    } catch (error: any) {
      session.status = MASS.FAILED
      session.error = error.message || 'Unknown error'
      callbacks?.onStatusChange?.(sessionId, MASS.FAILED)
      callbacks?.onError?.(error.message || 'Unknown error')
      return this.buildPartialResult(session)
    }
  }

  /** 创建新的多Agent会话上下文 */
  private createSession(
    sessionId: string, conversationId: string, originalQuery: string,
    mentionedAgents: MentionedAgent[], mainAgentId: string
  ): MultiAgentContext {
    const session: MultiAgentContext = {
      conversationId, originalQuery, mentionedAgents, mainAgentId,
      executionMode: EM.PARALLEL, tasks: [], status: MASS.IDLE,
      createdAt: Date.now(), updatedAt: Date.now(),
    }
    this.sessions.set(sessionId, session)
    return session
  }

  /** 将用户请求分解为子Agent任务，为每个提及的Agent生成独立任务 */
  private decomposeTask(session: MultiAgentContext): SubAgentTask[] {
    const { mentionedAgents, originalQuery, conversationId } = session
    const tasks: SubAgentTask[] = []

    for (let i = 0; i < mentionedAgents.length; i++) {
      const mentioned = mentionedAgents[i]
      const agentConfig = agentRegistry.getById(mentioned.agentId)
      if (!agentConfig) continue

      const taskPrompt = this.buildSubTaskPrompt(originalQuery, agentConfig, mentionedAgents.length > 1)

      tasks.push({
        id: `${conversationId}-task-${i}`,
        agentId: mentioned.agentId,
        agentName: mentioned.agentName,
        description: `Execute ${agentConfig.name} agent task`,
        prompt: taskPrompt,
        status: SAS.PENDING,
        dependsOn: this.resolveDependencies(agentConfig, tasks),
        contextShared: this.options.contextSharing,
        retryCount: 0,
      })
    }

    return tasks
  }

  /** 解析Agent依赖关系，过滤出已存在的依赖任务ID */
  private resolveDependencies(agentConfig: NonNullable<ReturnType<typeof agentRegistry.getById>>, existingTasks: SubAgentTask[]): string[] {
    if (!agentConfig.dependencies || agentConfig.dependencies.length === 0) return []
    return agentConfig.dependencies.filter(depId => existingTasks.some(t => t.agentId === depId))
  }

  /** 构建子Agent任务提示词 — 注入协作上下文、能力提示和输出格式要求 */
  private buildSubTaskPrompt(originalQuery: string, agentConfig: NonNullable<ReturnType<typeof agentRegistry.getById>>, isMultiAgent: boolean): string {
    const contextHeader = isMultiAgent
      ? `[Multi-Agent Collaboration] You are sub-agent "${agentConfig.name}" participating in a collaborative task. The user's original request is below:\n`
      : ''
    const capabilityHint = agentConfig.capabilities.length > 0
      ? `\n\nYour specialized capabilities: ${agentConfig.capabilities.join(', ')}. Focus ONLY on the aspects that match your expertise.`
      : ''
    const outputFormat = isMultiAgent
      ? '\n\nIMPORTANT: Your output will be aggregated by the main agent. Provide:\n1. A clear summary of your findings/actions (2-3 sentences)\n2. Detailed analysis with specific data points\n3. Actionable recommendations if applicable\n\nDo NOT produce vague or generic responses — be specific and substantive.'
      : ''
    return `${contextHeader}${originalQuery}${capabilityHint}${outputFormat}`
  }

  /** 决定执行模式：有依赖关系或包含summary-agent时串行，否则并行 */
  private determineExecutionMode(tasks: SubAgentTask[]): ExecutionMode {
    if (tasks.some(t => t.dependsOn?.length)) return EM.SEQUENTIAL
    if (tasks.find(t => t.agentId === 'summary-agent') && tasks.length > 1) return EM.SEQUENTIAL
    return EM.PARALLEL
  }

  private async executeTasks(
    session: MultiAgentContext, executionMode: ExecutionMode, callbacks?: CoordinatorCallbacks
  ): Promise<void> {
    if (executionMode === EM.PARALLEL) {
      await this.executeParallel(session, callbacks)
    } else {
      await this.executeSequential(session, callbacks)
    }
  }

  private async executeSequential(session: MultiAgentContext, callbacks?: CoordinatorCallbacks): Promise<void> {
    const { tasks } = session
    const executedResults: Map<string, string> = new Map()

    for (const task of tasks) {
      // 注入依赖任务的上下文结果
      if (task.dependsOn?.length) {
        const depResults: Record<string, string> = {}
        for (const depId of task.dependsOn) {
          const result = executedResults.get(depId)
          if (result) depResults[depId] = result
        }
        if (Object.keys(depResults).length > 0) {
          task.prompt += `\n\n[上下文共享 - 来自其他Agent的结果]\n${JSON.stringify(depResults, null, 2)}`
        }
      }

      // 注入交叉记忆上下文
      if (this.options.enableCrossMemory && this.options.contextSharing) {
        const otherAgentIds = tasks.filter(t => t.agentId !== task.agentId).map(t => t.agentId)
        if (otherAgentIds.length > 0) {
          const crossEntries = await crossMemoryService.retrieve({
            sourceAgentId: task.agentId, targetAgentIds: otherAgentIds,
            query: task.prompt, contextLimit: 3,
          })
          if (crossEntries.length > 0) {
            task.prompt += '\n' + crossMemoryService.formatCrossMemoryContext(crossEntries, 1500)
          }
        }
      }

      try {
        const result = await this.runSingleTask(task, session, callbacks)
        executedResults.set(task.id, result.result || '')
      } catch (error) {
        console.error(`[Coordinator] Sequential task ${task.id} failed:`, error)
        if (this.options.enableErrorLearning && task.agentId) {
          const errorMsg = error instanceof Error ? error.message : String(error)
          await errorLearningService.captureError(
            task.agentId, errorLearningService.categorizeError(error), errorMsg,
            session.originalQuery.slice(0, 200),
            JSON.stringify({ taskId: task.id, agentId: task.agentId, mode: session.executionMode }),
          )
        }
      }
    }
  }

  private async executeParallel(session: MultiAgentContext, callbacks?: CoordinatorCallbacks): Promise<void> {
    const { tasks } = session
    const independentTasks = tasks.filter(t => !t.dependsOn?.length)
    const dependentTasks = tasks.filter(t => t.dependsOn && t.dependsOn.length > 0)

    for (const task of independentTasks) {
      if (this.options.enableCrossMemory && this.options.contextSharing) {
        const otherAgentIds = tasks.filter(t => t.agentId !== task.agentId).map(t => t.agentId)
        if (otherAgentIds.length > 0) {
          const crossEntries = await crossMemoryService.retrieve({
            sourceAgentId: task.agentId, targetAgentIds: otherAgentIds,
            query: task.prompt, contextLimit: 3,
          })
          if (crossEntries.length > 0) {
            task.prompt += '\n' + crossMemoryService.formatCrossMemoryContext(crossEntries, 1500)
          }
        }
      }
    }

    const taskPromises = independentTasks.map(task =>
      this.runSingleTask(task, session, callbacks).catch(error => {
        console.error(`[Coordinator] Parallel task ${task.id} failed:`, error)
      })
    )

    await Promise.allSettled(taskPromises)

    const executedResults: Map<string, string> = new Map()
    for (const task of independentTasks) {
      if (task.result) executedResults.set(task.id, task.result)
    }

    for (const task of dependentTasks) {
      if (task.dependsOn?.length) {
        const depResults: Record<string, string> = {}
        for (const depId of task.dependsOn) {
          const result = executedResults.get(depId)
          if (result) depResults[depId] = result
        }
        if (Object.keys(depResults).length > 0) {
          task.prompt += `\n\n[上下文共享 - 来自其他Agent的结果]\n${JSON.stringify(depResults, null, 2)}`
        }
      }

      if (this.options.enableCrossMemory && this.options.contextSharing) {
        const otherAgentIds = tasks.filter(t => t.agentId !== task.agentId).map(t => t.agentId)
        if (otherAgentIds.length > 0) {
          const crossEntries = await crossMemoryService.retrieve({
            sourceAgentId: task.agentId, targetAgentIds: otherAgentIds,
            query: task.prompt, contextLimit: 3,
          })
          if (crossEntries.length > 0) {
            task.prompt += '\n' + crossMemoryService.formatCrossMemoryContext(crossEntries, 1500)
          }
        }
      }

      await this.runSingleTask(task, session, callbacks).catch(error => {
        console.error(`[Coordinator] Dependent task ${task.id} failed:`, error)
      })
    }
  }

  private runSingleTask(
    task: SubAgentTask, session: MultiAgentContext, callbacks?: CoordinatorCallbacks
  ) {
    const taskUpdateCb: TaskUpdateCallback = (taskId, updatedTask) => {
      const idx = session.tasks.findIndex(t => t.id === taskId)
      if (idx !== -1) session.tasks[idx] = updatedTask
      callbacks?.onTaskUpdate?.(taskId, updatedTask)
      session.updatedAt = Date.now()
    }

    const completeCb: TaskCompleteCallback = (result) => {
      callbacks?.onTaskUpdate?.(result.taskId, subAgentEngine.getTask(result.taskId) || task)
    }

    const errorCb: ErrorCallback = async (taskId, error) => {
      if (this.options.enableErrorLearning && task.agentId) {
        await errorLearningService.captureError(
          task.agentId, errorLearningService.categorizeError(error),
          String(error), session.originalQuery.slice(0, 200),
          JSON.stringify({ taskId, agentId: task.agentId, mode: session.executionMode }),
        )
      }
      callbacks?.onError?.(`Task ${taskId} failed: ${error}`)
    }

    const deadloopDetector = new DeadloopDetector(
      this.options.deadloopDetectionMs,
      this.options.maxRetryOnDeadloop,
    )
    deadloopDetector.recordStart(task.id)

    const executeWithDeadloopProtection = async (): Promise<SubAgentResult> => {
      const result = await subAgentEngine.executeTask(task, {
        timeoutMs: this.options.defaultTimeoutMs,
        maxRetries: this.options.enableAutoRetry ? 2 : 0,
        onStatusUpdate: taskUpdateCb,
        onComplete: completeCb,
        onError: errorCb,
        context: {
          conversationId: session.conversationId,
          mainAgentId: session.mainAgentId,
          originalQuery: session.originalQuery,
          otherAgents: session.mentionedAgents.map(a => a.agentId),
        },
      })

      if (deadloopDetector.isDeadloop(result)) {
        const retryCount = deadloopDetector.getRetryCount(task.id)
        if (retryCount < this.options.maxRetryOnDeadloop) {
          deadloopDetector.recordRetry(task.id)
          callbacks?.onTaskUpdate?.(task.id, {
            ...task,
            status: SAS.PENDING,
            retryCount: retryCount + 1,
            error: `Deadloop detected, retrying (${retryCount + 1}/${this.options.maxRetryOnDeadloop})`,
          })
          return executeWithDeadloopProtection()
        } else {
          callbacks?.onTaskUpdate?.(task.id, {
            ...task,
            status: SAS.FAILED,
            error: `Deadloop detected after ${this.options.maxRetryOnDeadloop} retries`,
          })
        }
      }

      return result
    }

    return executeWithDeadloopProtection()
  }

  /** 聚合所有子Agent执行结果，检测冲突并生成汇总报告 */
  private aggregateResults(session: MultiAgentContext): AggregatedResponse {
    const { tasks, originalQuery, mentionedAgents } = session
    const details: Record<string, { agentName: string; result: string; status: SubAgentStatus }> = {}
    const results: Array<{ agentName: string; result: string; status: SubAgentStatus; agentId: string }> = []

    for (const task of tasks) {
      const isLazy = task.status === SAS.COMPLETED && this.isLazyResult(task.result || '')
      const isStuck = task.retryCount >= 2 && task.status !== SAS.COMPLETED
      const detail = {
        agentName: task.agentName,
        result: isLazy
          ? `${task.result || 'No result'}\n\n⚠️ [Monitor] This agent may have produced a low-effort response — consider re-prompting with more specific instructions.`
          : isStuck
            ? `${task.error || task.result || 'No result'}\n\n⚠️ [Monitor] This agent appears stuck after ${task.retryCount} retries — consider simplifying the task or using a different approach.`
            : task.result || task.error || 'No result',
        status: task.status,
      }
      details[task.agentId] = detail
      results.push({ ...detail, agentId: task.agentId })
    }

    const conflicts = this.detectConflicts(results)
    const resolvedConflicts = conflicts.map(c => this.resolveConflict(c))
    const summary = this.generateSummary(originalQuery, results, resolvedConflicts)

    return { summary, details, conflicts: resolvedConflicts, suggestions: this.generateSuggestions(results) }
  }

  /** 检测"偷懒"结果 — 过短或包含通用敷衍短语 */
  private isLazyResult(result: string): boolean {
    if (!result) return true
    const trimmed = result.trim()
    if (trimmed.length < 50) return true
    const genericPhrases = [
      'task completed', 'done', 'finished', 'no issues found',
      'everything looks good', 'completed successfully', 'no action needed',
    ]
    const lower = trimmed.toLowerCase()
    return genericPhrases.some(p => lower.includes(p)) && trimmed.length < 150
  }

  private detectConflicts(results: Array<{ agentName: string; result: string; status: SubAgentStatus; agentId: string }>): ConflictResolution[] {
    if (results.length < 2) return []
    const conflicts: ConflictResolution[] = []
    const successResults = results.filter(r => r.status === SAS.COMPLETED)

    for (let i = 0; i < successResults.length; i++) {
      for (let j = i + 1; j < successResults.length; j++) {
        if (this.hasContentConflict(successResults[i].result, successResults[j].result)) {
          conflicts.push({
            type: 'user_decision', agents: [successResults[i].agentId, successResults[j].agentId],
            resolution: 'pending',
            reason: `Detected potential conflict between ${successResults[i].agentName} and ${successResults[j].agentName} outputs`,
          })
        }
      }
    }
    return conflicts
  }

  private hasContentConflict(result1: string, result2: string): boolean {
    const words1 = new Set(result1.toLowerCase().split(/\s+/))
    const words2 = new Set(result2.toLowerCase().split(/\s+/))
    const intersection = [...words1].filter(w => words2.has(w)).length
    const union = new Set([...words1, ...words2]).size
    const jaccard = union > 0 ? intersection / union : 0
    return jaccard < 0.3 && Math.abs(result1.length - result2.length) / Math.max(result1.length, result2.length) > 0.5
  }

  private resolveConflict(conflict: ConflictResolution): ConflictResolution {
    if (conflict.type === 'priority') return { ...conflict, resolution: 'used_priority_order', reason: 'Resolved by priority order' }
    return { ...conflict, resolution: 'presented_to_user', reason: 'Requires user decision' }
  }

  private generateSummary(originalQuery: string, results: Array<{ agentName: string; result: string; status: SubAgentStatus; agentId: string }>, _conflicts: ConflictResolution[]): string {
    const completedResults = results.filter(r => r.status === SAS.COMPLETED)
    const failedResults = results.filter(r => r.status !== SAS.COMPLETED)

    let summary = `## 多Agent协作结果\n\n`
    summary += `**原始问题**: ${originalQuery}\n\n`
    summary += `### 参与Agent (${completedResults.length}/${results.length})\n\n`

    for (const r of completedResults) {
      summary += `- **${r.agentName}**: ${r.result.slice(0, 200)}${r.result.length > 200 ? '...' : ''}\n\n`
    }

    if (failedResults.length > 0) {
      summary += `### ⚠️ 未成功完成的任务\n\n`
      for (const r of failedResults) {
        summary += `- **${r.agentName}**: 执行失败 (${r.status})\n\n`
      }
    }

    return summary + `---\n*以上内容由各子Agent协同处理，由主Agent汇总整合*\n`
  }

  private generateSuggestions(results: Array<{ agentName: string; result: string; status: SubAgentStatus; agentId: string }>): string[] | undefined {
    const completedCount = results.filter(r => r.status === SAS.COMPLETED).length
    if (completedCount === 0) return ['所有子Agent执行失败，建议检查网络连接或重试']
    if (completedCount < results.length) {
      return [`${results.filter(r => r.status !== SAS.COMPLETED).map(r => r.agentName).join(', ')} 执行未完成，可单独重新调用`]
    }
    return undefined
  }

  private buildPartialResult(session: MultiAgentContext): AggregatedResponse {
    const details: Record<string, { agentName: string; result: string; status: SubAgentStatus }> = {}
    for (const task of session.tasks) {
      details[task.agentId] = { agentName: task.agentName, result: task.result || task.error || 'Incomplete', status: task.status }
    }
    return { summary: `## 协作结果（部分完成）\n\n由于部分任务执行中断，以下为已获取的结果。\n\n错误: ${session.error || 'Unknown'}`, details, conflicts: [] }
  }

  getSession(sessionId: string): MultiAgentContext | undefined { return this.sessions.get(sessionId) }

  cancelSession(sessionId: string): boolean {
    const session = this.sessions.get(sessionId)
    if (!session) return false
    for (const task of session.tasks) subAgentEngine.cancelTask(task.id)
    session.status = MASS.FAILED
    session.error = 'Cancelled by user'
    return true
  }

  getActiveSessions(): MultiAgentContext[] {
    return Array.from(this.sessions.values()).filter(s =>
      s.status === MASS.PLANNING || s.status === MASS.EXECUTING || s.status === MASS.AGGREGATING
    )
  }
}

export const agentCoordinator = new AgentCoordinator()
export { AgentCoordinator }
export default AgentCoordinator

  /** 死循环检测器 — 通过结果哈希比对和超时判断检测Agent陷入循环 */
class DeadloopDetector {
  private detectionMs: number
  private maxRetries: number
  private retryCounts: Map<string, number> = new Map()
  private taskStartTimes: Map<string, number> = new Map()
  private lastResults: Map<string, string> = new Map()
  private lastOutputTimes: Map<string, number> = new Map()

  constructor(detectionMs: number, maxRetries: number) {
    this.detectionMs = detectionMs
    this.maxRetries = maxRetries
  }

  recordStart(taskId: string): void {
    this.taskStartTimes.set(taskId, Date.now())
  }

  isDeadloop(task: { status: SubAgentStatus; result?: string; error?: string; id?: string }): boolean {
    if (task.status !== SAS.TIMEOUT && task.status !== SAS.FAILED) return false

    const taskId = task.id || '__unknown__'

    const resultHash = this.simpleHash(task.result || task.error || '')
    const lastHash = this.lastResults.get(taskId)
    this.lastResults.set(taskId, resultHash)

    if (lastHash && lastHash === resultHash) return true
    if (task.status === SAS.TIMEOUT) return true

    return false
  }

  isIdle(taskId: string): boolean {
    const lastOutput = this.lastOutputTimes.get(taskId)
    if (!lastOutput) return false
    return Date.now() - lastOutput > this.detectionMs
  }

  recordOutput(taskId: string): void {
    this.lastOutputTimes.set(taskId, Date.now())
  }

  getRetryCount(taskId: string): number {
    return this.retryCounts.get(taskId) || 0
  }

  recordRetry(taskId: string): void {
    this.retryCounts.set(taskId, this.getRetryCount(taskId) + 1)
  }

  private simpleHash(str: string): string {
    let hash = 0
    for (let i = 0; i < str.length; i++) {
      const char = str.charCodeAt(i)
      hash = ((hash << 5) - hash) + char
      hash = hash & hash
    }
    return hash.toString(36)
  }
}
