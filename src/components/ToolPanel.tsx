// Claw Desktop - 工具面板 - 展示工具调用记录、技能管理、文件浏览等扩展面板
// 功能：Agent管理 / 技能管理 / 工具浏览 - 三大模块
// 布局：左侧Tab导航 + 右侧内容区

import { useState, useEffect, useCallback } from 'react'
import { useTranslation } from 'react-i18next'
import {
  isoAgentList, isoAgentCreate, isoAgentRename, isoAgentDelete,
  isoGetConfig, isoSetConfig, isoSetSkillsEnabled, isoAgentUpdateConfig,
  isoListSessions, isoListWorkspace, isoSetToolsConfig, isoIndexWorkspace,
  agentReadFile, agentWriteFile, agentDeleteFile,
  skillList, skillExecute, skillPermissionsList, skillTelemetryList, skillTelemetryClear,
  skillPermissionAdd, skillPermissionRemove, skillRegisterMcp,
  fsSkillList, fsSkillReload, fsSkillScan, fsSkillAdd, fsSkillRemove,
  fsSkillReadSource, fsSkillUpdateSource, fsSkillsDirPath,
  toolTaskCreate, toolTodoWrite, toolScheduleCron,
  toolListAll, toolTodoGet, toolTaskList, toolScheduleList,
} from '../api'
import { createPortal } from 'react-dom'
import type { AgentConfig, SkillDefinition, ToolDefinition, TodoItem, TaskItem, CronJob, AgentSession, AgentWorkspaceFile, FsSkillInfo, SkillTelemetryEvent, SkillPermissionRule, MarkdownComponentProps, GitStatus, GitCommit, GitBranch, GitDiff } from '../types'
import GitPanel from './config/panels/GitPanel'
import FileExplorer from './config/panels/FileExplorer'
import CostPanel from './config/panels/CostPanel'
import PlanEditor from './config/panels/PlanEditor'
import McpConfig from './config/panels/McpConfig'
import TagManager from './config/panels/TagManager'
import EnvViewer from './config/panels/EnvViewer'
import CodeReview from './config/panels/CodeReview'
import QuickActions from './config/panels/QuickActions'
import AboutPanel from './config/panels/AboutPanel'
import NotePanel from './config/panels/NotePanel'
import WebSearchPanel from './config/panels/WebSearchPanel'

interface ToolPanelProps {
  onClose: () => void          // Close panel callback
  conversationId?: string | null  // Current conversation ID (for Agent-isolated config)
}

/** Agent详情信息（从API返回的Agent数据结构） */
interface AgentInfo {
  id: string
  displayName?: string
  name?: string
  description?: string
  systemPrompt?: string
  modelOverride?: string
  maxTurns?: number
  temperature?: number
  toolsConfig?: string[]
  skillsEnabled?: string[]
  conversationCount?: number
  totalMessages?: number
  enabled?: boolean
  isActive?: boolean
  [key: string]: unknown
}

type PanelTab = 'agents' | 'skills' | 'tools' | 'git' | 'files' | 'tasks' | 'cost' | 'plan' | 'mcp' | 'tags' | 'env' | 'review' | 'quick' | 'notes' | 'web' | 'about' | 'cron'

export default function ToolPanel({ onClose, conversationId }: ToolPanelProps) {
  const { t } = useTranslation()
  const [activeTab, setActiveTab] = useState<PanelTab>('agents')

  const tabs: { id: PanelTab; label: string; icon: JSX.Element }[] = [
    { id: 'agents', label: 'Agents', icon: <svg className="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={2}><path strokeLinecap="round" strokeLinejoin="round" d="M17 20h5v-2a3 3 0 00-5.356-1.857M17 20H7m10 0v-2c0-.656-.126-1.283-.356-1.857M7 20H2v-2a3 3 0 015.356-1.857M7 20v-2c0-.656.126-1.283.356-1.857m0 0a5.002 5.002 0 019.288 0M15 7a3 3 0 11-6 0 3 3 0 016 0zm6 3a2 2 0 11-4 0 2 2 0 014 0zM7 10a2 2 0 11-4 0 2 2 0 014 0z" /></svg> },
    { id: 'skills', label: 'Skills', icon: <svg className="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={2}><path strokeLinecap="round" strokeLinejoin="round" d="M9.663 17h4.673M12 3v1m6.364 1.636l-.707.707M21 12h-1M4 12H3m3.343-5.657l-.707-.707m2.828 9.9a5 5 0 117.072 0l-.548.547A3.374 3.374 0 0014 18.469V19a2 2 0 11-4 0v-.531c0-.895-.356-1.754-.988-2.386l-.548-.547z" /></svg> },
    { id: 'tools', label: 'Tools', icon: <svg className="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={2}><path strokeLinecap="round" strokeLinejoin="round" d="M10.325 4.317c.426-1.756 2.924-1.756 3.35 0a1.724 1.724 0 002.573 1.066c1.543-.94 3.31.826 2.37 2.37a1.724 1.724 0 001.066 2.573c1.756.426 1.756 2.924 0 3.35a1.724 1.724 0 00-1.066 2.573c.94 1.543-.826 3.31-2.37 2.37a1.724 1.724 0 00-2.573 1.066c-.426 1.756-2.924 1.756-3.35 0a1.724 1.724 0 00-2.573-1.066c-1.543.94-3.31-.826-2.37-2.37a1.724 1.724 0 00-1.066-2.573c-1.756-.426-1.756-2.924 0-3.35a1.724 1.724 0 001.066-2.573c-.94-1.543.826-3.31 2.37-2.37.996.608 2.296.07 2.572-1.065z" /><path strokeLinecap="round" strokeLinejoin="round" d="M15 12a3 3 0 11-6 0 3 3 0 016 0z" /></svg> },
    { id: 'tasks', label: 'Tasks', icon: <svg className="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={2}><path strokeLinecap="round" strokeLinejoin="round" d="M9 5H7a2 2 0 00-2 2v12a2 2 0 002 2h10a2 2 0 002-2V7a2 2 0 00-2-2h-2M9 5a2 2 0 002 2h2a2 2 0 002-2M9 5a2 2 0 012-2h2a2 2 0 012 2m-6 9l2 2 4-4" /></svg> },
    { id: 'cost', label: 'Cost', icon: <svg className="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={2}><path strokeLinecap="round" strokeLinejoin="round" d="M12 8c-1.657 0-3 .895-3 2s1.343 2 3 2 3 .895 3 2-1.343 2-3 2m0-8c1.11 0 2.08.402 2.599 1M12 21V3m0 0l-3 3m3-3l3 3"/></svg> },
    { id: 'plan', label: 'Plan', icon: <svg className="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={2}><path strokeLinecap="round" strokeLinejoin="round" d="M9 5H7a2 2 0 00-2 2v12a2 2 0 002 2h10a2 2 0 002-2V7a2 2 0 00-2-2h-2M9 5a2 2 0 002 2h2a2 2 0 002-2M9 5a2 2 0 012-2h2a2 2 0 012 2m-6 9l2 2 4-4" /></svg> },
    { id: 'git', label: 'Git', icon: <svg className="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={2}><path strokeLinecap="round" strokeLinejoin="round" d="M13 7h8m0 0v8m0-8l-8 8-4-4-6 6" /></svg> },
    { id: 'files', label: 'Files', icon: <svg className="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={2}><path strokeLinecap="round" strokeLinejoin="round" d="M3 7v10a2 2 0 002 2h14a2 2 0 002-2V9a2 2 0 00-2-2h-6l-2-2H5a2 2 0 00-2 2z" /></svg> },
    { id: 'cron', label: 'Cron', icon: <svg className="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={2}><path strokeLinecap="round" strokeLinejoin="round" d="M12 8v4l3 3m6-3a9 9 0 11-18 0 9 9 0 0118 0z" /></svg> },
    { id: 'mcp', label: 'MCP', icon: <svg className="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={2}><path strokeLinecap="round" strokeLinejoin="round" d="M8 9l3 3-3 3m5 0h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z"/></svg> },
    { id: 'tags', label: 'Tags', icon: <svg className="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={2}><path strokeLinecap="round" strokeLinejoin="round" d="M7 7h.01M7 3h5c.512 0 1.024.195 1.414.586l7 7a2 2 0 010 2.828l-7 7a2 2 0 01-2.828 0l-7-7A1.994 1.994 0 013 12V7a4 4 0 014-4z"/></svg> },
    { id: 'env', label: 'Env', icon: <svg className="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={2}><path strokeLinecap="round" strokeLinejoin="round" d="M9.75 17L9 20l-1 1h8l-1-1-.75-3M3 13h18M5 17h14a2 2 0 002-2V5a2 2 0 00-2-2H5a2 2 0 00-2 2v10a2 2 0 002 2z"/></svg> },
    { id: 'review', label: 'Review', icon: <svg className="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={2}><path strokeLinecap="round" strokeLinejoin="round" d="M9 12l2 2 4-4m6 2a9 9 0 11-18 0 9 9 0 0118 0z"/></svg> },
    { id: 'quick', label: 'Quick', icon: <svg className="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={2}><path strokeLinecap="round" strokeLinejoin="round" d="M13 10V3L4 14h7v7l9-11h-7z"/></svg> },
    { id: 'notes', label: 'Notes', icon: <svg className="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={2}><path strokeLinecap="round" strokeLinejoin="round" d="M11 5H6a2 2 0 00-2 2v11a2 2 0 002 2h11a2 2 0 002-2v-5m-1.414-9.414a2 2 0 112.828 2.828L11.828 15H9v-2.828l8.586-8.586z"/></svg> },
    { id: 'web', label: 'Web', icon: <svg className="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={2}><path strokeLinecap="round" strokeLinejoin="round" d="M21 12a9 9 0 01-9 9m9-9a9 9 0 00-9-9m9 9H3m9 9a9 9 0 01-9-9m9 9c1.657 0 3-4.03 3-9s-1.343-9-3-9m0 18c-1.657 0-3-4.03-3-9s1.343-9 3-9m-9 9a9 9 0 019-9"/></svg> },
    { id: 'about', label: 'About', icon: <svg className="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={2}><path strokeLinecap="round" strokeLinejoin="round" d="M13 16h-1v-4h-1m1-4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z"/></svg> },
  ]

  return (
    <div className="flex flex-col h-full overflow-hidden">
      {/* Header */}
      <div className="flex items-center justify-between px-4 py-2.5 border-b border-dark-border shrink-0">
        <div className="flex items-center gap-2 min-w-0">
          <div className="w-6 h-6 rounded-lg bg-gradient-to-br from-primary-500 to-primary-700 flex items-center justify-center shrink-0">
            <svg className="w-3.5 h-3.5 text-white" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={2}><path strokeLinecap="round" strokeLinejoin="round" d="M10.325 4.317c.426-1.756 2.924-1.756 3.35 0a1.724 1.724 0 002.573 1.066c1.543-.94 3.31.826 2.37 2.37a1.724 1.724 0 001.066 2.573c1.756.426 1.756 2.924 0 3.35a1.724 1.724 0 00-1.066 2.573c.94 1.543-.826 3.31-2.37 2.37a1.724 1.724 0 00-2.573 1.066c-.426 1.756-2.924 1.756-3.35 0a1.724 1.724 0 00-2.573-1.066c-1.543.94-3.31-.826-2.37-2.37a1.724 1.724 0 00-1.066-2.573c-1.756-.426-1.756-2.924 0-3.35a1.724 1.724 0 001.066-2.573c-.94-1.543.826-3.31 2.37-2.37.996.608 2.296.07 2.572-1.065z" /><path strokeLinecap="round" strokeLinejoin="round" d="M15 12a3 3 0 11-6 0 3 3 0 016 0z" /></svg>
          </div>
          <h2 className="text-sm font-bold text-dark-text">Agent Config</h2>
          {conversationId && (
            <span className="text-[9px] font-mono text-dark-muted/40 bg-dark-bg px-1.5 py-0.5 rounded">{conversationId.slice(0, 8)}</span>
          )}
        </div>
        <button onClick={onClose} className="p-1.5 rounded-lg hover:bg-dark-border/50 text-dark-muted hover:text-dark-text transition-colors">
          <svg className="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={2}><path strokeLinecap="round" strokeLinejoin="round" d="M6 18L18 6M6 6l12 12" /></svg>
        </button>
      </div>

      {/* Body: Left nav + Right content */}
      <div className="flex flex-1 overflow-hidden">
        {/* Left navigation bar */}
        <div className="w-36 border-r border-dark-border py-2 shrink-0 overflow-y-auto custom-scrollbar">
          {tabs.map(tab => (
            <button key={tab.id} onClick={() => setActiveTab(tab.id)}
              className={`w-full flex items-center gap-2 px-3 py-2 text-[11px] transition-colors ${
                activeTab === tab.id ? 'bg-primary-600/10 text-primary-400 border-r-2 border-primary-500' : 'text-dark-muted hover:text-dark-text hover:bg-dark-border/30'
              }`}>
              {tab.icon}{tab.label}
            </button>
          ))}
        </div>

          {/* Right content area */}
          <div className="flex-1 overflow-y-auto p-5">
            {activeTab === 'agents' && <AgentsTab />}
            {activeTab === 'skills' && <SkillsTab />}
            {activeTab === 'tools' && <ToolsTab />}
            {activeTab === 'git' && <GitPanel />}
            {activeTab === 'files' && <FileExplorer />}
            {activeTab === 'tasks' && <TasksTab />}
            {activeTab === 'cost' && <CostPanel />}
            {activeTab === 'plan' && <PlanEditor />}
            {activeTab === 'cron' && <CronTab />}
            {activeTab === 'mcp' && <McpConfig />}
            {activeTab === 'tags' && <TagManager />}
            {activeTab === 'env' && <EnvViewer />}
            {activeTab === 'review' && <CodeReview />}
            {activeTab === 'quick' && <QuickActions />}
            {activeTab === 'notes' && <NotePanel />}
            {activeTab === 'web' && <WebSearchPanel />}
            {activeTab === 'about' && <AboutPanel />}
          </div>
        </div>
    </div>
  )
}

// ==================== Agents Management Sub-panel (filesystem-based) ====================

function AgentsTab() {
  const { t } = useTranslation()
  const [agents, setAgents] = useState<AgentInfo[]>([])
  const [loading, setLoading] = useState(true)
  const [selectedAgent, setSelectedAgent] = useState<AgentInfo | null>(null)
  const [workspaceFiles, setWorkspaceFiles] = useState<AgentWorkspaceFile[]>([])
  const [showCreate, setShowCreate] = useState(false)
  const [toast, setToast] = useState<string | null>(null)
  const [reloading, setReloading] = useState(false)

  // Per-agent isolation state
  const [agentSessions, setAgentSessions] = useState<AgentSession[]>([])
  const [agentConfigs, setAgentConfigs] = useState<Array<Record<string, unknown>>>([])
  const [agentTools, setAgentTools] = useState<string[]>([])
  const [agentSkills, setAgentSkills] = useState<string[]>([])
  const [editingName, setEditingName] = useState<string | null>(null)
  const [editNameValue, setEditNameValue] = useState('')

  const [createForm, setCreateForm] = useState({ name: '', description: '', systemPrompt: '' })
  const [viewingFile, setViewingFile] = useState<string | null>(null)
  const [fileContent, setFileContent] = useState('')
  const [editingFile, setEditingFile] = useState<string | null>(null)
  const [editContent, setEditContent] = useState('')
  const [showNewFile, setShowNewFile] = useState(false)
  const [newFileName, setNewFileName] = useState('')
  const [newFileContent, setNewFileContent] = useState('')

  useEffect(() => { loadAgents() }, [])

  const loadAgents = async () => {
    setLoading(true)
    try {
      const result = await isoAgentList() as unknown as { agents?: AgentInfo[] } & Record<string, unknown>
      const agentList = (result?.agents || result) as unknown as AgentInfo[]
      setAgents(Array.isArray(agentList) ? agentList : [])
    } catch { setAgents([]) }
    finally { setLoading(false) }
  }

  const showToast = (msg: string) => { setToast(msg); setTimeout(() => setToast(null), 2500) }

  // ===== ISO isolation command operations =====

  const handleReload = async () => {
    setReloading(true)
    try { await loadAgents(); showToast(t('toolPanel.agents.toastReloaded', { count: agents.length })) }
    catch (e) { showToast(`${t('toolPanel.agents.toastLoadFailed', { error: e })}`) }
    finally { setReloading(false) }
  }

  const handleCreateAgent = async () => {
    if (!createForm.name.trim() || !createForm.systemPrompt.trim()) return
    try {
      const agent = await isoAgentCreate({
        displayName: createForm.name.trim(),
        systemPrompt: createForm.systemPrompt,
      }) as unknown as { id: string; displayName?: string; [key: string]: unknown }
      showToast(t('toolPanel.agents.toastCreated', { name: createForm.name, id: agent.id.slice(0, 16) }))
      setCreateForm({ name: '', description: '', systemPrompt: '' }); setShowCreate(false); loadAgents()
    } catch (e) { showToast(t('toolPanel.agents.toastCreateFailed', { error: e })) }
  }

  const handleRenameAgent = async (id: string, newName: string) => {
    if (!newName.trim()) return
    try {
      await isoAgentRename({ agentId: id, newName: newName.trim() })
      showToast(t('toolPanel.agents.toastRenamed'))
      setEditingName(null); loadAgents()
      if (selectedAgent?.id === id) setSelectedAgent({ ...selectedAgent, displayName: newName.trim() } as AgentInfo)
    } catch (e) { showToast(t('toolPanel.agents.toastRenameFailed', { error: e })) }
  }

  const handleRemoveAgent = async (id: string, name: string) => {
    if (!window.confirm(t('toolPanel.agents.deleteConfirm', { name }))) return
    try {
      await isoAgentDelete({ agentId: id })
      showToast(t('toolPanel.agents.toastDeleted', { name }))
      if (selectedAgent?.id === id) { setSelectedAgent(null); setWorkspaceFiles([]) }
      loadAgents()
    } catch (e) { showToast(t('toolPanel.agents.toastDeleteFailed', { error: e })) }
  }

  const handleSelectAgent = async (agent: Record<string, unknown>) => {
    setSelectedAgent(agent as AgentInfo)
    try {
      const sessionsResult = await isoListSessions({ agentId: agent.id as string }) as unknown as { sessions?: AgentSession[] }
      const workspaceResult = await isoListWorkspace({ agentId: agent.id as string }).catch(() => null) as { entries?: AgentWorkspaceFile[] } | null
      const sessions = sessionsResult?.sessions || sessionsResult || []
      const workspace = workspaceResult?.entries || workspaceResult || []
      setAgentSessions(Array.isArray(sessions) ? sessions : [])
      setAgentConfigs([])
      setWorkspaceFiles(Array.isArray(workspace) ? workspace : [])
      setAgentTools(agent.toolsConfig as string[] || [])
      setAgentSkills(agent.skillsEnabled as string[] || [])
    } catch (e) { console.error('[AgentsTab]', e) }
  }

  const handleSetTools = async () => {
    if (!selectedAgent) return
    try {
      await isoSetToolsConfig({ agentId: String(selectedAgent.id), config: agentTools })
      showToast(t('toolPanel.agents.toastToolsSaved'))
    } catch (e) { showToast(t('toolPanel.agents.toastSaveFailed', { error: e })) }
  }

  const handleSetSkills = async () => {
    if (!selectedAgent) return
    try {
      await isoSetSkillsEnabled({ agentId: String(selectedAgent.id), enabled: agentSkills })
      showToast(t('toolPanel.agents.toastSkillsSaved'))
    } catch (e) { showToast(t('toolPanel.agents.toastSaveFailed', { error: e })) }
  }

  const handleSetConfig = async (key: string, value: string) => {
    if (!selectedAgent) return
    try {
      await isoSetConfig({ agentId: String(selectedAgent.id), key, value })
      showToast(t('toolPanel.agents.toastConfigSet', { key }))
    } catch (e) { showToast(t('toolPanel.agents.toastConfigSetFailed', { error: e })) }
  }

  const handleReadFile = async (filePath: string) => {
    setViewingFile(filePath)
    try {
      const result = await agentReadFile({ id: selectedAgent?.id as string, relPath: filePath }) as { content?: string }
      setFileContent(result.content || '')
    } catch (e) { setFileContent(t('toolPanel.agents.toastReadFailed', { error: e })) }
  }

  const handleEditFile = async (filePath: string) => {
    setEditingFile(filePath)
    try {
      const result = await agentReadFile({ id: selectedAgent?.id as string, relPath: filePath }) as { content?: string }
      setEditContent(result.content || '')
    } catch { setEditContent('') }
  }

  const handleSaveEdit = async () => {
    if (!editingFile) return
    try {
      await agentWriteFile({ id: selectedAgent?.id as string, relPath: editingFile, content: editContent })
      showToast(t('toolPanel.agents.toastFileSaved')); setEditingFile(null); setEditContent('')
      await isoIndexWorkspace({ agentId: selectedAgent?.id as string, path: editingFile })
    } catch (e) { showToast(t('toolPanel.agents.toastSaveFailed', { error: e })) }
  }

  const handleDeleteFile = async (filePath: string) => {
    if (!window.confirm(t('toolPanel.agents.deleteFileConfirm', { path: filePath }))) return
    try {
      await agentDeleteFile({ id: selectedAgent?.id as string, relPath: filePath })
      showToast(t('toolPanel.agents.toastFileDeleted'))
      const result = await isoListWorkspace({ agentId: selectedAgent?.id as string }) as { entries?: AgentWorkspaceFile[] }
      const workspaceList = result?.entries || result || []
      setWorkspaceFiles(Array.isArray(workspaceList) ? workspaceList : [])
      if (viewingFile === filePath) { setViewingFile(null); setFileContent('') }
    } catch (e) { showToast(t('toolPanel.agents.toastFileDeleteFailed', { error: e })) }
  }

  const handleCreateFile = async () => {
    if (!newFileName.trim() || !selectedAgent) return
    try {
      await agentWriteFile({ id: selectedAgent.id as string, relPath: newFileName.trim(), content: newFileContent })
      showToast(t('toolPanel.agents.toastFileCreated', { name: newFileName }))
      setNewFileName(''); setNewFileContent(''); setShowNewFile(false)
      const result = await isoListWorkspace({ agentId: selectedAgent.id as string }) as { entries?: AgentWorkspaceFile[] }
      const workspaceList = result?.entries || result || []
      setWorkspaceFiles(Array.isArray(workspaceList) ? workspaceList : [])
      await isoIndexWorkspace({ agentId: selectedAgent.id as string, path: newFileName.trim() })
    } catch (e) { showToast(t('toolPanel.agents.toastFileCreateFailed', { error: e })) }
  }

  if (loading) return <div className="flex justify-center py-12"><div className="w-7 h-7 border-2 border-primary-500 border-t-transparent rounded-full animate-spin"></div></div>

  return (
    <div className="space-y-5">
      {/* Header action bar */}
      <div className="flex items-center justify-between">
        <div><h3 className="text-base font-semibold text-dark-text">{t('toolPanel.agents.header')}</h3><p className="text-xs text-dark-muted mt-0.5">{t('toolPanel.agents.desc')}</p></div>
        <div className="flex items-center gap-2">
          <button onClick={handleReload} disabled={reloading} className={`px-2.5 py-1.5 rounded-lg text-xs border transition-colors ${reloading ? 'bg-dark-bg border-dark-border text-dark-muted' : 'bg-blue-600/10 text-blue-400 border-blue-500/20 hover:bg-blue-600/20'}`}>
            {reloading ? '⏳' : '🔄'} {t('toolPanel.agents.reload')}
          </button>
          <button onClick={() => setShowCreate(true)} className="px-3 py-1.5 rounded-lg bg-primary-600 hover:bg-primary-500 text-white text-xs font-medium transition-colors flex items-center gap-1.5">
            <svg className="w-3.5 h-3.5" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={2}><path strokeLinecap="round" strokeLinejoin="round" d="M12 4v16m8-8H4" /></svg>{t('toolPanel.agents.create')}
          </button>
        </div>
      </div>

      {toast && <div className="px-3 py-2 rounded-lg bg-primary-600/10 border border-primary-500/20 text-xs text-primary-300">{toast}</div>}

      {/* Main layout: Agent list + Detail view */}
      {!selectedAgent ? (
        <>
          {(!agents || agents.length === 0) ? (
            <div className="text-center py-16">
              <div className="text-4xl mb-3">🤖</div>
              <p className="text-sm text-dark-muted mb-1">{t('toolPanel.agents.empty')}</p>
              <p className="text-xs text-dark-muted/60">{t('toolPanel.agents.emptyHint')}</p>
            </div>
          ) : (
            <div className="grid grid-cols-1 gap-3">
              {agents.map(agent => (
                <div key={agent.id} onClick={() => handleSelectAgent(agent as unknown as Record<string, unknown>)} className={`p-4 rounded-xl border cursor-pointer transition-all ${agent.isActive ? 'bg-dark-bg border-dark-border hover:border-primary-500/30' : 'bg-dark-bg/50 border-dark-border/50 opacity-60 hover:opacity-80'}`}>
                  <div className="flex items-start justify-between">
                    <div className="flex-1 min-w-0">
                      <div className="flex items-center gap-2 mb-1">
                        {editingName === agent.id ? (
                          <input autoFocus value={editNameValue} onChange={e => setEditNameValue(e.target.value)}
                            onKeyDown={e => { if (e.key === 'Enter') handleRenameAgent(agent.id, editNameValue); if (e.key === 'Escape') setEditingName(null) }}
                            onBlur={() => handleRenameAgent(agent.id, editNameValue)}
                            onClick={e => e.stopPropagation()}
                            className="bg-dark-surface border border-primary-500 rounded px-2 py-0.5 text-sm text-dark-text outline-none font-mono w-40" />
                        ) : (
                          <>
                            <span className="text-sm font-semibold text-dark-text">{agent.displayName || agent.name || ''}</span>
                            <span className={`px-1.5 py-0.5 rounded text-[10px] ${agent.isActive ? 'bg-green-500/10 text-green-400' : 'bg-dark-border text-dark-muted'}`}>{agent.isActive ? t('toolPanel.agents.active') : t('toolPanel.agents.disabled')}</span>
                          </>
                        )}
                      </div>
                      <p className="text-[10px] font-mono text-cyan-400/50 truncate mb-1">ID: {agent.id}</p>
                      <p className="text-xs text-dark-muted line-clamp-2">{agent.description || t('toolPanel.agents.noDesc')}</p>
                      <div className="flex items-center gap-2 mt-1">
                        {Number(agent.conversationCount || 0) > 0 && <span className="text-[9px] text-dark-muted/60">💬 {String(agent.conversationCount || 0)} {t('toolPanel.agents.conversations', { count: agent.conversationCount || 0 })}</span>}
                        {Number(agent.totalMessages || 0) > 0 && <span className="text-[9px] text-dark-muted/60">📨 {String(agent.totalMessages || 0)} {t('toolPanel.agents.messages', { count: agent.totalMessages || 0 })}</span>}
                      </div>
                    </div>
                    <div className="flex items-center gap-1 ml-3 shrink-0" onClick={e => e.stopPropagation()}>
                      <button onClick={() => { setEditingName(agent.id); setEditNameValue(agent.displayName || agent.name || '') }} className="p-1.5 rounded-lg hover:bg-yellow-500/10 text-dark-muted hover:text-yellow-400" title={t('toolPanel.agents.renameTitle')}>
                        <svg className="w-3 h-3" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={2}><path strokeLinecap="round" strokeLinejoin="round" d="M11 5H6a2 2 0 00-2 2v11a2 2 0 002 2h11a2 2 0 002-2v-5m-1.414-9.414a2 2 0 112.828 2.828L11.828 15H9v-2.828l8.586-8.586z"/></svg>
                      </button>
                      <button onClick={() => handleRemoveAgent(agent.id, agent.displayName || agent.name || '')} className="p-1.5 rounded-lg hover:bg-red-500/10 text-dark-muted hover:text-red-400 transition-colors" title={t('toolPanel.agents.deleteTitle')}>
                        <svg className="w-3.5 h-3.5" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={2}><path strokeLinecap="round" strokeLinejoin="round" d="M19 7l-.867 12.142A2 2 0 0116.138 21H7.862a2 2 0 01-1.995-1.858L5 7m5 4v6m4-6v6m1-10V4a1 1 0 00-1-1h-4a1 1 0 00-1 1v3M4 7h16"/></svg>
                      </button>
                    </div>
                  </div>
                </div>
              ))}
            </div>
          )}
        </>
      ) : (
        /* ===== Agent isolation detail view ===== */
        <div className="space-y-4">
          {/* Back + Header */}
          <div className="flex items-center justify-between pb-3 border-b border-dark-border">
            <button onClick={() => { setSelectedAgent(null); setWorkspaceFiles([]) }} className="flex items-center gap-1.5 text-sm text-dark-muted hover:text-primary-400 transition-colors">
              <svg className="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={2}><path strokeLinecap="round" strokeLinejoin="round" d="M15 19l-7-7 7-7" /></svg>
              {t('toolPanel.agents.backToList')}
            </button>
            <div className="flex items-center gap-2">
              <span className="text-sm font-semibold text-dark-text">{selectedAgent.displayName || selectedAgent.name}</span>
              <span className={`px-1.5 py-0.5 rounded text-[10px] ${selectedAgent.isActive ? 'bg-green-500/10 text-green-400' : 'bg-dark-border text-dark-muted'}`}>{selectedAgent.isActive ? t('toolPanel.agents.active') : t('toolPanel.agents.disabled')}</span>
            </div>
          </div>

          {/* 唯一 ID（不可变）+ 统计 */}
          <div className="grid grid-cols-3 gap-2">
            <div className="p-2.5 rounded-lg bg-dark-bg border border-dark-border">
              <div className="text-[9px] text-dark-muted mb-0.5">{t('toolPanel.agents.uniqueId')}</div>
              <code className="text-[10px] font-mono text-cyan-400 break-all">{selectedAgent.id}</code>
            </div>
            <div className="p-2.5 rounded-lg bg-dark-bg border border-dark-border">
              <div className="text-[9px] text-dark-muted mb-0.5">{t('toolPanel.agents.sessionCount')}</div>
              <span className="text-sm font-semibold text-primary-300">{selectedAgent.conversationCount || 0}</span>
            </div>
            <div className="p-2.5 rounded-lg bg-dark-bg border border-dark-border">
              <div className="text-[9px] text-dark-muted mb-0.5">{t('toolPanel.agents.messageCount')}</div>
              <span className="text-sm font-semibold text-primary-300">{selectedAgent.totalMessages || 0}</span>
            </div>
          </div>

          {/* 元信息 + 配置 */}
          <div className="grid grid-cols-2 gap-3">
            <div className="p-3 rounded-xl border border-dark-border bg-dark-bg space-y-1.5">
              <h4 className="text-[11px] font-semibold text-dark-muted uppercase tracking-wider">{t('toolPanel.agents.basicInfo')}</h4>
              <p className="text-xs text-dark-text">{selectedAgent.description || t('toolPanel.agents.noDesc')}</p>
              {selectedAgent.systemPrompt && <pre className="text-[10px] text-dark-text whitespace-pre-wrap max-h-[100px] overflow-auto leading-relaxed opacity-70">{(selectedAgent.systemPrompt || '').slice(0, 200)}</pre>}
              {selectedAgent.modelOverride && <div><span className="text-[10px] text-dark-muted">{t('toolPanel.agents.modelLabel')}</span><code className="text-xs text-purple-300 ml-1">{selectedAgent.modelOverride}</code></div>}
              <div><span className="text-[10px] text-dark-muted">{t('toolPanel.agents.maxTurns')}</span><span className="text-xs text-dark-text ml-1">{String(selectedAgent.maxTurns || 20)}</span></div>
              <div><span className="text-[10px] text-dark-muted">{t('toolPanel.agents.temperature')}</span><span className="text-xs text-dark-text ml-1">{String(selectedAgent.temperature || 0.7)}</span></div>
            </div>

            {/* Per-Agent 工具/技能配置 */}
            <div className="p-3 rounded-xl border border-dark-border bg-dark-bg space-y-2">
              <h4 className="text-[11px] font-semibold text-dark-muted uppercase tracking-wider">{t('toolPanel.agents.perAgentConfig')}</h4>
              <div>
                <label className="text-[10px] text-dark-muted block mb-1">{t('toolPanel.agents.toolPermissions', { count: agentTools.length })}</label>
                <input value={agentTools.join(', ')} onChange={e => setAgentTools(e.target.value.split(',').map(t => t.trim()).filter(Boolean))}
                  placeholder="Read,Edit,Write,Bash,Glob..." className="w-full bg-dark-surface border border-dark-border rounded px-2 py-1 text-[11px] text-dark-text font-mono" />
                <button onClick={handleSetTools} className="mt-1 px-2 py-0.5 rounded text-[9px] bg-blue-600/10 text-blue-400 hover:bg-blue-600/20">{t('toolPanel.agents.saveToolsConfig')}</button>
              </div>
              <div>
                <label className="text-[10px] text-dark-muted block mb-1">{t('toolPanel.agents.enabledSkills', { count: agentSkills.length })}</label>
                <input value={agentSkills.join(', ')} onChange={e => setAgentSkills(e.target.value.split(',').map(t => t.trim()).filter(Boolean))}
                  placeholder="commit,review,debug..." className="w-full bg-dark-surface border border-dark-border rounded px-2 py-1 text-[11px] text-dark-text font-mono" />
                <button onClick={handleSetSkills} className="mt-1 px-2 py-0.5 rounded text-[9px] bg-purple-600/10 text-purple-400 hover:bg-purple-600/20">{t('toolPanel.agents.saveSkillsConfig')}</button>
              </div>
            </div>
          </div>

          {/* 会话列表 */}
          {agentSessions.length > 0 && (
            <div className="rounded-xl border border-dark-border overflow-hidden">
              <div className="px-3 py-2 bg-dark-bg border-b border-dark-border flex items-center justify-between">
                <h4 className="text-[11px] font-semibold text-dark-text">{t('toolPanel.agents.sessionsHeader', { count: agentSessions.length })}</h4>
              </div>
              <div className="max-h-[120px] overflow-y-auto divide-y divide-dark-border/30">
                {agentSessions.map((s: AgentSession) => (
                  <div key={s.id} className="px-3 py-1.5 flex items-center justify-between text-[10px]">
                    <div className="flex items-center gap-2">
                      <code className="font-mono text-dark-muted">{s.id.slice(0, 12)}</code>
                      <span className={`px-1 rounded ${s.status === 'active' ? 'bg-green-500/10 text-green-400' : s.status === 'completed' ? 'bg-blue-500/10 text-blue-400' : 'bg-dark-border text-dark-muted'}`}>{s.status}</span>
                    </div>
                    <span className="text-dark-muted/50">{s.turnCount ?? 0} rounds · {s.lastActive ? new Date(s.lastActive * 1000).toLocaleTimeString() : '-'}</span>
                  </div>
                ))}
              </div>
            </div>
          )}

          {/* 自定义配置项 */}
          {agentConfigs.length > 0 && (
            <div className="rounded-xl border border-dark-border overflow-hidden">
              <div className="px-3 py-2 bg-dark-bg border-b border-dark-border">
                <h4 className="text-[11px] font-semibold text-dark-text">{t('toolPanel.agents.customConfigHeader', { count: agentConfigs.length })}</h4>
              </div>
              <div className="max-h-[100px] overflow-y-auto divide-y divide-dark-border/30">
                {agentConfigs.map((c: Record<string, unknown>, i: number) => (
                  <div key={i} className="px-3 py-1.5 flex items-center justify-between text-[10px]">
                    <code className="font-mono text-primary-300">{String(c.key ?? '')}</code>
                    <span className="text-dark-muted truncate max-w-[180px]">{String(c.value ?? '').slice(0, 40)}</span>
                  </div>
                ))}
              </div>
            </div>
          )}

          {/* 工作区文件浏览器 */}
          <div className="rounded-xl border border-dark-border overflow-hidden">
            <div className="flex items-center justify-between px-4 py-2.5 bg-dark-bg border-b border-dark-border">
              <h4 className="text-xs font-semibold text-dark-text flex items-center gap-1.5">
                {t('toolPanel.agents.workspaceHeader', { count: workspaceFiles.length })}
              </h4>
              <button onClick={() => setShowNewFile(true)} className="px-2 py-1 rounded text-[10px] bg-primary-600/10 text-primary-300 hover:bg-primary-600/20 transition-colors flex items-center gap-1">
                <svg className="w-3 h-3" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={2}><path strokeLinecap="round" strokeLinejoin="round" d="M12 4v16m8-8H4" /></svg> {t('toolPanel.agents.newFile')}
              </button>
            </div>

            {workspaceFiles.length === 0 ? (
              <div className="py-8 text-center text-sm text-dark-muted">
                {t('toolPanel.agents.workspaceEmpty')}
              </div>
            ) : (
              <div className="divide-y divide-dark-border/50 max-h-[280px] overflow-y-auto">
                {workspaceFiles.map((f: AgentWorkspaceFile, i: number) => (
                  <div key={i} className="flex items-center justify-between px-4 py-2 group hover:bg-dark-surface/30 transition-colors">
                    <div className="flex items-center gap-2 min-w-0 flex-1">
                      <span className="text-dark-muted shrink-0">{f.is_dir ? '📁' : '📄'}</span>
                      <span className="text-xs text-dark-text truncate cursor-pointer hover:text-primary-400" onClick={() => !f.is_dir && handleReadFile(f.path)}>{f.name}</span>
                      <span className="text-[9px] text-dark-muted/50 shrink-0">{f.size || ''}</span>
                    </div>
                    {!f.is_dir && (
                      <div className="opacity-0 group-hover:opacity-100 flex gap-1 transition-opacity">
                        <button onClick={() => handleReadFile(f.path)} className="p-1 rounded text-dark-muted hover:text-blue-400" title={t('toolPanel.agents.viewTitle')}>👁️</button>
                        <button onClick={() => handleEditFile(f.path)} className="p-1 rounded text-dark-muted hover:text-yellow-400" title={t('toolPanel.agents.editTitle')}>✏️</button>
                        <button onClick={() => handleDeleteFile(f.path)} className="p-1 rounded text-dark-muted hover:text-red-400" title={t('toolPanel.agents.deleteTitle')}>🗑️</button>
                      </div>
                    )}
                  </div>
                ))}
              </div>
            )}
          </div>

          {/* 文件查看器 / 编辑器 */}
          {(viewingFile || editingFile) && (
            <div className="rounded-xl border border-dark-border overflow-hidden">
              <div className="flex items-center justify-between px-4 py-2 bg-dark-bg border-b border-dark-border">
                <span className="text-xs font-medium text-dark-text font-mono truncate max-w-[400px]">{editingFile ? t('toolPanel.agents.editLabel') : t('toolPanel.agents.viewLabel')} {viewingFile || editingFile}</span>
                <div className="flex items-center gap-2">
                  {viewingFile && !editingFile && (
                    <button onClick={() => { setEditingFile(viewingFile); setEditContent(fileContent); setViewingFile(null) }} className="px-2 py-1 rounded text-[10px] bg-yellow-500/10 text-yellow-400 hover:bg-yellow-500/20">{t('toolPanel.agents.editTitle')}</button>
                  )}
                  <button onClick={() => { setViewingFile(null); setEditingFile(null); setFileContent(''); setEditContent('') }} className="p-1 rounded text-dark-muted hover:text-dark-text"><svg className="w-3.5 h-3.5" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={2}><path strokeLinecap="round" strokeLinejoin="round" d="M6 18L18 6M6 6l12 12" /></svg></button>
                </div>
              </div>
              {editingFile ? (
                <div className="space-y-2 p-3">
                  <textarea value={editContent} onChange={e => setEditContent(e.target.value)} rows={14} spellCheck={false} className="w-full bg-dark-bg border border-dark-border rounded-lg px-3 py-2 text-xs text-green-400 font-mono focus:outline-none focus:border-primary-500 resize-none whitespace-pre" />
                  <div className="flex justify-end gap-2 pt-1">
                    <button onClick={() => { setEditingFile(null); setEditContent('') }} className="px-3 py-1.5 rounded-lg border border-dark-border text-xs text-dark-muted hover:text-dark-text">{t('toolPanel.agents.cancel')}</button>
                    <button onClick={handleSaveEdit} className="px-3 py-1.5 rounded-lg bg-primary-600 hover:bg-primary-500 text-white text-xs">{t('toolPanel.agents.save')}</button>
                  </div>
                </div>
              ) : (
                <pre className="p-4 text-[11px] text-dark-text bg-dark-bg max-h-[320px] overflow-auto font-mono whitespace-pre-wrap leading-relaxed">{fileContent}</pre>
              )}
            </div>
          )}
        </div>
      )}

      {/* ===== 创建隔离 Agent 弹窗（Portal） ===== */ }
      {showCreate && createPortal(
        <div className="fixed inset-0 z-[60] flex items-center justify-center bg-black/50 backdrop-blur-sm" onClick={() => setShowCreate(false)}>
          <div className="bg-dark-surface border border-dark-border rounded-2xl shadow-2xl w-[580px] max-h-[85vh] overflow-y-auto p-6 animate-fade-in" onClick={e => e.stopPropagation()}>
            <h3 className="text-base font-bold text-dark-text mb-2">{t('toolPanel.agents.createModalTitle')}</h3>
            <p className="text-[11px] text-dark-muted mb-4">{t('toolPanel.agents.createModalDesc')}</p>
            <div className="space-y-3">
              <div><label className="block text-xs font-medium text-dark-text mb-1">{t('toolPanel.agents.displayName')}</label><input value={createForm.name} onChange={e => setCreateForm({ ...createForm, name: e.target.value })} placeholder="Code Reviewer" className="w-full bg-dark-bg border border-dark-border rounded-lg px-3 py-2 text-sm text-dark-text focus:outline-none focus:border-primary-500 font-mono" /></div>
              <div><label className="block text-xs font-medium text-dark-text mb-1">{t('toolPanel.agents.description')}</label><input value={createForm.description} onChange={e => setCreateForm({ ...createForm, description: e.target.value })} placeholder={t('toolPanel.agents.descPlaceholder')} className="w-full bg-dark-bg border border-dark-border rounded-lg px-3 py-2 text-sm text-dark-text focus:outline-none focus:border-primary-500" /></div>
              <div><label className="block text-xs font-medium text-dark-text mb-1">{t('toolPanel.agents.systemPrompt')}</label><textarea value={createForm.systemPrompt} onChange={e => setCreateForm({ ...createForm, systemPrompt: e.target.value })} placeholder={t('toolPanel.agents.promptPlaceholder')} rows={5} className="w-full bg-dark-bg border border-dark-border rounded-lg px-3 py-2 text-sm text-dark-text focus:outline-none focus:border-primary-500 resize-none" /></div>
              <div className="flex justify-end gap-2 pt-3 border-t border-dark-border/50">
                <button onClick={() => setShowCreate(false)} className="px-4 py-2 rounded-lg border border-dark-border text-sm text-dark-muted hover:text-dark-text hover:bg-dark-border/30 transition-colors">{t('toolPanel.agents.cancel')}</button>
                <button onClick={handleCreateAgent} disabled={!createForm.name.trim() || !createForm.systemPrompt.trim()} className="px-4 py-2 rounded-lg bg-primary-600 hover:bg-primary-500 text-white text-sm font-medium transition-colors disabled:opacity-40 disabled:cursor-not-allowed">{t('toolPanel.agents.createButton')}</button>
              </div>
            </div>
          </div>
        </div>,
        document.body
      )}

      <NewFileModal
        open={showNewFile && !!selectedAgent}
        fileName={newFileName} setFileName={setNewFileName}
        fileContent={newFileContent} setFileContent={setNewFileContent}
        onClose={() => setShowNewFile(false)}
        onConfirm={handleCreateFile}
      />
    </div>
  )
}

// ==================== 新建工作区文件弹窗组件 ====================

function NewFileModal({ open, fileName, setFileName, fileContent, setFileContent, onClose, onConfirm }: {
  open: boolean; fileName: string; setFileName: (v: string) => void;
  fileContent: string; setFileContent: (v: string) => void;
  onClose: () => void; onConfirm: () => void;
}) {
  const { t } = useTranslation()
  if (!open) return null
  return createPortal(
    <div className="fixed inset-0 z-[60] flex items-center justify-center bg-black/50 backdrop-blur-sm" onClick={onClose}>
      <div className="bg-dark-surface border border-dark-border rounded-2xl shadow-2xl w-[550px] p-6 animate-fade-in" onClick={e => e.stopPropagation()}>
        <h3 className="text-base font-bold text-dark-text mb-4">{t('toolPanel.agents.newFileModalTitle')}</h3>
        <div className="space-y-3">
          <div><label className="block text-xs font-medium text-dark-text mb-1">{t('toolPanel.agents.filePathLabel')}</label><input value={fileName} onChange={e => setFileName(e.target.value)} placeholder={t('toolPanel.agents.filePathPlaceholder')} className="w-full bg-dark-bg border border-dark-border rounded-lg px-3 py-2 text-sm text-dark-text focus:outline-none focus:border-primary-500 font-mono" /></div>
          <div><label className="block text-xs font-medium text-dark-text mb-1">{t('toolPanel.agents.fileContentLabel')}</label><textarea value={fileContent} onChange={e => setFileContent(e.target.value)} placeholder={t('toolPanel.agents.fileContentPlaceholder')} rows={8} className="w-full bg-dark-bg border border-dark-border rounded-lg px-3 py-2 text-xs text-dark-text font-mono focus:outline-none focus:border-primary-500 resize-none" /></div>
          <div className="flex justify-end gap-2 pt-2">
            <button onClick={onClose} className="px-4 py-2 rounded-lg border border-dark-border text-sm text-dark-muted hover:text-dark-text">{t('toolPanel.agents.cancel')}</button>
            <button onClick={onConfirm} disabled={!fileName.trim()} className="px-4 py-2 rounded-lg bg-primary-600 hover:bg-primary-500 text-white text-sm font-medium disabled:opacity-40">{t('toolPanel.agents.createFileBtn')}</button>
          </div>
        </div>
      </div>
    </div>,
    document.body
  )
}

// ==================== Skills 管理子面板 ====================

function SkillsTab() {
  const { t } = useTranslation()
  const [skills, setSkills] = useState<any[]>([])
  const [fsSkills, setFsSkills] = useState<any[]>([])
  const [searchQuery, setSearchQuery] = useState('')
  const [loading, setLoading] = useState(true)
  const [executing, setExecuting] = useState<string | null>(null)
  const [executeResult, setExecuteResult] = useState<any>(null)
  const [activeSubTab, setActiveSubTab] = useState<'skills' | 'permissions' | 'telemetry'>('skills')
  const [permissions, setPermissions] = useState<any[]>([])
  const [telemetry, setTelemetry] = useState<any[]>([])
  const [newRuleTool, setNewRuleTool] = useState('')
  const [newRuleContent, setNewRuleContent] = useState('')
  const [newRuleBehavior, setNewRuleBehavior] = useState<'allow' | 'deny' | 'ask'>('ask')
  const [mcpName, setMcpName] = useState('')
  const [mcpDesc, setMcpDesc] = useState('')
  const [mcpTemplate, setMcpTemplate] = useState('')
  const [toast, setToast] = useState<string | null>(null)
  const [skillsDirPath, setSkillsDirPath] = useState<string>('')
  const [showAddSkill, setShowAddSkill] = useState(false)
  const [editingSourceSkill, setEditingSourceSkill] = useState<string | null>(null)
  const [sourceContent, setSourceContent] = useState('')
  const [sourceFormat, setSourceFormat] = useState<'json' | 'md'>('json')
  const [newSkillName, setNewSkillName] = useState('')
  const [newSkillContent, setNewSkillContent] = useState('')
  const [newSkillFormat, setNewSkillFormat] = useState<'json' | 'md'>('json')
  const [reloading, setReloading] = useState(false)

  useEffect(() => { loadSkills(); loadPermissions(); loadTelemetry(); loadFsSkills(); loadSkillsDir() }, [])

  const loadSkills = async () => {
    try { const data = await skillList() as unknown as { skills?: SkillDefinition[] }; setSkills(data.skills || []) } catch (e) { console.error('[SkillsTab]', e) }
    finally { setLoading(false) }
  }
  const loadPermissions = async () => { try { const data = await skillPermissionsList() as { rules?: SkillPermissionRule[] }; setPermissions(data.rules || []) } catch (e) { console.error('[SkillsTab]', e) } }
  const loadTelemetry = async () => { try { const data = await skillTelemetryList({ limit: 20 }) as { events?: SkillTelemetryEvent[] }; setTelemetry(data.events || []) } catch (e) { console.error('[SkillsTab]', e) } }
  const loadFsSkills = async () => {
    try {
      const result = await fsSkillList() as { skills?: FsSkillInfo[] }
      setFsSkills(result.skills || [])
    } catch (e) { console.error('[SkillsTab:fsSkillList]', e) }
  }
  const loadSkillsDir = async () => {
    try {
      const path: string = await fsSkillsDirPath() as unknown as string
      setSkillsDirPath(path)
    } catch (e) { console.error('[SkillsTab:dirPath]', e) }
  }

  const showToast = (msg: string) => { setToast(msg); setTimeout(() => setToast(null), 2500) }

  const handleReload = async () => {
    setReloading(true)
    try {
      const result = await fsSkillReload() as { skills?: FsSkillInfo[]; added?: number; removed?: number }
      setFsSkills(result.skills || [])
      showToast(t('toolPanel.skills.toastReloaded', { added: result.added || 0, removed: result.removed || 0 }))
      loadSkills()
    } catch (e) { showToast(t('toolPanel.skills.toastReloaded', { added: 0, removed: 0 })) }
    finally { setReloading(false) }
  }

  const handleScan = async () => {
    setReloading(true)
    try {
      const result = await fsSkillScan() as { skills?: FsSkillInfo[] }
      setFsSkills(result.skills || [])
      showToast(t('toolPanel.skills.toastScanComplete', { count: result.skills?.length || 0 }))
    } catch (e) { showToast(t('toolPanel.skills.toastScanComplete', { count: 0 })) }
    finally { setReloading(false) }
  }

  const handleAddSkill = async () => {
    if (!newSkillName.trim()) return
    try {
      const ext = newSkillFormat === 'md' ? '.md' : '.json'
      await fsSkillAdd({ name: newSkillName.trim(), content: newSkillContent, format: newSkillFormat })
      showToast(t('toolPanel.skills.toastSkillAdded', { name: newSkillName }))
      setNewSkillName(''); setNewSkillContent(''); setShowAddSkill(false)
      loadFsSkills(); loadSkills()
    } catch (e) { showToast(t('toolPanel.skills.toastAddFailed', { error: e })) }
  }

  const handleRemoveSkill = async (name: string) => {
    if (!window.confirm(t('toolPanel.skills.deleteSkillConfirm', { name }))) return
    try {
      await fsSkillRemove({ name })
      showToast(t('toolPanel.skills.toastSkillDeleted', { name }))
      loadFsSkills(); loadSkills()
    } catch (e) { showToast(t('toolPanel.skills.toastDeleteFailed', { error: e })) }
  }

  const handleReadSource = async (name: string) => {
    setEditingSourceSkill(name)
    try {
      const result = await fsSkillReadSource({ name }) as { content?: string; format?: string }
      setSourceContent(result.content || '')
      setSourceFormat((result.format || 'json') as 'json' | 'md')
    } catch (e) { setSourceContent(t('toolPanel.skills.toastReadFailed', { error: e })) }
  }

  const handleUpdateSource = async () => {
    if (!editingSourceSkill) return
    try {
      await fsSkillUpdateSource({ name: editingSourceSkill, content: sourceContent, format: sourceFormat })
      showToast(t('toolPanel.skills.toastSourceSaved')); setEditingSourceSkill(null); setSourceContent('')
      loadFsSkills(); loadSkills()
    } catch (e) { showToast(t('toolPanel.skills.toastSaveSourceFailed', { error: e })) }
  }

  const handleExecute = async (name: string) => {
    if (executing) return; setExecuting(name); setExecuteResult(null)
    try {
      const result = await skillExecute({ skill_name: name }) as Record<string, unknown>
      setExecuteResult(result)
      showToast(t('toolPanel.skills.toastExecuted', { name, status: result.status as string }))
      loadTelemetry()
    } catch (e) { setExecuteResult({ error: String(e) }); showToast(t('toolPanel.skills.toastExecFailed', { error: e })) }
    finally { setExecuting(null) }
  }

  const handleAddPermission = async () => {
    if (!newRuleTool || !newRuleContent) return
    try { await skillPermissionAdd({ tool_name: newRuleTool, rule_content: newRuleContent, behavior: newRuleBehavior }); showToast(t('toolPanel.skills.toastRuleAdded')); setNewRuleTool(''); setNewRuleContent(''); loadPermissions() }
    catch (e) { showToast(t('toolPanel.skills.toastRuleError', { error: e })) }
  }

  const handleRegisterMcp = async () => {
    if (!mcpName || !mcpDesc) return
    try { await skillRegisterMcp({ name: mcpName, description: mcpDesc, prompt_template: mcpTemplate }); showToast(t('toolPanel.skills.toastMcpRegistered', { name: mcpName })); setMcpName(''); setMcpDesc(''); setMcpTemplate(''); loadSkills() }
    catch (e) { showToast(t('toolPanel.skills.toastRuleError', { error: e })) }
  }

  const allSkills = [...skills, ...fsSkills]
  const filtered = allSkills.filter(s =>
    !searchQuery || s.name.toLowerCase().includes(searchQuery.toLowerCase()) || s.description.toLowerCase().includes(searchQuery.toLowerCase())
  )

  const CONTEXT_COLORS: Record<string, string> = { Inline: 'bg-blue-500/10 text-blue-400 border-blue-500/20', Forked: 'bg-purple-500/10 text-purple-400 border-purple-500/20' }
  const SOURCE_ICONS: Record<string, string> = { Bundled: '📦', Local: '📁', Mcp: '🔌', Remote: '☁️', Plugin: '🧩', File: '📄' }

  if (loading) return <div className="flex justify-center py-12"><div className="w-7 h-7 border-2 border-primary-500 border-t-transparent rounded-full animate-spin"></div></div>

  return (
    <div className="space-y-4">
      {/* 子标签导航 */}
      <div className="flex gap-1 p-1 rounded-lg bg-dark-bg border border-dark-border">
        {[{ id: 'skills' as const, label: t('toolPanel.skills.tabList', { count: allSkills.length }) }, { id: 'permissions' as const, label: t('toolPanel.skills.tabPermissions', { count: permissions.length }) }, { id: 'telemetry' as const, label: t('toolPanel.skills.tabTelemetry', { count: telemetry.length }) }].map(tab => (
          <button key={tab.id} onClick={() => setActiveSubTab(tab.id)} className={`px-3 py-1.5 rounded-md text-xs transition-all ${activeSubTab === tab.id ? 'bg-primary-600 text-white' : 'text-dark-muted hover:text-dark-text'}`}>{tab.label}</button>
        ))}
      </div>

      {/* ===== 技能列表 Tab ===== */}
      {activeSubTab === 'skills' && (
        <>
          <div className="flex items-center gap-2">
            <div className="relative flex-1"><svg className="absolute left-3 top-1/2 -translate-y-1/2 w-4 h-4 text-dark-muted" fill="none" viewBox="0 0 24 24" stroke="currentColor"><path strokeLinecap="round" strokeLinejoin="round" d="M21 21l-6-6m2-5a7 7 0 11-14 0 7 7 0 0114 0z"/></svg>
              <input value={searchQuery} onChange={e => setSearchQuery(e.target.value)} placeholder={t('toolPanel.skills.searchPlaceholder')} className="w-full bg-dark-bg border border-dark-border rounded-lg pl-10 pr-4 py-2 text-sm text-dark-text focus:outline-none focus:border-primary-500 placeholder-dark-muted/30" />
            </div>
            <button onClick={handleScan} disabled={reloading} title={t('toolPanel.skills.scanTitle')} className="p-2 rounded-lg bg-dark-bg border border-dark-border text-dark-muted hover:text-primary-400 transition-colors disabled:opacity-50">
              <svg className={`w-4 h-4 ${reloading ? 'animate-spin' : ''}`} fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={2}><path strokeLinecap="round" strokeLinejoin="round" d="M21 21l-6-6m2-5a7 7 0 11-14 0 7 7 0 0114 0z"/></svg>
            </button>
            <button onClick={handleReload} disabled={reloading} title={t('toolPanel.skills.reloadTitle')} className={`px-2.5 py-2 rounded-lg text-xs border transition-colors ${reloading ? 'bg-dark-bg border-dark-border text-dark-muted' : 'bg-blue-600/10 text-blue-400 border-blue-500/20 hover:bg-blue-600/20'}`}>
              {reloading ? '⏳' : '🔄'} {t('toolPanel.agents.reload')}
            </button>
            <button onClick={() => setShowAddSkill(true)} title={t('toolPanel.skills.addTitle')} className="px-2.5 py-2 rounded-lg bg-primary-600/10 text-primary-300 border border-primary-500/20 hover:bg-primary-600/20 text-xs transition-colors">{t('toolPanel.skills.addBtn')}</button>
          </div>

          {skillsDirPath && <div className="text-[10px] text-dark-muted/60 font-mono truncate">📂 {skillsDirPath}</div>}
          {toast && <div className="px-3 py-2 rounded-lg bg-primary-600/10 border border-primary-500/20 text-xs text-primary-300">{toast}</div>}

          {/* 文件来源技能 vs 内置技能 分组标题 */}
          {fsSkills.length > 0 && <div className="text-[11px] font-medium text-cyan-400/80 mt-1">{t('toolPanel.skills.diskSkills', { count: fsSkills.length })}</div>}
          <div className="grid grid-cols-2 gap-3 max-h-[400px] overflow-y-auto">
            {filtered.filter(s => fsSkills.some(fs => fs.name === s.name)).map((s: SkillDefinition) => (
              <div key={`fs-${s.name}`} className={`p-3 rounded-xl border bg-dark-bg hover:border-cyan-500/30 transition-all relative group`}>
                <div className="absolute top-2 right-2 opacity-0 group-hover:opacity-100 flex gap-1 transition-opacity">
                  <button onClick={() => handleReadSource(s.name)} className="p-1 rounded bg-dark-surface text-dark-muted hover:text-yellow-400" title={t('toolPanel.skills.editSourceTitle')}>✏️</button>
                  <button onClick={() => handleRemoveSkill(s.name)} className="p-1 rounded bg-dark-surface text-dark-muted hover:text-red-400" title={t('toolPanel.agents.deleteTitle')}>🗑️</button>
                </div>
                <div className="flex items-start justify-between mb-1.5 pr-16">
                  <div className="flex items-center gap-2">
                    <span>{SOURCE_ICONS[String(s.source ?? 'File')] || SOURCE_ICONS['File'] || '📄'}</span>
                    <span className="text-sm font-semibold text-dark-text">{s.name}</span>
                  </div>
                  <span className={`px-1.5 py-0.5 rounded text-[9px] border ${CONTEXT_COLORS[String(s.context ?? 'Inline')] || CONTEXT_COLORS['Inline']}`}>{String(s.context ?? '')}</span>
                </div>
                <p className="text-[11px] text-dark-muted line-clamp-2 mb-2">{s.description}</p>
                {s.file_path ? <div className="text-[9px] text-dark-muted/40 font-mono truncate mb-1">{String(s.file_path)}</div> : null}
                <div className="flex items-center justify-between">
                  <div className="flex gap-1 flex-wrap">{(Array.isArray(s.allowed_tools) ? s.allowed_tools : []).slice(0, 3).map((t: string) => (<span key={t} className="px-1 py-0.5 rounded bg-dark-surface text-[9px] text-dark-muted">{t}</span>))}</div>
                  <button onClick={() => handleExecute(s.name)} disabled={executing === s.name}
                    className="px-2 py-1 rounded text-[10px] bg-primary-600 hover:bg-primary-500 text-white disabled:opacity-50 flex items-center gap-1">
                    {executing === s.name ? <><span className="w-3 h-3 border border-white/30 border-t-transparent rounded-full animate-spin"></span></> : '▶'}
                  </button>
                </div>
              </div>
            ))}
          </div>

          {skills.length > 0 && <div className="text-[11px] font-medium text-primary-400/80 mt-3">{t('toolPanel.skills.builtinSkills', { count: skills.length })}</div>}
          <div className="grid grid-cols-2 gap-3 max-h-[400px] overflow-y-auto">
            {filtered.filter(s => !fsSkills.some(fs => fs.name === s.name)).map((s: any) => (
              <div key={`builtin-${s.name}`} className="p-3 rounded-xl border bg-dark-bg hover:border-dark-border/60 transition-all">
                <div className="flex items-start justify-between mb-1.5">
                  <div className="flex items-center gap-2">
                    <span>{SOURCE_ICONS[String(s.source ?? '')] || '📦'}</span>
                    <span className="text-sm font-semibold text-dark-text">{s.name}</span>
                  </div>
                  <span className={`px-1.5 py-0.5 rounded text-[9px] border ${CONTEXT_COLORS[String(s.context ?? 'Inline')] || CONTEXT_COLORS['Inline']}`}>{String(s.context ?? '')}</span>
                </div>
                <p className="text-[11px] text-dark-muted line-clamp-2 mb-2">{s.description}</p>
                {s.when_to_use && <p className="text-[10px] text-primary-400/60 mb-2 italic">"{s.when_to_use}"</p>}
                <div className="flex items-center justify-between">
                  <div className="flex gap-1 flex-wrap">{(Array.isArray(s.allowed_tools) ? s.allowed_tools : []).slice(0, 3).map((t: string) => (<span key={t} className="px-1 py-0.5 rounded bg-dark-surface text-[9px] text-dark-muted">{t}</span>))}</div>
                  <button onClick={() => handleExecute(s.name)} disabled={executing === s.name}
                    className="px-2 py-1 rounded text-[10px] bg-primary-600 hover:bg-primary-500 text-white disabled:opacity-50 flex items-center gap-1">
                    {executing === s.name ? <><span className="w-3 h-3 border border-white/30 border-t-transparent rounded-full animate-spin"></span>{t('toolPanel.skills.executing')}</> : t('toolPanel.skills.executeBtn')}
                  </button>
                </div>
              </div>
            ))}
          </div>
          {filtered.length === 0 && <div className="text-center py-8 text-sm text-dark-muted">{t('toolPanel.skills.noMatch')}</div>}

          {/* MCP 注册表单 */}
          <div className="mt-4 p-3 rounded-xl border border-dashed border-dark-border space-y-2">
            <h4 className="text-xs font-semibold text-dark-text flex items-center gap-1">{t('toolPanel.skills.registerMcpHeader')}</h4>
            <div className="grid grid-cols-2 gap-2">
              <input value={mcpName} onChange={e => setMcpName(e.target.value)} placeholder={t('toolPanel.skills.skillNamePlaceholder')} className="bg-dark-bg border border-dark-border rounded px-2 py-1.5 text-xs text-dark-text" />
              <input value={mcpDesc} onChange={e => setMcpDesc(e.target.value)} placeholder={t('toolPanel.skills.descPlaceholder')} className="bg-dark-bg border border-dark-border rounded px-2 py-1.5 text-xs text-dark-text" />
            </div>
            <textarea value={mcpTemplate} onChange={e => setMcpTemplate(e.target.value)} placeholder={t('toolPanel.skills.templatePlaceholder')} rows={2} className="w-full bg-dark-bg border border-dark-border rounded px-2 py-1.5 text-xs text-dark-text font-mono" />
            <button onClick={handleRegisterMcp} disabled={!mcpName} className="px-3 py-1.5 rounded-lg text-xs bg-cyan-600/10 text-cyan-400 border border-cyan-500/20 hover:bg-cyan-600/20 disabled:opacity-40">{t('toolPanel.skills.registerMcpBtn')}</button>
          </div>

          {/* 执行结果 */}
          {executeResult && (
            <div className="p-3 rounded-xl border border-dark-border bg-dark-bg space-y-2">
              <h4 className="text-xs font-semibold text-dark-text">{t('toolPanel.skills.resultHeader')}</h4>
              <pre className="text-[10px] text-green-400 bg-dark-surface rounded p-2 overflow-auto max-h-32 font-mono whitespace-pre-wrap">{JSON.stringify(executeResult, null, 2)}</pre>
            </div>
          )}
        </>
      )}

      {/* ===== 权限管理 Tab ===== */}
      {activeSubTab === 'permissions' && (
        <div className="space-y-4">
          <div className="p-3 rounded-xl border border-dark-border bg-dark-bg space-y-2">
            <h4 className="text-xs font-semibold text-dark-text">{t('toolPanel.skills.addRuleHeader')}</h4>
            <div className="grid grid-cols-3 gap-2">
              <input value={newRuleTool} onChange={e => setNewRuleTool(e.target.value)} placeholder={t('toolPanel.skills.toolPlaceholder')} className="bg-dark-surface border border-dark-border rounded px-2 py-1.5 text-xs text-dark-text" />
              <input value={newRuleContent} onChange={e => setNewRuleContent(e.target.value)} placeholder={t('toolPanel.skills.ruleContentPlaceholder')} className="bg-dark-surface border border-dark-border rounded px-2 py-1.5 text-xs text-dark-text" />
              <select value={newRuleBehavior} onChange={e => setNewRuleBehavior(e.target.value as 'allow' | 'deny' | 'ask')} className="bg-dark-surface border border-dark-border rounded px-2 py-1.5 text-xs text-dark-text">
                <option value="ask">{t('toolPanel.skills.askOption')}</option>
                <option value="allow">{t('toolPanel.skills.allowOption')}</option>
                <option value="deny">{t('toolPanel.skills.denyOption')}</option>
              </select>
            </div>
            <button onClick={handleAddPermission} disabled={!newRuleTool || !newRuleContent} className="px-3 py-1.5 rounded-lg text-xs bg-primary-600 text-white disabled:opacity-40">{t('toolPanel.skills.addRuleBtn')}</button>
          </div>
          <div className="space-y-1">
            {permissions.map((r: any, i: number) => (
              <div key={i} className="flex items-center justify-between p-2 rounded-lg bg-dark-bg border border-dark-border text-xs">
                <div className="flex items-center gap-2">
                  <span className={`px-1.5 py-0.5 rounded ${r.behavior === 'allow' ? 'bg-green-500/10 text-green-400' : r.behavior === 'deny' ? 'bg-red-500/10 text-red-400' : 'bg-yellow-500/10 text-yellow-400'}`}>
                    {r.behavior.toUpperCase()}
                  </span>
                  <code className="text-dark-text">{r.rule_content}</code>
                </div>
                <button onClick={() => skillPermissionRemove({ index: i }).then(loadPermissions)} className="text-red-400 hover:text-red-300">{t('toolPanel.skills.deleteBtn')}</button>
              </div>
            ))}
            {permissions.length === 0 && <div className="text-center py-6 text-sm text-dark-muted">{t('toolPanel.skills.noPermissions')}</div>}
          </div>
        </div>
      )}

      {/* ===== 遥测日志 Tab ===== */}
      {activeSubTab === 'telemetry' && (
        <div className="space-y-2">
          <div className="flex items-center justify-between">
            <h4 className="text-xs font-semibold text-dark-text">{t('toolPanel.skills.recentCallsHeader')}</h4>
            <button onClick={() => skillTelemetryClear().then(() => { setTelemetry([]); showToast(t('toolPanel.skills.toastCleared')) })} className="text-[10px] text-red-400 hover:text-red-300">{t('toolPanel.skills.clearAllBtn')}</button>
          </div>
          <div className="max-h-[350px] overflow-y-auto space-y-1">
            {telemetry.map((ev: any, i: number) => (
              <div key={i} className="flex items-center gap-2 p-2 rounded-lg bg-dark-bg border border-dark-border text-[11px]">
                <span className="font-mono text-primary-400 shrink-0">{ev.skill_name}</span>
                <span className={`shrink-0 px-1.5 py-0.5 rounded text-[9px] ${ev.status === 'success' ? 'bg-green-500/10 text-green-400' : 'bg-red-500/10 text-red-400'}`}>
                  {ev.status}
                </span>
                <span className="text-dark-muted">{ev.execution_context}</span>
                <span className="ml-auto text-dark-muted/50">{ev.duration_ms}ms · depth:{ev.query_depth}</span>
              </div>
            ))}
            {telemetry.length === 0 && <div className="text-center py-8 text-sm text-dark-muted">{t('toolPanel.skills.noCalls')}</div>}
          </div>
        </div>
      )}

      <AddSkillModal
        open={showAddSkill}
        skillName={newSkillName} setSkillName={setNewSkillName}
        skillContent={newSkillContent} setSkillContent={setNewSkillContent}
        skillFormat={newSkillFormat} setSkillFormat={setNewSkillFormat}
        onClose={() => setShowAddSkill(false)}
        onConfirm={handleAddSkill}
      />

      <EditSourceModal
        open={!!editingSourceSkill}
        skillName={editingSourceSkill || ''}
        sourceContent={sourceContent} setSourceContent={setSourceContent}
        sourceFormat={sourceFormat} setSourceFormat={setSourceFormat}
        onClose={() => { setEditingSourceSkill(null); setSourceContent('') }}
        onConfirm={handleUpdateSource}
      />
    </div>
  )
}

// ==================== Tools 浏览子面板 ====================

function ToolsTab() {
  const { t } = useTranslation()
  const [tools, setTools] = useState<ToolDefinition[]>([])
  const [searchQuery, setSearchQuery] = useState('')
  const [selectedTool, setSelectedTool] = useState<ToolDefinition | null>(null)
  const [loading, setLoading] = useState(true)

  useEffect(() => {
    toolListAll().then(data => {
      try {
        const parsed = typeof data === 'string' ? JSON.parse(data) : data
        const output = parsed.output || ''
        const match = output.match(/\ud83d\udd27\s*[\w\u4e00-\u9fff]+\s*\((\d+)\)/)
        if (match) {
          setTools(getAllTools())
        } else {
          setTools(getAllTools())
        }
      } catch { setTools(getAllTools()) }
    }).catch(() => setTools(getAllTools())).finally(() => setLoading(false))
  }, [])

  const categories = [
    { key: 'file', label: t('toolPanel.tools.catFile'), color: 'from-blue-500 to-blue-700' },
    { key: 'shell', label: t('toolPanel.tools.catShell'), color: 'from-orange-500 to-orange-700' },
    { key: 'search', label: t('toolPanel.tools.catSearch'), color: 'from-purple-500 to-purple-700' },
    { key: 'web', label: t('toolPanel.tools.catWeb'), color: 'from-cyan-500 to-cyan-700' },
    { key: 'agent', label: t('toolPanel.tools.catAgent'), color: 'from-pink-500 to-pink-700' },
    { key: 'misc', label: t('toolPanel.tools.catMisc'), color: 'from-gray-500 to-gray-700' },
  ]

  const getCategory = (name: string) => {
    if (['Read','Edit','Write'].includes(name)) return 'file'
    if (['Bash'].includes(name)) return 'shell'
    if (['Glob','Grep'].includes(name)) return 'search'
    if (['WebFetch','WebSearch'].includes(name)) return 'web'
    if (['Agent','TodoWrite','TaskCreate','TaskList','Workflow','Skill','EnterPlanMode','ExitPlanMode'].includes(name)) return 'agent'
    return 'misc'
  }

  const filtered = tools.filter(t =>
    !searchQuery || t.name.toLowerCase().includes(searchQuery.toLowerCase()) || t.description.toLowerCase().includes(searchQuery.toLowerCase())
  )

  if (loading) return <div className="flex justify-center py-12"><div className="w-7 h-7 border-2 border-primary-500 border-t-transparent rounded-full animate-spin"></div></div>

  return (
    <div className="space-y-5">
      <div><h3 className="text-base font-semibold text-dark-text">{t('toolPanel.tools.header')}</h3><p className="text-xs text-dark-muted mt-0.5">{t('toolPanel.tools.desc')}</p></div>

      {/* 搜索 + 统计 */}
      <div className="flex items-center gap-3">
        <div className="relative flex-1"><svg className="absolute left-3 top-1/2 -translate-y-1/2 w-4 h-4 text-dark-muted" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={2}><path strokeLinecap="round" strokeLinejoin="round" d="M21 21l-6-6m2-5a7 7 0 11-14 0 7 7 0 0114 0z" /></svg>
          <input value={searchQuery} onChange={e => setSearchQuery(e.target.value)} placeholder={t('toolPanel.tools.searchPlaceholder')} className="w-full bg-dark-bg border border-dark-border rounded-lg pl-10 pr-4 py-2 text-sm text-dark-text focus:outline-none focus:border-primary-500 placeholder-dark-muted/30" />
        </div>
        <span className="text-xs text-dark-muted px-2 py-1.5 rounded bg-dark-bg border border-dark-border whitespace-nowrap">{t('toolPanel.tools.toolCount', { count: filtered.length })}</span>
      </div>

      {/* 分类统计 */}
      <div className="flex gap-2 flex-wrap">
        {categories.map(cat => {
          const count = filtered.filter(t => getCategory(t.name) === cat.key).length
          if (count === 0) return null
          return <span key={cat.key} className="px-2.5 py-1 rounded-lg bg-dark-bg border border-dark-border text-[11px] text-dark-muted"><span className={`inline-block w-1.5 h-1.5 rounded-full mr-1.5 bg-gradient-to-r ${cat.color}`}/>{cat.label}: {count}</span>
        })}
      </div>

      {/* 工具卡片列表 */}
      <div className="space-y-2.5">
        {filtered.map(tool => {
          const cat = getCategory(tool.name)
          const catInfo = categories.find(c => c.key === cat)!
          return (
            <div key={tool.name} onClick={() => setSelectedTool(tool)} className="p-3.5 rounded-xl border border-dark-border bg-dark-bg hover:border-primary-500/30 cursor-pointer transition-all group">
              <div className="flex items-start gap-3">
                <div className={`w-9 h-9 rounded-lg bg-gradient-to-br ${catInfo.color} flex items-center justify-center shrink-0 shadow-lg opacity-80 group-hover:opacity-100 transition-opacity`}>
                  <ToolIcon name={tool.name} />
                </div>
                <div className="flex-1 min-w-0">
                  <div className="flex items-center gap-2 mb-0.5">
                    <code className="text-sm font-semibold text-primary-300">{tool.name}</code>
                    <span className="px-1.5 py-0.5 rounded text-[10px] bg-dark-border/50 text-dark-muted">{catInfo.label}</span>
                  </div>
                  <p className="text-xs text-dark-muted line-clamp-1">{tool.description}</p>
                  <div className="flex gap-1 mt-1.5">
                    {Object.keys(tool.input_schema.properties || {}).slice(0, 4).map(p => (
                      <span key={p} className="px-1.5 py-0.5 rounded bg-dark-surface text-[10px] text-dark-muted font-mono">{p}{tool.input_schema.required?.includes(p) ? '*' : ''}</span>
                    ))}
                    {Object.keys(tool.input_schema.properties || {}).length > 4 && <span className="px-1.5 py-0.5 rounded bg-dark-surface text-[10px] text-dark-muted">+{Object.keys(tool.input_schema.properties || {}).length - 4}</span>}
                  </div>
                </div>
                <svg className="w-4 h-4 text-dark-muted opacity-0 group-hover:opacity-100 shrink-0 transition-opacity" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={2}><path strokeLinecap="round" strokeLinejoin="round" d="M9 5l7 7-7 7" /></svg>
              </div>
            </div>
          )
        })}
      </div>

      <ToolDetailModal
        tool={selectedTool}
        onClose={() => setSelectedTool(null)}
        categories={categories}
        getCategory={getCategory}
      />
    </div>
  )
}

// ==================== 添加技能弹窗组件 ====================

function AddSkillModal({ open, skillName, setSkillName, skillContent, setSkillContent, skillFormat, setSkillFormat, onClose, onConfirm }: {
  open: boolean; skillName: string; setSkillName: (v: string) => void;
  skillContent: string; setSkillContent: (v: string) => void;
  skillFormat: 'json' | 'md'; setSkillFormat: (v: 'json' | 'md') => void;
  onClose: () => void; onConfirm: () => void;
}) {
  const { t } = useTranslation()
  if (!open) return null
  return createPortal(
    <div className="fixed inset-0 z-[60] flex items-center justify-center bg-black/50 backdrop-blur-sm" onClick={onClose}>
      <div className="bg-dark-surface border border-dark-border rounded-2xl shadow-2xl w-[650px] max-h-[80vh] overflow-y-auto p-6 animate-fade-in" onClick={e => e.stopPropagation()}>
        <h3 className="text-base font-bold text-dark-text mb-4">{t('toolPanel.skills.addModalTitle')}</h3>
        <div className="space-y-3">
          <div className="flex gap-3">
            <div className="flex-1"><label className="block text-xs font-medium text-dark-text mb-1">{t('toolPanel.skills.skillNameLabel')}</label><input value={skillName} onChange={e => setSkillName(e.target.value)} placeholder="my-custom-skill" className="w-full bg-dark-bg border border-dark-border rounded-lg px-3 py-2 text-sm text-dark-text focus:outline-none focus:border-primary-500 font-mono" /></div>
            <div className="w-32"><label className="block text-xs font-medium text-dark-text mb-1">{t('toolPanel.skills.formatLabel')}</label>
              <select value={skillFormat} onChange={e => setSkillFormat(e.target.value as 'json' | 'md')} className="w-full bg-dark-bg border border-dark-border rounded-lg px-3 py-2 text-sm text-dark-text">
                <option value="json">{t('toolPanel.skills.jsonFormat')}</option>
                <option value="md">{t('toolPanel.skills.mdFormat')}</option>
              </select>
            </div>
          </div>
          <div>
            <label className="block text-xs font-medium text-dark-text mb-1">{skillFormat === 'json' ? t('toolPanel.skills.skillContentLabel') : t('toolPanel.skills.skillContentMdLabel')}</label>
            {skillFormat === 'json' ? (
              <textarea value={skillContent} onChange={e => setSkillContent(e.target.value)} placeholder='{"name": "my-skill", "description": "...", "prompt_template": "..."}' rows={12} className="w-full bg-dark-bg border border-dark-border rounded-lg px-3 py-2 text-xs text-dark-text font-mono focus:outline-none focus:border-primary-500 resize-none" />
            ) : (
              <textarea value={skillContent} onChange={e => setSkillContent(e.target.value)} placeholder={'# Skill Name\n\n> Skill Description\n\n---\n\nPrompt template content...'} rows={12} className="w-full bg-dark-bg border border-dark-border rounded-lg px-3 py-2 text-xs text-dark-text font-mono focus:outline-none focus:border-primary-500 resize-none" />
            )}
          </div>
          <div className="flex justify-end gap-2 pt-2 border-t border-dark-border/50">
            <button onClick={onClose} className="px-4 py-2 rounded-lg border border-dark-border text-sm text-dark-muted hover:text-dark-text hover:bg-dark-border/30 transition-colors">{t('toolPanel.agents.cancel')}</button>
            <button onClick={onConfirm} disabled={!skillName.trim()} className="px-4 py-2 rounded-lg bg-primary-600 hover:bg-primary-500 text-white text-sm font-medium transition-colors disabled:opacity-40">{t('toolPanel.skills.saveToDisk')}</button>
          </div>
        </div>
      </div>
    </div>,
    document.body
  )
}

// ==================== 编辑源码弹窗组件 ====================

function EditSourceModal({ open, skillName, sourceContent, setSourceContent, sourceFormat, setSourceFormat, onClose, onConfirm }: {
  open: boolean; skillName: string;
  sourceContent: string; setSourceContent: (v: string) => void;
  sourceFormat: 'json' | 'md'; setSourceFormat: (v: 'json' | 'md') => void;
  onClose: () => void; onConfirm: () => void;
}) {
  const { t } = useTranslation()
  if (!open) return null
  return createPortal(
    <div className="fixed inset-0 z-[60] flex items-center justify-center bg-black/50 backdrop-blur-sm" onClick={onClose}>
      <div className="bg-dark-surface border border-dark-border rounded-2xl shadow-2xl w-[700px] max-h-[85vh] overflow-y-auto p-6 animate-fade-in" onClick={e => e.stopPropagation()}>
        <div className="flex items-center justify-between mb-4">
          <h3 className="text-base font-bold text-dark-text">{t('toolPanel.skills.editSourceModalTitle', { name: skillName })}</h3>
          <div className="flex items-center gap-2">
            <select value={sourceFormat} onChange={e => setSourceFormat(e.target.value as 'json' | 'md')} className="bg-dark-bg border border-dark-border rounded px-2 py-1 text-xs text-dark-text">
              <option value="json">JSON</option>
              <option value="md">Markdown</option>
            </select>
            <button onClick={onClose} className="p-1.5 rounded-lg hover:bg-dark-border/50 text-dark-muted"><svg className="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={2}><path strokeLinecap="round" strokeLinejoin="round" d="M6 18L18 6M6 6l12 12" /></svg></button>
          </div>
        </div>
        <textarea value={sourceContent} onChange={e => setSourceContent(e.target.value)} rows={20} spellCheck={false} className="w-full bg-dark-bg border border-dark-border rounded-lg px-3 py-2 text-xs text-green-400 font-mono focus:outline-none focus:border-primary-500 resize-none whitespace-pre" />
        <div className="flex justify-end gap-2 pt-3 mt-3 border-t border-dark-border/50">
          <button onClick={onClose} className="px-4 py-2 rounded-lg border border-dark-border text-sm text-dark-muted hover:text-dark-text hover:bg-dark-border/30 transition-colors">{t('toolPanel.agents.cancel')}</button>
          <button onClick={onConfirm} className="px-4 py-2 rounded-lg bg-primary-600 hover:bg-primary-500 text-white text-sm font-medium transition-colors">{t('toolPanel.skills.saveChanges')}</button>
        </div>
      </div>
    </div>,
    document.body
  )
}

// ==================== 工具详情弹窗组件 ====================

function ToolDetailModal({ tool, onClose, categories, getCategory }: {
  tool: ToolDefinition | null; onClose: () => void;
  categories: { key: string; label: string; color: string }[];
  getCategory: (name: string) => string;
}) {
  const { t } = useTranslation()
  if (!tool) return null
  return createPortal(
    <div className="fixed inset-0 z-[60] flex items-center justify-center bg-black/50 backdrop-blur-sm" onClick={onClose}>
      <div className="bg-dark-surface border border-dark-border rounded-2xl shadow-2xl w-[650px] max-h-[80vh] overflow-y-auto p-6 animate-fade-in" onClick={e => e.stopPropagation()}>
        <div className="flex items-center justify-between mb-4">
          <div className="flex items-center gap-3">
            <code className="text-lg font-bold text-primary-300">{tool.name}</code>
            <span className="px-2 py-0.5 rounded bg-dark-border text-xs text-dark-muted">{categories.find(c => c.key === getCategory(tool.name))?.label}</span>
          </div>
          <button onClick={onClose} className="p-1.5 rounded-lg hover:bg-dark-border/50 text-dark-muted"><svg className="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={2}><path strokeLinecap="round" strokeLinejoin="round" d="M6 18L18 6M6 6l12 12" /></svg></button>
        </div>
        <p className="text-sm text-dark-text mb-4 leading-relaxed">{tool.description}</p>
        <h4 className="text-xs font-semibold text-dark-text uppercase tracking-wider mb-2">{t('toolPanel.tools.paramSchema')}</h4>
        <pre className="p-4 rounded-lg bg-dark-bg border border-dark-border text-xs text-dark-muted overflow-auto font-mono leading-relaxed">{JSON.stringify(tool.input_schema, null, 2)}</pre>
      </div>
    </div>,
    document.body
  )
}

// ==================== Tasks 任务管理子面板 ====================

function TasksTab() {
  const { t } = useTranslation()
  const [tasks, setTasks] = useState<TaskItem[]>([])
  const [todos, setTodos] = useState<TodoItem[]>([])
  const [loading, setLoading] = useState(true)
  const [toast, setToast] = useState<string | null>(null)

  const loadData = useCallback(async () => {
    try {
      const [taskResult, todoResult] = await Promise.all([
        toolTaskList({ statusFilter: '' }).catch(() => ({ tool: 'TaskList', success: true, output: '' })),
        toolTodoGet().catch(() => ({ tool: 'TodoGet', success: true, output: '' })),
      ])
      if (Array.isArray(taskResult)) setTasks(taskResult)
      if ((todoResult as unknown as { todos?: TodoItem[] })?.todos) setTodos((todoResult as unknown as { todos: TodoItem[] }).todos)
    } catch (e) { console.error('[TasksTab]', e) }
    finally { setLoading(false) }
  }, [])

  useEffect(() => { loadData() }, [loadData])

  const showToast = (msg: string) => { setToast(msg); setTimeout(() => setToast(null), 2000) }

  const handleCreateTask = async () => {
    const prompt = window.prompt(t('toolPanel.tasks.inputTaskDesc'))
    if (!prompt?.trim()) return
    try {
      await toolTaskCreate({ prompt: prompt.trim(), description: '' })
      showToast(t('toolPanel.tasks.taskCreated')); loadData()
    } catch { showToast(t('toolPanel.tasks.taskCreateFailed')) }
  }

  const handleUpdateTodos = async (newTodos: TodoItem[]) => {
    setTodos(newTodos)
    try { await toolTodoWrite({ todos: newTodos }) }
    catch { showToast(t('toolPanel.tasks.saveFailed')) }
  }

  if (loading) return <div className="flex justify-center py-12"><div className="w-7 h-7 border-2 border-primary-500 border-t-transparent rounded-full animate-spin"></div></div>

  const statusColors: Record<string, string> = { pending: 'text-yellow-400 bg-yellow-400/10', running: 'text-blue-400 bg-blue-400/10', completed: 'text-green-400 bg-green-400/10', failed: 'text-red-400 bg-red-400/10' }
  const todoStatusIcons: Record<string, string> = { pending: '⬜', in_progress: '🔄', completed: '✅' }

  return (
    <div className="space-y-5">
      <div className="flex items-center justify-between">
        <div><h3 className="text-base font-semibold text-dark-text">{t('toolPanel.tasks.header')}</h3><p className="text-xs text-dark-muted mt-0.5">{t('toolPanel.tasks.desc')}</p></div>
        <button onClick={handleCreateTask} className="px-3 py-1.5 rounded-lg bg-primary-600 hover:bg-primary-500 text-white text-xs font-medium transition-colors flex items-center gap-1.5">
          <svg className="w-3.5 h-3.5" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={2}><path strokeLinecap="round" strokeLinejoin="round" d="M12 4v16m8-8H4" /></svg>{t('toolPanel.tasks.createBtn')}
        </button>
      </div>
      {toast && <div className="px-3 py-2 rounded-lg bg-primary-600/10 border border-primary-500/20 text-xs text-primary-300">{toast}</div>}

      {/* Todo 列表 */}
      <div className="p-4 rounded-xl border border-dark-border bg-dark-bg">
        <h4 className="text-xs font-semibold text-dark-text uppercase tracking-wider mb-3 flex items-center gap-2">{t('toolPanel.tasks.todoHeader', { count: todos.length })}</h4>
        {todos.length === 0 ? <p className="text-xs text-dark-muted text-center py-4">{t('toolPanel.tasks.noTodo')}</p> :
          <div className="space-y-1.5">
            {todos.map((todo, i) => (
              <div key={i} className="flex items-center gap-2.5 p-2 rounded-lg hover:bg-dark-surface/50 transition-colors group">
                <select value={todo.status} onChange={e => { const u = [...todos]; u[i] = { ...todo, status: e.target.value as 'pending' | 'in_progress' | 'completed' }; handleUpdateTodos(u) }} className="text-base bg-transparent cursor-pointer outline-none">
                  <option value="pending">⬜</option><option value="in_progress">🔄</option><option value="completed">✅</option>
                </select>
                <span className={`flex-1 text-xs ${todo.status === 'completed' ? 'line-through text-dark-muted' : 'text-dark-text'}`}>{todo.content}</span>
                <span className={`px-1.5 py-0.5 rounded text-[10px] ${todo.priority === 'high' ? 'bg-red-500/10 text-red-400' : todo.priority === 'medium' ? 'bg-yellow-500/10 text-yellow-400' : 'bg-dark-border text-dark-muted'}`}>{todo.priority}</span>
                <button onClick={() => handleUpdateTodos(todos.filter((_, j) => j !== i))} className="opacity-0 group-hover:opacity-100 text-dark-muted hover:text-red-400 transition-all"><svg className="w-3.5 h-3.5" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={2}><path strokeLinecap="round" strokeLinejoin="round" d="M6 18L18 6M6 6l12 12" /></svg></button>
              </div>
            ))}
          </div>}
      </div>

      {/* 后台任务列表 */}
      <div className="p-4 rounded-xl border border-dark-border bg-dark-bg">
        <h4 className="text-xs font-semibold text-dark-text uppercase tracking-wider mb-3 flex items-center gap-2">{t('toolPanel.tasks.bgTasksHeader', { count: tasks.length })}</h4>
        {tasks.length === 0 ? <p className="text-xs text-dark-muted text-center py-4">{t('toolPanel.tasks.noBgTasks')}</p> :
          <div className="space-y-2">
            {tasks.map(task => (
              <div key={task.id} className="p-3 rounded-lg bg-dark-surface/30 border border-dark-border/50">
                <div className="flex items-center justify-between mb-1">
                  <span className="text-xs font-medium text-dark-text truncate flex-1 mr-2">{(task.prompt || task.title || '').slice(0, 80)}</span>
                  <span className={`px-1.5 py-0.5 rounded text-[10px] font-medium ${statusColors[task.status] || 'text-dark-muted bg-dark-border'}`}>{task.status}</span>
                </div>
                {task.result && <pre className="mt-1.5 text-[11px] text-dark-muted bg-dark-bg p-2 rounded max-h-20 overflow-auto font-mono">{task.result.slice(0, 300)}</pre>}
              </div>
            ))}
          </div>}
      </div>
    </div>
  )
}

// ==================== Cron 定时任务子面板 ====================

function CronTab() {
  const { t } = useTranslation()
  const [crons, setCrons] = useState<CronJob[]>([])
  const [loading, setLoading] = useState(true)
  const [showCreate, setShowCreate] = useState(false)
  const [toast, setToast] = useState<string | null>(null)

  useEffect(() => {
    toolScheduleList().then((data: any) => {
      if (data?.output) {
        try { const parsed = JSON.parse(data.output); if (Array.isArray(parsed)) setCrons(parsed) }
        catch (e) { console.error('[CronTab:parse]', e) }
      }
    }).catch((e) => { console.error(e) }).finally(() => setLoading(false))
  }, [])

  const showToast = (msg: string) => { setToast(msg); setTimeout(() => setToast(null), 2000) }

  const handleRegister = async (job: CronJob) => {
    try {
      await toolScheduleCron({ name: job.name || '', schedule: job.schedule, task: job.task || job.command || '', enabled: job.enabled })
      setCrons([...crons, job]); setShowCreate(false); showToast(t('toolPanel.cron.registered', { name: job.name }))
    } catch { showToast(t('toolPanel.cron.registerFailed')) }
  }

  if (loading) return <div className="flex justify-center py-12"><div className="w-7 h-7 border-2 border-primary-500 border-t-transparent rounded-full animate-spin"></div></div>

  return (
    <div className="space-y-5">
      <div className="flex items-center justify-between">
        <div><h3 className="text-base font-semibold text-dark-text">{t('toolPanel.cron.header')}</h3><p className="text-xs text-dark-muted mt-0.5">{t('toolPanel.cron.desc')}</p></div>
        <button onClick={() => setShowCreate(!showCreate)} className="px-3 py-1.5 rounded-lg bg-primary-600 hover:bg-primary-500 text-white text-xs font-medium transition-colors flex items-center gap-1.5">
          <svg className="w-3.5 h-3.5" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={2}><path strokeLinecap="round" strokeLinejoin="round" d="M12 4v16m8-8H4" /></svg>{t('toolPanel.cron.createBtn')}
        </button>
      </div>
      {toast && <div className="px-3 py-2 rounded-lg bg-primary-600/10 border border-primary-500/20 text-xs text-primary-300">{toast}</div>}

      {/* 创建表单 */}
      {showCreate && (
        <div className="p-4 rounded-xl border border-primary-500/20 bg-primary-600/5 space-y-3">
          <h4 className="text-xs font-semibold text-primary-400">{t('toolPanel.cron.newJobHeader')}</h4>
          <CronForm onSubmit={handleRegister} onCancel={() => setShowCreate(false)} />
        </div>
      )}

      {/* Cron 列表 */}
      <div className="space-y-2">
        {crons.length === 0 ? <div className="text-center py-12 text-sm text-dark-muted">{t('toolPanel.cron.noCronJobs')}</div> :
          crons.map((job, i) => (
            <div key={i} className="p-3.5 rounded-xl border border-dark-border bg-dark-bg flex items-center gap-4">
              <div className={`w-2 h-2 rounded-full ${job.enabled ? 'bg-green-400 shadow-sm shadow-green-400/50' : 'bg-dark-border'}`} />
              <div className="flex-1 min-w-0">
                <div className="flex items-center gap-2"><span className="text-sm font-medium text-dark-text">{job.name}</span><code className="text-[11px] text-primary-300 bg-primary-600/10 px-1.5 py-0.5 rounded font-mono">{job.schedule}</code></div>
                <p className="text-xs text-dark-muted mt-0.5 truncate">{job.task}</p>
              </div>
              <span className={`px-1.5 py-0.5 rounded text-[10px] ${job.enabled ? 'bg-green-500/10 text-green-400' : 'bg-dark-border text-dark-muted'}`}>{job.enabled ? t('toolPanel.cron.running') : t('toolPanel.cron.paused')}</span>
            </div>
          ))
        }
      </div>

      {/* Cron 表达式帮助 */}
      <div className="p-4 rounded-xl border border-dark-border bg-dark-bg/50">
        <h4 className="text-xs font-semibold text-dark-muted mb-2">{t('toolPanel.cron.cronFormatHeader')}</h4>
        <div className="grid grid-cols-2 gap-x-6 gap-y-1 text-[11px] text-dark-muted font-mono">
          <div><span className="text-dark-text">* * * * *</span> — {t('toolPanel.cron.everyMin')}</div>
          <div><span className="text-dark-text">0 * * * *</span> — {t('toolPanel.cron.hourly')}</div>
          <div><span className="text-dark-text">0 0 * * *</span> — {t('toolPanel.cron.daily')}</div>
          <div><span className="text-dark-text">0 0 * * 1</span> — {t('toolPanel.cron.weekly')}</div>
          <div><span className="text-dark-text">0 0 1 * *</span> — {t('toolPanel.cron.monthly')}</div>
          <div><span className="text-dark-text">*/30 * * * *</span> — {t('toolPanel.cron.every30min')}</div>
        </div>
      </div>
    </div>
  )
}

function CronForm({ onSubmit, onCancel }: { onSubmit: (job: CronJob) => void; onCancel: () => void }) {
  const { t } = useTranslation()
  const [form, setForm] = useState<CronJob>({ name: '', schedule: '0 * * * *', task: '', enabled: true } as CronJob)
  return (
    <div className="grid grid-cols-2 gap-3">
      <div><label className="block text-[11px] font-medium text-dark-text mb-1">{t('toolPanel.cron.formName')}</label><input value={form.name} onChange={e => setForm({ ...form, name: e.target.value })} placeholder={t('toolPanel.cron.formNamePlaceholder')} className="w-full bg-dark-bg border border-dark-border rounded-lg px-3 py-1.5 text-xs text-dark-text focus:outline-none focus:border-primary-500 font-mono" /></div>
      <div><label className="block text-[11px] font-medium text-dark-text mb-1">{t('toolPanel.cron.formCronExpr')}</label><input value={form.schedule} onChange={e => setForm({ ...form, schedule: e.target.value })} placeholder={t('toolPanel.cron.formCronPlaceholder')} className="w-full bg-dark-bg border border-dark-border rounded-lg px-3 py-1.5 text-xs text-dark-text focus:outline-none focus:border-primary-500 font-mono" /></div>
      <div className="col-span-2"><label className="block text-[11px] font-medium text-dark-text mb-1">{t('toolPanel.cron.formTaskCmd')}</label><input value={form.task} onChange={e => setForm({ ...form, task: e.target.value })} placeholder={t('toolPanel.cron.formTaskPlaceholder')} className="w-full bg-dark-bg border border-dark-border rounded-lg px-3 py-1.5 text-xs text-dark-text focus:outline-none focus:border-primary-500" /></div>
      <div className="col-span-2 flex justify-end gap-2 pt-1">
        <button onClick={onCancel} className="px-3 py-1.5 rounded-lg border border-dark-border text-xs text-dark-muted hover:text-dark-text transition-colors">{t('toolPanel.cron.formCancel')}</button>
        <button onClick={() => { if (form.name && form.schedule && form.task) onSubmit(form) }} disabled={!form.name || !form.schedule || !form.task} className="px-3 py-1.5 rounded-lg bg-primary-600 hover:bg-primary-500 text-white text-xs font-medium transition-colors disabled:opacity-40">{t('toolPanel.cron.formRegister')}</button>
      </div>
    </div>
  )
}

// ==================== 内置默认数据 ====================

function getDefaultAgents(): AgentConfig[] {
  return [
    { id: 'default-agent', name: 'General Assistant', description: 'Default general-purpose AI assistant with access to all tools', systemPrompt: 'You are a helpful AI assistant. Use appropriate tools to complete tasks based on user requirements. Keep responses concise and professional.', tools: [], maxTurns: 20, enabled: true, createdAt: Date.now(), updatedAt: Date.now() },
    { id: 'code-reviewer', name: 'Code Reviewer', description: 'Expert focused on code review, bug finding and quality improvement', systemPrompt: 'You are a senior code review expert. Focus on: 1) Finding potential bugs and edge cases 2) Code style and best practices 3) Performance optimization suggestions 4) Security vulnerability checks. Use Read/Edit/Grep/Glob tools to analyze code. Provide specific line numbers and suggestions.', tools: ['Read','Edit','Write','Glob','Grep'], maxTurns: 15, enabled: true, createdAt: Date.now(), updatedAt: Date.now() },
    { id: 'file-explorer', name: 'File Explorer', description: 'Specialized agent for quickly browsing project structure and file contents', systemPrompt: 'You are a file browsing assistant. When users ask about project structure or file contents, use Glob and Read tools to quickly locate and display information. Keep responses concise.', tools: ['Read','Glob','Grep'], maxTurns: 10, enabled: true, createdAt: Date.now(), updatedAt: Date.now() },
    { id: 'web-researcher', name: 'Web Researcher', description: 'Specialized agent for web search and information gathering', systemPrompt: 'You are a web research assistant. Use WebSearch and WebFetch tools to help users search for information, fetch webpage content, and organize research results. Summarize and analyze search results.', tools: ['WebSearch','WebFetch','Read','Write'], maxTurns: 12, enabled: false, createdAt: Date.now(), updatedAt: Date.now() },
    { id: 'devops-agent', name: 'DevOps Assistant', description: 'Operations agent for shell commands, build/deploy, and environment configuration', systemPrompt: 'You are a DevOps expert. You can execute shell commands for building, testing, and deployment operations. Use Bash tool to run commands, use Read/Edit to modify config files. Be security-conscious, confirm command meaning before execution.', tools: ['Bash','Read','Edit','Write','Glob','Grep'], maxTurns: 15, enabled: false, createdAt: Date.now(), updatedAt: Date.now() },
  ]
}

function getBuiltinSkills(): SkillDefinition[] {
  return [
    { id: 'skill-pdf', name: 'PDF Processing', version: '1.0.0', description: 'PDF file reading, parsing, text/table extraction, PDF generation and merging', author: 'Claw Team', category: 'coding', tags: ['pdf','document','extract'], installed: true, commands: [{ name: 'pdf_read', description: 'Read PDF file content' }, { name: 'pdf_extract', description: 'Extract text and data from PDF' }] },
    { id: 'skill-xlsx', name: 'Excel Processing', version: '1.0.0', description: 'Excel/CSV file read/write, data analysis, chart generation, formula calculation', author: 'Claw Team', category: 'coding', tags: ['excel','csv','spreadsheet'], installed: true, commands: [{ name: 'xlsx_read', description: 'Read Excel file' }, { name: 'xlsx_write', description: 'Write Excel file' }, { name: 'xlsx_analyze', description: 'Analyze data and generate report' }] },
    { id: 'skill-git', name: 'Git Version Control', version: '1.2.0', description: 'Git operations: commit, branch, merge, conflict resolution, changelog generation', author: 'Claw Team', category: 'coding', tags: ['git','version-control','commit'], installed: true, commands: [{ name: 'git_status', description: 'View Git status' }, { name: 'git_commit', description: 'Smart commit message generation and commit' }, { name: 'git_branch', description: 'Branch management' }, { name: 'git_diff', description: 'View code diff' }] },
    { id: 'skill-docker', name: 'Docker Container Management', version: '1.0.0', description: 'Docker image build, container orchestration, Compose management, log viewing', author: 'Claw Team', category: 'coding', tags: ['docker','container','deploy'], installed: false, commands: [{ name: 'docker_build', description: 'Build Docker image' }, { name: 'docker_up', description: 'Start container service' }, { name: 'docker_logs', description: 'View container logs' }] },
    { id: 'skill-database', name: 'Database Operations', version: '1.0.0', description: 'SQL query execution, database migration, schema design, performance analysis', author: 'Claw Team', category: 'coding', tags: ['database','sql','migration'], installed: false, commands: [{ name: 'db_query', description: 'Execute SQL query' }, { name: 'db_migrate', description: 'Execute database migration' }] },
    { id: 'skill-translate', name: 'Multi-language Translation', version: '1.0.0', description: 'High-quality translation supporting 50+ languages, technical doc localization, terminology consistency', author: 'Claw Team', category: 'misc', tags: ['translate','i18n','localization'], installed: false },
    { id: 'skill-image', name: 'Image Processing', version: '1.0.0', description: 'Image format conversion, compression, cropping, watermarking, OCR text recognition', author: 'Claw Team', category: 'misc', tags: ['image','ocr','compress'], installed: false, commands: [{ name: 'img_convert', description: 'Convert image format' }, { name: 'img_ocr', description: 'OCR text recognition' }] },
    { id: 'skill-testing', name: 'Automated Testing', version: '1.0.0', description: 'Unit test generation, E2E testing, coverage report, mock data generation', author: 'Claw Team', category: 'coding', tags: ['testing','unit-test','e2e'], installed: false, commands: [{ name: 'test_generate', description: 'Auto-generate unit tests' }, { name: 'test_run', description: 'Run test suite' }] },
    { id: 'skill-docgen', name: 'Documentation Generation', version: '1.0.0', description: 'API docs generation, README auto-writing, code comment completion, architecture diagram drawing', author: 'Claw Team', category: 'coding', tags: ['documentation','api','readme'], installed: false },
    { id: 'skill-monitor', name: 'System Monitoring', version: '1.0.0', description: 'CPU/memory/disk monitoring, process management, performance analysis, alert notification', author: 'Claw Team', category: 'misc', tags: ['monitor','system','performance'], installed: false },
  ]
}

function getAllTools(): ToolDefinition[] {
  return [
    { name: 'Read', description: 'Read the contents of a file. Use for viewing source code, configs, logs, or any text file.', input_schema: { "type": "object", "properties": { "file_path": { "type": "string", "description": "Absolute or relative path to the file to read" }, "offset": { "type": "integer", "description": "Line number to start reading from (1-based, default: 1)" }, "limit": { "type": "integer", "description": "Maximum number of lines to read (default: all)" } }, "required": ["file_path"] } },
    { name: 'Edit', description: 'Make edits to a file using string replacement. Finds old_string and replaces with new_string.', input_schema: { "type": "object", "properties": { "file_path": { "type": "string" }, "edits": { "type": "array", "items": { "type": "object", "properties": { "old_string": { "type": "string" }, "new_string": { "type": "string" } }, "required": ["old_string", "new_string"] } }, "dry_run": { "type": "boolean" } }, "required": ["file_path", "edits"] } },
    { name: 'Write', description: 'Write content to a file, creating it if it doesn\'t exist or overwriting if it does.', input_schema: { "type": "object", "properties": { "file_path": { "type": "string" }, "content": { "type": "string" }, "create_dirs": { "type": "boolean" } }, "required": ["file_path", "content"] } },
    { name: 'Bash', description: 'Execute a shell command in the user\'s environment. Can run any CLI command, build tools, git operations, package managers, etc.', input_schema: { "type": "object", "properties": { "command": { "type": "string" }, "working_dir": { "type": "string" }, "timeout_secs": { "type": "integer" } }, "required": ["command"] } },
    { name: 'Glob', description: 'Find files matching a glob pattern (e.g., \'**/*.rs\', \'src/**/*.tsx\'). Supports ** wildcards.', input_schema: { "type": "object", "properties": { "pattern": { "type": "string" }, "path": { "type": "string" }, "exclude_patterns": { "type": "array", "items": { "type": "string" } } }, "required": ["pattern"] } },
    { name: 'Grep', description: 'Search file contents using regex patterns. Like grep -rn but with structured output.', input_schema: { "type": "object", "properties": { "pattern": { "type": "string" }, "path": { "type": "string" }, "include_pattern": { "type": "string" }, "exclude_pattern": { "type": "string" } }, "required": ["pattern"] } },
    { name: 'WebFetch', description: 'Fetch and return the content of a URL. Useful for reading documentation, APIs, or web pages.', input_schema: { "type": "object", "properties": { "url": { "type": "string" }, "max_length": { "type": "integer" } }, "required": ["url"] } },
    { name: 'WebSearch', description: 'Search the internet for information. Returns relevant results from search engines.', input_schema: { "type": "object", "properties": { "query": { "type": "string" }, "num_results": { "type": "integer" } }, "required": ["query"] } },
    { name: 'Agent', description: 'Spawn a sub-agent to handle a subtask independently. The agent runs in its own context and returns a summary.', input_schema: { "type": "object", "properties": { "prompt": { "type": "string" }, "mode": { "type": "string" }, "model_override": { "type": "string" } }, "required": ["prompt"] } },
    { name: 'TodoWrite', description: 'Create or update a todo list to track progress on multi-step tasks.', input_schema: { "type": "object", "properties": { "todos": { "type": "array", "items": { "type": "object", "properties": { "content": { "type": "string" }, "status": { "type": "string" }, "priority": { "type": "string" } }, "required": ["content", "status"] } } }, "required": ["todos"] } },
    { name: 'TaskCreate', description: 'Create a new background task that runs independently.', input_schema: { "type": "object", "properties": { "prompt": { "type": "string" }, "description": { "type": "string" } }, "required": ["prompt"] } },
    { name: 'TaskList', description: 'List all background tasks with their status.', input_schema: { "type": "object", "properties": { "status_filter": { "type": "string" } } } },
    { name: 'Workflow', description: 'Execute a predefined workflow or task template with multiple steps.', input_schema: { "type": "object", "properties": { "name": { "type": "string" }, "steps": { "type": "array" }, "inputs": { "type": "object" } }, "required": ["name"] } },
    { name: 'Skill', description: 'Invoke a named skill or slash command programmatically.', input_schema: { "type": "object", "properties": { "skill_name": { "type": "string" }, "args": { "type": "object" } }, "required": ["skill_name"] } },
    { name: 'EnterPlanMode', description: 'Enter plan mode where the AI creates a detailed plan before making changes.', input_schema: { "type": "object", "properties": {} } },
    { name: 'ExitPlanMode', description: 'Exit plan mode and begin executing the planned changes.', input_schema: { "type": "object", "properties": {} } },
    { name: 'Brief', description: 'Send a brief message or notification to the user.', input_schema: { "type": "object", "properties": { "message": { "type": "string" }, "attachments": { "type": "array", "items": { "type": "string" } } }, "required": ["message"] } },
    { name: 'Config', description: 'Read or update global application configuration settings.', input_schema: { "type": "object", "properties": { "action": { "type": "string" }, "key": { "type": "string" }, "value": {} }, "required": ["action"] } },
    { name: 'NotebookEdit', description: 'Edit a Jupyter notebook cell by index.', input_schema: { "type": "object", "properties": { "file_path": { "type": "string" }, "cell_index": { "type": "integer" }, "source": { "type": "array", "items": { "type": "string" } } }, "required": ["file_path", "cell_index"] } },
    { name: 'ScheduleCron', description: 'Schedule a recurring task using cron-like syntax.', input_schema: { "type": "object", "properties": { "name": { "type": "string" }, "schedule": { "type": "string" }, "task": { "type": "string" }, "enabled": { "type": "boolean" } }, "required": ["name", "schedule", "task"] } },
    { name: 'AskUserQuestion', description: 'Ask the user one or more questions interactively when clarification is needed.', input_schema: { "type": "object", "properties": { "questions": { "type": "array", "items": { "type": "object", "properties": { "question": { "type": "string" }, "header": { "type": "string" }, "options": { "type": "array" }, "multiSelect": { "type": "boolean" } }, "required": ["question"] } } }, "required": ["questions"] } },
    { name: 'ToolSearch', description: 'Search across available tools to find tools matching a query.', input_schema: { "type": "object", "properties": { "query": { "type": "string" }, "max_results": { "type": "integer" } }, "required": ["query"] } },
  ]
}

// ==================== UI 辅助组件 ====================

function SkillIcon({ category }: { category: string }) {
  const icons: Record<string, string> = {
    coding: '💻', search: '🔍', web: '🌐', agent: '🤖', misc: '🔧',
  }
  return <span>{icons[category] || '📦'}</span>
}

function getCategoryStyle(category: string): string {
  const styles: Record<string, string> = {
    coding: 'bg-blue-500/10 text-blue-400', search: 'bg-purple-500/10 text-purple-400',
    web: 'bg-cyan-500/10 text-cyan-400', agent: 'bg-pink-500/10 text-pink-400',
    misc: 'bg-gray-500/10 text-gray-400',
  }
  return styles[category] || 'bg-dark-border text-dark-muted'
}

function ToolIcon({ name }: { name: string }) {
  const iconMap: Record<string, JSX.Element> = {
    Read: <svg className="w-4 h-4 text-white" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={2}><path strokeLinecap="round" strokeLinejoin="round" d="M9 12h6m-6 4h6m2 5H7a2 2 0 01-2-2V5a2 2 0 012-2h5.586a1 1 0 01.707.293l5.414 5.414a1 1 0 01.293.707V19a2 2 0 01-2 2z" /></svg>,
    Edit: <svg className="w-4 h-4 text-white" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={2}><path strokeLinecap="round" strokeLinejoin="round" d="M11 5H6a2 2 0 00-2 2v11a2 2 0 002 2h11a2 2 0 002-2v-5m-1.414-9.414a2 2 0 112.828 2.828L11.828 15H9v-2.828l8.586-8.586z" /></svg>,
    Write: <svg className="w-4 h-4 text-white" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={2}><path strokeLinecap="round" strokeLinejoin="round" d="M12 4v16m8-8H4" /></svg>,
    Bash: <svg className="w-4 h-4 text-white" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={2}><path strokeLinecap="round" strokeLinejoin="round" d="M8 9l3 3-3 3m5 0h3M5 20h14a2 2 0 002-2V6a2 2 0 00-2-2H5a2 2 0 00-2 2v12a2 2 0 002 2z" /></svg>,
    Glob: <svg className="w-4 h-4 text-white" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={2}><path strokeLinecap="round" strokeLinejoin="round" d="M3 7v10a2 2 0 002 2h14a2 2 0 002-2V9a2 2 0 00-2-2h-6l-2-2H5a2 2 0 00-2 2z" /></svg>,
    Grep: <svg className="w-4 h-4 text-white" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={2}><path strokeLinecap="round" strokeLinejoin="round" d="M21 21l-6-6m2-5a7 7 0 11-14 0 7 7 0 0114 0z" /></svg>,
    WebFetch: <svg className="w-4 h-4 text-white" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={2}><path strokeLinecap="round" strokeLinejoin="round" d="M21 12a9 9 0 01-9 9m9-9a9 9 0 00-9-9m9 9H3m9 9a9 9 0 01-9-9m9 9c1.657 0 3-4.03 3-9s-1.343-9-3-9m0 18c-1.657 0-3-4.03-3-9s1.343-9 3-9m-9 9a9 9 0 019-9" /></svg>,
    WebSearch: <svg className="w-4 h-4 text-white" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={2}><path strokeLinecap="round" strokeLinejoin="round" d="M21 21l-6-6m2-5a7 7 0 11-14 0 7 7 0 0114 0z" /><path strokeLinecap="round" strokeLinejoin="round" d="M10 7v6m3-3H7" /></svg>,
    Agent: <svg className="w-4 h-4 text-white" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={2}><path strokeLinecap="round" strokeLinejoin="round" d="M9.75 17L9 20l-1 1h8l-1-1-.75-3M3 13h18M5 17h14a2 2 0 002-2V5a2 2 0 00-2-2H5a2 2 0 00-2 2v10a2 2 0 002 2z" /></svg>,
    TodoWrite: <svg className="w-4 h-4 text-white" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={2}><path strokeLinecap="round" strokeLinejoin="round" d="M9 5H7a2 2 0 00-2 2v12a2 2 0 002 2h10a2 2 0 002-2V7a2 2 0 00-2-2h-2M9 5a2 2 0 002 2h2a2 2 0 002-2M9 5a2 2 0 012-2h2a2 2 0 012 2m-6 9l2 2 4-4" /></svg>,
    TaskCreate: <svg className="w-4 h-4 text-white" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={2}><path strokeLinecap="round" strokeLinejoin="round" d="M12 4v16m8-8H4" /></svg>,
    TaskList: <svg className="w-4 h-4 text-white" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={2}><path strokeLinecap="round" strokeLinejoin="round" d="M4 6h16M4 10h16M4 14h16M4 18h16" /></svg>,
    Workflow: <svg className="w-4 h-4 text-white" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={2}><path strokeLinecap="round" strokeLinejoin="round" d="M13 10V3L4 14h7v7l9-11h-7z" /></svg>,
    Skill: <svg className="w-4 h-4 text-white" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={2}><path strokeLinecap="round" strokeLinejoin="round" d="M9.663 17h4.673M12 3v1m6.364 1.636l-.707.707M21 12h-1M4 12H3m3.343-5.657l-.707-.707m2.828 9.9a5 5 0 117.072 0l-.548.547A3.374 3.374 0 0014 18.469V19a2 2 0 11-4 0v-.531c0-.895-.356-1.754-.988-2.386l-.548-.547z" /></svg>,
    EnterPlanMode: <svg className="w-4 h-4 text-white" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={2}><path strokeLinecap="round" strokeLinejoin="round" d="M9 5H7a2 2 0 00-2 2v12a2 2 0 002 2h10a2 2 0 002-2V7a2 2 0 00-2-2h-2M9 5a2 2 0 002 2h2a2 2 0 002-2M9 5a2 2 0 012-2h2a2 2 0 012 2m-6 9l2 2 4-4" /></svg>,
    ExitPlanMode: <svg className="w-4 h-4 text-white" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={2}><path strokeLinecap="round" strokeLinejoin="round" d="M11 16l-4-4m0 0l4-4m-4 4h14m-5 4v1a3 3 0 01-3 3H6a3 3 0 01-3-3V7a3 3 0 013-3h7a3 3 0 013 3v1" /></svg>,
    Brief: <svg className="w-4 h-4 text-white" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={2}><path strokeLinecap="round" strokeLinejoin="round" d="M15 17h5l-1.405-1.405A2.032 2.032 0 0118 14.158V11a6.002 6.002 0 00-4-5.659V5a2 2 0 10-4 0v.341C7.67 6.165 6 8.388 6 11v3.159c0 .538-.214 1.055-.595 1.436L4 17h5m6 0v1a3 3 0 11-6 0v-1m6 0H9" /></svg>,
    Config: <svg className="w-4 h-4 text-white" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={2}><path strokeLinecap="round" strokeLinejoin="round" d="M10.325 4.317c.426-1.756 2.924-1.756 3.35 0a1.724 1.724 0 002.573 1.066c1.543-.94 3.31.826 2.37 2.37a1.724 1.724 0 001.066 2.573c1.756.426 1.756 2.924 0 3.35a1.724 1.724 0 00-1.066 2.573c.94 1.543-.826 3.31-2.37 2.37a1.724 1.724 0 00-2.573 1.066c-.426 1.756-2.924 1.756-3.35 0a1.724 1.724 0 00-2.573-1.066c-1.543.94-3.31-.826-2.37-2.37a1.724 1.724 0 00-1.066-2.573c-1.756-.426-1.756-2.924 0-3.35a1.724 1.724 0 001.066-2.573c-.94-1.543.826-3.31 2.37-2.37.996.608 2.296.07 2.572-1.065z" /><path strokeLinecap="round" strokeLinejoin="round" d="M15 12a3 3 0 11-6 0 3 3 0 016 0z" /></svg>,
    NotebookEdit: <svg className="w-4 h-4 text-white" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={2}><path strokeLinecap="round" strokeLinejoin="round" d="M11 5H6a2 2 0 00-2 2v11a2 2 0 002 2h11a2 2 0 002-2v-5m-1.414-9.414a2 2 0 112.828 2.828L11.828 15H9v-2.828l8.586-8.586z" /></svg>,
    ScheduleCron: <svg className="w-4 h-4 text-white" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={2}><path strokeLinecap="round" strokeLinejoin="round" d="M12 8v4l3 3m6-3a9 9 0 11-18 0 9 9 0 0118 0z" /></svg>,
    AskUserQuestion: <svg className="w-4 h-4 text-white" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={2}><path strokeLinecap="round" strokeLinejoin="round" d="M8.228 9c.549-1.165 2.03-2 3.772-2 2.21 0 4 1.343 4 3 0 1.4-1.278 2.575-3.006 2.907-.542.104-.994.54-.994 1.093m0 3h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z" /></svg>,
    ToolSearch: <svg className="w-4 h-4 text-white" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={2}><path strokeLinecap="round" strokeLinejoin="round" d="M21 21l-6-6m2-5a7 7 0 11-14 0 7 7 0 0114 0z" /></svg>,
  }
  return iconMap[name] || <svg className="w-4 h-4 text-white" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={2}><path strokeLinecap="round" strokeLinejoin="round" d="M13 10V3L4 14h7v7l9-11h-7z" /></svg>
}
