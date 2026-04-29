// Claw Desktop - 成本面板 - 查看Token消耗和API调用成本统计
// 使用真实后端数据，支持按Agent过滤

import { useState, useEffect } from 'react'
import { getUsageStats } from '../../../api/system'
import { useTranslation } from 'react-i18next'

interface UsageStats {
  agentId: string | null
  conversationCount: number
  messageCount: number
  inputTokens: number
  outputTokens: number
  totalTokens: number
  cacheReadTokens: number
  cacheCreationTokens: number
  estimatedCostUsd: number
  mostUsedModel: string
  modelBreakdown: Record<string, number>
  conversations: Array<{ id: string; title: string; messageCount: number; agentId?: string }>
}

interface CostPanelProps {
  agentId?: string | null
}

export default function CostPanel({ agentId }: CostPanelProps) {
  const [stats, setStats] = useState<UsageStats | null>(null)
  const [loading, setLoading] = useState(true)
  const [activeTab, setActiveTab] = useState<'overview' | 'history'>('overview')
  const { t } = useTranslation()

  useEffect(() => {
    loadStats()
  }, [agentId])

  const loadStats = async () => {
    setLoading(true)
    try {
      const data = await getUsageStats() as unknown as UsageStats
      setStats(data)
    } catch (e) {
      console.error('Failed to load usage stats:', e)
    } finally { setLoading(false) }
  }

  if (loading) return <div className="flex justify-center py-12"><div className="w-7 h-7 border-2 border-primary-500 border-t-transparent rounded-full animate-spin"></div></div>

  if (!stats) return <div className="text-center py-8 text-sm text-dark-muted">{t('panels.cost.loadFailed')}</div>

  const s = {
    conversationCount: stats.conversationCount ?? 0,
    messageCount: stats.messageCount ?? 0,
    inputTokens: stats.inputTokens ?? 0,
    outputTokens: stats.outputTokens ?? 0,
    totalTokens: stats.totalTokens ?? 0,
    cacheReadTokens: stats.cacheReadTokens ?? 0,
    cacheCreationTokens: stats.cacheCreationTokens ?? 0,
    estimatedCostUsd: stats.estimatedCostUsd ?? 0,
    mostUsedModel: stats.mostUsedModel || '',
    modelBreakdown: stats.modelBreakdown || {},
    conversations: Array.isArray(stats.conversations) ? stats.conversations : [],
  }

  const avgCostPerConv = s.conversationCount > 0 ? s.estimatedCostUsd / s.conversationCount : 0
  const avgCostPerMsg = s.messageCount > 0 ? s.estimatedCostUsd / s.messageCount : 0

  return (
    <div className="space-y-5">
      <div className="flex items-center justify-between">
        <div>
          <h3 className="text-base font-semibold text-dark-text flex items-center gap-2">
            <svg className="w-5 h-5 text-green-400" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={2}><path strokeLinecap="round" strokeLinejoin="round" d="M12 8c-1.657 0-3 .895-3 2s1.343 2 3 2 3 .895 3 2-1.343 2-3 2m0-8c1.11 0 2.08.402 2.599 1M12 21V3m0 0l-3 3m3-3l3 3"/></svg>
            {t('panels.cost.title')}
          </h3>
          <p className="text-xs text-dark-muted mt-0.5">
            {agentId ? t('panels.cost.agentStats') : t('panels.cost.globalStats')} · {s.conversationCount} {t('panels.cost.conversationsUnit')} · {s.messageCount} {t('panels.cost.messagesUnit')}
          </p>
        </div>
        <button onClick={loadStats} className="p-1.5 rounded-lg hover:bg-dark-bg text-dark-muted hover:text-dark-text transition-colors">
          <svg className="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={2}><path strokeLinecap="round" strokeLinejoin="round" d="M4 4v5h.582m15.356 2A8.001 8.001 0 004.582 9m0 0H9m11 11v-5h-.581m0 0a8.003 8.003 0 01-15.357-2m15.357 2H15"/></svg>
        </button>
      </div>

      {/* Tabs */}
      <div className="flex gap-1 p-1 rounded-lg bg-dark-bg border border-dark-border w-fit">
        <button onClick={() => setActiveTab('overview')} className={`px-3 py-1.5 rounded-md text-xs transition-colors ${activeTab === 'overview' ? 'bg-primary-600 text-white' : 'text-dark-muted hover:text-dark-text'}`}>{t('panels.cost.tabOverview')}</button>
        <button onClick={() => setActiveTab('history')} className={`px-3 py-1.5 rounded-md text-xs transition-colors ${activeTab === 'history' ? 'bg-primary-600 text-white' : 'text-dark-muted hover:text-dark-text'}`}>{t('panels.cost.tabHistory')}</button>
      </div>

      {activeTab === 'overview' && (
        <div className="grid grid-cols-3 gap-3">
          {[
            { label: t('panels.cost.totalCost'), value: `$${s.estimatedCostUsd.toFixed(4)}`, sub: t('panels.cost.usdEstimate'), color: 'from-green-500 to-emerald-600', icon: '$' },
            { label: t('panels.cost.inputTokens'), value: formatNumber(s.inputTokens), sub: `${(s.inputTokens / 1e6).toFixed(2)}M tokens`, color: 'from-blue-500 to-cyan-600', icon: '↓' },
            { label: t('panels.cost.outputTokens'), value: formatNumber(s.outputTokens), sub: `${(s.outputTokens / 1e6).toFixed(2)}M tokens`, color: 'from-purple-500 to-pink-600', icon: '↑' },
            { label: t('panels.cost.conversations'), value: s.conversationCount.toString(), sub: `${s.messageCount} ${t('panels.cost.messagesUnit')}`, color: 'from-orange-500 to-red-500', icon: '#' },
            { label: t('panels.cost.avgCostPerConv'), value: `$${avgCostPerConv.toFixed(4)}`, sub: t('panels.cost.perConvCost'), color: 'from-indigo-500 to-blue-600', icon: '=' },
            { label: t('panels.cost.cacheHit'), value: formatNumber(s.cacheReadTokens + s.cacheCreationTokens), sub: `${t('panels.cost.cacheRead')}:${formatNumber(s.cacheReadTokens)} ${t('panels.cost.cacheCreate')}:${formatNumber(s.cacheCreationTokens)}`, color: 'from-teal-500 to-cyan-600', icon: 'C' },
          ].map((card, i) => (
            <div key={i} className={`p-4 rounded-xl bg-gradient-to-br ${card.color} shadow-lg`}>
              <div className="flex items-center gap-2 mb-2">
                <span className="w-6 h-6 rounded-md bg-white/20 flex items-center justify-center text-xs font-bold">{card.icon}</span>
                <span className="text-[10px] text-white/70 uppercase tracking-wider">{card.label}</span>
              </div>
              <div className="text-xl font-bold text-white">{card.value}</div>
              <div className="text-[10px] text-white/50 mt-0.5">{card.sub}</div>
            </div>
          ))}
          
          {/* Model Distribution */}
          {Object.keys(s.modelBreakdown).length > 0 && (
            <div className="col-span-3 p-4 rounded-xl bg-dark-bg border border-dark-border">
              <div className="text-[11px] font-medium text-dark-text mb-2">{t('panels.cost.modelDistribution')}</div>
              <div className="space-y-1.5">
                {Object.entries(s.modelBreakdown).sort((a, b) => b[1] - a[1]).slice(0, 5).map(([model, count]) => (
                  <div key={model} className="flex items-center gap-2">
                    <span className="text-[10px] text-dark-muted w-24 truncate font-mono">{(model || '').split('-')[0]}</span>
                    <div className="flex-1 h-1.5 bg-dark-border rounded-full overflow-hidden">
                      <div className="h-full bg-primary-500 rounded-full" style={{ width: `${Math.min(100, (count / s.messageCount) * 100)}%` }} />
                    </div>
                    <span className="text-[10px] text-dark-muted w-8 text-right">{count}</span>
                  </div>
                ))}
              </div>
              <div className="text-[9px] text-dark-muted mt-1">{t('panels.cost.mostUsedModel')} {(s.mostUsedModel || '').split('-')[0]}</div>
            </div>
          )}
        </div>
      )}

      {activeTab === 'history' && (
        <div className="rounded-xl border border-dark-border overflow-hidden">
          <div className="max-h-[420px] overflow-y-auto divide-y divide-dark-border">
            {s.conversations.length === 0 ? (
              <div className="text-center py-8 text-sm text-dark-muted">
                {agentId ? t('panels.cost.noAgentHistory') : t('panels.cost.noHistory')}
              </div>
            ) : s.conversations.map((c, i) => (
              <div key={i} className="px-4 py-3 hover:bg-dark-bg/50 transition-colors">
                <div className="flex items-center justify-between mb-1">
                  <span className="text-xs font-medium text-dark-text truncate max-w-[240px]">{c.title || (c.id || '').slice(0, 12)}</span>
                  <span className="text-[10px] text-dark-muted">{c.messageCount} {t('panels.cost.messagesUnit')}</span>
                </div>
                <div className="flex items-center gap-3 text-[10px] text-dark-muted">
                  <span>{(c.agentId || '').slice(0, 8) ?? '-'}</span>
                  <span>ID: {(c.id || '').slice(0, 8)}...</span>
                </div>
              </div>
            ))}
          </div>
        </div>
      )}
    </div>
  )
}

function formatNumber(n: number): string {
  if (n >= 1e6) return `${(n / 1e6).toFixed(1)}M`
  if (n >= 1e3) return `${(n / 1e3).toFixed(1)}K`
  return n.toLocaleString()
}
