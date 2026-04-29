// Claw Desktop - Agent注册表模块
// 管理内置Agent定义（搜索、编码、文件、Shell、浏览器、记忆、规划等），
// 提供Agent的注册/查询/过滤/分类能力，支持3D可视化的分类布局
import type { AgentRegistryEntry, AgentCategory } from './types'
import { AgentCategory as AC } from './types'

const DEFAULT_TIMEOUT_MS = 30000
const DEFAULT_MAX_RETRIES = 2

export const BUILT_IN_AGENTS: AgentRegistryEntry[] = [
  {
    id: 'search-agent',
    name: 'Search',
    description: '网络搜索与信息检索，支持实时搜索最新资讯、文档查询和知识库检索',
    systemPrompt: '你是一个专业的搜索助手，擅长使用搜索工具查找信息。请根据用户需求进行精确搜索并返回结构化的结果。',
    mentionable: true,
    category: AC.SEARCH,
    icon: '🔍',
    capabilities: ['web_search', 'web_fetch', 'document_retrieval'],
    tools: ['web_search', 'web_fetch'],
    maxTurns: 5,
    enabled: true,
    timeoutMs: DEFAULT_TIMEOUT_MS,
    maxRetries: DEFAULT_MAX_RETRIES,
    createdAt: Date.now(),
    updatedAt: Date.now(),
  },
  {
    id: 'code-agent',
    name: 'Code',
    description: '代码分析、编写、调试和重构，支持多种编程语言',
    systemPrompt: '你是一个专业的代码助手，擅长代码分析、编写、调试和重构。请提供高质量的代码解决方案。',
    mentionable: true,
    category: AC.CODE,
    icon: '💻',
    capabilities: ['code_analysis', 'code_generation', 'debugging', 'refactoring'],
    tools: ['read', 'edit', 'write', 'grep', 'glob'],
    maxTurns: 10,
    enabled: true,
    timeoutMs: 60000,
    maxRetries: DEFAULT_MAX_RETRIES,
    createdAt: Date.now(),
    updatedAt: Date.now(),
  },
  {
    id: 'analysis-agent',
    name: 'Analysis',
    description: '数据分析、报告生成和深度研究，擅长处理复杂信息并产出结构化结论',
    systemPrompt: '你是一个专业的分析师，擅长数据处理、分析和报告生成。请提供深入的分析结果和可行的建议。',
    mentionable: true,
    category: AC.ANALYSIS,
    icon: '📊',
    capabilities: ['data_analysis', 'report_generation', 'research', 'summarization'],
    tools: [],
    maxTurns: 8,
    enabled: true,
    timeoutMs: 45000,
    maxRetries: DEFAULT_MAX_RETRIES,
    createdAt: Date.now(),
    updatedAt: Date.now(),
  },
  {
    id: 'creative-agent',
    name: 'Creative',
    description: '创意写作、内容生成和文案优化，支持多风格和多语言输出',
    systemPrompt: '你是一个创意写作专家，擅长各类文案创作、内容优化和创意构思。请根据用户需求创作高质量的内容。',
    mentionable: true,
    category: AC.CREATIVE,
    icon: '✨',
    capabilities: ['writing', 'content_creation', 'copywriting', 'translation'],
    tools: [],
    maxTurns: 6,
    enabled: true,
    timeoutMs: 40000,
    maxRetries: DEFAULT_MAX_RETRIES,
    createdAt: Date.now(),
    updatedAt: Date.now(),
  },
  {
    id: 'summary-agent',
    name: 'Summary',
    description: '信息摘要、关键点提取和内容精炼，可将长文本压缩为精要总结',
    systemPrompt: '你是一个专业的摘要助手，擅长从大量信息中提炼关键要点，生成简洁准确的摘要。',
    mentionable: true,
    category: AC.ANALYSIS,
    icon: '📝',
    capabilities: ['summarization', 'key_extraction', 'content_condensing'],
    tools: [],
    maxTurns: 4,
    enabled: true,
    timeoutMs: 20000,
    maxRetries: DEFAULT_MAX_RETRIES,
    createdAt: Date.now(),
    updatedAt: Date.now(),
  },
  {
    id: 'desktop-agent',
    name: 'Desktop',
    description: '桌面自动化操作 — 打开应用、点击UI元素、键盘输入、屏幕截图与OCR识别、文件双击打开等桌面交互',
    systemPrompt: 'You are a desktop automation agent. You control the user\'s computer through screen capture, OCR, mouse, and keyboard tools.\n\n## Core Workflow\n1. ALWAYS start by capturing the screen to understand current state: use CaptureScreen\n2. For clicking elements, first use OcrRecognizeScreen to get coordinates, then MouseClick or MouseDoubleClick\n3. For opening applications: KeyboardPress("Super") → KeyboardType("app name") → KeyboardPress("Enter")\n4. After each action, verify with CaptureScreen\n5. Report results clearly to the main agent\n\n## Rules\n- NEVER guess coordinates — always use OCR results\n- Use MouseDoubleClick for opening desktop icons/files\n- Use MouseClick for buttons and links\n- Use KeyboardPress for special keys (Enter, Tab, Escape, Super, Ctrl, Alt)\n- If an app is not found, report it back — do NOT try to install anything\n- Keep responses concise and focused on the task result\n- End with [RESPONSE_COMPLETE] when done',
    mentionable: true,
    category: AC.GENERAL,
    icon: '🖥️',
    capabilities: ['desktop_automation', 'screen_capture', 'ocr', 'mouse_control', 'keyboard_control', 'app_launch'],
    tools: ['ExecuteAutomation', 'CaptureScreen', 'OcrRecognizeScreen', 'MouseClick', 'MouseDoubleClick', 'MouseRightClick', 'KeyboardType', 'KeyboardPress'],
    maxTurns: 15,
    enabled: true,
    timeoutMs: 120000,
    maxRetries: 3,
    createdAt: Date.now(),
    updatedAt: Date.now(),
  },
]

class AgentRegistry {
  private agents: Map<string, AgentRegistryEntry> = new Map()

  constructor() {
    this.initializeBuiltInAgents()
  }

  private initializeBuiltInAgents() {
    BUILT_IN_AGENTS.forEach(agent => this.agents.set(agent.id, agent))
  }

  getAll(): AgentRegistryEntry[] {
    return Array.from(this.agents.values()).filter(a => a.enabled)
  }

  getMentionable(): AgentRegistryEntry[] {
    return this.getAll().filter(a => a.mentionable)
  }

  getById(id: string): AgentRegistryEntry | undefined {
    return this.agents.get(id)
  }

  getByName(name: string): AgentRegistryEntry | undefined {
    return this.getAll().find(a =>
      a.name.toLowerCase() === name.toLowerCase() ||
      a.id.toLowerCase() === name.toLowerCase()
    )
  }

  getByCategory(category: AgentCategory): AgentRegistryEntry[] {
    return this.getAll().filter(a => a.category === category)
  }

  register(agent: AgentRegistryEntry): void {
    this.agents.set(agent.id, { ...agent, updatedAt: Date.now() })
  }

  registerCustomAgents(agents: Array<{ id: string; displayName: string; description?: string; purpose?: string; scope?: string }>): void {
    for (const a of agents) {
      if (this.agents.has(a.id)) {
        const existing = this.agents.get(a.id)!
        if (existing.category === AC.CUSTOM) {
          this.agents.set(a.id, {
            ...existing,
            name: a.displayName || existing.name,
            description: a.description || a.purpose || existing.description,
            mentionable: true,
            enabled: true,
            updatedAt: Date.now(),
          })
        }
        continue
      }
      this.agents.set(a.id, {
        id: a.id,
        name: a.displayName,
        description: a.description || a.purpose || '',
        systemPrompt: '',
        mentionable: true,
        category: AC.CUSTOM,
        icon: '🤖',
        capabilities: [],
        tools: [],
        maxTurns: 10,
        enabled: true,
        timeoutMs: 60000,
        maxRetries: 2,
        createdAt: Date.now(),
        updatedAt: Date.now(),
      })
    }
  }

  unregister(id: string): boolean {
    return this.agents.delete(id)
  }

  update(id: string, updates: Partial<AgentRegistryEntry>): boolean {
    const existing = this.agents.get(id)
    if (!existing) return false
    this.agents.set(id, { ...existing, ...updates, updatedAt: Date.now() })
    return true
  }

  setEnabled(id: string, enabled: boolean): boolean {
    return this.update(id, { enabled })
  }

  exists(id: string): boolean {
    return this.agents.has(id)
  }

  search(query: string): AgentRegistryEntry[] {
    const lowerQuery = query.toLowerCase()
    return this.getAll().filter(a =>
      a.name.toLowerCase().includes(lowerQuery) ||
      a.description.toLowerCase().includes(lowerQuery) ||
      a.capabilities.some(c => c.toLowerCase().includes(lowerQuery)) ||
      a.id.toLowerCase().includes(lowerQuery)
    )
  }
}

export const agentRegistry = new AgentRegistry()
export { AgentRegistry }
export default AgentRegistry
