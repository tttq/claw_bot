// Claw Desktop - 记忆面板 - RAG记忆系统的存储、检索、统计和导出界面
// 功能：展示记忆统计、实体图谱、检索结果详情、记忆管理

import { useState, useEffect, useCallback } from 'react'
import { useTranslation } from 'react-i18next'
import { memoryStats, memoryListEntities, memoryRetrieve, memoryExport, memoryDelete } from '../../api/memory'

interface MemoryPanelProps {
  agentId: string
  onClose?: () => void
}

interface MemoryStats {
  total_memories: number
  total_entities: number
  fact_types: { world: number; experience: number; observation: number }
  vector_dimension: number
  search_methods: string[]
}

interface Entity {
  id: string
  name: string
  type: string
  mention_count: number
  first_seen: number
  last_seen: number
}

interface EnhancedMemoryResult {
  id: string
  text: string
  fact_type: string
  source_type: string
  tags: string | null
  semantic_score: number
  bm25_score: number
  temporal_score: number
  final_score: number
}

export default function MemoryPanel({ agentId, onClose }: MemoryPanelProps) {
  const { t } = useTranslation()
  const [activeTab, setActiveTab] = useState<'overview' | 'entities' | 'search' | 'stats'>('overview')
  const [stats, setStats] = useState<MemoryStats | null>(null)
  const [entities, setEntities] = useState<Entity[]>([])
  const [searchQuery, setSearchQuery] = useState('')
  const [searchResults, setSearchResults] = useState<EnhancedMemoryResult[]>([])
  const [loading, setLoading] = useState(false)
  const [error, setError] = useState<string | null>(null)
  const [lastRefresh, setLastRefresh] = useState<Date>(new Date())

  const refreshData = useCallback(async () => {
    try {
      setLoading(true)
      setError(null)
      const [statsResult, entitiesResult] = await Promise.all([
        memoryStats(agentId),
        memoryListEntities(agentId)
      ])
      setStats(statsResult)
      setEntities(entitiesResult.entities || [])
      setLastRefresh(new Date())
    } catch (e) {
      setError(t('panels.memoryPanel.loadError', { error: String(e) }))
    } finally {
      setLoading(false)
    }
  }, [agentId])

  useEffect(() => {
    refreshData()
  }, [refreshData])

  const handleSearch = async () => {
    if (!searchQuery.trim()) return
    try {
      setLoading(true)
      setError(null)
      const result = await memoryRetrieve({ query: searchQuery, agent_id: agentId, limit: 10 }) as unknown as { count: number; results: EnhancedMemoryResult[] }
      setSearchResults(result.results || [])
    } catch (e) {
      setError(t('panels.memoryPanel.searchError', { error: String(e) }))
    } finally {
      setLoading(false)
    }
  }

  const handleExport = async () => {
    try {
      setLoading(true)
      const exportData = await memoryExport(agentId) as unknown as string
      const blob = new Blob([exportData], { type: 'application/json' })
      const url = URL.createObjectURL(blob)
      const a = document.createElement('a')
      a.href = url
      a.download = `memory-export-${agentId}-${new Date().toISOString().slice(0,10)}.json`
      a.click()
      URL.revokeObjectURL(url)
    } catch (e) {
      setError(t('panels.memoryPanel.exportError', { error: String(e) }))
    } finally {
      setLoading(false)
    }
  }

  const handleDeleteMemory = async (unitId: string) => {
    if (!confirm(t('panels.memoryPanel.confirmDelete'))) return
    try {
      await memoryDelete(unitId)
      setSearchResults(prev => prev.filter(r => r.id !== unitId))
      refreshData()
    } catch (e) {
      setError(t('panels.memoryPanel.deleteError', { error: String(e) }))
    }
  }

  const copyToClipboard = (text: string) => {
    navigator.clipboard.writeText(text).then(() => {
      showToast(t('panels.memoryPanel.copiedToClipboard'))
    })
  }

  const [toastMsg, setToastMsg] = useState<string | null>(null)
  const showToast = (msg: string) => {
    setToastMsg(msg)
    setTimeout(() => setToastMsg(null), 2000)
  }

  const getFactTypeStyle = (type: string) => {
    switch (type) {
      case 'world': return 'bg-blue-100 text-blue-800 dark:bg-blue-900/30 dark:text-blue-300'
      case 'experience': return 'bg-green-100 text-green-800 dark:bg-green-900/30 dark:text-green-300'
      case 'observation': return 'bg-purple-100 text-purple-800 dark:bg-purple-900/30 dark:text-purple-300'
      default: return 'bg-gray-100 text-gray-800 dark:bg-gray-700 dark:text-gray-300'
    }
  }

  const getScoreColor = (score: number) => {
    if (score >= 0.8) return 'text-green-600 dark:text-green-400 font-semibold'
    if (score >= 0.5) return 'text-yellow-600 dark:text-yellow-400'
    return 'text-gray-500 dark:text-gray-400'
  }

  const formatTime = (timestamp: number) => {
    return new Date(timestamp * 1000).toLocaleString(undefined, {
      month: 'short', day: 'numeric', hour: '2-digit', minute: '2-digit'
    })
  }

  return (
    <div className="h-full flex flex-col bg-gray-50 dark:bg-gray-900 rounded-lg shadow-lg">
      {/* 标题栏 */}
      <div className="flex items-center justify-between px-4 py-3 border-b border-gray-200 dark:border-gray-700 bg-white dark:bg-gray-800 rounded-t-lg">
        <div className="flex items-center gap-2">
          <button onClick={refreshData} disabled={loading}
            className="p-1.5 hover:bg-gray-100 dark:hover:bg-gray-700 rounded transition-colors disabled:opacity-50"
            title={t('panels.memoryPanel.refreshData')}>
            <svg className={`w-4 h-4 text-gray-600 dark:text-gray-300 ${loading ? 'animate-spin' : ''}`} fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={2}>
              <path strokeLinecap="round" strokeLinejoin="round" d="M4 4v5h.582m15.356 2A8.001 8.001 0 004.582 9m0 0H9m11 11v-5h-.581m0 0a8.003 8.003 0 01-15.357-2m15.357 2H15" />
            </svg>
          </button>
          {onClose && (
            <button onClick={onClose} className="p-1.5 hover:bg-gray-100 dark:hover:bg-gray-700 rounded transition-colors">
              <svg className="w-5 h-5 text-gray-600 dark:text-gray-300" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={2}>
                <path strokeLinecap="round" strokeLinejoin="round" d="M6 18L18 6M6 6l12 12" />
              </svg>
            </button>
          )}
        </div>
      </div>

      {/* Tab 导航 */}
      <div className="flex gap-1 px-4 py-2 bg-gray-100 dark:bg-gray-800 border-b">
        {[
          { id: 'overview' as const, label: t('panels.memoryPanel.tabOverview'), icon: '📊' },
          { id: 'entities' as const, label: `🏷️ ${t('panels.memoryPanel.tabEntities', { count: entities.length > 0 ? entities.length : 0 })}`, icon: '🏷️' },
          { id: 'search' as const, label: t('panels.memoryPanel.tabSearch'), icon: '🔍' },
          { id: 'stats' as const, label: t('panels.memoryPanel.tabStats'), icon: '📈' },
        ].map(tab => (
          <button key={tab.id} onClick={() => setActiveTab(tab.id)}
            className={`px-3 py-1.5 text-sm rounded transition-all ${
              activeTab === tab.id ? 'bg-blue-600 text-white shadow-md' : 'hover:bg-gray-200 dark:hover:bg-gray-700 text-gray-700 dark:text-gray-300'
            }`}>
            {tab.label}
          </button>
        ))}
      </div>

      {/* Toast 提示 */}
      {toastMsg && (
        <div className="mx-4 mt-2 px-3 py-2 bg-green-50 dark:bg-green-900/30 border border-green-200 dark:border-green-700 text-green-700 dark:text-green-300 rounded text-sm text-center animate-fade-in">
          ✓ {toastMsg}
        </div>
      )}

      {/* 错误提示 */}
      {error && (
        <div className="mx-4 mt-2 p-3 bg-red-50 dark:bg-red-900/20 border border-red-200 dark:border-red-700 text-red-700 dark:text-red-300 rounded text-sm">
          ⚠️ {error}
          <button onClick={() => setError(null)} className="ml-2 underline">{t('panels.memoryPanel.close')}</button>
        </div>
      )}

      {/* 内容区域 */}
      <div className="flex-1 overflow-y-auto p-4">
        {loading && activeTab === 'overview' && !stats && (
          <div className="flex items-center justify-center py-12">
            <div className="animate-spin rounded-full h-10 w-10 border-b-2 border-blue-600"></div>
          </div>
        )}

        {/* 概览 Tab */}
        {activeTab === 'overview' && stats && (
          <div className="space-y-4 animate-fade-in">
            <div className="grid grid-cols-2 gap-3">
              <div className="p-4 bg-white dark:bg-gray-800 rounded-lg shadow-sm hover:shadow-md transition-shadow">
                <div className="text-3xl font-bold text-blue-600">{stats.total_memories}</div>
                <div className="text-sm text-gray-500 dark:text-gray-400 mt-1">{t('panels.memoryPanel.totalMemories')}</div>
              </div>
              <div className="p-4 bg-white dark:bg-gray-800 rounded-lg shadow-sm hover:shadow-md transition-shadow">
                <div className="text-3xl font-bold text-green-600">{stats.total_entities}</div>
                <div className="text-sm text-gray-500 dark:text-gray-400 mt-1">{t('panels.memoryPanel.totalEntities')}</div>
              </div>
            </div>

            <div className="p-4 bg-white dark:bg-gray-800 rounded-lg shadow-sm">
              <h3 className="font-semibold mb-3 text-gray-900 dark:text-white flex items-center gap-2">
                {t('panels.memoryPanel.memoryTypeDistribution')}
                <span className="text-xs text-gray-400 normal-font">{t('panels.memoryPanel.lastUpdated', { time: lastRefresh.toLocaleTimeString() })}</span>
              </h3>
              <div className="space-y-3">
                {(() => {
                  const ft = stats.fact_types || { world: 0, experience: 0, observation: 0 }
                  return [
                    { type: 'world', label: t('panels.memoryPanel.typeWorldKnowledge'), count: ft.world, color: 'bg-blue-500', textColor: 'text-blue-600 dark:text-blue-400' },
                    { type: 'experience', label: t('panels.memoryPanel.typeExperience'), count: ft.experience, color: 'bg-green-500', textColor: 'text-green-600 dark:text-green-400' },
                    { type: 'observation', label: t('panels.memoryPanel.typeObservation'), count: ft.observation, color: 'bg-purple-500', textColor: 'text-purple-600 dark:text-purple-400' },
                  ]
                })().map(item => (
                  <div key={item.type} className="space-y-1">
                    <div className="flex items-center justify-between text-sm">
                      <span className="text-gray-700 dark:text-gray-300">{item.label}</span>
                      <span className={`font-medium ${item.textColor}`}>{item.count}</span>
                    </div>
                    <div className="w-full h-2 bg-gray-200 dark:bg-gray-700 rounded-full overflow-hidden">
                      <div className={`h-full ${item.color} transition-all duration-500`}
                        style={{ width: `${Math.min(100, (item.count / Math.max(stats.total_memories, 1)) * 100)}%` }}></div>
                    </div>
                  </div>
                ))}
              </div>
            </div>

            <div className="p-4 bg-gradient-to-r from-blue-50 to-purple-50 dark:from-gray-800 dark:to-gray-750 rounded-lg">
              <h3 className="font-semibold mb-2 text-gray-900 dark:text-white">{t('panels.memoryPanel.systemTraits')}</h3>
              <div className="grid grid-cols-2 gap-x-4 gap-y-2 text-xs text-gray-700 dark:text-gray-300">
                <div>{t('panels.memoryPanel.vectorDimensionLabel')}: <span className="font-mono font-bold">{stats.vector_dimension}d</span></div>
                <div>{t('panels.memoryPanel.featureFTS5')}</div>
                <div>{t('panels.memoryPanel.featureTemporal')}</div>
                <div>{t('panels.memoryPanel.featureRRF')}</div>
                <div>{t('panels.memoryPanel.featureEntity')}</div>
                <div>{t('panels.memoryPanel.featureGraph')}</div>
              </div>
            </div>

            <div className="p-3 bg-yellow-50 dark:bg-yellow-900/20 rounded-lg text-xs text-yellow-800 dark:text-yellow-200 leading-relaxed">
              {t('panels.memoryPanel.usageTip')}
            </div>
          </div>
        )}

        {/* 实体 Tab */}
        {activeTab === 'entities' && (
          <div className="space-y-2 animate-fade-in">
            <div className="flex items-center justify-between mb-3">
              <h3 className="font-semibold mb-2 text-gray-900 dark:text-white">{t('panels.memoryPanel.recognizedEntities')}</h3>
              <span className="text-xs text-gray-500">{t('panels.memoryPanel.entityCount', { count: entities.length })}</span>
            </div>
            {entities.length === 0 ? (
              <div className="text-center py-12 text-gray-400">
                <div className="text-4xl mb-3">🏷️</div>
                <div>{t('panels.memoryPanel.noEntityData')}</div>
                <div className="text-xs mt-1">{t('panels.memoryPanel.entityHint')}</div>
              </div>
            ) : (
              <div className="grid gap-2">
                {entities.map(entity => (
                  <div key={entity.id}
                    className="flex items-center justify-between p-3 bg-white dark:bg-gray-800 rounded-lg shadow-sm hover:shadow-md transition-all group">
                    <div className="flex items-center gap-3 min-w-0">
                      <span className={`px-2 py-1 text-xs rounded font-medium ${
                        entity.type === 'technology' ? 'bg-blue-100 text-blue-700 dark:bg-blue-900/40 dark:text-blue-300' :
                        entity.type === 'organization' ? 'bg-orange-100 text-orange-700 dark:bg-orange-900/40 dark:text-orange-300' :
                        'bg-gray-100 text-gray-700 dark:bg-gray-700 dark:text-gray-300'
                      }`}>
                        {entity.type}
                      </span>
                      <span className="font-medium text-gray-900 dark:text-white truncate">{entity.name || entity.id || t('panels.memoryPanel.unnamedEntity')}</span>
                    </div>
                    <div className="flex items-center gap-3">
                      <div className="text-right">
                        <div className="text-lg font-bold text-blue-600">{entity.mention_count}</div>
                        <div className="text-[10px] text-gray-400">{t('panels.memoryPanel.mentions')}</div>
                      </div>
                      <button onClick={() => copyToClipboard(entity.name)} className="opacity-0 group-hover:opacity-100 p-1 hover:bg-gray-100 dark:hover:bg-gray-700 rounded transition-all">
                        <svg className="w-4 h-4 text-gray-400" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={2}>
                          <path strokeLinecap="round" strokeLinejoin="round" d="M8 16H6a2 2 0 01-2-2V6a2 2 0 012-2h8a2 2 0 012 2v2m-6 12h8a2 2 0 002-2v-8a2 2 0 00-2-2h-8a2 2 0 00-2 2v8a2 2 0 002 2z" />
                        </svg>
                      </button>
                    </div>
                  </div>
                ))}
              </div>
            )}
          </div>
        )}

        {/* 搜索 Tab */}
        {activeTab === 'search' && (
          <div className="space-y-4 animate-fade-in">
            <div className="flex gap-2">
              <input type="text" value={searchQuery} onChange={(e) => setSearchQuery(e.target.value)}
                onKeyPress={(e) => e.key === 'Enter' && handleSearch()}
                placeholder={t('panels.memoryPanel.searchPlaceholder')}
                className="flex-1 px-4 py-2.5 border border-gray-300 dark:border-gray-600 rounded-lg focus:ring-2 focus:ring-blue-500 focus:border-transparent bg-white dark:bg-gray-800 text-gray-900 dark:text-white placeholder-gray-400" />
              <button onClick={handleSearch} disabled={loading || !searchQuery.trim()}
                className="px-6 py-2.5 bg-blue-600 text-white rounded-lg hover:bg-blue-700 disabled:opacity-50 disabled:cursor-not-allowed transition-all font-medium shadow-sm">
                {t('panels.memoryPanel.searchButton')}
              </button>
            </div>

            {searchResults.length > 0 && (
              <div className="space-y-3">
                <div className="flex items-center justify-between text-sm text-gray-500 dark:text-gray-400">
                  <span dangerouslySetInnerHTML={{ __html: t('panels.memoryPanel.foundResults', { count: searchResults.length }) }} />
                  <span className="text-xs">{t('panels.memoryPanel.sortByRRF')}</span>
                </div>
                {searchResults.map((mem, idx) => (
                  <div key={mem.id}
                    className="p-4 bg-white dark:bg-gray-800 rounded-lg shadow-sm border-l-4 border-l-blue-500 hover:shadow-md transition-all group relative">
                    <div className="absolute top-2 right-2 opacity-0 group-hover:opacity-100 flex gap-1 transition-opacity">
                      <button onClick={() => copyToClipboard(mem.text)} className="p-1 hover:bg-gray-100 dark:hover:bg-gray-700 rounded" title={t('panels.memoryPanel.copyContent')}>
                        <svg className="w-4 h-4 text-gray-400" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={2}><path strokeLinecap="round" strokeLinejoin="round" d="M8 16H6a2 2 0 01-2-2V6a2 2 0 012-2h8a2 2 0 012 2v2m-6 12h8a2 2 0 002-2v-8a2 2 0 00-2-2h-8a2 2 0 00-2 2v8a2 2 0 002 2z" /></svg>
                      </button>
                      <button onClick={() => handleDeleteMemory(mem.id)} className="p-1 hover:bg-red-50 dark:hover:bg-red-900/20 rounded" title={t('panels.memoryPanel.deleteMemory')}>
                        <svg className="w-4 h-4 text-red-400" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={2}><path strokeLinecap="round" strokeLinejoin="round" d="M19 7l-.867 12.142A2 2 0 0116.138 21H7.862a2 2 0 01-1.995-1.858L5 7m5 4v6m4-6v6m1-10V4a1 1 0 00-1-1h-4a1 1 0 00-1 1v3M4 7h16" /></svg>
                      </button>
                    </div>

                    <div className="flex items-center justify-between mb-2 pr-16">
                      <div className="flex items-center gap-2">
                        <span className="text-xs font-bold text-gray-400">#{idx + 1}</span>
                        <span className={`px-2 py-0.5 text-xs rounded font-medium ${getFactTypeStyle(mem.fact_type)}`}>
                          {mem.fact_type}
                        </span>
                        <span className="text-xs text-gray-400">{mem.source_type}</span>
                      </div>
                    </div>

                    <div className="grid grid-cols-4 gap-2 mb-3 text-xs">
                      <div className="p-1.5 bg-gray-50 dark:bg-gray-700/50 rounded text-center">
                        <div className={getScoreColor(mem.semantic_score)}>{t('panels.memoryPanel.semanticScore')}: {mem.semantic_score.toFixed(2)}</div>
                      </div>
                      <div className="p-1.5 bg-gray-50 dark:bg-gray-700/50 rounded text-center">
                        <div className={getScoreColor(mem.bm25_score)}>BM25: {mem.bm25_score.toFixed(2)}</div>
                      </div>
                      <div className="p-1.5 bg-gray-50 dark:bg-gray-700/50 rounded text-center">
                        <div className={getScoreColor(mem.temporal_score)}>{t('panels.memoryPanel.temporalScore')}: {mem.temporal_score.toFixed(2)}</div>
                      </div>
                      <div className="p-1.5 bg-blue-50 dark:bg-blue-900/20 rounded text-center">
                        <div className="text-green-600 dark:text-green-400 font-bold">{t('panels.memoryPanel.finalScore', { score: mem.final_score.toFixed(3) })}</div>
                      </div>
                    </div>

                    <div className="text-sm text-gray-700 dark:text-gray-300 line-clamp-3 leading-relaxed">
                      {mem.text}
                    </div>
                  </div>
                ))}
              </div>
            )}

            {searchResults.length === 0 && searchQuery && !loading && (
              <div className="text-center py-12 text-gray-400">
                <div className="text-4xl mb-3">🔍</div>
                <div>{t('panels.memoryPanel.noResults')}</div>
                <div className="text-xs mt-1">{t('panels.memoryPanel.emptyHint')}</div>
              </div>
            )}

            {!searchQuery && (
              <div className="text-center py-12 text-gray-400">
                <div className="text-4xl mb-3">💭</div>
                <div>{t('panels.memoryPanel.enterKeywords')}</div>
                <div className="text-xs mt-1 space-y-1">
                  <div>{t('panels.memoryPanel.searchExample1')}</div>
                  <div>{t('panels.memoryPanel.searchExample2')}</div>
                </div>
              </div>
            )}
          </div>
        )}

        {/* 统计 Tab */}
        {activeTab === 'stats' && stats && (
          <div className="space-y-4 animate-fade-in">
            <div className="p-4 bg-white dark:bg-gray-800 rounded-lg shadow-sm">
              <h3 className="font-semibold mb-3 text-gray-900 dark:text-white">{t('panels.memoryPanel.statsDetails')}</h3>
              <div className="space-y-3 text-sm">
                <div className="flex justify-between items-center p-2 bg-gray-50 dark:bg-gray-700/50 rounded">
                  <span className="text-gray-600 dark:text-gray-400">{t('panels.memoryPanel.totalMemoryUnits')}</span>
                  <span className="text-xl font-bold text-gray-900 dark:text-white">{stats.total_memories}</span>
                </div>
                <div className="flex justify-between items-center p-2 bg-gray-50 dark:bg-gray-700/50 rounded">
                  <span className="text-gray-600 dark:text-gray-400">{t('panels.memoryPanel.totalEntities')}</span>
                  <span className="text-xl font-bold text-gray-900 dark:text-white">{stats.total_entities}</span>
                </div>
                <hr className="border-gray-200 dark:border-gray-700" />
                <div className="space-y-2">
                  {(() => {
                    const ft = stats.fact_types || { world: 0, experience: 0, observation: 0 }
                    return (
                      <>
                        <div className="flex justify-between"><span className="text-gray-600 dark:text-gray-400">{t('panels.memoryPanel.worldKnowledge')}</span><span className="text-blue-600 font-medium">{ft.world}</span></div>
                        <div className="flex justify-between"><span className="text-gray-600 dark:text-gray-400">{t('panels.memoryPanel.experienceKnowledge')}</span><span className="text-green-600 font-medium">{ft.experience}</span></div>
                        <div className="flex justify-between"><span className="text-gray-600 dark:text-gray-400">{t('panels.memoryPanel.observationRecord')}</span><span className="text-purple-600 font-medium">{ft.observation}</span></div>
                      </>
                    )
                  })()}
                </div>
              </div>
            </div>

            <div className="p-4 bg-white dark:bg-gray-800 rounded-lg shadow-sm">
              <h3 className="font-semibold mb-3 text-gray-900 dark:text-white">{t('panels.memoryPanel.sysConfig')}</h3>
              <div className="space-y-2 text-sm">
                <div className="flex justify-between"><span className="text-gray-600 dark:text-gray-400">{t('panels.memoryPanel.vectorDimension')}</span><code className="px-2 py-0.5 bg-gray-100 dark:bg-gray-700 rounded text-blue-600">{stats.vector_dimension}d</code></div>
                <div className="flex justify-between"><span className="text-gray-600 dark:text-gray-400">Agent ID</span><code className="px-2 py-0.5 bg-gray-100 dark:bg-gray-700 rounded text-gray-700 dark:text-gray-300 max-w-[200px] truncate">{agentId}</code></div>
              </div>
            </div>

            <div className="flex gap-2">
              <button onClick={handleExport} disabled={loading || stats.total_memories === 0}
                className="flex-1 py-2.5 bg-green-600 text-white rounded-lg hover:bg-green-700 disabled:opacity-50 disabled:cursor-not-allowed transition-all font-medium text-sm flex items-center justify-center gap-2">
                <svg className="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={2}><path strokeLinecap="round" strokeLinejoin="round" d="M4 16v1a3 3 0 003 3h10a3 3 0 003-3v-1m-4-4l-4 4m0 0l-4-4m4 4V4" /></svg>
                {t('panels.memoryPanel.exportJson')}
              </button>
              <button onClick={refreshData} disabled={loading}
                className="py-2.5 px-4 bg-gray-200 dark:bg-gray-700 text-gray-700 dark:text-gray-300 rounded-lg hover:bg-gray-300 dark:hover:bg-gray-600 disabled:opacity-50 transition-all text-sm font-medium">
                {t('panels.memoryPanel.refresh')}
              </button>
            </div>

            <div className="p-3 bg-blue-50 dark:bg-blue-900/20 rounded-lg text-xs text-blue-800 dark:text-blue-200 leading-relaxed">
              {t('panels.memoryPanel.exportInfo')}
            </div>
          </div>
        )}
      </div>
    </div>
  )
}
