// Claw Desktop - 状态栏组件 - 显示连接状态、会话信息、快速模式切换、系统诊断
// 功能：就绪/处理中状态、消息计数、操作按钮（复制/压缩/清空/统计/诊断/导出）、Toast通知

import { useState, useRef, useEffect } from 'react'
import { useTranslation } from 'react-i18next'
import { getSessionInfo, getDbStats, runDoctorCheck } from '../../api/system'
import { toggleFastMode } from '../../api/env'
import { getMessages as getConvMessages } from '../../api/conversations'

interface StatusBarProps {
  conversationId: string | null          // Current conversation ID (for stats and actions)
  messageCount: number                  // Current message count
  isLoading: boolean                     // Whether waiting for LLM response
  onCompact: () => void                 // Callback to compact conversation
  onClear: () => void                   // Callback to clear messages
}

function StatusBar({ conversationId, messageCount, isLoading, onCompact, onClear }: StatusBarProps) {
  const { t } = useTranslation()
  const [showStats, setShowStats] = useState(false)
  const [showDoctor, setShowDoctor] = useState(false)
  const [statsData, setStatsData] = useState<string | null>(null)
  const [doctorResults, setDoctorResults] = useState<Array<{ name: string; status: string; message: string }> | null>(null)
  const [toastMsg, setToastMsg] = useState<string | null>(null)
  const [fastMode, setFastMode] = useState(false)
  const toastTimeoutRef = useRef<ReturnType<typeof setTimeout> | null>(null)

  useEffect(() => { return () => { if (toastTimeoutRef.current) clearTimeout(toastTimeoutRef.current) } }, [])

  /// Display internal toast (auto-dismiss after 2s with cleanup)
  const showToast = (msg: string) => {
    setToastMsg(msg)
    if (toastTimeoutRef.current) clearTimeout(toastTimeoutRef.current)
    toastTimeoutRef.current = setTimeout(() => setToastMsg(null), 2000)
  }

  /// Fetch and display database statistics
  const handleShowStats = async () => {
    try {
      const sessionInfo = await getSessionInfo() as unknown as { totalMessages: number; userMessages: number; assistantMessages: number; totalCharacters: number }
      const dbStats = await getDbStats() as unknown as { databaseSizeMB: number; totalConversations: number }

      setStatsData(
        `Session: ${sessionInfo.totalMessages} messages (${sessionInfo.userMessages} user + ${sessionInfo.assistantMessages} assistant)\n` +
        `Chars: ${sessionInfo.totalCharacters.toLocaleString()} | DB: ${dbStats.databaseSizeMB} MB | Total convs: ${dbStats.totalConversations}`
      )
      setShowStats(true)
      setShowDoctor(false)
    } catch (e) {
      const errMsg = e instanceof Error ? e.message : String(e)
      setStatsData(`Error: ${errMsg}`)
      setShowStats(true)
      setShowDoctor(false)
    }
  }

  /// Run diagnostic checks and display results
  const handleShowDoctor = async () => {
    try {
      const results = await runDoctorCheck() as unknown as Array<{ name: string; status: string; message: string }>
      setDoctorResults(results)
      setShowDoctor(true)
      setShowStats(false)  // Close other panels
    } catch (e) { console.error(e) }
  }

  /// Copy current conversation content to clipboard
  const handleCopyConversation = async () => {
    if (!conversationId) return
    try {
      const msgs = await getConvMessages({ conversationId }) as Array<{ role: string; content: string }>
      const text = msgs.map(m => `${m.role}: ${m.content}`).join('\n\n---\n\n')
      await navigator.clipboard.writeText(text)
      showToast(t('statusBar.copiedToast'))
    } catch {
      showToast(t('statusBar.copyFailedToast'))
    }
  }

  /// Toggle fast mode
  const handleToggleFastMode = async () => {
    try {
      const result = await toggleFastMode() as { fast_mode: boolean; enabled?: boolean }
      setFastMode(result.fast_mode ?? result.enabled ?? !fastMode)
      showToast(result.fast_mode ?? result.enabled ?? !fastMode ? t('statusBar.fastModeOnToast') : t('statusBar.fastModeOffToast'))
    } catch {
      showToast(t('statusBar.toggleFailedToast'))
    }
  }

  return (
    <div className="shrink-0 px-3 py-1.5 bg-dark-surface border-t border-dark-border flex items-center gap-2 text-[10px] select-none relative">
      {/* ===== Left: Status indicator ===== */}
      <span className={`inline-flex items-center gap-1 ${isLoading ? 'text-primary-400' : 'text-green-400'}`}>
        <span className={`w-1.5 h-1.5 rounded-full ${isLoading ? 'bg-primary-400 animate-pulse' : 'bg-green-400'}`}></span>
        {isLoading ? t('statusBar.processing') : t('statusBar.ready')}
      </span>

      {/* Fast mode toggle */}
      <button onClick={handleToggleFastMode} title={t('statusBar.toggleFastMode')} className={`px-1.5 py-0.5 rounded text-[9px] font-medium transition-colors ${fastMode ? 'bg-orange-500/15 text-orange-400 border border-orange-500/30' : 'text-dark-muted hover:text-orange-400 border border-transparent hover:border-orange-500/20'}`}>
        {fastMode ? '⚡ ' + t('statusBar.fastModeOn') : t('statusBar.fastMode')}
      </button>

      {/* Message count */}
      <span className="text-dark-muted">{t('statusBar.messagesCount', { count: String(messageCount) })}</span>

      {/* Divider */}
      <div className="w-px h-3 bg-dark-border mx-1"></div>

      {/* ===== Action buttons ===== */}
      <button onClick={handleCopyConversation} className="px-2 py-1 rounded text-[11px] text-dark-muted hover:text-primary-400 hover:bg-primary-500/10 transition-all" title={t('statusBar.copyConversationTitle')}>
        <svg className="w-3.5 h-3.5" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={2}><path strokeLinecap="round" strokeLinejoin="round" d="M8 16H6a2 2 0 01-2-2V6a2 2 0 012-2h8a2 2 0 012 2v2m-6 12h8a2 2 0 002-2v-8a2 2 0 00-2-2h-8a2 2 0 00-2 2v8a2 2 0 002 2z"/></svg>
      </button>
      <button onClick={onCompact} className="px-2 py-1 rounded text-[11px] text-dark-muted hover:text-primary-400 hover:bg-primary-500/10 transition-all" title={t('statusBar.compactConversationTitle')}>
        <svg className="w-3.5 h-3.5" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={2}><path strokeLinecap="round" strokeLinejoin="round" d="M4 8V4m0 0h4M4 4l5 5m11-1V4m0 0h-4m4 0l-5 5M4 16v4m0 0h4m-4 0l5-5m11 5l-5-5m5 5v-4m0 4h-4"/></svg>
      </button>
      <button onClick={onClear} className="px-2 py-1 rounded text-[11px] text-dark-muted hover:text-red-400 hover:bg-red-500/10 transition-all" title={t('statusBar.clearMessagesTitle')}>
        <svg className="w-3.5 h-3.5" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={2}><path strokeLinecap="round" strokeLinejoin="round" d="M19 7l-.867 12.142A2 2 0 0116.138 21H7.862a2 2 0 01-1.995-1.858L5 7m5 4v6m4-6v6m1-10V4a1 1 0 00-1-1h-4a1 1 0 00-1 1v3M4 7h16"/></svg>
      </button>

      <div className="w-px h-3 bg-white/10 mx-1"></div>

      <button onClick={handleShowStats} className={`px-2 py-1 rounded text-[11px] transition-all ${showStats ? 'text-primary-400 bg-primary-500/20' : 'text-dark-muted hover:text-primary-400 hover:bg-primary-500/10'}`} title={t('statusBar.viewStatsTitle')}>
        <svg className="w-3.5 h-3.5" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={2}><path strokeLinecap="round" strokeLinejoin="round" d="M9 19v-6a2 2 0 00-2-2H5a2 2 0 00-2 2v6a2 2 0 002 2h2a2 2 0 002-2zm0 0V9a2 2 0 012-2h2a2 2 0 012 2v10m-6 0a2 2 0 002 2h2a2 2 0 002-2m0 0V5a2 2 0 012-2h2a2 2 0 012 2v14a2 2 0 01-2 2h-2a2 2 0 01-2-2z"/></svg>
      </button>
      <button onClick={handleShowDoctor} className="px-2 py-1 rounded text-[11px] text-dark-muted hover:text-green-400 hover:bg-green-500/10 transition-all" title={t('statusBar.runDiagnosticsTitle')}>
        <svg className="w-3.5 h-3.5" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={2}><path strokeLinecap="round" strokeLinejoin="round" d="M9 12l2 2 4-4m6 2a9 9 0 11-18 0 9 9 0 0118 0z"/></svg>
      </button>

      {/* ===== Expandable stats panel ===== */}
      {showStats && statsData && (
        <div className="absolute bottom-full left-0 right-0 mb-1 p-3 bg-dark-surface border border-dark-border rounded-lg shadow-xl z-10 animate-fade-in">
          <div className="font-semibold text-xs mb-1.5 text-dark-text">{t('statusBar.statisticsLabel')}</div>
          <pre className="whitespace-pre-wrap text-[11px] leading-relaxed text-dark-muted font-mono">{statsData}</pre>
          <button onClick={() => setShowStats(false)} className="mt-2 text-[10px] text-primary-400 hover:underline">{t('statusBar.closeButton')}</button>
        </div>
      )}

      {/* ===== Expandable diagnostics panel ===== */}
      {showDoctor && doctorResults && (
        <div className="absolute bottom-full left-0 right-0 mb-1 p-3 bg-dark-surface border border-dark-border rounded-lg shadow-xl z-10 animate-fade-in max-h-60 overflow-y-auto">
          <div className="font-semibold text-xs mb-1.5 text-dark-text">{t('statusBar.diagnosticsLabel')}</div>
          <div className="space-y-1">
            {doctorResults.map((r, i) => (
              <div key={i} className="flex items-center gap-2 text-[11px]">
                <span className={`w-1.5 h-1.5 rounded-full ${
                  r.status === 'ok' ? 'bg-green-400' :
                  r.status === 'warning' ? 'bg-yellow-400' : 'bg-red-400'
                }`}></span>
                <span className="font-medium text-dark-text w-28 truncate">{r.name}</span>
                <span className={
                  r.status === 'ok' ? 'text-green-400' :
                  r.status === 'warning' ? 'text-yellow-400' : 'text-red-400'
                }>{r.message}</span>
              </div>
            ))}
          </div>
          <button onClick={() => setShowDoctor(false)} className="mt-2 text-[10px] text-primary-400 hover:underline">{t('statusBar.closeButton')}</button>
        </div>
      )}

      {/* ===== Internal Toast notification ===== */}
      {toastMsg && (
        <div className="absolute bottom-full left-1/2 -translate-x-1/2 mb-1.5 px-2.5 py-1 bg-dark-bg border border-dark-border rounded-md shadow-lg text-[10px] text-dark-text whitespace-nowrap animate-fade-in z-20">
          {toastMsg}
        </div>
      )}
    </div>
  )
}

export default StatusBar
