// Claw Desktop - 多Agent模块入口
// 统一导出多Agent系统的类型、注册表、协调器、交叉记忆、错误学习、提及解析、子Agent引擎
export * from './types'
export { agentRegistry, AgentRegistry, BUILT_IN_AGENTS } from './agentRegistry'
export {
  parseMentions,
  hasMentions,
  getMentionAtPosition,
  filterAgentsByQuery,
  buildMentionDisplayText,
} from './mentionParser'
export { subAgentEngine, SubAgentEngine } from './subAgentEngine'
export { agentCoordinator, AgentCoordinator } from './coordinator'
