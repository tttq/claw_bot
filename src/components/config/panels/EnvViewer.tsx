// Claw Desktop - 环境变量查看器 - 查看和管理系统环境变量
// 用于调试配置问题

import { useState, useEffect } from 'react'
import { useTranslation } from 'react-i18next'
import { getEnvVariables } from '../../../api/env'

interface EnvVar { name: string; value: string }

export default function EnvViewer({ agentId }: { agentId?: string }) {
  const { t } = useTranslation()
  const [vars, setVars] = useState<EnvVar[]>([])
  const [filter, setFilter] = useState('')
  const [loading, setLoading] = useState(false)
  const [showSecrets, setShowSecrets] = useState(false)

  useEffect(() => { loadVars() }, [agentId])

  const loadVars = async () => {
    setLoading(true)
    try {
      const data = await getEnvVariables() as unknown as { variables: EnvVar[] }
      setVars(data.variables || [])
    } catch (e) { console.error(e) }
    finally { setLoading(false) }
  }

  useEffect(() => { const t = setTimeout(loadVars, 300); return () => clearTimeout(t) }, [filter])

  const isSecret = (name: string, value: string): boolean => {
    const lower = name.toLowerCase()
    return lower.includes('key') || lower.includes('secret') || lower.includes('token') || lower.includes('password') ||
           lower.includes('api') || lower.includes('auth') || (value.length > 80 && !lower.includes('path'))
  }

  const maskValue = (v: string): string => {
    if (v.length <= 8) return '•••••••'
    return `${v.substring(0, 6)}${'•'.repeat(Math.min(v.length - 6, 20))}${v.length > 26 ? '...' : ''}`
  }

  const filteredVars = vars.filter(v =>
    !filter || v.name.toLowerCase().includes(filter.toLowerCase()) || v.value.toLowerCase().includes(filter.toLowerCase())
  )

  const secretCount = vars.filter(v => isSecret(v.name, v.value)).length

  return (
    <div className="space-y-4">
      <div className="flex items-center justify-between">
        <div>
          <h3 className="text-base font-semibold text-dark-text flex items-center gap-2">
            <svg className="w-5 h-5 text-yellow-400" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={2}><path strokeLinecap="round" strokeLinejoin="round" d="M9.75 17L9 20l-1 1h8l-1-1-.75-3M3 13h18M5 17h14a2 2 0 002-2V5a2 2 0 00-2-2H5a2 2 0 00-2 2v10a2 2 0 002 2z"/></svg>
            {t('panels.env.title')}
          </h3>
          <p className="text-xs text-dark-muted mt-0.5">{t('panels.env.subtitle', { count: String(vars.length), secretCount: String(secretCount) })}</p>
        </div>
        <button onClick={() => setShowSecrets(!showSecrets)} className={`px-2.5 py-1 rounded text-[11px] border transition-colors ${showSecrets ? 'bg-red-500/10 text-red-400 border-red-500/30' : 'border border-dark-border text-dark-muted hover:text-yellow-400'}`}>
          {showSecrets ? t('panels.env.hide_secrets') : t('panels.env.show_secrets')}
        </button>
      </div>

      {/* Search */}
      <input value={filter} onChange={e => setFilter(e.target.value)} placeholder={t('panels.env.searchPlaceholder')} className="w-full bg-dark-bg border border-dark-border rounded-lg px-3 py-1.5 text-xs text-dark-text outline-none focus:border-primary-500" />

      {/* Variables list */}
      <div className="rounded-xl border border-dark-border overflow-hidden">
        {loading ? (
          <div className="flex justify-center py-8"><div className="w-6 h-6 border-2 border-primary-500 border-t-transparent rounded-full animate-spin"></div></div>
        ) : filteredVars.length === 0 ? (
          <div className="text-center py-8 text-sm text-dark-muted">{t('panels.env.no_match')}</div>
        ) : (
          <div className="max-h-[400px] overflow-y-auto divide-y divide-dark-border font-mono text-[11px]">
            {filteredVars.map((v, i) => {
              const secret = isSecret(v.name, v.value)
              return (
                <div key={i} className={`px-3 py-2 hover:bg-dark-bg/50 transition-colors ${secret && !showSecrets ? 'bg-yellow-500/[0.02]' : ''}`}>
                  <div className="flex items-center gap-2 mb-1">
                    {secret && <span className="text-[9px] px-1 py-0.5 rounded bg-yellow-500/10 text-yellow-500">SECRET</span>}
                    <span className="text-green-400 font-semibold">{v.name}</span>
                  </div>
                  <div className={`${secret && !showSecrets ? 'text-red-300/60' : 'text-dark-muted'} break-all pl-[secret ? 38 : 0]`}>
                    {secret && !showSecrets ? maskValue(v.value) : v.value}
                  </div>
                </div>
              )
            })}
          </div>
        )}
      </div>
    </div>
  )
}
