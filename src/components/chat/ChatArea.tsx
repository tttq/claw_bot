// Claw Desktop - 聊天区域核心组件
// 负责消息列表渲染（用户/AI/工具消息）、流式输出气泡、思考过程折叠、
// 工具执行步骤卡片、Markdown渲染（代码高亮/表格/链接）、确认对话框等
import React, { useRef, useEffect, useState, useCallback } from 'react'
import { useTranslation } from 'react-i18next'
import Markdown from 'react-markdown'
import remarkGfm from 'remark-gfm'
import { Prism as SyntaxHighlighter } from 'react-syntax-highlighter'
import { oneDark } from 'react-syntax-highlighter/dist/esm/styles/prism'
import type { Message, MarkdownComponentProps } from '../../types'
import MultiAgentMessage from './MultiAgentMessage'
import type { MultiAgentMessageContent } from '../../multiagent/types'
import type { ToolExecutionDetail } from '../../stores/conversationStore'
import { rehypeAutolink } from '../../utils/rehypeAutolink'
import { sanitizeHtml } from '../../utils/sanitizeHtml'

/** 聊天区域组件属性 */
interface ChatAreaProps {
  messages: Message[]
  isLoading: boolean
  conversationTitle?: string
  messagesEndRef: React.RefObject<HTMLDivElement>
  conversationId?: string | null
  activeAgentId?: string | null
  showToolExecutions?: boolean
  streamingText?: string
  thinkingText?: string
  multiAgentMessages?: MultiAgentMessageContent[]
  toolExecutions?: Array<{ toolName: string; durationMs: number; index: number; total: number }>
  toolExecutionDetails?: ToolExecutionDetail[]
  pendingConfirmation?: { conversationId: string; prompt: string; options: Array<{ label: string; value: string }> } | null
  onConfirm?: (value: string) => void
  onDismissConfirm?: () => void
  onSuggestionClick?: (text: string) => void
}

/** 工具名称 → 图标映射表 */
const TOOL_ICONS_MAP: Record<string, string> = {
  Read:'📄',Edit:'✏️',Write:'📝',Bash:'⚡',Glob:'🔍',Grep:'🔎',WebFetch:'🌐',WebSearch:'🔍',
  Agent:'🤖',TodoWrite:'✅',TaskCreate:'📋',Skill:'🛠️',BrowserLaunch:'🌐',BrowserNavigate:'🧭',
  BrowserGetContent:'📄',BrowserScreenshot:'📸',BrowserClick:'🖱️',BrowserFillInput:'✏️',BrowserExecuteJs:'⌨️',
  ExecuteAutomation:'🤖',CaptureScreen:'📸',OcrRecognizeScreen:'👁️',MouseClick:'🖱️',
  MouseDoubleClick:'🖱️',MouseRightClick:'🖱️',KeyboardType:'⌨️',KeyboardPress:'⌨️',
  ListInstalledApps:'📱',LaunchApplication:'🚀',
}

/** 可折叠区域组件 — 用于思考过程、工具步骤等可展开/收起的内容块 */
function CollapsibleSection({
  title,
  icon,
  badge,
  defaultOpen = false,
  children,
  headerClassName = '',
  contentClassName = '',
}: {
  title: string
  icon?: React.ReactNode
  badge?: React.ReactNode
  defaultOpen?: boolean
  children: React.ReactNode
  headerClassName?: string
  contentClassName?: string
}) {
  const [open, setOpen] = useState(defaultOpen)

  return (
    <div className="rounded-lg border border-dark-border/40 overflow-hidden">
      <button
        className={`w-full flex items-center gap-2 px-3 py-2 text-left hover:bg-white/[0.02] transition-colors ${headerClassName}`}
        onClick={() => setOpen(!open)}
      >
        {icon && <span className="shrink-0">{icon}</span>}
        <span className="text-xs font-medium text-dark-muted flex-1">{title}</span>
        {badge && <span className="shrink-0">{badge}</span>}
        <svg
          className={`w-3 h-3 text-dark-muted/60 transition-transform shrink-0 ${open ? 'rotate-180' : ''}`}
          fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={2}
        >
          <path strokeLinecap="round" strokeLinejoin="round" d="M19 9l-7 7-7-7" />
        </svg>
      </button>
      {open && (
        <div className={`border-t border-dark-border/30 ${contentClassName}`}>
          {children}
        </div>
      )}
    </div>
  )
}

/** 思考过程折叠块 — 展示AI的推理/思考文本，流式时显示脉冲动画 */
function ThinkingBlock({ thinkingText, isStreaming = false }: { thinkingText: string; isStreaming?: boolean }) {
  const { t } = useTranslation()
  if (!thinkingText) return null

  const preview = thinkingText.length > 120 ? thinkingText.slice(0, 120) + '...' : thinkingText

  return (
    <CollapsibleSection
      title={t('chatArea.thinkingProcess')}
      icon={
        <svg className="w-3.5 h-3.5 text-purple-400" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={2}>
          <path strokeLinecap="round" strokeLinejoin="round" d="M9.663 17h4.673M12 3v1m6.364 1.636l-.707.707M21 12h-1M4 12H3m3.343-5.657l-.707-.707m2.828 9.9a5 5 0 117.072 0l-.548.547A3.374 3.374 0 0014 18.469V19a2 2 0 11-4 0v-.531c0-.895-.356-1.754-.988-2.386l-.548-.547z" />
        </svg>
      }
      badge={
        isStreaming ? (
          <span className="flex items-center gap-1 text-[9px] text-purple-400">
            <span className="w-1.5 h-1.5 rounded-full bg-purple-400 animate-pulse" />
            {t('chatArea.thinkingActive')}
          </span>
        ) : undefined
      }
      headerClassName="bg-purple-500/5"
      contentClassName="bg-purple-500/[0.02]"
    >
      <div className="px-3 py-2.5">
        <pre className="text-[11px] text-purple-300/70 font-mono whitespace-pre-wrap break-words max-h-[300px] overflow-y-auto custom-scrollbar leading-relaxed">
          {thinkingText}
        </pre>
        {isStreaming && (
          <span className="inline-block w-1.5 h-3.5 bg-purple-400 animate-pulse ml-0.5 align-text-bottom" />
        )}
      </div>
    </CollapsibleSection>
  )
}

/** 单个工具执行步骤卡片 — 展示工具名称、输入/输出、执行耗时 */
function ToolStepCard({ detail, isLast }: { detail: ToolExecutionDetail; isLast?: boolean }) {
  const { t } = useTranslation()
  const [expanded, setExpanded] = useState(false)
  const isRunning = detail.status === 'running'
  const isCompleted = detail.status === 'completed'

  return (
    <div className={`rounded-lg border overflow-hidden transition-all ${
      isRunning ? 'border-blue-500/30 bg-blue-500/5' :
      isCompleted ? 'border-dark-border/30 bg-dark-bg/50' :
      'border-dark-border/30 bg-dark-bg/50'
    }`}>
      <button
        className="w-full flex items-center gap-2 px-2.5 py-2 text-left hover:bg-white/[0.02] transition-colors"
        onClick={() => setExpanded(!expanded)}
      >
        <span className="text-sm shrink-0">{TOOL_ICONS_MAP[detail.toolName] || '🔧'}</span>
        <code className="text-[11px] font-semibold text-primary-300 font-mono flex-1 truncate">{detail.toolName}</code>
        {detail.round && (
          <span className="text-[9px] text-dark-muted/50 font-mono shrink-0">R{detail.round}</span>
        )}
        <span className={`text-[9px] tabular-nums shrink-0 ${
          isRunning ? 'text-blue-400' : isCompleted ? 'text-green-400' : 'text-dark-muted'
        }`}>
          {isRunning ? t('chatArea.toolRunning') : isCompleted ? `${detail.durationMs}ms` : ''}
        </span>
        {isRunning && (
          <span className="w-1.5 h-1.5 rounded-full bg-blue-400 animate-pulse shrink-0" />
        )}
        {isCompleted && (
          <span className="text-green-400 text-[10px] shrink-0">✓</span>
        )}
        <svg
          className={`w-3 h-3 text-dark-muted/60 transition-transform shrink-0 ${expanded ? 'rotate-180' : ''}`}
          fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={2}
        >
          <path strokeLinecap="round" strokeLinejoin="round" d="M19 9l-7 7-7-7" />
        </svg>
      </button>
      {expanded && (
        <div className="border-t border-dark-border/20 px-2.5 pb-2.5 space-y-2">
          {detail.toolInput && (
            <div>
              <div className="text-[9px] text-dark-muted/60 uppercase tracking-wider mb-1 font-medium">
                {t('chatArea.toolCall.input')}
              </div>
              <pre className="p-2 rounded bg-dark-bg text-[10px] text-dark-muted font-mono max-h-32 overflow-auto whitespace-pre-wrap break-all leading-relaxed">
                {detail.toolInput.length > 2000 ? detail.toolInput.slice(0, 2000) + '...' : detail.toolInput}
              </pre>
            </div>
          )}
          {detail.toolResult && (
            <div>
              <div className="text-[9px] text-dark-muted/60 uppercase tracking-wider mb-1 font-medium">
                {t('chatArea.toolCall.output')}
              </div>
              <pre className="p-2 rounded bg-dark-bg text-[10px] text-dark-text/80 font-mono max-h-40 overflow-auto whitespace-pre-wrap break-all leading-relaxed">
                {detail.toolResult.length > 2000 ? detail.toolResult.slice(0, 2000) + '...' : detail.toolResult}
              </pre>
            </div>
          )}
        </div>
      )}
    </div>
  )
}

/** 工具步骤汇总块 — 折叠展示所有工具执行步骤，显示完成/运行计数和总耗时 */
function ToolStepsBlock({
  toolDetails,
  isStreaming = false,
}: {
  toolDetails: ToolExecutionDetail[]
  isStreaming?: boolean
}) {
  const { t } = useTranslation()
  if (!toolDetails || toolDetails.length === 0) return null

  const completedCount = toolDetails.filter(d => d.status === 'completed').length
  const runningCount = toolDetails.filter(d => d.status === 'running').length
  const totalDuration = toolDetails.filter(d => d.status === 'completed').reduce((sum, d) => sum + d.durationMs, 0)

  const rounds = new Set(toolDetails.map(d => d.round).filter(Boolean))
  const roundLabel = rounds.size > 1
    ? t('chatArea.toolRounds', { count: String(rounds.size) })
    : t('chatArea.toolSteps', { count: String(toolDetails.length) })

  return (
    <CollapsibleSection
      title={roundLabel}
      icon={
        <svg className="w-3.5 h-3.5 text-blue-400" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={2}>
          <path strokeLinecap="round" strokeLinejoin="round" d="M10.325 4.317c.426-1.756 2.924-1.756 3.35 0a1.724 1.724 0 002.573 1.066c1.543-.94 3.31.826 2.37 2.37a1.724 1.724 0 001.066 2.573c1.756.426 1.756 2.924 0 3.35a1.724 1.724 0 00-1.066 2.573c.94 1.543-.826 3.31-2.37 2.37a1.724 1.724 0 00-2.573 1.066c-.426 1.756-2.924 1.756-3.35 0a1.724 1.724 0 00-2.573-1.066c-1.543.94-3.31-.826-2.37-2.37a1.724 1.724 0 00-1.066-2.573c-1.756-.426-1.756-2.924 0-3.35a1.724 1.724 0 001.066-2.573c-.94-1.543.826-3.31 2.37-2.37.996.608 2.296.07 2.572-1.065z" />
          <path strokeLinecap="round" strokeLinejoin="round" d="M15 12a3 3 0 11-6 0 3 3 0 016 0z" />
        </svg>
      }
      badge={
        <span className="flex items-center gap-1.5 text-[9px]">
          {runningCount > 0 && (
            <span className="text-blue-400 flex items-center gap-0.5">
              <span className="w-1 h-1 rounded-full bg-blue-400 animate-pulse" />
              {runningCount} {t('chatArea.toolRunningShort')}
            </span>
          )}
          <span className="text-green-400">{completedCount}/{toolDetails.length}</span>
          {completedCount > 0 && totalDuration > 0 && (
            <span className="text-dark-muted/50">{(totalDuration / 1000).toFixed(1)}s</span>
          )}
        </span>
      }
      defaultOpen={isStreaming}
      headerClassName="bg-blue-500/5"
      contentClassName="bg-blue-500/[0.02]"
    >
      <div className="px-2.5 py-2 space-y-1.5">
        {toolDetails.map((detail, i) => (
          <ToolStepCard
            key={`${detail.toolName}-${detail.round}-${i}`}
            detail={detail}
            isLast={isStreaming && i === toolDetails.length - 1}
          />
        ))}
      </div>
    </CollapsibleSection>
  )
}

/** 未选择Agent时的空状态提示界面 */
function NoAgentScreen() {
  const { t } = useTranslation()
  return (
    <div className="flex-1 flex flex-col items-center justify-center p-6 select-none">
      <div className="w-16 h-16 rounded-2xl bg-dark-bg border border-dark-border flex items-center justify-center mb-5">
        <svg className="w-8 h-8 text-dark-muted/30" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={1.5}><path strokeLinecap="round" strokeLinejoin="round" d="M17 20h5v-2a3 3 0 00-5.356-1.857M17 20H7m10 0v-2c0-.656-.126-1.283-.356-1.857M7 20H2v-2a3 3 0 015.356-1.857M7 20v-2c0-.656.126-1.283.356-1.857m0 0a5.002 5.002 0 019.288 0M15 7a3 3 0 11-6 0 3 3 0 016 0zm6 3a2 2 0 11-4 0 2 2 0 014 0zM7 10a2 2 0 11-4 0 2 2 0 014 0z"/></svg>
      </div>
      <h2 className="text-lg font-bold text-dark-text mb-1.5">{t('chatArea.selectAgent')}</h2>
      <p className="text-sm text-dark-muted mb-4 max-w-md text-center leading-relaxed" dangerouslySetInnerHTML={{ __html: sanitizeHtml(t('chatArea.selectAgentDesc')) }} />
      <div className="flex items-center gap-2 px-4 py-2 rounded-xl bg-dark-bg border border-dark-border/50 text-xs text-dark-muted"><svg className="w-4 h-4 text-primary-400/50 shrink-0" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={2}><path strokeLinecap="round" strokeLinejoin="round" d="M13 16h-1v-4h-1m1-4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z"/></svg>{t('chatArea.clickHint')}</div>
    </div>
  )
}

/** 欢迎界面 — 无消息时展示快捷建议卡片 */
function WelcomeScreen({ onSuggestionClick }: { onSuggestionClick?: (text: string) => void }) {
  const { t } = useTranslation()
  const suggestions = [
    { icon: '💡', title: t('chatArea.suggestions.explainCode.title'), desc: t('chatArea.suggestions.explainCode.desc'), prompt: t('chatArea.suggestions.explainCode.prompt') },
    { icon: '🐛', title: t('chatArea.suggestions.debugIssue.title'), desc: t('chatArea.suggestions.debugIssue.desc'), prompt: t('chatArea.suggestions.debugIssue.prompt') },
    { icon: '✨', title: t('chatArea.suggestions.newFeature.title'), desc: t('chatArea.suggestions.newFeature.desc'), prompt: t('chatArea.suggestions.newFeature.prompt') },
    { icon: '📝', title: t('chatArea.suggestions.codeReview.title'), desc: t('chatArea.suggestions.codeReview.desc'), prompt: t('chatArea.suggestions.codeReview.prompt') },
    { icon: '🔧', title: t('chatArea.suggestions.toolCenter.title'), desc: t('chatArea.suggestions.toolCenter.desc'), prompt: t('chatArea.suggestions.toolCenter.prompt') },
  ]
  return (
    <div className="flex-1 flex flex-col items-center justify-center p-6 select-none">
      <div className="w-14 h-14 rounded-2xl bg-gradient-to-br from-primary-500 to-primary-700 flex items-center justify-center shadow-xl shadow-primary-500/25 mb-5">
        <svg className="w-8 h-8 text-white" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={1.5}><path strokeLinecap="round" strokeLinejoin="round" d="M8 10h.01M12 10h.01M16 10h.01M9 16H5a2 2 0 01-2-2V6a2 2 0 012-2h14a2 2 0 012 2v8a2 2 0 01-2 2h-5l-5 5v-5z"/></svg>
      </div>
      <h2 className="text-lg font-bold text-dark-text mb-1.5">{t('chatArea.welcomeTitle')}</h2>
      <p className="text-sm text-dark-muted mb-4 max-w-md text-center leading-relaxed" dangerouslySetInnerHTML={{ __html: sanitizeHtml(t('chatArea.welcomeDesc')) }} />
      <div className="grid grid-cols-2 gap-2.5 w-full max-w-sm">
        {suggestions.map((s, i) => (
          <button key={i} onClick={() => onSuggestionClick?.(s.prompt)} className="p-3 rounded-xl bg-dark-bg border border-dark-border hover:border-primary-500/40 hover:bg-primary-500/5 text-left transition-all duration-150 group cursor-pointer">
            <span className="text-base mb-1 block">{s.icon}</span>
            <span className="text-xs font-medium text-dark-text block group-hover:text-primary-400 transition-colors">{s.title}</span>
            <span className="text-[10px] text-dark-muted mt-0.5 block leading-snug">{s.desc}</span>
          </button>
        ))}
      </div>
    </div>
  )
}

/** 消息气泡组件 — 渲染单条用户/AI消息，含思考过程、工具步骤、复制按钮 */
function MessageBubble({ message }: { message: Message }) {
  const { t } = useTranslation()
  const [showActions, setShowActions] = useState(false)
  const [copied, setCopied] = useState(false)
  const isUser = message.role === 'user'
  const { toolCalls } = parseToolCalls(message.content)
  const hasToolCalls = toolCalls.length > 0
  const hasThinking = !!(message.thinkingText && message.thinkingText.trim())
  const hasToolDetails = !!(message.toolExecutionDetails && message.toolExecutionDetails.length > 0)
  const hasExtra = hasThinking || hasToolDetails || hasToolCalls

  const signalConfig: Record<string, { icon: string; label: string; color: string }> = {
    response_complete: { icon: '✓', label: t('chatArea.responseComplete', '完成'), color: 'text-green-400 bg-green-400/10' },
    input_required: { icon: '✋', label: t('chatArea.inputRequired', '需要输入'), color: 'text-amber-400 bg-amber-400/10' },
    confirm_required: { icon: '⚠️', label: t('chatArea.confirmRequired', '需要确认'), color: 'text-red-400 bg-red-400/10' },
    task_in_progress: { icon: '⏳', label: t('chatArea.taskInProgress', '进行中'), color: 'text-blue-400 bg-blue-400/10' },
  }
  const signal = message.signalStatus ? signalConfig[message.signalStatus] : null

  const handleCopy = async () => {
    try { await navigator.clipboard.writeText(message.content); setCopied(true); setTimeout(() => setCopied(false), 2000) } catch (e) { console.error('[ChatArea:copy]', e) }
  }

  return (
    <div className={`group flex ${isUser ? 'justify-end' : 'justify-start'} animate-fade-in`} onMouseEnter={() => setShowActions(true)} onMouseLeave={() => setShowActions(false)}>
      <div className={`max-w-[85%] ${
        isUser ? 'bg-primary-600 text-white rounded-2xl rounded-tr-sm' :
        message.isError ? 'bg-red-900/30 text-red-300 border border-red-800/30 rounded-2xl rounded-tl-sm' : 'bg-dark-bg text-dark-text border border-dark-border rounded-2xl rounded-tl-sm'
      } px-4 py-3 shadow-sm transition-all duration-150`}>
        <SafeMarkdown content={message.content} className="prose prose-invert prose-sm max-w-none [&_pre]:max-w-[calc(100vw-220px)] [&_pre]:overflow-x-auto [&_code]:text-[13px]" />

        {signal && !isUser && (
          <div className={`mt-2 px-2 py-1 rounded-md text-[11px] font-medium inline-flex items-center gap-1 ${signal.color}`}>
            <span>{signal.icon}</span>
            <span>{signal.label}</span>
          </div>
        )}

        {hasExtra && (
          <div className="mt-3 pt-3 border-t border-white/5 space-y-2">
            {hasThinking && (
              <ThinkingBlock thinkingText={message.thinkingText!} />
            )}
            {hasToolDetails && (
              <ToolStepsBlock toolDetails={message.toolExecutionDetails!} />
            )}
            {hasToolCalls && !hasToolDetails && (
              <div className="space-y-2">
                {toolCalls.map(tc => <ToolCallCard key={tc.id} toolCall={tc} t={t} />)}
              </div>
            )}
          </div>
        )}

        {!isUser && showActions && (
          <div className="flex items-center justify-end mt-2 pt-1 border-t border-white/5">
            <button onClick={handleCopy} className="text-[10px] text-dark-muted hover:text-primary-400 transition-colors flex items-center gap-1">
              {copied ? (<><svg className="w-3 h-3" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={2}><path strokeLinecap="round" strokeLinejoin="round" d="M5 13l4 4L19 7"/></svg>{t('chatArea.copied')}</>) : (<><svg className="w-3 h-3" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={2}><path strokeLinecap="round" strokeLinejoin="round" d="M8 16H6a2 2 0 01-2-2V6a2 2 0 012-2h8a2 2 0 012 2v2m-6 12h8a2 2 0 002-2v-8a2 2 0 00-2-2h-8a2 2 0 00-2 2v8a2 2 0 002 2z"/></svg>{t('chatArea.copy')}</>)}
            </button>
          </div>
        )}
      </div>
    </div>
  )
}

/** 工具执行结果卡片 — 解析JSON格式的工具消息，展示工具名/输入/输出 */
function ToolExecutionCard({ message }: { message: Message }) {
  const { t } = useTranslation()
  const [expanded, setExpanded] = useState(false)
  let toolData: { name: string; input: Record<string, unknown>; result: string; duration_ms: number } | null = null
  try { toolData = JSON.parse(message.content) } catch { return null }
  if (!toolData) return null

  return (
    <div className="flex justify-start animate-fade-in">
      <div className="max-w-[85%] bg-dark-bg/80 border border-dark-border/60 rounded-xl overflow-hidden shadow-sm">
        <div className="flex items-center gap-2 px-3 py-2 bg-dark-surface/50 border-b border-dark-border/40 cursor-pointer" onClick={() => setExpanded(!expanded)}>
          <span className="text-base">{TOOL_ICONS_MAP[toolData.name] || '🔧'}</span>
          <code className="text-xs font-semibold text-primary-300 font-mono">{toolData.name}</code>
          <span className="text-[10px] text-dark-muted ml-auto">{toolData.duration_ms}ms</span>
          <svg className={`w-3 h-3 text-dark-muted transition-transform ${expanded ? 'rotate-180' : ''}`} fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={2}><path strokeLinecap="round" strokeLinejoin="round" d="M19 9l-7 7-7-7"/></svg>
        </div>
        <div className="px-3 py-1.5 space-y-1">
          {toolData.input && Object.keys(toolData.input).length > 0 && (
            <div className="flex flex-wrap gap-1">
              {Object.entries(toolData.input).slice(0, 4).map(([k, v]) => (
                <span key={k} className="text-[9px] px-1.5 py-0.5 rounded bg-dark-surface text-dark-muted font-mono max-w-[140px] truncate inline-block">{k}: {typeof v === 'string' ? v.slice(0, 20) : JSON.stringify(v).slice(0, 20)}</span>
              ))}
            </div>
          )}
        </div>
        {expanded && (
          <div className="border-t border-dark-border/30">
            <div className="text-[10px] px-3 py-1 text-dark-muted/50 uppercase tracking-wider">Output ({toolData.result.length} chars)</div>
            <pre className="px-3 py-2 text-[11px] text-dark-muted/80 font-mono whitespace-pre-wrap break-all max-h-[300px] overflow-y-auto custom-scrollbar leading-relaxed">
              {toolData.result.length > 2000 ? toolData.result.slice(0, 2000) + `\n${t('chatArea.toolCall.truncated')}` : toolData.result}
            </pre>
          </div>
        )}
      </div>
    </div>
  )
}

/** 流式输出气泡 — 实时展示AI回复文本、思考过程和工具执行进度 */
function StreamingBubble({
  text,
  thinkingText,
  tools,
  toolDetails,
}: {
  text: string
  thinkingText?: string
  tools?: Array<{ toolName: string; durationMs: number; index: number; total: number }>
  toolDetails?: ToolExecutionDetail[]
}) {
  const { t } = useTranslation()
  const hasContent = text.trim().length > 0
  const hasThinking = !!(thinkingText && thinkingText.trim())
  const hasTools = tools && tools.length > 0
  const hasToolDetails = toolDetails && toolDetails.length > 0
  const isThinking = !hasContent && !hasThinking

  return (
    <div className="flex justify-start">
      <div className="bg-dark-bg border border-primary-500/20 rounded-2xl rounded-tl-sm px-4 py-3 shadow-sm max-w-[85%] min-h-[32px]">
        {isThinking ? (
          <div className="flex items-center gap-2">
            <div className="flex gap-1">
              <span className="w-1.5 h-1.5 rounded-full bg-primary-400 animate-bounce" style={{ animationDelay: '0ms' }}></span>
              <span className="w-1.5 h-1.5 rounded-full bg-primary-400 animate-bounce" style={{ animationDelay: '150ms' }}></span>
              <span className="w-1.5 h-1.5 rounded-full bg-primary-400 animate-bounce" style={{ animationDelay: '300ms' }}></span>
            </div>
            <span className="text-xs text-dark-muted ml-1">{t('chatArea.thinking')}</span>
          </div>
        ) : (
          <div className="space-y-3">
            {hasThinking && (
              <ThinkingBlock thinkingText={thinkingText!} isStreaming={true} />
            )}
            {hasToolDetails && (
              <ToolStepsBlock toolDetails={toolDetails!} isStreaming={true} />
            )}
            {!hasToolDetails && hasTools && (
              <div className="space-y-1.5">
                <div className="text-[10px] text-primary-400/70 font-medium uppercase tracking-wider flex items-center gap-1.5">
                  <svg className="w-3 h-3 animate-spin" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={2}><path strokeLinecap="round" strokeLinejoin="round" d="M4 4v5h.582m15.356 2A8.001 8.001 0 004.582 9m0 0H9m11 11v-5h-.581m0 0a8.003 8.003 0 01-15.357-2m15.357 2H15"/></svg>
                  {t('chatArea.toolsLabel')} ({tools![tools!.length - 1].index}/{tools![tools!.length - 1].total})
                </div>
                {tools!.map((tool, i) => {
                  const isLast = i === tools!.length - 1
                  return (
                    <div key={i} className={`flex items-center gap-2 px-2.5 py-1.5 rounded-lg text-xs transition-all ${isLast ? 'bg-primary-500/10 border border-primary-500/20' : 'bg-dark-surface/50'}`}>
                      <span>{TOOL_ICONS_MAP[tool.toolName] || '🔧'}</span>
                      <code className="font-mono font-medium flex-1 truncate">{tool.toolName}</code>
                      <span className={`text-[9px] tabular-nums ${isLast ? 'text-primary-400' : 'text-dark-muted'}`}>
                        {isLast ? `${tool.durationMs}ms` : '✓'}
                      </span>
                      {isLast && (<span className="w-1.5 h-1.5 rounded-full bg-primary-400 animate-pulse"></span>)}
                    </div>
                  )
                })}
              </div>
            )}
            {hasContent && (
              <>
                <SafeMarkdown content={text} className="prose prose-invert prose-sm max-w-none [&_pre]:max-w-[calc(100vw-220px)] [&_pre]:overflow-x-auto [&_code]:text-[13px]" />
                {!hasToolDetails && hasTools && (
                  <div className="pt-2 border-t border-white/5 flex flex-wrap gap-1.5">
                    {tools!.map((ti, i) => (
                      <span key={i} className="text-[9px] px-1.5 py-0.5 rounded-full bg-dark-surface text-dark-muted font-mono flex items-center gap-1">
                        {TOOL_ICONS_MAP[ti.toolName] || '🔧'}{ti.toolName} <span className="text-dark-muted/50">{ti.durationMs}ms</span>
                      </span>
                    ))}
                  </div>
                )}
                <span className="inline-block w-1.5 h-4 bg-primary-400 animate-pulse ml-0.5 align-middle"></span>
              </>
            )}
            {!hasContent && !hasThinking && !hasToolDetails && hasTools && (
              <div className="flex items-center gap-2 pt-1">
                <div className="flex gap-1">
                  <span className="w-1.5 h-1.5 rounded-full bg-primary-400 animate-bounce" style={{ animationDelay: '0ms' }}></span>
                  <span className="w-1.5 h-1.5 rounded-full bg-primary-400 animate-bounce" style={{ animationDelay: '150ms' }}></span>
                  <span className="w-1.5 h-1.5 rounded-full bg-primary-400 animate-bounce" style={{ animationDelay: '300ms' }}></span>
                </div>
                <span className="text-xs text-dark-muted ml-1">{t('chatArea.processingResponse')}</span>
              </div>
            )}
          </div>
        )}
      </div>
    </div>
  )
}

/** 解析消息内容中的 <tool_call/> 和 <tool_result/> XML标签，提取工具调用信息 */
function parseToolCalls(content: string): { text: string; toolCalls: ParsedToolCall[] } {
  const toolCalls: ParsedToolCall[] = []
  let cleaned = content
  const toolUseRegex = /<tool_call[^>]*>\s*<tool_name>([\s\S]*?)<\/tool_name>\s*<tool_input>([\s\S]*?)<\/tool_input>\s*<\/tool_call>/gi
  let toolMatch: RegExpExecArray | null
  while ((toolMatch = toolUseRegex.exec(content)) !== null) {
    toolCalls.push({ id: `tc_${toolCalls.length}`, toolName: toolMatch[1].trim(), inputStr: toolMatch[2].trim(), status: 'calling' })
  }
  const toolResultRegex = /<tool_result[^>]*>\s*<tool_name>([\s\S]*?)<\/tool_name>\s*(?:<output>([\s\S]*?)<\/output>)?\s*<\/tool_result>/gi
  while ((toolMatch = toolResultRegex.exec(content)) !== null) {
    const m = toolMatch
    const existingCall = toolCalls.find(tc => tc.toolName === m[1].trim())
    if (existingCall) { existingCall.status = 'success'; existingCall.outputStr = m[2]?.trim() || '' }
    else { toolCalls.push({ id: `tr_${toolCalls.length}`, toolName: m[1].trim(), inputStr: '', outputStr: m[2]?.trim() || '', status: 'success' }) }
  }
  return { text: cleaned, toolCalls }
}

/** 解析后的工具调用数据结构 */
interface ParsedToolCall {
  id: string
  toolName: string
  inputStr: string
  outputStr?: string
  status: 'calling' | 'success' | 'error'
}

/** 工具调用卡片 — 展示单个工具调用的状态（调用中/成功/失败）和输入输出 */
function ToolCallCard({ toolCall, t }: { toolCall: ParsedToolCall; t: (key: string, params?: Record<string, unknown>) => string }) {
  const [expanded, setExpanded] = useState(false)
  const statusConfig = {
    calling: { color: 'text-blue-400 bg-blue-500/10 border-blue-500/20', icon: '⏳', label: t('chatArea.toolCall.calling') },
    success: { color: 'text-green-400 bg-green-500/10 border-green-500/20', icon: '✅', label: t('chatArea.toolCall.done') },
    error:   { color: 'text-red-400 bg-red-500/10 border-red-500/20', icon: '❌', label: t('chatArea.toolCall.error') },
  }
  const cfg = statusConfig[toolCall.status]
  return (
    <div className={`rounded-lg border ${cfg.color.split(' ')[2]} overflow-hidden`}>
      <div className="flex items-center gap-2 px-3 py-2 cursor-pointer" onClick={() => setExpanded(!expanded)}>
        <span className="text-sm">{cfg.icon}</span>
        <code className="text-xs font-semibold text-primary-300 flex-1">{toolCall.toolName}</code>
        <span className={`text-[10px] px-1.5 py-0.5 rounded ${cfg.color.split(' ').slice(0, 2).join(' ')}`}>{cfg.label}</span>
        <svg className={`w-3 h-3 text-dark-muted transition-transform ${expanded ? 'rotate-180' : ''}`} fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={2}><path strokeLinecap="round" strokeLinejoin="round" d="M19 9l-7 7-7-7" /></svg>
      </div>
      {expanded && (
        <div className="px-3 pb-2.5 space-y-2 border-t border-white/5">
          {toolCall.inputStr && (<div><div className="text-[10px] text-dark-muted mb-1 uppercase tracking-wider">{t('chatArea.toolCall.input')}</div><pre className="p-2 rounded bg-dark-bg text-[11px] text-dark-muted font-mono max-h-32 overflow-auto whitespace-pre-wrap break-all">{toolCall.inputStr.slice(0, 1000)}</pre></div>)}
          {toolCall.outputStr && (<div><div className="text-[10px] text-dark-muted mb-1 uppercase tracking-wider">{t('chatArea.toolCall.output')}</div><pre className="p-2 rounded bg-dark-bg text-[11px] text-dark-text font-mono max-h-40 overflow-auto whitespace-pre-wrap break-all">{toolCall.outputStr.slice(0, 2000)}</pre><div className="text-[9px] text-dark-muted/40 mt-1">{t('chatArea.toolCall.outputChars', { count: toolCall.outputStr.length })}</div></div>)}
        </div>
      )}
    </div>
  )
}

/** Markdown自定义渲染组件映射 — 代码高亮、表格样式、链接处理等 */
const markdownComponents = {
  code({ className, children, ...props }: MarkdownComponentProps) {
    const match = /language-(\w+)/.exec(className || '')
    const codeString = String(children).replace(/\n$/, '')
    const isInline = !match
    if (!isInline && match) {
      return (
        <div className="relative my-3 rounded-lg overflow-hidden border border-dark-border">
          <div className="flex items-center justify-between px-3 py-1.5 bg-dark-surface/80 border-b border-dark-border">
            <span className="text-[10px] font-mono text-dark-muted uppercase tracking-wider">{match[1]}</span>
            <button onClick={(e) => { e.preventDefault(); navigator.clipboard.writeText(codeString) }} className="text-[10px] text-dark-muted hover:text-primary-400 transition-colors">Copy</button>
          </div>
          <SyntaxHighlighter style={oneDark as Record<string, React.CSSProperties>} language={match[1]} PreTag="div" showLineNumbers customStyle={{ margin: 0 }}>{codeString}</SyntaxHighlighter>
        </div>
      )
    }
    return <code className={`${className || ''} px-1.5 py-0.5 rounded bg-dark-surface/70 text-primary-300 text-[13px]`} {...props}>{children}</code>
  },
  table({ children, ...props }: MarkdownComponentProps) {
    return <div className="my-3 overflow-x-auto rounded-lg border border-dark-border"><table className="min-w-full divide-y divide-dark-border text-sm" {...props}>{children}</table></div>
  },
  th({ children, ...props }: MarkdownComponentProps) {
    return <th className="px-3 py-2 bg-dark-surface/50 text-left text-xs font-semibold text-dark-muted uppercase tracking-wider" {...props}>{children}</th>
  },
  td({ children, ...props }: MarkdownComponentProps) {
    return <td className="px-3 py-2 border-t border-dark-border" {...props}>{children}</td>
  },
  a({ href, children, ...props }: MarkdownComponentProps) {
    const url = href || ''
    const isExternal = url.startsWith('http://') || url.startsWith('https://')
    if (!isExternal) return <a href={url} className="text-primary-400 hover:text-primary-300 underline underline-offset-2 decoration-primary-500/40 hover:decoration-primary-400 transition-colors" target="_blank" rel="noopener noreferrer" {...props}>{children}</a>
    return (
      <span className="inline-flex items-center gap-1 group/link relative">
        <a href={url} className="text-primary-400 hover:text-primary-300 underline underline-offset-2 decoration-primary-500/40 hover:decoration-primary-400 transition-colors max-w-[280px] truncate inline-block align-bottom break-all" target="_blank" rel="noopener noreferrer" onClick={(e) => e.stopPropagation()} {...props}>
          {children || url}
        </a>
        <svg className="w-3 h-3 text-primary-400/50 shrink-0 opacity-0 group-hover/link:opacity-100 transition-opacity" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={2}><path strokeLinecap="round" strokeLinejoin="round" d="M10 6H6a2 2 0 00-2 2v10a2 2 0 002 2h10a2 2 0 002-2v-4M14 4h6m0 0v6m0-6L10 14"/></svg>
      </span>
    )
  },
  p({ children, ...props }: MarkdownComponentProps) {
    return <p className="mb-3 last:mb-0 leading-relaxed" {...props}>{children}</p>
  },
  ul({ children, ...props }: MarkdownComponentProps) {
    return <ul className="list-disc list-inside mb-3 space-y-1 marker:text-primary-400" {...props}>{children}</ul>
  },
  ol({ children, ...props }: MarkdownComponentProps) {
    return <ol className="list-decimal list-inside mb-3 space-y-1 marker:text-primary-400" {...props}>{children}</ol>
  },
  li({ children, ...props }: MarkdownComponentProps) {
    return <li className="pl-1" {...props}>{children}</li>
  },
  h1({ children, ...props }: MarkdownComponentProps) { return <h1 className="text-xl font-bold mt-4 mb-2 text-dark-text" {...props}>{children}</h1> },
  h2({ children, ...props }: MarkdownComponentProps) { return <h2 className="text-lg font-bold mt-4 mb-2 text-dark-text" {...props}>{children}</h2> },
  h3({ children, ...props }: MarkdownComponentProps) { return <h3 className="text-base font-bold mt-3 mb-1.5 text-dark-text" {...props}>{children}</h3> },
  strong({ children, ...props }: MarkdownComponentProps) { return <strong className="font-bold text-white" {...props}>{children}</strong> },
  blockquote({ children, ...props }: MarkdownComponentProps) { return <blockquote className="border-l-3 border-primary-500/40 pl-3 my-3 text-dark-text/80 italic" {...props}>{children}</blockquote> },
}

/** Markdown渲染错误边界 — 捕获渲染异常时降级为纯文本显示 */
class MarkdownErrorBoundary extends React.Component<{ children: React.ReactNode; fallback: string }, { hasError: boolean }> {
  constructor(props: { children: React.ReactNode; fallback: string }) {
    super(props)
    this.state = { hasError: false }
  }
  static getDerivedStateFromError() {
    return { hasError: true }
  }
  componentDidCatch(error: Error) {
    console.warn('Markdown ErrorBoundary caught:', error.message?.slice(0, 200))
  }
  render() {
    if (this.state.hasError) {
      return <pre className="whitespace-pre-wrap break-words text-sm text-dark-text/90 leading-relaxed">{this.props.fallback}</pre>
    }
    return this.props.children
  }
}

/** 安全Markdown渲染 — 包裹ErrorBoundary，渲染失败时回退为纯文本 */
function SafeMarkdown({ content, className = '' }: { content: string; className?: string }) {
  const [error, setError] = useState<Error | null>(null)
  const [key, setKey] = useState(0)

  useEffect(() => { setError(null); setKey(k => k + 1) }, [content])

  if (error) {
    return <pre className={`whitespace-pre-wrap break-words text-sm text-dark-text/90 leading-relaxed ${className}`}>{content}</pre>
  }

  return (
    <MarkdownErrorBoundary fallback={content}>
      <div className={className}>
        <Markdown
          key={key}
          remarkPlugins={[remarkGfm]}
          components={markdownComponents as any}
        >
          {content}
        </Markdown>
      </div>
    </MarkdownErrorBoundary>
  )
}

/** 聊天区域主组件 — 消息列表、流式气泡、确认对话框的容器 */
function ChatArea({
  messages, isLoading, conversationTitle, messagesEndRef, conversationId, activeAgentId,
  showToolExecutions = true, streamingText, thinkingText, multiAgentMessages,
  toolExecutions, toolExecutionDetails, pendingConfirmation, onConfirm, onDismissConfirm, onSuggestionClick,
}: ChatAreaProps) {
  const { t } = useTranslation()
  const chatContainerRef = useRef<HTMLDivElement>(null)

  useEffect(() => {
    if (chatContainerRef.current) chatContainerRef.current.scrollTop = chatContainerRef.current.scrollHeight
  }, [messages, streamingText, thinkingText, multiAgentMessages])

  const scrollToBottom = useCallback(() => {
    if (chatContainerRef.current) chatContainerRef.current.scrollTop = chatContainerRef.current.scrollHeight
  }, [])

  return (
    <div className="flex-1 flex flex-col min-h-0 relative" role="region" aria-label={t('chatArea.chatRegion', 'Chat area')}>
      <div className="shrink-0 px-4 py-2 bg-dark-surface/50 border-b border-dark-border flex items-center justify-between">
        <div className="flex items-center gap-2 min-w-0">
          {conversationTitle && (<h3 className="text-sm font-semibold text-dark-text truncate">{conversationTitle}</h3>)}
          {conversationId && (<span className="text-[9px] font-mono text-dark-muted/40 shrink-0">{conversationId.slice(0, 8)}</span>)}
          {!activeAgentId && (<span className="text-[10px] px-2 py-0.5 rounded-full bg-yellow-500/10 text-yellow-400/80 border border-yellow-500/20">{t('chatArea.noAgentSelected')}</span>)}
        </div>
      </div>

      <div ref={chatContainerRef} className="flex-1 min-h-0 overflow-y-auto">
        {!activeAgentId ? (
          <NoAgentScreen />
        ) : messages.length === 0 ? (
          <WelcomeScreen />
        ) : (
          <div className="space-y-4 max-w-5xl mx-auto py-4" role="log" aria-label={t('chatArea.messageLog', 'Message log')}>
            {messages.map(msg => (
              msg.role === 'tool' && showToolExecutions
                ? <ToolExecutionCard key={msg.id} message={msg} />
                : msg.role !== 'tool' ? <MessageBubble key={msg.id} message={msg} /> : null
            ))}

            {multiAgentMessages && multiAgentMessages.length > 0 && (
              <div className="space-y-4 mt-2">
                {multiAgentMessages.map((maMsg) => (
                  <MultiAgentMessage key={maMsg.sessionId} mainResponse={maMsg.mainResponse} subAgents={maMsg.subAgents} status={maMsg.status} timestamp={maMsg.timestamp} isStreaming={maMsg.status === 'executing' || maMsg.status === 'planning' || maMsg.status === 'aggregating'} streamingText={maMsg.streamingText} streamingAgentId={maMsg.streamingAgentId} />
                ))}
              </div>
            )}

            {(streamingText || thinkingText || (toolExecutions && toolExecutions.length > 0)) && (
              <StreamingBubble
                text={streamingText || ''}
                thinkingText={thinkingText}
                tools={toolExecutions}
                toolDetails={toolExecutionDetails}
              />
            )}

            {isLoading && !streamingText && !thinkingText && !(toolExecutions && toolExecutions.length > 0) && (
              <div className="flex justify-start animate-fade-in">
                <div className="bg-dark-bg border border-dark-border rounded-2xl rounded-tl-sm px-4 py-3 shadow-sm">
                  <div className="flex items-center gap-2">
                    <div className="flex gap-1">
                      <span className="w-1.5 h-1.5 rounded-full bg-primary-400 animate-bounce" style={{ animationDelay: '0ms' }}></span>
                      <span className="w-1.5 h-1.5 rounded-full bg-primary-400 animate-bounce" style={{ animationDelay: '150ms' }}></span>
                      <span className="w-1.5 h-1.5 rounded-full bg-primary-400 animate-bounce" style={{ animationDelay: '300ms' }}></span>
                    </div>
                    <span className="text-xs text-dark-muted ml-1">{t('chatArea.processingText')}</span>
                  </div>
                </div>
              </div>
            )}

            {pendingConfirmation && onConfirm && (
              <div className="flex justify-start animate-fade-in mt-2">
                <div className="max-w-[85%] bg-gradient-to-br from-primary-900/40 to-primary-800/20 border border-primary-500/30 rounded-2xl rounded-tl-sm px-4 py-3 shadow-sm">
                  <div className="flex items-center gap-2 mb-2.5">
                    <span className="w-2 h-2 rounded-full bg-primary-400 animate-pulse"></span>
                    <span className="text-xs font-medium text-primary-300">{t('chatArea.waitingConfirm')}</span>
                  </div>
                  {pendingConfirmation.prompt.length > 0 && (
                    <p className="text-xs text-dark-text/70 mb-3 italic leading-relaxed">"{pendingConfirmation.prompt.slice(-120)}"</p>
                  )}
                  <div className="flex items-center gap-2 mb-3">
                    <input
                      type="text"
                      placeholder={t('chatArea.typeReplyPlaceholder')}
                      className="flex-1 px-3 py-1.5 rounded-lg bg-dark-bg/60 border border-dark-border/50 text-xs text-dark-text placeholder-dark-muted/40 focus:outline-none focus:border-primary-500/40 focus:ring-1 focus:ring-primary-500/20"
                      onKeyDown={(e) => {
                        if (e.key === 'Enter' && !e.shiftKey) {
                          e.preventDefault()
                          const value = (e.target as HTMLInputElement).value.trim()
                          if (value) onConfirm(value)
                        }
                      }}
                      id="confirmation-input"
                    />
                    <button
                      onClick={() => {
                        const input = document.getElementById('confirmation-input') as HTMLInputElement
                        const value = input?.value?.trim()
                        if (value) onConfirm(value)
                      }}
                      className="px-3 py-1.5 rounded-lg text-xs font-medium transition-all duration-150 bg-primary-600 hover:bg-primary-500 text-white hover:shadow-lg hover:shadow-primary-500/20 active:scale-95 shrink-0"
                    >
                      {t('chatArea.submitReply')}
                    </button>
                  </div>
                  <div className="flex flex-wrap gap-2">
                    {pendingConfirmation.options.map((opt) => (
                      <button
                        key={opt.value}
                        onClick={() => onConfirm(opt.value)}
                        className="px-3.5 py-1.5 rounded-lg text-xs font-medium transition-all duration-150 bg-dark-surface text-dark-muted hover:text-dark-text border border-dark-border hover:border-primary-500/30 active:scale-95"
                      >
                        {opt.label}
                      </button>
                    ))}
                    {onDismissConfirm && (
                      <button
                        onClick={onDismissConfirm}
                        className="px-3.5 py-1.5 rounded-lg text-xs font-medium transition-all duration-150 bg-dark-surface text-dark-muted hover:text-dark-text border border-dark-border hover:border-dark-border/80 active:scale-95"
                      >
                        {t('chatArea.dismiss')}
                      </button>
                    )}
                  </div>
                </div>
              </div>
            )}

            <div ref={messagesEndRef} />
          </div>
        )}
      </div>
    </div>
  )
}

export default ChatArea
