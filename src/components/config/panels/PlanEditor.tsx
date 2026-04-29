// Claw Desktop - 计划编辑器 - 管理和编辑Agent执行计划步骤
// 可视化计划创建、编辑、审批和执行流程

import { useState, useEffect } from 'react'
import { useTranslation } from 'react-i18next'
import { toolEnterPlanMode, toolExitPlanMode, toolGetPlanStatus } from '../../../api/tools'

interface PlanStep {
  id: string
  title: string
  description: string
  status: 'pending' | 'in_progress' | 'completed' | 'skipped' | 'blocked'
  tools?: string[]
  dependencies?: string[]
}

interface Plan {
  id: string
  title: string
  description: string
  steps: PlanStep[]
  status: 'draft' | 'reviewing' | 'approved' | 'executing' | 'completed' | 'cancelled'
  createdAt: number
  updatedAt: number
}

export default function PlanEditor({ agentId }: { agentId?: string }) {
  const { t } = useTranslation()
  const [planMode, setPlanMode] = useState<'active' | 'inactive'>('inactive')
  const [plan, setPlan] = useState<Plan | null>(null)
  const [editingTitle, setEditingTitle] = useState('')
  const [editingDesc, setEditingDesc] = useState('')
  const [newStepTitle, setNewStepTitle] = useState('')

  useEffect(() => { checkPlanStatus() }, [])

  const checkPlanStatus = async () => {
    try {
      const result = await toolGetPlanStatus() as { output?: string }
      if (result.output?.includes('PLAN MODE ACTIVE')) setPlanMode('active')
      else setPlanMode('inactive')
    } catch (e) { console.error(e) }
  }

  const STATUS_CONFIG: Record<string, { color: string; icon: string; labelKey: string }> = {
    pending: { color: 'text-gray-400 bg-gray-500/10', icon: '○', labelKey: 'panels.plan_editor.pending' },
    in_progress: { color: 'text-blue-400 bg-blue-500/10', icon: '◐', labelKey: 'panels.plan_editor.in_progress' },
    completed: { color: 'text-green-400 bg-green-500/10', icon: '●', labelKey: 'panels.plan_editor.completed' },
    skipped: { color: 'text-yellow-400 bg-yellow-500/10', icon: '⊘', labelKey: 'panels.plan_editor.skipped' },
    blocked: { color: 'text-red-400 bg-red-500/10', icon: '⊘', labelKey: 'panels.plan_editor.blocked' },
  }

  const STATUS_LABELS: Record<string, string> = {
    draft: t('panels.plan_editor.status_draft'), reviewing: t('panels.plan_editor.status_reviewing'), approved: t('panels.plan_editor.status_approved'),
    executing: t('panels.plan_editor.status_executing'), completed: t('panels.plan_editor.status_completed'), cancelled: t('panels.plan_editor.status_cancelled'),
  }

  const handleEnterPlanMode = async () => {
    await toolEnterPlanMode()
    setPlanMode('active')
    setPlan({ id: crypto.randomUUID(), title: '', description: '', steps: [], status: 'draft', createdAt: Date.now(), updatedAt: Date.now() })
  }

  const handleExitPlanMode = async () => {
    await toolExitPlanMode()
    setPlanMode('inactive')
    setPlan(null)
  }

  const handleAddStep = () => {
    if (!newStepTitle.trim() || !plan) return
    setPlan(prev => prev ? { ...prev, steps: [...prev.steps, { id: crypto.randomUUID(), title: newStepTitle.trim(), description: '', status: 'pending', tools: [], dependencies: [] }], updatedAt: Date.now() } : null)
    setNewStepTitle('')
  }

  const handleUpdateStep = (stepId: string, updates: Partial<PlanStep>) => {
    setPlan(prev => prev ? { ...prev, steps: prev.steps.map(s => s.id === stepId ? { ...s, ...updates } : s), updatedAt: Date.now() } : null)
  }

  const handleRemoveStep = (stepId: string) => {
    setPlan(prev => prev ? { ...prev, steps: prev.steps.filter(s => s.id !== stepId), updatedAt: Date.now() } : null)
  }

  const handleApprovePlan = async () => {
    if (!plan) return
    setPlan(prev => prev ? { ...prev, status: 'approved' as const, updatedAt: Date.now() } : null)
  }

  return (
    <div className="space-y-5">
      <div className="flex items-center justify-between">
        <div>
          <h3 className="text-base font-semibold text-dark-text flex items-center gap-2">
            <svg className="w-5 h-5 text-purple-400" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={2}><path strokeLinecap="round" strokeLinejoin="round" d="M9 5H7a2 2 0 00-2 2v12a2 2 0 002 2h10a2 2 0 002-2V7a2 2 0 00-2-2h-2M9 5a2 2 0 002 2h2a2 2 0 002-2M9 5a2 2 0 012-2m-3 7h8m-8 4h8m-8 4h8"/></svg>
            {t('panels.plan_editor.title')}
          </h3>
          <p className="text-xs text-dark-muted mt-0.5">{t('panels.plan_editor.description')}</p>
        </div>
        <div className={`px-3 py-1.5 rounded-full text-xs font-medium flex items-center gap-1.5 ${planMode === 'active' ? 'bg-purple-600/15 text-purple-300 border border-purple-500/30' : 'bg-dark-bg border border-dark-border text-dark-muted'}`}>
          <span className={`w-1.5 h-1.5 rounded-full ${planMode === 'active' ? 'bg-purple-400 animate-pulse' : 'bg-dark-muted'}`} />
          {planMode === 'active' ? t('panels.plan_editor.plan_mode_active') : t('panels.plan_editor.normal_mode')}
        </div>
      </div>

      {/* Plan mode toggle */}
      <div className="flex gap-2">
        {planMode === 'inactive' ? (
          <button onClick={handleEnterPlanMode} className="flex-1 py-2.5 px-4 rounded-xl bg-purple-600 hover:bg-purple-500 text-white text-sm font-medium transition-all flex items-center justify-center gap-2 shadow-lg shadow-purple-600/20">
            <svg className="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={2}><path strokeLinecap="round" strokeLinejoin="round" d="M11 5H6a2 2 0 00-2 2v11a2 2 0 002 2h11a2 2 0 002-2v-5m-1.414-9.414a2 2 0 112.828 2.828L11.828 15H9v-2.828l8.586-8.586z"/></svg>
            {t('panels.plan_editor.enter_plan_mode')}
          </button>
        ) : (
          <>
            <button onClick={handleExitPlanMode} className="py-2 px-4 rounded-lg border border-dark-border text-sm text-dark-muted hover:text-red-400 hover:border-red-500/30 transition-colors">{t('panels.plan_editor.exit')}</button>
            {plan && plan.status !== 'approved' && (<button onClick={handleApprovePlan} disabled={!plan.title || plan.steps.length === 0} className="flex-1 py-2 px-4 rounded-lg bg-primary-600 hover:bg-primary-500 text-white text-sm font-medium disabled:opacity-40 transition-colors">{t('panels.plan_editor.approve_plan')}</button>)}
            {plan && plan.status === 'approved' && (<button className="flex-1 py-2 px-4 rounded-lg bg-green-600 hover:bg-green-500 text-white text-sm font-medium transition-colors">{t('panels.plan_editor.start_execution')}</button>)}
          </>
        )}
      </div>

      {/* Plan editor */}
      {planMode === 'active' && (
        <div className="space-y-4 p-4 rounded-xl border border-purple-500/20 bg-purple-600/5">
          {!plan ? (
            <div className="text-center py-8 text-sm text-dark-muted">{t('panels.plan_editor.click_to_enter')}</div>
          ) : (
            <>
              <div className="space-y-3">
                <input value={plan.title} onChange={e => setPlan(p => p ? { ...p, title: e.target.value, updatedAt: Date.now() } : null)} placeholder={t('panels.plan_editor.plan_title')} className="w-full bg-transparent text-base font-semibold text-dark-text outline-none placeholder-dark-muted/30" />
                <textarea value={plan.description} onChange={e => setPlan(p => p ? { ...p, description: e.target.value, updatedAt: Date.now() } : null)} placeholder={t('panels.plan_editor.plan_description')} rows={2} className="w-full bg-dark-bg border border-dark-border rounded-lg px-3 py-2 text-xs text-dark-text outline-none focus:border-purple-500 resize-none placeholder-dark-muted/30" />
                <div className="flex items-center gap-2">
                  <span className="text-[10px] text-dark-muted">{t('panels.plan_editor.status')}:</span>
                  <span className={`px-2 py-0.5 rounded text-[10px] font-medium ${plan.status === 'draft' ? 'bg-gray-500/10 text-gray-400' : plan.status === 'approved' ? 'bg-green-500/10 text-green-400' : plan.status === 'executing' ? 'bg-blue-500/10 text-blue-400' : plan.status === 'completed' ? 'bg-primary-500/10 text-primary-300' : 'bg-dark-border text-dark-muted'}`}>{STATUS_LABELS[plan.status]}</span>
                </div>
              </div>

              <div className="space-y-2">
                <div className="flex items-center justify-between">
                  <span className="text-xs font-semibold text-dark-text">{t('panels.plan_editor.steps_count', { count: String(plan.steps.length) })}</span>
                  <span className="text-[10px] text-dark-muted">{t('panels.plan_editor.steps_description')}</span>
                </div>
                {plan.steps.map((step, i) => {
                  const cfg = STATUS_CONFIG[step.status]
                  return (
                    <div key={step.id} className="group p-3 rounded-lg bg-dark-bg border border-dark-border space-y-2">
                      <div className="flex items-start gap-3">
                        <span className={`w-6 h-6 rounded-full flex items-center justify-center text-[10px] font-bold shrink-0 mt-0.5 ${cfg.color}`}>{i + 1}</span>
                        <div className="flex-1 min-w-0">
                          <input value={step.title} onChange={e => handleUpdateStep(step.id, { title: e.target.value })} className="w-full bg-transparent text-sm font-medium text-dark-text outline-none" />
                          <input value={step.description} onChange={e => handleUpdateStep(step.id, { description: e.target.value })} placeholder={t('panels.plan_editor.step_description')} className="w-full bg-transparent text-[11px] text-dark-muted outline-none mt-0.5 placeholder-dark-muted/30" />
                        </div>
                        <select value={step.status} onChange={e => handleUpdateStep(step.id, { status: e.target.value as 'pending' | 'in_progress' | 'completed' | 'skipped' | 'blocked' })} className="text-[10px] bg-dark-surface border border-dark-border rounded px-1.5 py-1 text-dark-text outline-none opacity-0 group-hover:opacity-100 transition-opacity">
                          <option value="pending">{t(STATUS_CONFIG.pending.labelKey)}</option>
                          <option value="in_progress">{t(STATUS_CONFIG.in_progress.labelKey)}</option>
                          <option value="completed">{t(STATUS_CONFIG.completed.labelKey)}</option>
                          <option value="skipped">{t(STATUS_CONFIG.skipped.labelKey)}</option>
                        </select>
                        <button onClick={() => handleRemoveStep(step.id)} className="opacity-0 group-hover:opacity-100 p-1 rounded hover:bg-red-500/10 text-dark-muted hover:text-red-400 transition-all shrink-0"><svg className="w-3.5 h-3.5" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={2}><path strokeLinecap="round" strokeLinejoin="round" d="M19 7l-.867 12.142A2 2 0 0116.138 21H7.862a2 2 0 01-1.995-1.858L5 7m5 4v6m4-6v6m1-10V4a1 1 0 00-1-1h-4a1 1 0 00-1 1v3M4 7h16"/></svg></button>
                      </div>
                      <div className="flex items-center gap-2 ml-9">
                        <div className="flex-1 h-1 bg-dark-border rounded-full overflow-hidden">
                          <div className={`h-full rounded-full transition-all ${step.status === 'completed' ? 'bg-green-500 w-full' : step.status === 'in_progress' ? 'bg-blue-500 w-1/2 animate-pulse' : step.status === 'blocked' ? 'bg-red-500 w-0' : 'bg-dark-muted w-0'}`} />
                        </div>
                      </div>
                    </div>
                  )
                })}
                <div className="flex gap-2">
                  <input value={newStepTitle} onChange={e => setNewStepTitle(e.target.value)} onKeyDown={e => { if (e.key === 'Enter') handleAddStep() }} placeholder={t('panels.plan_editor.add_step_placeholder')} className="flex-1 bg-dark-bg border border-dashed border-dark-border rounded-lg px-3 py-2 text-xs text-dark-text outline-none focus:border-purple-500 placeholder-dark-muted/30" />
                  <button onClick={handleAddStep} disabled={!newStepTitle.trim()} className="px-3 py-2 rounded-lg bg-purple-600/10 text-purple-400 border border-purple-500/20 hover:bg-purple-600/20 text-xs font-medium disabled:opacity-40 transition-colors">{t('panels.plan_editor.add')}</button>
                </div>
              </div>
            </>
          )}
        </div>
      )}

      {planMode === 'inactive' && (
        <div className="rounded-xl border border-dark-border p-6 text-center">
          <div className="w-12 h-12 rounded-xl bg-dark-bg flex items-center justify-center mx-auto mb-3">
            <svg className="w-6 h-6 text-dark-muted" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={1.5}><path strokeLinecap="round" strokeLinejoin="round" d="M9 5H7a2 2 0 00-2 2v12a2 2 0 002 2h10a2 2 0 002-2V7a2 2 0 00-2-2h-2M9 5a2 2 0 002 2h2a2 2 0 002-2M9 5a2 2 0 012-2m-3 7h8m-8 4h8m-8 4h8"/></svg>
          </div>
          <p className="text-sm text-dark-text mb-1">{t('panels.plan_editor.plan_mode_inactive')}</p>
          <p className="text-xs text-dark-muted max-w-sm mx-auto">{t('panels.plan_editor.plan_mode_description')}</p>
        </div>
      )}
    </div>
  )
}
