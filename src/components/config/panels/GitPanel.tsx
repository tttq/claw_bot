// Claw Desktop - Git面板 - 查看Git状态、差异对比、提交历史
// 完整GUI操作：状态查看/差异对比/提交/分支管理/日志

import { useState, useEffect, useCallback } from 'react'
import { useTranslation } from 'react-i18next'
import { gitAdd, gitReset, gitCommit, gitDiff, gitStash, gitStashPop, gitCreateBranch, gitCheckoutBranch, gitStatus, gitLog, gitBranchList, gitIsRepository } from '../../../api/git'

interface GitStatusItem { file: string; status: string; staged: boolean }
interface GitCommit { hash: string; short_hash: string; author: string; message: string; date: string }
interface GitBranch { name: string; is_current: boolean; is_remote: boolean }

const STATUS_MAP: Record<string, { labelKey: string; color: string; icon: string }> = {
  M: { labelKey: 'panels.git.statusModified', color: 'text-yellow-400 bg-yellow-500/10', icon: 'M' },
  A: { labelKey: 'panels.git.statusAdded', color: 'text-green-400 bg-green-500/10', icon: 'A' },
  D: { labelKey: 'panels.git.statusDeleted', color: 'text-red-400 bg-red-500/10', icon: 'D' },
  R: { labelKey: 'panels.git.statusRenamed', color: 'text-blue-400 bg-blue-500/10', icon: 'R' },
  C: { labelKey: 'panels.git.statusCopied', color: 'text-blue-400 bg-blue-500/10', icon: 'C' },
  '?': { labelKey: 'panels.git.statusUntracked', color: 'text-gray-400 bg-gray-500/10', icon: '?' },
  U: { labelKey: 'panels.git.statusUpdated', color: 'text-cyan-400 bg-cyan-500/10', icon: 'U' },
}

export default function GitPanel({ agentId }: { agentId?: string }) {
  const [isRepo, setIsRepo] = useState<boolean | null>(null)
  const [statusItems, setStatusItems] = useState<GitStatusItem[]>([])
  const [branch, setBranch] = useState('')
  const [commits, setCommits] = useState<GitCommit[]>([])
  const [branches, setBranches] = useState<GitBranch[]>([])
  const [diffContent, setDiffContent] = useState<any>(null)
  const [activeTab, setActiveTab] = useState<'status' | 'diff' | 'log' | 'branch'>('status')
  const [selectedFiles, setSelectedFiles] = useState<Set<string>>(new Set())
  const [commitMsg, setCommitMsg] = useState('')
  const [newBranchName, setNewBranchName] = useState('')
  const [loading, setLoading] = useState(false)
  const [toast, setToast] = useState<string | null>(null)
  const { t } = useTranslation()

  useEffect(() => { checkRepo(); loadStatus() }, [])

  const showToast = (msg: string) => { setToast(msg); setTimeout(() => setToast(null), 2500) }

  const checkRepo = async () => {
    try {
      const result = await gitIsRepository() as unknown as boolean | null
      setIsRepo(result)
    } catch { setIsRepo(null) }
  }

  const loadStatus = async () => {
    setLoading(true)
    try {
      const data = await gitStatus() as { branch?: string; items?: any[] }
      setBranch(data.branch || '(unknown)')
      setStatusItems(data.items || [])
    } catch { /* not a repo */ }
    finally { setLoading(false) }
  }

  const loadCommits = async () => {
    try {
      const data = await gitLog({ limit: 30 }) as { commits?: any[] }
      setCommits(data.commits || [])
    } catch (e) { console.error(e) }
  }

  const loadBranches = async () => {
    try {
      const data = await gitBranchList() as { branches?: any[]; current?: string }
      setBranches(data.branches || [])
      setBranch(data.current || '')
    } catch (e) { console.error(e) }
  }

  const handleFileSelect = (file: string) => {
    setSelectedFiles(prev => { const s = new Set(prev); if (s.has(file)) s.delete(file); else s.add(file); return s })
  }

  const handleSelectAll = () => {
    if (selectedFiles.size === statusItems.length) setSelectedFiles(new Set())
    else setSelectedFiles(new Set(statusItems.map(i => i.file)))
  }

  const handleStageSelected = async () => {
    if (selectedFiles.size === 0) return
    try {
      await gitAdd({ files: Array.from(selectedFiles) })
      showToast(t('panels.git.staged_count', { count: String(selectedFiles.size) }))
      setSelectedFiles(new Set()); await loadStatus()
    } catch (e) { showToast(t('panels.git.stage_failed', { error: String(e) })) }
  }

  const handleUnstageSelected = async () => {
    if (selectedFiles.size === 0) return
    try {
      await gitReset({ files: Array.from(selectedFiles) })
      showToast(t('panels.git.unstagedCount', { count: selectedFiles.size }))
      setSelectedFiles(new Set()); await loadStatus()
    } catch (e) { showToast(t('panels.git.unstageFailed', { error: String(e) })) }
  }

  const handleCommit = async () => {
    if (!commitMsg.trim()) { showToast(t('panels.git.enter_commit_msg')); return }
    try {
      const files = selectedFiles.size > 0 ? Array.from(selectedFiles) : undefined
      await gitCommit({ message: commitMsg.trim(), files })
      showToast(t('panels.git.commit_success'))
      setCommitMsg(''); setSelectedFiles(new Set()); await loadStatus()
    } catch (e) { showToast(t('panels.git.commit_failed', { error: String(e) })) }
  }

  const handleViewDiff = async (filePath?: string) => {
    try {
      const data = await gitDiff({ file_path: filePath })
      setDiffContent(data)
      if (activeTab !== 'diff') setActiveTab('diff')
    } catch (e) { showToast(t('panels.git.diff_failed', { error: String(e) })) }
  }

  const handleStash = async () => {
    try { await gitStash(); showToast(t('panels.git.stash_saved')); await loadStatus() }
    catch (e) { showToast(t('panels.git.stash_failed', { error: String(e) })) }
  }

  const handleStashPop = async () => {
    try { await gitStashPop(); showToast(t('panels.git.stash_restored')); await loadStatus() }
    catch (e) { showToast(t('panels.git.stash_restore_failed', { error: String(e) })) }
  }

  const handleCreateBranch = async () => {
    if (!newBranchName.trim()) { showToast(t('panels.git.enter_branch_name')); return }
    try {
      await gitCreateBranch({ name: newBranchName.trim() })
      showToast(t('panels.git.branch_created', { name: newBranchName.trim() }))
      setNewBranchName(''); await loadBranches(); await loadStatus()
    } catch (e) { showToast(t('panels.git.branch_create_failed', { error: String(e) })) }
  }

  const handleCheckout = async (name: string) => {
    try { await gitCheckoutBranch({ name }); showToast(t('panels.git.checked_out', { name })); await loadBranches(); await loadStatus() }
    catch (e) { showToast(t('panels.git.checkout_failed', { error: String(e) })) }
  }

  useEffect(() => { if (activeTab === 'log') loadCommits(); if (activeTab === 'branch') loadBranches() }, [activeTab])

  if (isRepo === false) {
    return (
      <div className="flex flex-col items-center justify-center py-16 text-dark-muted">
        <svg className="w-12 h-12 mb-3 opacity-30" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={1.5}><path strokeLinecap="round" strokeLinejoin="round" d="M6 18 18 6M6 6l12 12"/></svg>
        <p className="text-sm">{t('panels.git.not_git_repo')}</p>
        <p className="text-xs mt-1 opacity-50">{t('panels.git.open_git_project')}</p>
      </div>
    )
  }

  const tabs = [
    { id: 'status' as const, labelKey: 'panels.git.tabChanges', icon: <svg className="w-3.5 h-3.5" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={2}><path strokeLinecap="round" strokeLinejoin="round" d="M9 12h6m-6 4h6m2 5H7a2 2 0 01-2-2V5a2 2 0 012-2h5.586a1 1 0 01.707.293l5.414 5.414a1 1 0 01.293.707V19a2 2 0 01-2 2z" /></svg> },
    { id: 'diff' as const, labelKey: 'panels.git.tabDiff', icon: <svg className="w-3.5 h-3.5" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={2}><path strokeLinecap="round" strokeLinejoin="round" d="M8 9l4-4-4-4m5 4H4" /></svg> },
    { id: 'log' as const, labelKey: 'panels.git.tabHistory', icon: <svg className="w-3.5 h-3.5" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={2}><path strokeLinecap="round" strokeLinejoin="round" d="M12 8v4l3 3m6-3a9 9 0 11-18 0 9 9 0 0118 0z" /></svg> },
    { id: 'branch' as const, labelKey: 'panels.git.tabBranches', icon: <svg className="w-3.5 h-3.5" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={2}><path strokeLinecap="round" strokeLinejoin="round" d="M13 7h8m0 0v8m0-8l-8 8-4-4-6 6" /></svg> },
  ]

  return (
    <div className="space-y-4">
      {/* Header */}
      <div className="flex items-center justify-between">
        <div>
          <h3 className="text-base font-semibold text-dark-text flex items-center gap-2">
            <svg className="w-5 h-5 text-orange-400" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={2}><path strokeLinecap="round" strokeLinejoin="round" d="M13 7h8m0 0v8m0-8l-8 8-4-4-6 6" /></svg>
            {t('panels.git.title')}
          </h3>
          <p className="text-xs text-dark-muted mt-0.5">{t('panels.git.current_branch')}: <code className="px-1.5 py-0.5 rounded bg-primary-600/10 text-primary-300">{branch}</code></p>
        </div>
        <div className="flex gap-1.5">
          <button onClick={() => { loadStatus(); loadBranches() }} className="p-1.5 rounded-lg hover:bg-dark-bg text-dark-muted hover:text-dark-text transition-colors" title={t('panels.git.refresh')}>
            <svg className="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={2}><path strokeLinecap="round" strokeLinejoin="round" d="M4 4v5h.582m15.356 2A8.001 8.001 0 004.582 9m0 0H9m11 11v-5h-.581m0 0a8.003 8.003 0 01-15.357-2m15.357 2H15" /></svg>
          </button>
        </div>
      </div>

      {toast && <div className="px-3 py-2 rounded-lg bg-primary-600/10 border border-primary-500/20 text-xs text-primary-300">{toast}</div>}

      {/* Tabs */}
      <div className="flex gap-1 p-1 rounded-lg bg-dark-bg border border-dark-border">
        {tabs.map(tab => (
          <button key={tab.id} onClick={() => setActiveTab(tab.id)} className={`flex items-center gap-1.5 px-3 py-1.5 rounded-md text-xs transition-colors ${activeTab === tab.id ? 'bg-primary-600 text-white shadow-sm' : 'text-dark-muted hover:text-dark-text'}`}>
            {tab.icon}{t(tab.labelKey)}
            {tab.id === 'status' && statusItems.length > 0 && <span className={`ml-1 px-1.5 py-0.5 rounded-full text-[10px] ${activeTab === tab.id ? 'bg-white/20' : 'bg-red-500/10 text-red-400'}`}>{statusItems.length}</span>}
          </button>
        ))}
      </div>

      {/* Status Tab */}
      {activeTab === 'status' && (
        <div className="space-y-3">
          {/* Actions bar */}
          <div className="flex items-center gap-2 flex-wrap">
            <button onClick={handleSelectAll} className="px-2.5 py-1 rounded text-[11px] border border-dark-border text-dark-muted hover:text-dark-text hover:border-primary-500/30 transition-colors">
              {selectedFiles.size === statusItems.length && statusItems.length > 0 ? t('panels.git.deselect_all') : t('panels.git.select_all')}
            </button>
            <button onClick={handleStageSelected} disabled={selectedFiles.size === 0} className="px-2.5 py-1 rounded text-[11px] bg-green-600/10 text-green-400 border border-green-500/20 hover:bg-green-600/20 disabled:opacity-40 transition-colors">{t('panels.git.stage')}</button>
            <button onClick={handleUnstageSelected} disabled={selectedFiles.size === 0} className="px-2.5 py-1 rounded text-[11px] bg-yellow-600/10 text-yellow-400 border border-yellow-500/20 hover:bg-yellow-600/20 disabled:opacity-40 transition-colors">{t('panels.git.unstage')}</button>
            <button onClick={() => handleViewDiff()} disabled={selectedFiles.size === 0} className="px-2.5 py-1 rounded text-[11px] bg-blue-600/10 text-blue-400 border border-blue-500/20 hover:bg-blue-600/20 disabled:opacity-40 transition-colors">{t('panels.git.view_diff')}</button>
            <div className="flex-1" />
            <button onClick={handleStash} className="px-2.5 py-1 rounded text-[11px] border border-dark-border text-dark-muted hover:text-dark-text transition-colors">{t('panels.git.stash')}</button>
            <button onClick={handleStashPop} className="px-2.5 py-1 rounded text-[11px] border border-dark-border text-dark-muted hover:text-dark-text transition-colors">{t('panels.git.stashPop')}</button>
          </div>

          {/* Commit area */}
          <div className="p-3 rounded-xl border border-dark-border bg-dark-bg space-y-2">
            <input value={commitMsg} onChange={e => setCommitMsg(e.target.value)} placeholder={t('panels.git.commit_placeholder')} className="w-full bg-transparent text-sm text-dark-text placeholder-dark-muted/30 outline-none" onKeyDown={e => { if (e.key === 'Enter' && e.ctrlKey) handleCommit() }} />
            <div className="flex justify-end gap-2">
              <span className="text-[10px] text-dark-muted self-center">{selectedFiles.size > 0 ? t('panels.git.selected_count', { count: String(selectedFiles.size) }) : t('panels.git.will_commit_all')}</span>
              <button onClick={handleCommit} disabled={!commitMsg.trim()} className="px-3 py-1.5 rounded-lg bg-primary-600 hover:bg-primary-500 text-white text-xs font-medium disabled:opacity-40 transition-colors">{t('panels.git.commit_btn')}</button>
            </div>
          </div>

          {/* File list */}
          <div className="rounded-xl border border-dark-border overflow-hidden">
            {loading ? (
              <div className="flex justify-center py-8"><div className="w-6 h-6 border-2 border-primary-500 border-t-transparent rounded-full animate-spin"></div></div>
            ) : statusItems.length === 0 ? (
              <div className="text-center py-8 text-sm text-dark-muted">{t('panels.git.clean_workspace')}</div>
            ) : (
              <div className="divide-y divide-dark-border max-h-[360px] overflow-y-auto">
                {statusItems.map((item, i) => {
                  const info = STATUS_MAP[item.status] || { labelKey: 'panels.git.statusUnknown', color: 'text-gray-400 bg-gray-500/10', icon: item.status }
                  return (
                    <div key={i} onClick={() => handleFileSelect(item.file)} className={`flex items-center gap-3 px-3 py-2 cursor-pointer transition-colors group ${selectedFiles.has(item.file) ? 'bg-primary-600/10' : 'hover:bg-dark-bg'}`}>
                      <input type="checkbox" checked={selectedFiles.has(item.file)} onChange={() => {}} className="rounded border-dark-border" onClick={e => e.stopPropagation()} readOnly />
                      <span className={`w-6 h-6 rounded flex items-center justify-center text-[10px] font-bold ${info.color}`}>{info.icon}</span>
                      <span className="flex-1 text-xs text-dark-text truncate font-mono">{item.file}</span>
                      <span className={`text-[10px] px-1.5 py-0.5 rounded ${info.color}`}>{t(info.labelKey)}</span>
                      <span className={`text-[9px] ${item.staged ? 'text-green-400' : 'text-dark-muted'}`}>{item.staged ? t('panels.git.staged') : ''}</span>
                      <button onClick={(e) => { e.stopPropagation(); handleViewDiff(item.file) }} className="opacity-0 group-hover:opacity-100 p-1 rounded hover:bg-dark-surface text-dark-muted hover:text-primary-400 transition-all">
                        <svg className="w-3.5 h-3.5" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={2}><path strokeLinecap="round" strokeLinejoin="round" d="M15 12a3 3 0 11-6 0 3 3 0 016 0z"/><path strokeLinecap="round" strokeLinejoin="round" d="M2.458 12C3.732 7.943 7.523 5 12 5c4.478 0 8.268 2.943 9.542 7-1.274 4.057-5.064 7-9.542 7-4.477 0-8.268-2.943-9.542-7z"/></svg>
                      </button>
                    </div>
                  )
                })}
              </div>
            )}
          </div>
        </div>
      )}

      {/* Diff Tab */}
      {activeTab === 'diff' && (
        <div className="rounded-xl border border-dark-border overflow-hidden">
          {!diffContent ? (
            <div className="text-center py-8 text-sm text-dark-muted">{t('panels.git.select_to_view_diff')}</div>
          ) : (
            <div className="max-h-[500px] overflow-auto">
              {(diffContent.files || []).length === 0 && <div className="text-center py-8 text-sm text-dark-muted">{t('panels.git.no_diff')}</div>}
              {(diffContent.files || []).map((file: any, fi: number) => (
                <div key={fi}>
                  <div className="sticky top-0 z-10 px-3 py-1.5 bg-dark-surface border-b border-dark-border text-xs font-mono text-dark-text flex items-center gap-2">
                    <span className={`px-1.5 py-0.5 rounded text-[10px] ${
                      file.status === 'added' ? 'bg-green-500/10 text-green-400' :
                      file.status === 'deleted' ? 'bg-red-500/10 text-red-400' :
                      file.status === 'renamed' ? 'bg-blue-500/10 text-blue-400' :
                      'bg-yellow-500/10 text-yellow-400'
                    }`}>{file.status}</span>
                    <span className="truncate">{file.new_path || file.old_path}</span>
                  </div>
                  <div className="font-mono text-[11px] leading-relaxed">
                    {(file.lines || []).map((line: any, li: number) => (
                      <div key={li} className={`px-3 whitespace-pre-wrap ${
                        line.type_ === '+' ? 'bg-green-500/5 text-green-300' :
                        line.type_ === '-' ? 'bg-red-500/5 text-red-300' :
                        line.type_ === 'header' ? 'bg-primary-600/5 text-primary-300' :
                        'text-dark-muted'
                      }`}>
                        <span className="inline-block w-6 text-right mr-3 opacity-40 select-none">
                          {line.old_line ?? ''}
                        </span>
                        <span className="inline-block w-6 text-right mr-3 opacity-40 select-none">
                          {line.new_line ?? ''}
                        </span>
                        <span>{line.content}</span>
                      </div>
                    ))}
                  </div>
                </div>
              ))}
            </div>
          )}
        </div>
      )}

      {/* Log Tab */}
      {activeTab === 'log' && (
        <div className="rounded-xl border border-dark-border overflow-hidden">
          <div className="max-h-[500px] overflow-y-auto divide-y divide-dark-border">
            {commits.length === 0 ? <div className="text-center py-8 text-sm text-dark-muted">{t('panels.git.no_commits')}</div> :
              commits.map((c, i) => (
                <div key={i} className="px-4 py-3 hover:bg-dark-bg/50 transition-colors group">
                  <div className="flex items-start gap-3">
                    <code className="text-[11px] font-mono text-primary-300 shrink-0 mt-0.5">{c.short_hash}</code>
                    <div className="flex-1 min-w-0">
                      <p className="text-xs text-dark-text font-medium">{c.message}</p>
                      <div className="flex items-center gap-2 mt-1">
                        <span className="text-[10px] text-dark-muted">{c.author}</span>
                        <span className="text-[10px] text-dark-muted/50">{c.date}</span>
                      </div>
                    </div>
                  </div>
                </div>
              ))
            }
          </div>
        </div>
      )}

      {/* Branch Tab */}
      {activeTab === 'branch' && (
        <div className="space-y-3">
          {/* Create branch */}
          <div className="flex gap-2">
            <input value={newBranchName} onChange={e => setNewBranchName(e.target.value)} placeholder={t('panels.git.new_branch_name')} className="flex-1 bg-dark-bg border border-dark-border rounded-lg px-3 py-1.5 text-xs text-dark-text outline-none focus:border-primary-500 font-mono" onKeyDown={e => { if (e.key === 'Enter') handleCreateBranch() }} />
            <button onClick={handleCreateBranch} disabled={!newBranchName.trim()} className="px-3 py-1.5 rounded-lg bg-primary-600 hover:bg-primary-500 text-white text-xs font-medium disabled:opacity-40 transition-colors">{t('panels.git.create_branch')}</button>
          </div>

          {/* Branch list */}
          <div className="rounded-xl border border-dark-border overflow-hidden">
            <div className="max-h-[420px] overflow-y-auto divide-y divide-dark-border">
              {branches.map((b, i) => (
                <div key={i} className={`flex items-center gap-3 px-4 py-2.5 transition-colors group ${b.is_current ? 'bg-primary-600/10' : 'hover:bg-dark-bg'}`}>
                  {b.is_current && <span className="w-1.5 h-1.5 rounded-full bg-primary-400" />}
                  {!b.is_current && <span className="w-1.5" />}
                  <span className="text-xs text-dark-text font-mono flex-1 truncate">{b.name}</span>
                  {b.is_current && <span className="text-[10px] px-1.5 py-0.5 rounded bg-primary-600/20 text-primary-300">{t('panels.git.current')}</span>}
                  {b.is_remote && <span className="text-[10px] px-1.5 py-0.5 rounded bg-dark-border text-dark-muted">{t('panels.git.remote')}</span>}
                  {!b.is_current && !b.is_remote && (
                    <button onClick={() => handleCheckout(b.name)} className="opacity-0 group-hover:opacity-100 text-[10px] text-primary-400 hover:underline transition-opacity">{t('panels.git.checkout')}</button>
                  )}
                </div>
              ))}
            </div>
          </div>
        </div>
      )}
    </div>
  )
}
