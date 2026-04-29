// Claw Desktop - 多Agent消息组件 - 展示多Agent协作消息、子Agent状态和结果
import { useState, useEffect } from 'react'
import { useTranslation } from 'react-i18next'
import ReactMarkdown from 'react-markdown'
import remarkGfm from 'remark-gfm'
import type {
  SubAgentResult,
  SubAgentStatus,
  MultiAgentSessionStatus,
} from '../../multiagent/types'
import { SubAgentStatus as SAS, MultiAgentSessionStatus as MASS } from '../../multiagent/types'
import { rehypeAutolink } from '../../utils/rehypeAutolink'

function SafeMarkdown({ content, className = '' }: { content: string; className?: string }) {
  const [error, setError] = useState<Error | null>(null)
  const [key, setKey] = useState(0)

  if (error) {
    return <pre className={`whitespace-pre-wrap break-words text-sm leading-relaxed ${className}`}>{content}</pre>
  }

  return (
    <div className={className}>
      <ReactMarkdown
        key={key}
        remarkPlugins={[remarkGfm]}
        rehypePlugins={[rehypeAutolink]}
      >
        {content}
      </ReactMarkdown>
    </div>
  )
}

interface MultiAgentMessageProps {
  mainResponse: string
  subAgents: SubAgentResult[]
  status: MultiAgentSessionStatus
  timestamp: number
  isStreaming?: boolean
  streamingText?: string
  streamingAgentId?: string
}

function StatusBadge({ status }: { status: SubAgentStatus | MultiAgentSessionStatus }) {
  const { t } = useTranslation()
  const config: Record<string, { color: string; bg: string; border: string; icon: string; labelKey: string }> = {
    [SAS.PENDING]:     { color: 'text-gray-400', bg: 'bg-gray-500/10', border: 'border-gray-500/20', icon: '⏳', labelKey: 'multiAgent.status.pending' },
    [SAS.RUNNING]:     { color: 'text-blue-400', bg: 'bg-blue-500/10', border: 'border-blue-500/20', icon: '🔄', labelKey: 'multiAgent.status.running' },
    [MASS.IDLE]:       { color: 'text-gray-400', bg: 'bg-gray-500/10', border: 'border-gray-500/20', icon: '💤', labelKey: 'multiAgent.status.idle' },
    [MASS.PLANNING]:   { color: 'text-purple-400', bg: 'bg-purple-500/10', border: 'border-purple-500/20', icon: '📋', labelKey: 'multiAgent.status.planning' },
    [MASS.EXECUTING]:  { color: 'text-blue-400', bg: 'bg-blue-500/10', border: 'border-blue-500/20', icon: '⚡', labelKey: 'multiAgent.status.executing' },
    [MASS.AGGREGATING]:{ color: 'text-cyan-400', bg: 'bg-cyan-500/10', border: 'border-cyan-500/20', icon: '�', labelKey: 'multiAgent.status.aggregating' },
    [SAS.COMPLETED]:   { color: 'text-green-400', bg: 'bg-green-500/10', border: 'border-green-500/20', icon: '✅', labelKey: 'multiAgent.status.completed' },
    [SAS.FAILED]:      { color: 'text-red-400', bg: 'bg-red-500/10', border: 'border-red-500/20', icon: '❌', labelKey: 'multiAgent.status.failed' },
    [SAS.TIMEOUT]:     { color: 'text-orange-400', bg: 'bg-orange-500/10', border: 'border-orange-500/20', icon: '⏱️', labelKey: 'multiAgent.status.timeout' },
    [SAS.WAITING_INPUT]: { color: 'text-yellow-400', bg: 'bg-yellow-500/10', border: 'border-yellow-500/20', icon: '💬', labelKey: 'multiAgent.status.waitingInput' },
  }

  const cfg = config[status] || config[SAS.PENDING]

  return (
    <span className={`inline-flex items-center gap-1 text-[10px] px-2 py-0.5 rounded-full font-medium ${cfg.color} ${cfg.bg} ${cfg.border} border`}>
      <span>{cfg.icon}</span>
      <span>{t(cfg.labelKey)}</span>
    </span>
  )
}

function SubAgentCard({ agent, isStreamingActive }: { agent: SubAgentResult; isStreamingActive?: boolean }) {
  const { t } = useTranslation()
  const [expanded, setExpanded] = useState(false)
  const isActive = agent.status === SAS.RUNNING || agent.status === SAS.PENDING

  const displayText = agent.streamingText || agent.result || agent.rawOutput

  return (
    <div className={`rounded-lg border overflow-hidden transition-all ${
      agent.status === SAS.COMPLETED ? 'border-green-500/20 bg-green-500/5' :
      agent.status === SAS.FAILED || agent.status === SAS.TIMEOUT ? 'border-red-500/20 bg-red-500/5' :
      isActive ? 'border-blue-500/30 bg-blue-500/5 animate-pulse-slow' :
      'border-dark-border/50 bg-dark-bg/50'
    }`}>
      <div
        className="flex items-center gap-2.5 px-3 py-2.5 cursor-pointer hover:bg-white/[0.03] transition-colors"
        onClick={() => setExpanded(!expanded)}
      >
        <div className="relative">
          <div className={`w-8 h-8 rounded-lg flex items-center justify-center text-sm border ${
            agent.status === SAS.COMPLETED ? 'bg-green-500/10 border-green-500/20' :
            agent.status === SAS.FAILED || agent.status === SAS.TIMEOUT ? 'bg-red-500/10 border-red-500/20' :
            'bg-dark-surface border-dark-border/30'
          }`}>
            {getAgentIcon(agent.agentId)}
          </div>
          {isActive && (
            <span className="absolute -top-0.5 -right-0.5 w-2.5 h-2.5 bg-blue-400 rounded-full border-2 border-dark-bg animate-ping" />
          )}
        </div>

        <div className="flex-1 min-w-0">
          <div className="flex items-center gap-2">
            <span className="text-xs font-semibold text-dark-text">{agent.agentName}</span>
            <StatusBadge status={agent.status} />
          </div>
          {agent.durationMs ? (
            <span className="text-[9px] text-dark-muted font-mono">{(agent.durationMs / 1000).toFixed(1)}s</span>
          ) : isActive && agent.streamingText ? (
            <span className="text-[9px] text-primary-400 font-mono">{t('multiAgent.generating')}</span>
          ) : null}
        </div>

        <svg className={`w-3.5 h-3.5 text-dark-muted transition-transform ${expanded ? 'rotate-180' : ''}`} fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={2}>
          <path strokeLinecap="round" strokeLinejoin="round" d="M19 9l-7 7-7-7" />
        </svg>
      </div>

      {(isStreamingActive || expanded) && displayText && (
        <div className="px-3 pb-3 space-y-2 border-t border-white/5">
          <div>
            <div className="text-[10px] text-dark-muted mb-1.5 uppercase tracking-wider flex items-center gap-1">
              <svg className="w-3 h-3" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={2}><path strokeLinecap="round" strokeLinejoin="round" d="M9 12h6m-6 4h6m2 5H7a2 2 0 01-2-2V5a2 2 0 012-2h5.586a1 1 0 01.707.293l5.414 5.414a1 1 0 01.293.707V19a2 2 0 01-2 2z"/></svg>
              {isActive ? t('multiAgent.generating') : t('multiAgent.outputResult')}
            </div>
            <div className="p-2.5 rounded-md bg-dark-bg text-[11px] text-dark-text/90 font-mono max-h-[240px] overflow-auto whitespace-pre-wrap break-all leading-relaxed custom-scrollbar">
              {displayText}
              {isActive && <span className="inline-block w-1.5 h-3.5 bg-primary-400 animate-pulse ml-0.5 align-text-bottom" />}
            </div>
          </div>

          {agent.error && (
            <div className="rounded-md bg-red-500/5 border border-red-500/15 p-2.5">
              <div className="text-[10px] text-red-400 mb-1 uppercase tracking-wider font-medium">{t('multiAgent.errorMsg')}</div>
              <p className="text-[11px] text-red-300/80 font-mono">{agent.error}</p>
            </div>
          )}
        </div>
      )}

      {expanded && !displayText && !isStreamingActive && agent.error && (
        <div className="px-3 pb-3 border-t border-white/5">
          <div className="rounded-md bg-red-500/5 border border-red-500/15 p-2.5">
            <div className="text-[10px] text-red-400 mb-1 uppercase tracking-wider font-medium">{t('multiAgent.errorMsg')}</div>
            <p className="text-[11px] text-red-300/80 font-mono">{agent.error}</p>
          </div>
        </div>
      )}
    </div>
  )
}

function getAgentIcon(agentId: string): string {
  const icons: Record<string, string> = {
    'search-agent': '🔍',
    'code-agent': '💻',
    'analysis-agent': '📊',
    'creative-agent': '✨',
    'summary-agent': '📝',
  }
  return icons[agentId] || '🤖'
}

function SessionHeader({ status, agentCount, isStreaming }: { status: MultiAgentSessionStatus; agentCount: number; isStreaming?: boolean }) {
  const { t } = useTranslation()
  return (
    <div className="flex items-center gap-3 px-4 py-2.5 bg-dark-surface/40 border-b border-white/5 rounded-t-xl">
      <div className="flex items-center gap-2">
        <div className={`w-6 h-6 rounded-md flex items-center justify-center text-xs ${
          status === 'completed' ? 'bg-green-500/15' : 'bg-primary-500/15'
        }`}>
          🤝
        </div>
        <span className="text-xs font-semibold text-dark-text">{t('multiAgent.collaboration')}</span>
      </div>

      <StatusBadge status={status} />

      <div className="flex items-center gap-1.5 text-[10px] text-dark-muted">
        <svg className="w-3 h-3" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={2}>
          <path strokeLinecap="round" strokeLinejoin="round" d="M17 20h5v-2a3 3 0 00-5.356-1.857M17 20H7m10 0v-2c0-.656-.126-1.283-.356-1.857M7 20H2v-2a3 3 0 015.356-1.857M7 20v-2c0-.656.126-1.283.356-1.857m0 0a5.002 5.002 0 009.288 0M15 7a3 3 0 11-6 0 3 3 0 016 0z"/>
        </svg>
        <span>{t('multiAgent.subAgentCount', { count: String(agentCount) })}</span>
      </div>

      {isStreaming && (
        <div className="flex items-center gap-1.5 ml-auto">
          <span className="w-1.5 h-1.5 rounded-full bg-primary-400 animate-pulse"></span>
          <span className="text-[10px] text-primary-400 font-medium">{t('multiAgent.processing')}</span>
        </div>
      )}
    </div>
  )
}

function MultiAgentMessage({ mainResponse, subAgents, status, timestamp, isStreaming, streamingText, streamingAgentId }: MultiAgentMessageProps) {
  const { t } = useTranslation()
  const [showDetails, setShowDetails] = useState(false)

  const completedCount = subAgents.filter(s => s.status === SAS.COMPLETED).length
  const failedCount = subAgents.filter(s => s.status === SAS.FAILED || s.status === SAS.TIMEOUT).length
  const hasActiveAgents = subAgents.some(s => s.status === SAS.RUNNING || s.status === SAS.PENDING)

  useEffect(() => {
    if (hasActiveAgents && !showDetails) setShowDetails(true)
  }, [hasActiveAgents])

  return (
    <div className="flex justify-start animate-fade-in max-w-[90%]">
      <div className="w-full bg-dark-bg border border-dark-border rounded-xl shadow-sm overflow-hidden">
        <SessionHeader status={status} agentCount={subAgents.length} isStreaming={isStreaming} />

        <div className="px-4 py-3">
          <SafeMarkdown content={mainResponse || (isStreaming ? '' : '(No summary)')} className="prose prose-invert prose-sm max-w-none [&_pre]:max-w-none [&_code]:text-[13px] text-[13px] text-dark-text/90 leading-relaxed" />
          {isStreaming && !mainResponse && (
            <div className="flex items-center gap-2 mt-2">
              <div className="flex gap-1">
                <span className="w-1.5 h-1.5 rounded-full bg-primary-400 animate-bounce" style={{ animationDelay: '0ms' }}></span>
                <span className="w-1.5 h-1.5 rounded-full bg-primary-400 animate-bounce" style={{ animationDelay: '150ms' }}></span>
                <span className="w-1.5 h-1.5 rounded-full bg-primary-400 animate-bounce" style={{ animationDelay: '300ms' }}></span>
              </div>
              <span className="text-xs text-dark-muted">{t('multiAgent.agentsCollaborating')}</span>
            </div>
          )}
        </div>

        <div className="border-t border-white/5">
          <button
            onClick={() => setShowDetails(!showDetails)}
            className="w-full flex items-center justify-between px-4 py-2 hover:bg-white/[0.02] transition-colors"
          >
            <div className="flex items-center gap-2 text-[11px] text-dark-muted">
              <svg className="w-3.5 h-3.5" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={2}>
                <path strokeLinecap="round" strokeLinejoin="round" d="M19 11H5m14 0a2 2 0 012 2v6a2 2 0 01-2 2H5a2 2 0 01-2-2v-6a2 2 0 012-2m14 0V9a2 2 0 00-2-2M5 11V9a2 2 0 012-2m0 0V5a2 2 0 012-2h6a2 2 0 012 2v2M7 7h10"/>
              </svg>
              <span>{t('multiAgent.subAgentDetails')}</span>
              <span className="flex items-center gap-1.5">
                {completedCount > 0 && (
                  <span className="text-green-400">{t('multiAgent.completed', { count: String(completedCount) })}</span>
                )}
                {failedCount > 0 && (
                  <span className="text-red-400">{t('multiAgent.failed', { count: String(failedCount) })}</span>
                )}
                {subAgents.length - completedCount - failedCount > 0 && (
                  <span className="text-blue-400">{t('multiAgent.inProgress', { count: String(subAgents.length - completedCount - failedCount) })}</span>
                )}
              </span>
            </div>
            <svg className={`w-3.5 h-3.5 text-dark-muted transition-transform ${showDetails ? 'rotate-180' : ''}`} fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={2}>
              <path strokeLinecap="round" strokeLinejoin="round" d="M19 9l-7 7-7-7" />
            </svg>
          </button>

          {showDetails && (
            <div className="px-4 pb-3 space-y-2">
              {subAgents.map((agent) => (
                <SubAgentCard
                  key={agent.taskId}
                  agent={agent}
                  isStreamingActive={agent.agentId === streamingAgentId && (agent.status === SAS.RUNNING || agent.status === SAS.PENDING)}
                />
              ))}
            </div>
          )}
        </div>

        <div className="px-4 py-1.5 bg-dark-surface/20 border-t border-white/5 flex items-center justify-between">
          <span className="text-[9px] text-dark-muted/40 font-mono">
            {new Date(timestamp).toLocaleTimeString()}
          </span>
          <span className="text-[9px] text-dark-muted/40">
            {t('multiAgent.modeLabel', { count: String(subAgents.length) })}
          </span>
        </div>
      </div>
    </div>
  )
}

export default MultiAgentMessage
