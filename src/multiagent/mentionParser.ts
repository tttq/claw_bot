// Claw Desktop - @提及解析模块
// 解析用户输入中的@AgentName提及，支持模糊匹配、位置查询、显示文本构建
import type { MentionedAgent, ParsedMentions } from './types'
import { agentRegistry } from './agentRegistry'

const MENTION_REGEX = /@(\S+?)(?=\s|$|@)/g

export function parseMentions(input: string): ParsedMentions {
  const mentions: MentionedAgent[] = []
  let match: RegExpExecArray | null
  const regex = new RegExp(MENTION_REGEX.source, 'g')

  while ((match = regex.exec(input)) !== null) {
    const mentionText = match[1]
    const agent = agentRegistry.getByName(mentionText)

    if (agent) {
      mentions.push({
        agentId: agent.id,
        agentName: agent.name,
        startIndex: match.index,
        endIndex: match.index + match[0].length,
      })
    }
  }

  const cleanText = input.replace(MENTION_REGEX, '').replace(/\s+/g, ' ').trim()

  return {
    rawText: input,
    mentions,
    cleanText,
  }
}

export function hasMentions(input: string): boolean {
  MENTION_REGEX.lastIndex = 0
  return MENTION_REGEX.test(input)
}

export function getMentionAtPosition(input: string, cursorPosition: number): string | null {
  const textBeforeCursor = input.slice(0, cursorPosition)
  const atIndex = textBeforeCursor.lastIndexOf('@')

  if (atIndex === -1) return null

  const afterAt = textBeforeCursor.slice(atIndex + 1)
  if (/\s/.test(afterAt)) return null

  return afterAt
}

export function filterAgentsByQuery(query: string) {
  return agentRegistry.getMentionable().filter(agent => {
    if (!query) return true
    const lowerQuery = query.toLowerCase()
    return (
      agent.name.toLowerCase().includes(lowerQuery) ||
      agent.description.toLowerCase().includes(lowerQuery)
    )
  })
}

export function buildMentionDisplayText(mentions: MentionedAgent[]): string {
  if (mentions.length === 0) return ''
  return mentions.map(m => `@${m.agentName}`).join(' ')
}
