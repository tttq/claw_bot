// Claw Desktop - 交叉记忆服务模块
// 实现Agent间的记忆共享：通过@提及触发跨Agent记忆检索，
// 解析提及目标、获取相关记忆条目、构建交叉记忆上下文
import { harnessCrossMemoryRetrieve, harnessCrossMemoryParseMentions } from '../api/harness'

export interface CrossMemoryEntry {
  sourceAgentId: string
  sourceAgentName: string
  content: string
  relevanceScore: number
  factType: string
  occurredAt?: number | null
}

export interface CrossMemoryRequest {
  sourceAgentId: string
  targetAgentIds: string[]
  query: string
  contextLimit?: number
}

class CrossMemoryService {
  private enabled: boolean = true

  setEnabled(enabled: boolean) {
    this.enabled = enabled
  }

  async retrieve(request: CrossMemoryRequest): Promise<CrossMemoryEntry[]> {
    if (!this.enabled || request.targetAgentIds.length === 0) {
      return []
    }

    try {
      const result = await harnessCrossMemoryRetrieve({
        source_agent_id: request.sourceAgentId,
        target_agent_ids: request.targetAgentIds,
        query: request.query,
        context_limit: request.contextLimit,
      }) as unknown as { entries: CrossMemoryEntry[]; count: number }
      return result.entries || []
    } catch (error) {
      console.warn('[CrossMemory] Failed to retrieve cross-agent memory:', error)
      return []
    }
  }

  async parseMentions(input: string, knownAgentIds: string[]): Promise<string[]> {
    if (!input.includes('@')) return []

    try {
      const result = await harnessCrossMemoryParseMentions({ input, known_agent_ids: knownAgentIds }) as unknown as { mentions: string[] }
      return result.mentions || []
    } catch (error) {
      console.warn('[CrossMemory] Failed to parse mentions:', error)
      return this.fallbackParseMentions(input, knownAgentIds)
    }
  }

  formatCrossMemoryContext(entries: CrossMemoryEntry[], maxChars: number = 2000): string {
    if (entries.length === 0) return ''

    let context = '\n## Cross-Agent Memory Context\n'
    context += 'The following information was retrieved from other agents\' memories:\n\n'

    let totalChars = 0

    for (let i = 0; i < entries.length; i++) {
      const entry = entries[i]
      const entryText = entry.content.length > 200
        ? `${entry.content.slice(0, 200)}...`
        : entry.content

      const line = `### From @${entry.sourceAgentName} (relevance: ${entry.relevanceScore.toFixed(2)})\n${entryText.trim()}\n`

      if (totalChars + line.length > maxChars) {
        context += `\n... (truncated, ${entries.length - i} more entries available)\n`
        break
      }

      context += line
      totalChars += line.length
    }

    context += '--- End Cross-Agent Context ---\n'
    return context
  }

  private fallbackParseMentions(input: string, knownAgentIds: string[]): string[] {
    const mentions: string[] = []

    for (const word of input.split(/\s+/)) {
      if (word.startsWith('@')) {
        const target = word.slice(1).replace(/[.,!?;:]$/, '')

        for (const aid of knownAgentIds) {
          if (aid === target || aid.toLowerCase().includes(target.toLowerCase())) {
            mentions.push(aid)
            break
          }
        }
      }
    }

    return [...new Set(mentions)].sort()
  }
}

export const crossMemoryService = new CrossMemoryService()
export default CrossMemoryService
