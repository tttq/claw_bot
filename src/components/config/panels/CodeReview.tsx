// Claw Desktop - 代码审查面板 - AI驱动的代码Review功能
// 获取diff → 发送给LLM → 展示审查结果

import { useState } from 'react'
import { getCodeChangesSummary, runCodeReview } from '../../../api/env'
import { getConfig } from '../../../api/config'
import { useTranslation } from 'react-i18next'

export default function CodeReviewPanel({ agentId }: { agentId?: string }) {
  const { t } = useTranslation()
  const [stagedOnly, setStagedOnly] = useState(false)
  const [changes, setChanges] = useState<any>(null)
  const [review, setReview] = useState<any>(null)
  const [loading, setLoading] = useState(false)
  const [reviewing, setReviewing] = useState(false)

  const handleFetchChanges = async () => {
    setLoading(true); setReview(null)
    try {
      const data = await getCodeChangesSummary({ staged_only: stagedOnly })
      setChanges(data)
    } catch (e) { console.error(e) }
    finally { setLoading(false) }
  }

  const handleReview = async () => {
    if (!changes) return
    setReviewing(true); setReview(null)
    try {
      const appConfig = await getConfig()
      const data = await runCodeReview({ config: appConfig, changes_summary: changes })
      setReview(data)
    } catch (e) { setReview({ success: false, error: String(e) }) }
    finally { setReviewing(false) }
  }

  return (
    <div className="space-y-4">
      <div className="flex items-center justify-between">
        <h3 className="text-base font-semibold text-dark-text flex items-center gap-2">
          <svg className="w-5 h-5 text-cyan-400" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={2}><path strokeLinecap="round" strokeLinejoin="round" d="M9 12l2 2 4-4m6 2a9 9 0 11-18 0 9 9 0 0118 0z"/></svg>
          {t('panels.codeReview.title')}
        </h3>
        <label className="flex items-center gap-1.5 cursor-pointer text-xs text-dark-muted hover:text-dark-text">
          <input type="checkbox" checked={stagedOnly} onChange={e => setStagedOnly(e.target.checked)} className="rounded border-dark-border" />
          {t('panels.codeReview.stagedOnly')}
        </label>
      </div>

      {/* Actions */}
      <div className="flex gap-2">
        <button onClick={handleFetchChanges} disabled={loading} className="px-3 py-2 rounded-lg bg-blue-600/10 text-blue-400 border border-blue-500/20 hover:bg-blue-600/20 text-xs font-medium disabled:opacity-40 transition-colors flex items-center gap-1.5">
          {loading ? <div className="w-3.5 h-3.5 border-2 border-blue-400 border-t-transparent rounded-full animate-spin" /> : <svg className="w-3.5 h-3.5" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={2}><path strokeLinecap="round" strokeLinejoin="round" d="M4 4v5h.582m15.356 2A8.001 8.001 0 004.582 9m0 0H9m11 11v-5h-.581m0 0a8.003 8.003 0 01-15.357-2m15.357 2H15"/></svg>}
          {t('panels.codeReview.fetchChanges')}
        </button>
        {changes && !review && (
          <button onClick={handleReview} disabled={reviewing} className="px-3 py-2 rounded-lg bg-primary-600 hover:bg-primary-500 text-white text-xs font-medium disabled:opacity-40 transition-colors flex items-center gap-1.5">
            {reviewing ? <div className="w-3.5 h-3.5 border-2 border-white/30 border-t-transparent rounded-full animate-spin" /> : <svg className="w-3.5 h-3.5" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={2}><path strokeLinecap="round" strokeLinejoin="round" d="M15 12a3 3 0 11-6 0 3 3 0 016 0z"/><path strokeLinecap="round" strokeLinejoin="round" d="M2.458 12C3.732 7.943 7.523 5 12 5c4.478 0 8.268 2.943 9.542 7-1.274 4.057-5.064 7-9.542 7-4.477 0-8.268-2.943-9.542-7z"/></svg>}
            {t('panels.codeReview.aiReview')}
          </button>
        )}
      </div>

      {/* Changes summary */}
      {changes && (
        <div className="rounded-xl border border-dark-border p-3 space-y-2">
          <div className="flex items-center gap-2 text-xs">
            <span className="font-semibold text-dark-text">{t('panels.codeReview.changedFiles')}</span>
            <span className="px-1.5 py-0.5 rounded-full bg-blue-500/10 text-blue-400">{changes.total_files || 0}</span>
            <span className="text-dark-muted">|</span>
            <span className={`text-[10px] px-1.5 py-0.5 rounded ${stagedOnly ? 'bg-green-500/10 text-green-400' : 'bg-yellow-500/10 text-yellow-400'}`}>{stagedOnly ? t('panels.codeReview.staged') : t('panels.codeReview.unstaged')}</span>
          </div>
          {(changes.files_changed || []).map((f: any, i: number) => (
            <div key={i} className="flex items-center gap-2 px-2 py-1 rounded bg-dark-bg text-[11px] font-mono">
              <span className={`w-5 h-5 rounded flex items-center justify-center text-[9px] font-bold ${
                f.status === 'M' ? 'bg-yellow-500/10 text-yellow-400' :
                f.status === 'A' ? 'bg-green-500/10 text-green-400' :
                f.status === 'D' ? 'bg-red-500/10 text-red-400' :
                f.status === 'R' ? 'bg-blue-500/10 text-blue-400' : 'bg-gray-500/10 text-gray-400'
              }`}>{f.status || '?'}</span>
              <span className="text-dark-text truncate">{f.file}</span>
            </div>
          ))}
          {changes.summary && (
            <pre className="text-[10px] text-dark-muted bg-black/20 rounded-lg p-2 overflow-x-auto whitespace-pre-wrap mt-2">{changes.summary}</pre>
          )}
        </div>
      )}

      {/* Review result */}
      {review && (
        <div className={`rounded-xl border p-4 space-y-3 ${review.success ? 'border-green-500/20 bg-green-500/5' : 'border-red-500/20 bg-red-500/5'}`}>
          {review.success && (
            <>
              <div className="flex items-center gap-2">
                <span className="text-sm font-bold text-green-400">{t('panels.codeReview.reviewComplete')}</span>
                {review.model && <span className="text-[10px] px-1.5 py-0.5 rounded bg-dark-border text-dark-muted">{review.model}</span>}
                {review.usage && <span className="text-[10px] text-dark-muted">Tokens: {review.usage.input_tokens?.toLocaleString()} in / {review.usage.output_tokens?.toLocaleString()} out</span>}
                <span className="text-[10px] text-dark-muted ml-auto">{t('panels.codeReview.filesReviewed', { count: String(review.files_reviewed) })}</span>
              </div>
              <div className="prose prose-invert prose-sm max-w-none text-xs text-dark-text leading-relaxed whitespace-pre-wrap">
                {review.review}
              </div>
            </>
          )}
          {!review.success && (
            <p className="text-sm text-red-400">{review.error || t('panels.codeReview.reviewFailed')}</p>
          )}
        </div>
      )}

      {!changes && !loading && (
        <div className="rounded-xl border border-dashed border-dark-border p-6 text-center">
          <p className="text-sm text-dark-muted">{t('panels.codeReview.emptyHint')}</p>
        </div>
      )}
    </div>
  )
}
