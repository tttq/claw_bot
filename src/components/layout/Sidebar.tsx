// Claw Desktop - 侧边栏组件 - 会话列表、Agent选择、新建/删除/重命名会话
// 功能：Logo、新建Agent按钮、Agent列表（含会话计数）、设置/导出/导入/诊断

import { useState, useEffect, useRef, useCallback } from 'react'
import { useTranslation } from 'react-i18next'
import { isoAgentList, isoAgentCreate, isoAgentRename, isoAgentDelete, isoGeneratePrompt } from '../../api/iso'
import { useConfigStore } from '../../stores/configStore'
import { createPortal } from 'react-dom'

interface AgentInfo {
  id: string
  displayName: string
  description: string
  isActive: boolean
  conversationCount: number
  totalMessages: number
  createdAt: number
}

interface Conversation {
  agentId?: string
  id: string
  title: string
  messageCount?: number
}

interface SidebarProps {
  conversations: Conversation[]
  activeId: string | null
  onSelectConversation: (id: string) => void
  onNewConversation: () => void
  onDeleteConversation: (id: string) => void
  onRenameConversation: (id: string, newTitle: string) => void
  onOpenSettings: () => void
  onOpenMemory?: () => void
  onOpenBrowser?: () => void
  onOpenPerformanceMonitor?: () => void
  onExport: () => void
  onImport: () => void
  onDoctor: () => void
  onSelectAgent?: (agentId: string) => void
  activeAgentId?: string | null
  onAgentsLoaded?: (agents: AgentInfo[]) => void
  onConfigureAgent?: (agentId: string, agentName: string) => void
  wsReady?: boolean
}

function Sidebar({
  conversations,
  activeId,
  onSelectConversation,
  onNewConversation,
  onDeleteConversation,
  onRenameConversation,
  onOpenSettings,
  onOpenMemory,
  onOpenBrowser,
  onOpenPerformanceMonitor,
  onExport,
  onImport,
  onDoctor,
  onSelectAgent,
  activeAgentId,
  onAgentsLoaded,
  onConfigureAgent,
  wsReady,
}: SidebarProps) {
  const { t } = useTranslation()
  const [searchQuery, setSearchQuery] = useState('')
  const [editingConvId, setEditingConvId] = useState<string | null>(null)
  const [editConvTitle, setEditConvTitle] = useState('')
  const [agents, setAgents] = useState<AgentInfo[]>([])
  const [showCreateAgent, setShowCreateAgent] = useState(false)
  const [newAgentName, setNewAgentName] = useState('')
  const [newAgentDesc, setNewAgentDesc] = useState('')
  const [newAgentPrompt, setNewAgentPrompt] = useState('')
  const [newAgentCategory, setNewAgentCategory] = useState('general')
  const [newAgentPurpose, setNewAgentPurpose] = useState('')
  const [newAgentScope, setNewAgentScope] = useState('')
  const [expandedAgent, setExpandedAgent] = useState<string | null>(null)
  const [toast, setToast] = useState<string | null>(null)
  const [generatingPrompt, setGeneratingPrompt] = useState(false)
  const toastTimeoutRef = useRef<ReturnType<typeof setTimeout> | null>(null)
  const config = useConfigStore(s => s.config)

  useEffect(() => { return () => { if (toastTimeoutRef.current) clearTimeout(toastTimeoutRef.current) } }, [])

  const showToast = useCallback((msg: string) => {
    setToast(msg)
    if (toastTimeoutRef.current) clearTimeout(toastTimeoutRef.current)
    toastTimeoutRef.current = setTimeout(() => setToast(null), 2000)
  }, [])

  // Agent 重命名状态
  const [renamingAgentId, setRenamingAgentId] = useState<string | null>(null)
  const [renameAgentValue, setRenameAgentValue] = useState('')

  useEffect(() => { if (wsReady) loadAgents() }, [wsReady])

  const loadAgents = async () => {
    try {
      const result = await isoAgentList() as unknown as { agents?: AgentInfo[] } & Record<string, unknown>
      const agentList = (result?.agents || result) as unknown as AgentInfo[]
      setAgents(Array.isArray(agentList) ? agentList : [])
      if (onAgentsLoaded) onAgentsLoaded(Array.isArray(agentList) ? agentList : [])
    } catch (e) { /* silently handle loadAgents error */ }
  }

  const handleCreateAgent = async () => {
    if (!newAgentName.trim()) return
    try {
      const agent: any = await isoAgentCreate({
        displayName: newAgentName.trim(),
        systemPrompt: newAgentPrompt,
        description: newAgentPurpose || undefined,
        category: newAgentCategory || undefined,
        purpose: newAgentPurpose || undefined,
        scope: newAgentScope || undefined,
      })
      showToast(t('sidebar.agentCreated', { name: newAgentName }))
      setShowCreateAgent(false)
      setNewAgentName(''); setNewAgentDesc(''); setNewAgentPrompt(''); setNewAgentCategory('general'); setNewAgentPurpose(''); setNewAgentScope('')
      loadAgents()
      if (onSelectAgent) onSelectAgent(agent.id)
      onNewConversation()
    } catch (e) { showToast(t('sidebar.createFailed', { error: String(e) })) }
  }

  const handleGeneratePrompt = async () => {
    if (!config) {
      showToast(t('sidebar.createAgentModal.noModelConfig'))
      return
    }
    const hasModel = !!(config.model?.default_model && (config.model?.provider || config.model?.custom_url))
    if (!hasModel) {
      showToast(t('sidebar.createAgentModal.noModelConfig'))
      return
    }
    setGeneratingPrompt(true)
    try {
      const result = await isoGeneratePrompt({
        displayName: newAgentName.trim() || undefined,
        category: newAgentCategory || undefined,
        purpose: newAgentPurpose || undefined,
        scope: newAgentScope || undefined,
        description: newAgentDesc || undefined,
        config,
      }) as any
      const prompt = result?.prompt || result?.data?.prompt || ''
      if (prompt) {
        setNewAgentPrompt(prompt)
        showToast(t('sidebar.createAgentModal.promptGenerated'))
      } else {
        showToast(t('sidebar.createAgentModal.promptGenerateFailed'))
      }
    } catch (e) {
      showToast(t('sidebar.createAgentModal.promptGenerateFailed') + ': ' + String(e))
    } finally {
      setGeneratingPrompt(false)
    }
  }

  const handleAgentClick = (agent: AgentInfo) => {
    if (expandedAgent === agent.id) {
      setExpandedAgent(null)
    } else {
      setExpandedAgent(agent.id)
      if (onSelectAgent) onSelectAgent(agent.id)
    }
  }

  // Agent 重命名
  const handleRenameAgent = async (agentId: string, newName: string) => {
    if (!newName.trim()) return
    try {
      await isoAgentRename({ agentId: agentId, newName: newName.trim() })
      showToast(t('sidebar.renamed'))
      setRenamingAgentId(null); setRenameAgentValue('')
      loadAgents()
    } catch (e) { showToast(t('sidebar.renameFailed', { error: String(e) })) }
  }

  const handleDeleteAgent = async (e: React.MouseEvent, agentId: string, agentName: string) => {
    e.stopPropagation()
    if (!confirm(t('sidebar.confirmDeleteAgent', { name: agentName }))) return
    try {
      await isoAgentDelete({ agentId: agentId })
      showToast(t('sidebar.agentDeleted', { name: agentName }))
      if (activeAgentId === agentId) {
        if (onSelectAgent) onSelectAgent('')
      }
      loadAgents()
    } catch (err) { showToast(t('sidebar.deleteFailed', { error: String(err) })) }
  }

  const handleConfigureAgent = (e: React.MouseEvent, agent: AgentInfo) => {
    e.stopPropagation()
    if (onConfigureAgent) onConfigureAgent(agent.id, agent.displayName)
  }

  const handleDeleteConversation = async (e: React.MouseEvent, convId: string) => {
    e.stopPropagation()
    if (!confirm(t('sidebar.confirmDeleteConv'))) return
    try {
      onDeleteConversation(convId)
      showToast(t('sidebar.convDeleted'))
    } catch (err) { showToast(t('sidebar.deleteFailed', { error: String(err) })) }
  }

  // 会话标题双击编辑
  const handleStartRenameConv = (e: React.MouseEvent, convId: string, currentTitle: string) => {
    e.stopPropagation()
    setEditingConvId(convId)
    setEditConvTitle(currentTitle)
  }

  const handleCommitRenameConv = async (convId: string) => {
    if (!editConvTitle.trim()) { setEditingConvId(null); return }
    try {
      onRenameConversation(convId, editConvTitle.trim())
      setEditingConvId(null); setEditConvTitle('')
    } catch (e) { console.error('[Sidebar] Rename failed:', e) }
  }
  const safeAgents = Array.isArray(agents) ? agents : []
  const safeConversations = Array.isArray(conversations) ? conversations : []
  const filteredAgents = safeAgents.filter(a =>
    !searchQuery || a.displayName.toLowerCase().includes(searchQuery.toLowerCase()) || a.description.toLowerCase().includes(searchQuery.toLowerCase())
  )

  // Filter logic:
  //   - If no agent expanded → show all conversations
  //   - If agent expanded → filter by conversation.agentId (backend already linked)
  const visibleConversations = expandedAgent
    ? safeConversations.filter(c => c.agentId === expandedAgent)
    : safeConversations

  return (
    <div className="w-64 bg-dark-surface border-r border-dark-border flex flex-col h-full">
      {/* ===== 头部区域：Logo + 新建 Agent 按钮 ===== */}
      <div className="p-4 border-b border-dark-border">
        <div className="flex items-center gap-3 mb-4">
          <div className="w-9 h-9 rounded-xl bg-gradient-to-br from-primary-500 to-primary-700 flex items-center justify-center shadow-lg shadow-primary-500/20">
            <svg className="w-5 h-5 text-white" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={2}>
              <path strokeLinecap="round" strokeLinejoin="round" d="M8 10h.01M12 10h.01M16 10h.01M9 16H5a2 2 0 01-2-2V6a2 2 0 012-2h14a2 2 0 012 2v8a2 2 0 01-2 2h-5l-5 5v-5z" />
            </svg>
          </div>
          <span className="text-lg font-bold text-dark-text">{t('sidebar.appName')}</span>
        </div>

        <button
          onClick={() => setShowCreateAgent(true)}
          className="w-full py-2.5 px-4 rounded-xl bg-primary-600 hover:bg-primary-500 text-white font-medium text-sm transition-all duration-200 flex items-center justify-center gap-2 shadow-lg shadow-primary-600/20 hover:shadow-primary-500/30 active:scale-[0.98]"
        >
          <svg className="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={2}>
            <path strokeLinecap="round" strokeLinejoin="round" d="M17 20h5v-2a3 3 0 00-5.356-1.857M17 20H7m10 0v-2c0-.656-.126-1.283-.356-1.857M7 20H2v-2a3 3 0 015.356-1.857M7 20v-2c0-.656.126-1.283.356-1.857m0 0a5.002 5.002 0 019.288 0M15 7a3 3 0 11-6 0 3 3 0 016 0zm6 3a2 2 0 11-4 0 2 2 0 014 0zM7 10a2 2 0 11-4 0 2 2 0 014 0z" />
          </svg>
          {t('sidebar.newAgent')}
        </button>
      </div>

      {/* Toast 提示 */}
      {toast && (
        <div className="mx-3 mt-2 px-3 py-1.5 rounded-lg bg-primary-600/15 border border-primary-500/30 text-[10px] text-primary-300 animate-fade-in">{toast}</div>
      )}

      {/* ===== 搜索框 ===== */}
      <div className="px-3 py-3">
        <input
          type="text"
          value={searchQuery}
          onChange={e => setSearchQuery(e.target.value)}
          placeholder={t('sidebar.searchPlaceholder')}
          className="w-full px-3 py-1.5 rounded-lg bg-dark-bg border border-dark-border text-xs text-dark-text placeholder-dark-muted/40 focus:outline-none focus:border-primary-500 transition-colors"
        />
      </div>

      {/* ===== Agent 列表（可滚动） ===== */}
      <div className="flex-1 overflow-y-auto overflow-x-hidden px-2 pb-2 space-y-1 custom-scrollbar">
        {filteredAgents.length === 0 ? (
          <div className="text-center py-8 space-y-2">
            <div className="text-3xl">🤖</div>
            <div className="text-xs text-dark-muted">{t('sidebar.noAgents')}</div>
            <button onClick={() => setShowCreateAgent(true)} className="text-[10px] text-primary-400 hover:text-primary-300 underline">{t('sidebar.clickCreateFirst')}</button>
          </div>
        ) : (
          filteredAgents.map(agent => {
            const isExpanded = expandedAgent === agent.id
            const isActive = activeAgentId === agent.id

            return (
              <div key={agent.id} className={`rounded-xl border transition-all overflow-hidden ${isActive ? 'bg-primary-500/10 border-primary-500/25' : 'bg-transparent border-transparent hover:bg-dark-bg/50'}`}>
                <div className="flex items-center overflow-hidden">
                  <button
                    onClick={() => handleAgentClick(agent)}
                    className={`flex-1 min-w-0 text-left px-3 py-2.5 rounded-xl text-sm transition-all group`}
                  >
                    <div className="flex items-center gap-2 min-w-0 overflow-hidden">
                      <div className={`w-6 h-6 rounded-md flex items-center justify-center shrink-0 text-[10px] font-bold ${agent.isActive ? 'bg-primary-600/20 text-primary-400' : 'bg-dark-border text-dark-muted'}`}>
                        {agent.displayName?.charAt(0)?.toUpperCase() || '?'}
                      </div>
                      <div className="min-w-0 overflow-hidden">
                        {renamingAgentId === agent.id ? (
                          <input
                            autoFocus
                            value={renameAgentValue}
                            onChange={e => setRenameAgentValue(e.target.value)}
                            onKeyDown={e => {
                              if (e.key === 'Enter') handleRenameAgent(agent.id, renameAgentValue)
                              if (e.key === 'Escape') { setRenamingAgentId(null); setRenameAgentValue('') }
                            }}
                            onBlur={() => handleRenameAgent(agent.id, renameAgentValue)}
                            onClick={e => e.stopPropagation()}
                            className="bg-dark-surface border border-primary-500 rounded px-1.5 py-0.5 text-sm text-dark-text outline-none font-mono w-32"
                          />
                        ) : (
                          <div
                            className={`font-medium truncate text-sm cursor-pointer hover:text-primary-400 ${isActive ? 'text-primary-300' : 'text-dark-text'}`}
                            onDoubleClick={(e) => { e.stopPropagation(); setRenamingAgentId(agent.id); setRenameAgentValue(agent.displayName) }}
                            title={t('sidebar.doubleClickRename')}
                          >
                            {agent.displayName}
                          </div>
                        )}
                        <div className="text-[9px] text-dark-muted/60 truncate">{agent.description || t('sidebar.noDescription')}</div>
                      </div>
                    </div>
                  </button>

                  <div className="flex items-center gap-0.5 mr-1.5 shrink-0" onClick={e => e.stopPropagation()}>
                    <span className="text-[9px] text-dark-muted/50 tabular-nums">{agent.conversationCount}c</span>
                    <button
                      onClick={(e) => handleConfigureAgent(e, agent)}
                      className="p-1.5 rounded-md text-dark-muted/30 hover:text-blue-400 hover:bg-blue-500/10 transition-colors"
                      title={t('sidebar.configAgent')}
                    >
                      <svg className="w-3 h-3" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={2}><path strokeLinecap="round" strokeLinejoin="round" d="M10.325 4.317c.426-1.756 2.924-1.756 3.35 0a1.724 1.724 0 002.573 1.066c1.543-.94 3.31.826 2.37 2.37a1.724 1.724 0 001.066 2.573c1.756.426 1.756 2.924 0 3.35a1.724 1.724 0 00-1.066 2.573c.94 1.543-.826 3.31-2.37 2.37a1.724 1.724 0 00-2.573 1.066c-.426 1.756-2.924 1.756-3.35 0a1.724 1.724 0 00-2.573-1.066c-1.543.94-3.31-.826-2.37-2.37a1.724 1.724 0 00-1.066-2.573c-1.756-.426-1.756-2.924 0-3.35a1.724 1.724 0 001.066-2.573c-.94-1.543.826-3.31 2.37-2.37.996.608 2.296.07 2.572-1.065z"/><path strokeLinecap="round" strokeLinejoin="round" d="M15 12a3 3 0 11-6 0 3 3 0 016 0z"/></svg>
                    </button>
                    <button
                      onClick={(e) => { e.stopPropagation(); setRenamingAgentId(agent.id); setRenameAgentValue(agent.displayName) }}
                      className="p-1.5 rounded-md text-dark-muted/30 hover:text-yellow-400 hover:bg-yellow-500/10 transition-colors"
                      title={t('sidebar.renameAgent')}
                    >
                      <svg className="w-3 h-3" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={2}><path strokeLinecap="round" strokeLinejoin="round" d="M11 5H6a2 2 0 00-2 2v11a2 2 0 002 2h11a2 2 0 002-2v-5m-1.414-9.414a2 2 0 112.828 2.828L11.828 15H9v-2.828l8.586-8.586z"/></svg>
                    </button>
                    <button
                      onClick={(e) => handleDeleteAgent(e, agent.id, agent.displayName)}
                      className="p-1.5 rounded-md text-dark-muted/30 hover:text-red-400 hover:bg-red-500/10 transition-colors"
                      title={t('sidebar.deleteAgent')}
                    >
                      <svg className="w-3 h-3" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={2}><path strokeLinecap="round" strokeLinejoin="round" d="M19 7l-.867 12.142A2 2 0 0116.138 21H7.862a2 2 0 01-1.995-1.858L5 7m5 4v6m4-6v6m1-10V4a1 1 0 00-1-1h-4a1 1 0 00-1 1v3M4 7h16"/></svg>
                    </button>
                  </div>
                </div>

                {/* 展开的会话列表 — 按当前 Agent 过滤 */}
                {isExpanded && (
                  <div className="px-2 pb-2 space-y-0.5 animate-fade-in">
                    <button
                      onClick={() => { if (onSelectAgent) onSelectAgent(agent.id); onNewConversation() }}
                      className="w-full text-left px-2.5 py-1.5 rounded-md text-[11px] text-primary-400 hover:bg-primary-600/10 transition-colors flex items-center gap-1.5"
                    >
                      <svg className="w-3 h-3" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={2}><path strokeLinecap="round" strokeLinejoin="round" d="M12 4v16m8-8H4"/></svg>
                      {t('sidebar.startNewChat')}
                    </button>

                    {visibleConversations.length === 0 && !isExpanded ? null : (
                      <>
                        {visibleConversations.map(conv => {
                          const convActive = activeId === conv.id
                          return (
                            <div key={conv.id} className={`group flex items-center rounded-md transition-all ${convActive ? 'bg-dark-bg border border-dark-border' : 'hover:bg-dark-bg/50'}`}>
                              {editingConvId === conv.id ? (
                                <input
                                  autoFocus
                                  value={editConvTitle}
                                  onChange={e => setEditConvTitle(e.target.value)}
                                  onKeyDown={e => { if (e.key === 'Enter') handleCommitRenameConv(conv.id); if (e.key === 'Escape') { setEditingConvId(null); setEditConvTitle('') } }}
                                  onBlur={() => handleCommitRenameConv(conv.id)}
                                  onClick={e => e.stopPropagation()}
                                  className="flex-1 text-left px-2.5 py-1.5 rounded-md text-[11px] bg-dark-surface border border-primary-500 text-dark-text outline-none font-mono min-w-0"
                                />
                              ) : (
                                <button
                                  onClick={() => onSelectConversation(conv.id)}
                                  onDoubleClick={(e) => handleStartRenameConv(e, conv.id, conv.title)}
                                  className="flex-1 text-left px-2.5 py-1.5 rounded-md text-[11px] transition-all truncate"
                                  title={t('sidebar.renameConvTitle')}
                                >
                                  <span className="truncate block">{conv.title}</span>
                                  <span className="block text-[9px] opacity-40 mt-0.5">{t('sidebar.messagesCount', { count: String(conv.messageCount ?? 0) })}</span>
                                </button>
                              )}
                              {/* 对话重命名按钮 */}
                              <button
                                onClick={(e) => { e.stopPropagation(); handleStartRenameConv(e, conv.id, conv.title) }}
                                className="p-1 rounded opacity-0 group-hover:opacity-100 text-dark-muted/30 hover:text-primary-400 hover:bg-primary-500/10 transition-all shrink-0"
                                title={t('sidebar.renameConvBtn')}
                              >
                                <svg className="w-3 h-3" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={2}>
                                  <path strokeLinecap="round" strokeLinejoin="round" d="M11 5H6a2 2 0 00-2 2v11a2 2 0 002 2h11a2 2 0 002-2v-5m-1.414-9.414a2 2 0 112.828 2.828L11.828 15H9v-2.828l8.586-8.586z" />
                                </svg>
                              </button>
                              {/* 会话删除按钮 */}
                              <button
                                onClick={(e) => handleDeleteConversation(e, conv.id)}
                                className="mr-1 p-1 rounded opacity-0 group-hover:opacity-100 text-dark-muted/30 hover:text-red-400 hover:bg-red-500/10 transition-all shrink-0"
                                title={t('sidebar.deleteConvBtn')}
                              >
                                <svg className="w-3 h-3" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={2}>
                                  <path strokeLinecap="round" strokeLinejoin="round" d="M19 7l-.867 12.142A2 2 0 0116.138 21H7.862a2 2 0 01-1.995-1.858L5 7m5 4v6m4-6v6m1-10V4a1 1 0 00-1-1h-4a1 1 0 00-1 1v3M4 7h16" />
                                </svg>
                              </button>
                            </div>
                          )
                        })}
                        {visibleConversations.length === 0 && (
                          <div className="text-center py-3 text-[9px] text-dark-muted/40">{t('sidebar.convNoMessages')}</div>
                        )}
                      </>
                    )}
                  </div>
                )}
              </div>
            )
          })
        )}
      </div>

      {/* ===== 底部操作栏 ===== */}
      <div className="p-3 border-t border-dark-border space-y-1">
        <button onClick={onOpenSettings}
          className="w-full py-1.5 px-3 rounded-lg text-xs text-dark-muted hover:text-dark-text hover:bg-dark-bg transition-all flex items-center gap-2">
          <svg className="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={2}><path strokeLinecap="round" strokeLinejoin="round" d="M10.325 4.317c.426-1.756 2.924-1.756 3.35 0a1.724 1.724 0 002.573 1.066c1.543-.94 3.31.826 2.37 2.37a1.724 1.724 0 001.066 2.573c1.756.426 1.756 2.924 0 3.35a1.724 1.724 0 00-1.066 2.573c.94 1.543-.826 3.31-2.37 2.37a1.724 1.724 0 00-2.573 1.066c-.426 1.756-2.924 1.756-3.35 0a1.724 1.724 0 00-2.573-1.066c-1.543.94-3.31-.826-2.37-2.37a1.724 1.724 0 00-1.066-2.573c-1.756-.426-1.756-2.924 0-3.35a1.724 1.724 0 001.066-2.573c-.94-1.543.826-3.31 2.37-2.37.996.608 2.296.07 2.572-1.065z"/><path strokeLinecap="round" strokeLinejoin="round" d="M15 12a3 3 0 11-6 0 3 3 0 016 0z"/></svg>
          {t('sidebar.settings')}
        </button>

        {onOpenPerformanceMonitor && (
          <button onClick={onOpenPerformanceMonitor}
            className="w-full py-1.5 px-3 rounded-lg text-xs text-dark-muted hover:text-primary-400 hover:bg-primary-500/10 transition-all flex items-center gap-2"
            title={t('sidebar.performanceMonitorTitle')}>
            <svg className="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={2}>
              <path strokeLinecap="round" strokeLinejoin="round" d="M9 19v-6a2 2 0 00-2-2H5a2 2 0 00-2 2v6a2 2 0 002 2h2a2 2 0 002-2zm0 0V9a2 2 0 012-2h2a2 2 0 012 2v10m-6 0a2 2 0 002 2h2a2 2 0 002-2m0 0V5a2 2 0 012-2h2a2 2 0 012 2v14a2 2 0 01-2 2h-2a2 2 0 01-2-2z"/>
            </svg>
            {t('sidebar.performanceMonitor')}
          </button>
        )}

        <div className="flex gap-1">
          <button onClick={onExport} className="flex-1 py-1.5 px-2 rounded-lg text-[11px] text-dark-muted hover:text-dark-text hover:bg-dark-bg transition-all flex items-center justify-center gap-1" title={t('sidebar.exportTitle')}>
            <svg className="w-3.5 h-3.5" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={2}><path strokeLinecap="round" strokeLinejoin="round" d="M4 16v1a3 3 0 003 3h10a3 3 0 003-3v-1m-4-8l-4-4m0 0L8 8m4-4v12"/></svg> {t('sidebar.export')}
          </button>
          <button onClick={onImport} className="flex-1 py-1.5 px-2 rounded-lg text-[11px] text-dark-muted hover:text-dark-text hover:bg-dark-bg transition-all flex items-center justify-center gap-1" title={t('sidebar.importTitle')}>
            <svg className="w-3.5 h-3.5" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={2}><path strokeLinecap="round" strokeLinejoin="round" d="M4 16v1a3 3 0 003 3h10a3 3 0 003-3v-1m-4-4l-4 4m0 0l-4-4m4 4V4"/></svg> {t('sidebar.import')}
          </button>
          <button onClick={onDoctor} className="py-1.5 px-2 rounded-lg text-[11px] text-dark-muted hover:text-dark-text hover:bg-dark-bg transition-all flex items-center justify-center gap-1" title={t('sidebar.doctorTitle')}>
            <svg className="w-3.5 h-3.5" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={2}><path strokeLinecap="round" strokeLinejoin="round" d="M9 12l2 2 4-4m6 2a9 9 0 11-18 0 9 9 0 0118 0z"/></svg>
          </button>
        </div>
      </div>

      {/* ===== 创建 Agent 弹窗（Portal 到 body） ===== */}
      {showCreateAgent && createPortal(
        <div className="fixed inset-0 z-[70] flex items-center justify-center bg-black/50 backdrop-blur-sm" onClick={() => setShowCreateAgent(false)}>
          <div className="bg-dark-surface border border-dark-border rounded-2xl shadow-2xl w-[520px] p-5 animate-fade-in" onClick={e => e.stopPropagation()}>
            <h3 className="text-base font-bold text-dark-text mb-1">{t('sidebar.createAgentModal.title')}</h3>
            <p className="text-[11px] text-dark-muted mb-4">{t('sidebar.createAgentModal.desc')}</p>
            <div className="space-y-2.5">
              <div><label className="block text-[11px] font-medium text-dark-text mb-1">{t('sidebar.createAgentModal.nameLabel')}</label>
                <input value={newAgentName} onChange={e => setNewAgentName(e.target.value)} placeholder="Code Reviewer" autoFocus
                  className="w-full bg-dark-bg border border-dark-border rounded-lg px-3 py-2 text-sm text-dark-text focus:outline-none focus:border-primary-500 font-mono" />
              </div>
              <div className="grid grid-cols-2 gap-2.5">
                <div><label className="block text-[11px] font-medium text-dark-text mb-1">{t('sidebar.createAgentModal.categoryLabel')}</label>
                  <select value={newAgentCategory} onChange={e => setNewAgentCategory(e.target.value)}
                    className="w-full bg-dark-bg border border-dark-border rounded-lg px-3 py-2 text-sm text-dark-text focus:outline-none focus:border-primary-500">
                    <option value="general">{t('sidebar.createAgentModal.categories.general')}</option>
                    <option value="code">{t('sidebar.createAgentModal.categories.code')}</option>
                    <option value="search">{t('sidebar.createAgentModal.categories.search')}</option>
                    <option value="analysis">{t('sidebar.createAgentModal.categories.analysis')}</option>
                    <option value="creative">{t('sidebar.createAgentModal.categories.creative')}</option>
                  </select>
                </div>
                <div><label className="block text-[11px] font-medium text-dark-text mb-1">{t('sidebar.createAgentModal.descriptionLabel')}</label>
                  <input value={newAgentDesc} onChange={e => setNewAgentDesc(e.target.value)} placeholder={t('sidebar.createAgentModal.purposePlaceholder')}
                    className="w-full bg-dark-bg border border-dark-border rounded-lg px-3 py-2 text-sm text-dark-text focus:outline-none focus:border-primary-500" />
                </div>
              </div>
              <div><label className="block text-[11px] font-medium text-dark-text mb-1">{t('sidebar.createAgentModal.purposeLabel')}</label>
                <input value={newAgentPurpose} onChange={e => setNewAgentPurpose(e.target.value)} placeholder={t('sidebar.createAgentModal.purposePlaceholder')}
                  className="w-full bg-dark-bg border border-dark-border rounded-lg px-3 py-2 text-sm text-dark-text focus:outline-none focus:border-primary-500" />
              </div>
              <div><label className="block text-[11px] font-medium text-dark-text mb-1">{t('sidebar.createAgentModal.scopeLabel')}</label>
                <input value={newAgentScope} onChange={e => setNewAgentScope(e.target.value)} placeholder={t('sidebar.createAgentModal.scopePlaceholder')}
                  className="w-full bg-dark-bg border border-dark-border rounded-lg px-3 py-2 text-sm text-dark-text focus:outline-none focus:border-primary-500" />
              </div>
              <div><label className="block text-[11px] font-medium text-dark-text mb-1">{t('sidebar.createAgentModal.systemPromptLabel')}</label>
                <div className="relative">
                  <textarea value={newAgentPrompt} onChange={e => setNewAgentPrompt(e.target.value)}
                    placeholder={t('sidebar.createAgentModal.systemPromptPlaceholder')} rows={3}
                    className="w-full bg-dark-bg border border-dark-border rounded-lg px-3 py-2 text-xs text-dark-text focus:outline-none focus:border-primary-500 resize-none pr-9" />
                  <button
                    onClick={handleGeneratePrompt}
                    disabled={generatingPrompt || !config?.model?.default_model}
                    className={`absolute bottom-2 right-2 p-1.5 rounded-md transition-all ${
                      generatingPrompt
                        ? 'text-primary-400 animate-pulse cursor-wait'
                        : config?.model?.default_model
                          ? 'text-dark-muted/40 hover:text-primary-400 hover:bg-primary-500/10 cursor-pointer'
                          : 'text-dark-muted/20 cursor-not-allowed'
                    }`}
                    title={config?.model?.default_model ? t('sidebar.createAgentModal.generatePromptTitle') : t('sidebar.createAgentModal.noModelConfig')}
                  >
                    {generatingPrompt ? (
                      <svg className="w-3.5 h-3.5 animate-spin" fill="none" viewBox="0 0 24 24">
                        <circle className="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" strokeWidth="4" />
                        <path className="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4z" />
                      </svg>
                    ) : (
                      <svg className="w-3.5 h-3.5" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={2}>
                        <path strokeLinecap="round" strokeLinejoin="round" d="M9.813 15.904L9 18.75l-.813-2.846a4.5 4.5 0 00-3.09-3.09L2.25 12l2.846-.813a4.5 4.5 0 003.09-3.09L9 5.25l.813 2.846a4.5 4.5 0 003.09 3.09L15.75 12l-2.846.813a4.5 4.5 0 00-3.09 3.09zM18.259 8.715L18 9.75l-.259-1.035a3.375 3.375 0 00-2.455-2.456L14.25 6l1.036-.259a3.375 3.375 0 002.455-2.456L18 2.25l.259 1.035a3.375 3.375 0 002.455 2.456L21.75 6l-1.036.259a3.375 3.375 0 00-2.455 2.456z" />
                      </svg>
                    )}
                  </button>
                </div>
              </div>
              <div className="flex justify-end gap-2 pt-2">
                <button onClick={() => setShowCreateAgent(false)} className="px-4 py-1.5 rounded-lg border border-dark-border text-xs text-dark-muted hover:text-dark-text">{t('sidebar.createAgentModal.cancel')}</button>
                <button onClick={handleCreateAgent} disabled={!newAgentName.trim()} className={`px-4 py-1.5 rounded-lg text-white text-xs font-medium transition-colors ${!newAgentName.trim() ? 'bg-dark-border text-dark-muted cursor-not-allowed' : 'bg-primary-600 hover:bg-primary-500'}`}>{t('sidebar.createAgentModal.createAndStart')}</button>
              </div>
            </div>
          </div>
        </div>,
        document.body
      )}

    </div>
  )
}

export default Sidebar
