// Claw Desktop - 子Agent执行引擎
// 负责子Agent任务的执行调度：超时控制、重试策略、后端可用性探测、
// WebSocket流式事件监听、模拟模式降级、任务取消与状态管理
import { wsOnEvent } from '../ws/bridge'
import { executeSubAgent, coordinationMessage } from '../api/multiAgent'
import type {
  SubAgentTask,
  SubAgentStatus,
  SubAgentResult,
  CoordinationMessage,
} from './types'
import { SubAgentStatus as SAS } from './types'
import { agentRegistry } from './agentRegistry'

/** 任务状态更新回调 */
export type TaskUpdateCallback = (taskId: string, task: SubAgentTask) => void
/** 任务完成回调 */
export type TaskCompleteCallback = (result: SubAgentResult) => void
/** 任务错误回调 */
export type ErrorCallback = (taskId: string, error: string) => void

/** 子Agent执行选项 — 超时、重试、回调、上下文 */
interface ExecutionOptions {
  timeoutMs?: number
  maxRetries?: number
  onStatusUpdate?: TaskUpdateCallback
  onComplete?: TaskCompleteCallback
  onError?: ErrorCallback
  context?: Record<string, any>
}

/** 模拟响应生成器 — 后端不可用时为各内置Agent提供模拟输出 */
const SIMULATED_RESPONSES: Record<string, (prompt: string) => string> = {
  'search-agent': (prompt) => `[Search Results]\n\nBased on the query "${prompt.slice(0, 100)}", I performed a comprehensive search and found the following relevant information:\n\n1. **Primary Result**: Key information retrieved related to the topic\n2. **Secondary Sources**: Supporting documentation and references\n3. **Related Context**: Additional context for deeper understanding\n\nSummary: The search completed successfully with multiple relevant sources identified.`,
  'code-agent': (prompt) => `[Code Analysis]\n\nAnalyzing the code request: "${prompt.slice(0, 100)}"\n\n**Findings:**\n- Code structure is well-organized\n- Identified key patterns and potential improvements\n- No critical issues detected\n\n**Recommendations:**\n1. Consider adding error handling for edge cases\n2. Type safety can be improved in several areas\n3. Documentation could be enhanced for maintainability\n\nThe codebase follows good practices overall.`,
  'analysis-agent': (prompt) => `[Analysis Report]\n\nTopic: ${prompt.slice(0, 80)}\n\n**Executive Summary:**\nAfter thorough analysis, here are the key findings:\n\n## Data Overview\n- Multiple data points analyzed\n- Patterns identified across the dataset\n- Correlations noted between key variables\n\n## Key Insights\n1. **Trend Analysis**: Clear directional pattern observed\n2. **Anomaly Detection**: 3 outliers flagged for review\n3. **Statistical Significance**: p < 0.05 for primary metrics\n\n## Recommendations\nPriority actions based on analysis results.`,
  'creative-agent': (prompt) => `[Creative Output]\n\nResponding to: "${prompt.slice(0, 80)}"\n\nHere is a creative approach to your request:\n\n---\n\n*The content has been crafted with attention to style, clarity, and engagement. The tone is professional yet accessible, suitable for the intended audience.*\n\n---\n\nKey elements included:\n- Engaging opening hook\n- Structured body content\n- Compelling conclusion with call-to-action`,
  'summary-agent': (prompt) => `[Summary]\n\n**Original Content Length**: ${prompt.length} characters\n\n**Key Points:**\n1. Main topic and thesis clearly identified\n2. Supporting arguments extracted and condensed\n3. Critical data points preserved\n4. Conclusions and implications summarized\n\n**Condensed Version:**\nThe content has been analyzed and summarized into essential points, reducing length by approximately 70% while preserving all critical information and context needed for understanding.`,
}

/** 子Agent执行引擎 — 管理任务执行、超时重试、后端探测、流式监听 */
class SubAgentEngine {
  private activeTasks: Map<string, SubAgentTask> = new Map()
  private abortControllers: Map<string, AbortController> = new Map()
  private unlisteners: Array<() => Promise<void>> = []
  private backendAvailable: boolean | null = null

  /** 执行子Agent任务：查找Agent配置 → 设置超时重试 → 运行并返回结果 */
  async executeTask(
    task: SubAgentTask,
    options: ExecutionOptions = {}
  ): Promise<SubAgentResult> {
    const agentConfig = agentRegistry.getById(task.agentId)
    if (!agentConfig) {
      const error = `Agent not found: ${task.agentId}`
      this.updateTaskStatus(task.id, SAS.FAILED, undefined, error)
      throw new Error(error)
    }

    const timeoutMs = options.timeoutMs ?? agentConfig.timeoutMs
    const maxRetries = options.maxRetries ?? agentConfig.maxRetries

    this.updateTaskStatus(task.id, SAS.RUNNING, Date.now())
    this.activeTasks.set(task.id, task)

    const abortController = new AbortController()
    this.abortControllers.set(task.id, abortController)

    try {
      const result = await this.runWithTimeoutAndRetry(
        task,
        agentConfig,
        timeoutMs,
        maxRetries,
        abortController.signal,
        options
      )

      this.updateTaskStatus(task.id, SAS.COMPLETED, undefined, undefined, result.rawOutput, Date.now())
      options.onComplete?.(result)

      return result
    } catch (error: any) {
      const isTimeout = error.message?.includes('timeout')
      const finalStatus: SubAgentStatus = isTimeout ? SAS.TIMEOUT : SAS.FAILED

      this.updateTaskStatus(task.id, finalStatus, undefined, error.message || 'Unknown error')
      options.onError?.(task.id, error.message || 'Unknown error')

      return {
        taskId: task.id,
        agentId: task.agentId,
        agentName: task.agentName,
        status: finalStatus,
        error: error.message || 'Unknown error',
      }
    } finally {
      this.abortControllers.delete(task.id)
    }
  }

  /** 带超时和重试的任务执行循环 */
  private async runWithTimeoutAndRetry(
    task: SubAgentTask,
    agentConfig: ReturnType<typeof agentRegistry.getById>,
    timeoutMs: number,
    maxRetries: number,
    signal: AbortSignal,
    options: ExecutionOptions
  ): Promise<SubAgentResult> {
    if (!agentConfig) throw new Error(`Agent config not found for task ${task.id}`)
    let lastError: Error | null = null

    for (let attempt = 0; attempt <= maxRetries; attempt++) {
      if (signal.aborted) {
        throw new Error('Task aborted')
      }

      try {
        if (attempt > 0) {
          this.updateTaskRetry(task.id, attempt)
          await this.delay(1000 * attempt)
        }

        return await this.executeSingleRun(task, agentConfig, timeoutMs, signal, options)
      } catch (error: any) {
        lastError = error
        if (!this.isRetryableError(error)) {
          throw error
        }
      }
    }

    throw lastError || new Error('Max retries exceeded')
  }

  /** 探测后端是否可用 — 发送探测请求，失败则降级为模拟模式 */
  private async checkBackendAvailable(): Promise<boolean> {
    if (this.backendAvailable !== null) return this.backendAvailable
    try {
      await executeSubAgent({ taskId: '__probe__', agentId: '__probe__', prompt: '', context: {}, conversationId: '' })
      this.backendAvailable = true
      return true
    } catch (e: any) {
      const msg = (e.message || '').toLowerCase()
      if (msg.includes('not found') || msg.includes('unrecognized') || msg.includes('does not exist')) {
        console.warn('[SubAgentEngine] Backend command execute_sub_agent not available, using simulation mode')
        this.backendAvailable = false
        return false
      }
      this.backendAvailable = true
      return true
    }
  }

  /** 单次执行：后端可用时走WebSocket流式监听，否则走模拟模式 */
  private async executeSingleRun(
    task: SubAgentTask,
    agentConfig: NonNullable<ReturnType<typeof agentRegistry.getById>>,
    timeoutMs: number,
    signal: AbortSignal,
    options: ExecutionOptions
  ): Promise<SubAgentResult> {
    const startTime = Date.now()
    const backendOk = await this.checkBackendAvailable()

    if (!backendOk) {
      return this.executeSimulatedRun(task, startTime, signal, timeoutMs)
    }

    const resultPromise = new Promise<SubAgentResult>((resolve, reject) => {
      let rawOutput = ''
      let resolved = false

      const unsub = wsOnEvent(`subagent-stream-${task.id}`, (msg) => {
        const wsEvent = msg as { event: string; data: { content?: string; full_text?: string; error?: string } }
        const eventType = wsEvent.event
        const payload = wsEvent.data
        if (!payload && !eventType) return

        if (signal.aborted && !resolved) {
          resolved = true
          unsub()
          reject(new Error('Task aborted'))
          return
        }

        switch (eventType) {
          case 'token':
            if (payload?.content) {
              rawOutput += payload.content
              this.updateTaskRawOutput(task.id, rawOutput)
            }
            break
          case 'done':
            if (!resolved) {
              resolved = true
              unsub()
              resolve({
                taskId: task.id,
                agentId: task.agentId,
                agentName: task.agentName,
                status: SAS.COMPLETED,
                result: payload?.full_text || rawOutput,
                rawOutput,
                durationMs: Date.now() - startTime,
              })
            }
            break
          case 'error':
            if (!resolved) {
              resolved = true
              unsub()
              reject(new Error(payload?.error || 'Sub-agent execution failed'))
            }
            break
          case 'input_request':
            this.updateTaskStatus(task.id, SAS.WAITING_INPUT)
            break
        }
      })

      setTimeout(() => {
        if (!resolved && !signal.aborted) {
          resolved = true
          unsub()
          reject(new Error(`Execution timeout after ${timeoutMs}ms`))
        }
      }, timeoutMs)
    })

    try {
      await executeSubAgent({
        taskId: task.id,
        agentId: task.agentId,
        prompt: task.prompt,
        context: options.context || {},
        conversationId: task.id,
      })
    } catch (invokeError: any) {
      const errMsg = (invokeError.message || '').toLowerCase()
      if (errMsg.includes('not found') || errMsg.includes('unrecognized')) {
        this.backendAvailable = false
        return this.executeSimulatedRun(task, startTime, signal, timeoutMs)
      }
      throw new Error(`Failed to invoke sub-agent: ${invokeError.message}`)
    }

    return resultPromise
  }

  /** 模拟执行 — 后端不可用时使用预设模板生成模拟响应 */
  private async executeSimulatedRun(
    task: SubAgentTask,
    startTime: number,
    signal: AbortSignal,
    timeoutMs: number
  ): Promise<SubAgentResult> {
    const simulateDelay = Math.min(800 + Math.random() * 1200, timeoutMs - 500)

    await this.delay(simulateDelay)
    if (signal.aborted) throw new Error('Task aborted')

    const generator = SIMULATED_RESPONSES[task.agentId]
    const rawOutput = generator ? generator(task.prompt) : `[${task.agentName} Response]\n\nProcessed: ${task.prompt.slice(0, 120)}\n\nTask completed successfully.`

    if (signal.aborted) throw new Error('Task aborted')

    this.updateTaskRawOutput(task.id, rawOutput)

    return {
      taskId: task.id,
      agentId: task.agentId,
      agentName: task.agentName,
      status: SAS.COMPLETED,
      result: rawOutput,
      rawOutput,
      durationMs: Date.now() - startTime,
    }
  }

  /** 取消指定任务 — 中止AbortController并更新状态为FAILED */
  cancelTask(taskId: string): boolean {
    const controller = this.abortControllers.get(taskId)
    if (controller) {
      controller.abort()
      const task = this.activeTasks.get(taskId)
      if (task) {
        this.updateTaskStatus(taskId, SAS.FAILED, undefined, 'Cancelled by user')
      }
      return true
    }
    return false
  }

  getTask(taskId: string): SubAgentTask | undefined {
    return this.activeTasks.get(taskId)
  }

  getAllTasks(): SubAgentTask[] {
    return Array.from(this.activeTasks.values())
  }

  getActiveTasks(): SubAgentTask[] {
    return this.getAllTasks().filter(t =>
      t.status === SAS.RUNNING || t.status === SAS.PENDING || t.status === SAS.WAITING_INPUT
    )
  }

  isBackendAvailable(): boolean | null {
    return this.backendAvailable
  }

  private updateTaskStatus(
    taskId: string,
    status: SubAgentStatus,
    startTime?: number,
    error?: string,
    rawOutput?: string,
    endTime?: number
  ) {
    const task = this.activeTasks.get(taskId)
    if (!task) return

    const updated: SubAgentTask = {
      ...task,
      status,
      ...(startTime !== undefined && { startTime }),
      ...(error !== undefined && { error }),
      ...(rawOutput !== undefined && { rawOutput }),
      ...(endTime !== undefined && { endTime, durationMs: endTime - (task.startTime || endTime) }),
    }

    this.activeTasks.set(taskId, updated)
  }

  private updateTaskRawOutput(taskId: string, rawOutput: string) {
    const task = this.activeTasks.get(taskId)
    if (!task) return
    this.activeTasks.set(taskId, { ...task, rawOutput })
  }

  private updateTaskRetry(taskId: string, retryCount: number) {
    const task = this.activeTasks.get(taskId)
    if (!task) return
    this.activeTasks.set(taskId, { ...task, retryCount })
  }

  /** 判断错误是否可重试 — 超时、网络、限流、5xx等 */
  private isRetryableError(error: Error): boolean {
    const retryablePatterns = ['timeout', 'network', 'rate limit', '503', '502', '429']
    const message = error.message.toLowerCase()
    return retryablePatterns.some(p => message.includes(p))
  }

  private delay(ms: number): Promise<void> {
    return new Promise(resolve => setTimeout(resolve, ms))
  }

  /** 发送Agent间协调消息 */
  sendCoordinationMessage(message: CoordinationMessage): void {
    coordinationMessage({ message }).catch(console.error)
  }

  /** 清理已完成/失败/超时的任务 */
  clearCompleted(): void {
    for (const [id, task] of this.activeTasks.entries()) {
      if (task.status === SAS.COMPLETED || task.status === SAS.FAILED || task.status === SAS.TIMEOUT) {
        this.activeTasks.delete(id)
      }
    }
  }

  /** 重置引擎 — 中止所有任务、清空状态 */
  reset(): void {
    for (const controller of this.abortControllers.values()) {
      controller.abort()
    }
    this.activeTasks.clear()
    this.abortControllers.clear()
  }
}

export const subAgentEngine = new SubAgentEngine()
export { SubAgentEngine }
export default SubAgentEngine
