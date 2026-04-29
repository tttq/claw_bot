// Claw Desktop - Agent人物画像编辑器 - 编辑Agent的性格、专长、沟通风格等画像信息

import { useState, useEffect, useCallback } from 'react'
import { harnessPersonaUpdate, harnessPersonaGet, harnessPersonaBuildEnhancedPrompt } from '../../../api/harness'
import { AgentPersona, CommunicationStyle, CommunicationStyleLabels } from '../../../types'
import { useTranslation } from 'react-i18next'

const CMD = {
  GET: 'harness_persona_get',
  UPDATE: 'harness_persona_update',
} as const

interface AgentPersonaEditorProps {
  agentId: string
  agentName: string
}

const PERSONALITY_PRESETS = [
  'Rigorous', 'Humorous', 'Patient', 'Direct', 'Creative', 'Cautious',
  '热情', '理性', '务实', '细致', '果断', '包容',
]

const LANGUAGE_OPTIONS = [
  { value: 'zh-CN', labelKey: 'panels.persona.langZhCN' },
  { value: 'zh-TW', labelKey: 'panels.persona.langZhTW' },
  { value: 'en', labelKey: 'panels.persona.langEn' },
  { value: 'ja', labelKey: 'panels.persona.langJa' },
  { value: 'ko', labelKey: 'panels.persona.langKo' },
]

export default function AgentPersonaEditor({ agentId, agentName }: AgentPersonaEditorProps) {
  const { t } = useTranslation()
  const [persona, setPersona] = useState<AgentPersona | null>(null)
  const [loading, setLoading] = useState(true)
  const [saving, setSaving] = useState(false)
  const [toast, setToast] = useState<string | null>(null)
  const [hasChanges, setHasChanges] = useState(false)

  useEffect(() => { loadPersona() }, [agentId])

  const showToast = (msg: string) => {
    setToast(msg)
    setTimeout(() => setToast(null), 2500)
  }

  const loadPersona = useCallback(async () => {
    setLoading(true)
    try {
      const result = await harnessPersonaGet({ agent_id: agentId }) as unknown as AgentPersona
      setPersona(result)
    } catch {
      const now = Date.now()
      const defaultPersona: AgentPersona = {
        agent_id: agentId,
        display_name: agentName,
        personality_traits: ['专业', '乐于助人'],
        communication_style: CommunicationStyle.Friendly,
        expertise_domain: '',
        behavior_constraints: [],
        response_tone_instruction: '',
        language_preference: 'zh-CN',
        created_at: now,
        updated_at: now,
      }
      setPersona(defaultPersona)
    } finally {
      setLoading(false)
      setHasChanges(false)
    }
  }, [agentId, agentName])

  const handleSave = async () => {
    if (!persona) return
    setSaving(true)
    try {
      await harnessPersonaUpdate(persona as unknown as Record<string, unknown>)
      setHasChanges(false)
      showToast(t('panels.persona.saved'))
    } catch (e) {
      showToast(`${t('panels.persona.saveFailed')} ${e instanceof Error ? e.message : String(e)}`)
    } finally {
      setSaving(false)
    }
  }

  const updateField = <K extends keyof AgentPersona>(key: K, value: AgentPersona[K]) => {
    if (!persona) return
    setPersona({ ...persona, [key]: value })
    setHasChanges(true)
  }

  const toggleTrait = (trait: string) => {
    if (!persona) return
    const traits = persona.personality_traits.includes(trait)
      ? persona.personality_traits.filter(tr => tr !== trait)
      : [...persona.personality_traits, trait]
    updateField('personality_traits', traits)
  }

  const addTrait = (trait: string) => {
    if (!persona || !trait.trim()) return
    if (!persona.personality_traits.includes(trait.trim())) {
      updateField('personality_traits', [...persona.personality_traits, trait.trim()])
    }
  }

  const removeTrait = (trait: string) => {
    if (!persona) return
    updateField('personality_traits', persona.personality_traits.filter(tr => tr !== trait))
  }

  const addConstraint = (constraint: string) => {
    if (!persona || !constraint.trim()) return
    updateField('behavior_constraints', [...persona.behavior_constraints, constraint.trim()])
  }

  const removeConstraint = (idx: number) => {
    if (!persona) return
    updateField('behavior_constraints', persona.behavior_constraints.filter((_, i) => i !== idx))
  }

  if (loading) {
    return (
      <div className="flex justify-center py-12">
        <div className="w-7 h-7 border-2 border-primary-500 border-t-transparent rounded-full animate-spin" />
      </div>
    )
  }

  if (!persona) return null

  return (
    <div className="space-y-5">
      {/* Header */}
      <div className="flex items-center justify-between">
        <div>
          <h3 className="text-base font-semibold text-dark-text flex items-center gap-2">
            <svg className="w-5 h-5 text-purple-400" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={2}>
              <path strokeLinecap="round" strokeLinejoin="round" d="M16 7a4 4 0 11-8 0 4 4 0 018 0zM12 14a7 7 0 00-7 7h14a7 7 0 00-7-7z" />
            </svg>
            {t('panels.persona.title')}
          </h3>
          <p className="text-[10px] text-dark-muted mt-0.5">{t('panels.persona.description')}</p>
        </div>
        <div className="flex items-center gap-2">
          {hasChanges && <span className="text-[10px] text-yellow-400">{t('panels.persona.unsavedChanges')}</span>}
          <button
            onClick={handleSave}
            disabled={saving || !hasChanges}
            className={`px-3 py-1.5 rounded-lg text-[11px] font-medium transition-all ${saving ? 'bg-primary-600/50 text-white/70 cursor-wait' : hasChanges ? 'bg-primary-600 hover:bg-primary-500 text-white shadow-lg shadow-primary-600/20' : 'bg-dark-border text-dark-muted cursor-not-allowed'}`}
          >
            {saving ? t('panels.persona.saving') : t('panels.persona.save')}
          </button>
        </div>
      </div>

      {toast && (
        <div className="px-3 py-2 rounded-lg bg-primary-600/10 border border-primary-500/20 text-xs text-primary-300 animate-fade-in">
          {toast}
        </div>
      )}

      {/* Display Name */}
      <div>
        <label className="block text-[11px] font-medium text-dark-text mb-1.5">{t('panels.persona.displayNameLabel')}</label>
        <input
          value={persona.display_name}
          onChange={e => updateField('display_name', e.target.value)}
          placeholder={t('panels.persona.displayNamePlaceholder')}
          className="w-full bg-dark-bg border border-dark-border rounded-lg px-3 py-2 text-sm text-dark-text outline-none focus:border-primary-500 transition-colors"
        />
        <p className="text-[9px] text-dark-muted/50 mt-1">{t('panels.persona.displayNameHint')}</p>
      </div>

      {/* Personality Traits */}
      <div>
        <label className="block text-[11px] font-medium text-dark-text mb-1.5">{t('panels.persona.personalityLabel')}</label>
        <div className="flex flex-wrap gap-1.5 mb-2">
          {persona.personality_traits.map(trait => (
            <span key={trait} className="group inline-flex items-center gap-1 px-2.5 py-1 rounded-full bg-purple-500/10 border border-purple-500/20 text-xs text-purple-300 transition-all">
              {trait}
              <button onClick={() => removeTrait(trait)} className="opacity-50 group-hover:opacity-100 hover:text-red-400 transition-opacity">
                <svg className="w-3 h-3" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={2}><path strokeLinecap="round" strokeLinejoin="round" d="M6 18L18 6M6 6l12 12" /></svg>
              </button>
            </span>
          ))}
        </div>
        <div className="flex flex-wrap gap-1 mb-2">
          {PERSONALITY_PRESETS.filter(p => !persona.personality_traits.includes(p)).map(preset => (
            <button key={preset} onClick={() => toggleTrait(preset)} className="px-2 py-0.5 rounded text-[10px] bg-dark-bg border border-dark-border text-dark-muted hover:border-purple-500/30 hover:text-purple-300 transition-all">
              + {preset}
            </button>
          ))}
        </div>
        <TraitInput onAdd={addTrait} placeholderKey="panels.persona.customTraitPlaceholder" addKey="panels.persona.add" />
      </div>

      {/* Communication Style */}
      <div>
        <label className="block text-[11px] font-medium text-dark-text mb-1.5">{t('panels.persona.commStyleLabel')}</label>
        <div className="grid grid-cols-3 gap-1.5">
          {Object.entries(CommunicationStyleLabels).map(([value, label]) => (
            <button
              key={value}
              onClick={() => updateField('communication_style', value as CommunicationStyle)}
              className={`px-3 py-2 rounded-lg text-xs text-center transition-all ${persona.communication_style === value ? 'bg-primary-600 text-white shadow-md shadow-primary-600/20' : 'bg-dark-bg border border-dark-border text-dark-muted hover:border-primary-500/30 hover:text-dark-text'}`}
            >
              {t(label)}
            </button>
          ))}
        </div>
        <p className="text-[9px] text-dark-muted/50 mt-1.5">{t('panels.persona.commStyleHint')}</p>
      </div>

      {/* Expertise Domain */}
      <div>
        <label className="block text-[11px] font-medium text-dark-text mb-1.5">{t('panels.persona.expertiseLabel')}</label>
        <input
          value={persona.expertise_domain}
          onChange={e => updateField('expertise_domain', e.target.value)}
          placeholder={t('panels.persona.expertisePlaceholder')}
          className="w-full bg-dark-bg border border-dark-border rounded-lg px-3 py-2 text-sm text-dark-text outline-none focus:border-primary-500 transition-colors"
        />
        <p className="text-[9px] text-dark-muted/50 mt-1">{t('panels.persona.expertiseHint')}</p>
      </div>

      {/* Behavior Constraints */}
      <div>
        <label className="block text-[11px] font-medium text-dark-text mb-1.5">{t('panels.persona.constraintsLabel')}</label>
        <div className="space-y-1.5 mb-2">
          {persona.behavior_constraints.map((constraint, idx) => (
            <div key={idx} className="group flex items-center gap-2 px-3 py-1.5 rounded-lg bg-amber-500/5 border border-amber-500/15">
              <span className="flex-1 text-xs text-amber-300/80">{constraint}</span>
              <button onClick={() => removeConstraint(idx)} className="opacity-40 group-hover:opacity-100 hover:text-red-400 transition-opacity">
                <svg className="w-3.5 h-3.5" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={2}><path strokeLinecap="round" strokeLinejoin="round" d="M6 18L18 6M6 6l12 12" /></svg>
              </button>
            </div>
          ))}
          {persona.behavior_constraints.length === 0 && (
            <p className="text-[10px] text-dark-muted/40 py-1">{t('panels.persona.noConstraints')}</p>
          )}
        </div>
        <ConstraintInput onAdd={addConstraint} placeholderKey="panels.persona.constraintPlaceholder" addKey="panels.persona.add" />
      </div>

      {/* Response Tone Instruction */}
      <div>
        <label className="block text-[11px] font-medium text-dark-text mb-1.5">{t('panels.persona.toneLabel')}</label>
        <textarea
          value={persona.response_tone_instruction}
          onChange={e => updateField('response_tone_instruction', e.target.value)}
          rows={3}
          placeholder={t('panels.persona.tonePlaceholder')}
          className="w-full bg-dark-bg border border-dark-border rounded-lg px-3 py-2 text-xs text-dark-text outline-none focus:border-primary-500 resize-none font-mono leading-relaxed transition-colors"
        />
        <p className="text-[9px] text-dark-muted/50 mt-1">{t('panels.persona.toneHint')}</p>
      </div>

      {/* Language Preference */}
      <div>
        <label className="block text-[11px] font-medium text-dark-text mb-1.5">{t('panels.persona.languageLabel')}</label>
        <div className="flex gap-1.5">
          {LANGUAGE_OPTIONS.map(opt => (
            <button
              key={opt.value}
              onClick={() => updateField('language_preference', opt.value)}
              className={`px-3 py-1.5 rounded-lg text-xs transition-all ${persona.language_preference === opt.value ? 'bg-primary-600 text-white' : 'bg-dark-bg border border-dark-border text-dark-muted hover:border-primary-500/30'}`}
            >
              {t(opt.labelKey)}
            </button>
          ))}
        </div>
      </div>

      {/* Prompt Preview */}
      <PersonaPreview agentId={agentId} persona={persona} previewPromptKey="panels.persona.previewPrompt" />

      {/* Timestamps */}
      <div className="flex items-center gap-4 text-[9px] text-dark-muted/40 pt-2 border-t border-dark-border/50">
        <span>{t('panels.persona.createdAt')} {new Date(persona.created_at).toLocaleString()}</span>
        <span>{t('panels.persona.updatedAt')} {new Date(persona.updated_at).toLocaleString()}</span>
      </div>
    </div>
  )
}

// ==================== Sub-component: Trait Input ====================

function TraitInput({ onAdd, placeholderKey, addKey }: { onAdd: (trait: string) => void; placeholderKey: string; addKey: string }) {
  const { t } = useTranslation()
  const [value, setValue] = useState('')
  const handleAdd = () => {
    if (value.trim()) { onAdd(value.trim()); setValue('') }
  }
  return (
    <div className="flex gap-1.5">
      <input
        value={value}
        onChange={e => setValue(e.target.value)}
        onKeyDown={e => e.key === 'Enter' && handleAdd()}
        placeholder={t(placeholderKey)}
        className="flex-1 bg-dark-bg border border-dark-border rounded-lg px-2.5 py-1.5 text-[11px] text-dark-text outline-none focus:border-purple-500 transition-colors"
      />
      <button onClick={handleAdd} disabled={!value.trim()} className="px-3 py-1.5 rounded-lg bg-purple-600/80 hover:bg-purple-500 text-white text-[11px] disabled:opacity-30 transition-all">
        {t(addKey)}
      </button>
    </div>
  )
}

// ==================== Sub-component: Constraint Input ====================

function ConstraintInput({ onAdd, placeholderKey, addKey }: { onAdd: (constraint: string) => void; placeholderKey: string; addKey: string }) {
  const { t } = useTranslation()
  const [value, setValue] = useState('')
  const handleAdd = () => {
    if (value.trim()) { onAdd(value.trim()); setValue('') }
  }
  return (
    <div className="flex gap-1.5">
      <input
        value={value}
        onChange={e => setValue(e.target.value)}
        onKeyDown={e => e.key === 'Enter' && handleAdd()}
        placeholder={t(placeholderKey)}
        className="flex-1 bg-dark-bg border border-dark-border rounded-lg px-2.5 py-1.5 text-[11px] text-dark-text outline-none focus:border-amber-500 transition-colors"
      />
      <button onClick={handleAdd} disabled={!value.trim()} className="px-3 py-1.5 rounded-lg bg-amber-600/80 hover:bg-amber-500 text-white text-[11px] disabled:opacity-30 transition-all">
        {t(addKey)}
      </button>
    </div>
  )
}

// ==================== Sub-component: Prompt Preview ====================

function PersonaPreview({ agentId, persona, previewPromptKey }: { agentId: string; persona: AgentPersona; previewPromptKey: string }) {
  const { t } = useTranslation()
  const [preview, setPreview] = useState<string | null>(null)
  const [loading, setLoading] = useState(false)

  const handlePreview = async () => {
    setLoading(true)
    try {
      const result = await harnessPersonaBuildEnhancedPrompt({
        agent_id: agentId,
        base_prompt: '[base system prompt]',
      }) as { enhanced_prompt?: string }
      setPreview(result.enhanced_prompt || '')
    } catch {
      setPreview(buildLocalPreview(persona))
    } finally {
      setLoading(false)
    }
  }

  return (
    <div className="rounded-xl border border-dark-border bg-dark-bg overflow-hidden">
      <button
        onClick={handlePreview}
        disabled={loading}
        className="w-full flex items-center justify-between px-4 py-2.5 text-xs font-medium text-dark-muted hover:text-dark-text hover:bg-dark-surface/50 transition-colors"
      >
        <span className="flex items-center gap-1.5">
          <svg className="w-3.5 h-3.5" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={2}>
            <path strokeLinecap="round" strokeLinejoin="round" d="M15 12a3 3 0 11-6 0 3 3 0 016 0z" />
            <path strokeLinecap="round" strokeLinejoin="round" d="M2.458 12C3.732 7.943 7.523 5 12 5c4.478 0 8.268 2.943 9.542 7-1.274 4.057-5.064 7-9.542 7-4.477 0-8.268-2.943-9.542-7z" />
          </svg>
          {t(previewPromptKey)}
        </span>
        <svg className={`w-3.5 h-3.5 transition-transform ${preview ? 'rotate-180' : ''}`} fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={2}>
          <path strokeLinecap="round" strokeLinejoin="round" d="M19 9l-7 7-7-7" />
        </svg>
      </button>
      {preview && (
        <div className="px-4 py-3 border-t border-dark-border">
          <pre className="text-[10px] text-dark-muted/70 font-mono leading-relaxed whitespace-pre-wrap">{preview}</pre>
        </div>
      )}
    </div>
  )
}

function buildLocalPreview(p: AgentPersona): string {
  const lines: string[] = [`## Your Persona: ${p.display_name}`]
  if (p.personality_traits.length) lines.push(`- Personality traits: ${p.personality_traits.join(', ')}`)
  lines.push(`- Communication style: ${p.communication_style}`)
  if (p.expertise_domain) lines.push(`- Expertise domain: ${p.expertise_domain}`)
  if (p.behavior_constraints.length) lines.push(`- Constraints: ${p.behavior_constraints.join('; ')}`)
  if (p.response_tone_instruction) lines.push(`- Tone instruction: ${p.response_tone_instruction}`)
  if (p.language_preference) lines.push(`- Respond in: ${p.language_preference}`)
  return lines.join('\n')
}
