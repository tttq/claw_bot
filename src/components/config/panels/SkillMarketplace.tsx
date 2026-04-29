// Claw Desktop - 技能市场 - 浏览、搜索和安装社区技能
import { useState, useEffect, useCallback } from 'react'
import { useTranslation } from 'react-i18next'
import { isoGetConfig, skillMarketplaceList, skillMarketplaceFiles, skillInstall } from '../../../api'

interface MarketplaceSkill {
  slug: string
  name: string
  description: string
  description_zh: string
  category: string
  version: string
  ownerName: string
  homepage: string
  iconUrl: string | null
  stars: number
  downloads: number
  installs: number
  score: number
  updatedAt: number
}

interface SkillFile {
  path: string
  sha256: string
  size: number
}

const SKILL_DOWNLOAD_BASE = 'https://skillhub-1388575217.cos.accelerate.myqcloud.com/skills'

const CATEGORY_LABELS: Record<string, { labelKey: string; icon: string; color: string }> = {
  'developer-tools': { labelKey: 'panels.skill_marketplace.category_dev', icon: '⚙️', color: 'bg-blue-500/10 text-blue-400 border-blue-500/20' },
  'data-analysis': { labelKey: 'panels.skill_marketplace.category_data', icon: '📊', color: 'bg-green-500/10 text-green-400 border-green-500/20' },
  'content-creation': { labelKey: 'panels.skill_marketplace.category_content', icon: '✍️', color: 'bg-purple-500/10 text-purple-400 border-purple-500/20' },
  'ai-intelligence': { labelKey: 'panels.skill_marketplace.category_ai', icon: '🧠', color: 'bg-orange-500/10 text-orange-400 border-orange-500/20' },
  'productivity': { labelKey: 'panels.skill_marketplace.category_productivity', icon: '⚡', color: 'bg-yellow-500/10 text-yellow-400 border-yellow-500/20' },
}

export default function SkillMarketplace({ agentId, onInstalled }: { agentId: string; onInstalled?: () => void }) {
  const { t } = useTranslation()
  const [skills, setSkills] = useState<MarketplaceSkill[]>([])
  const [installedSlugs, setInstalledSlugs] = useState<Set<string>>(new Set())
  const [loading, setLoading] = useState(true)
  const [error, setError] = useState<string | null>(null)
  const [keyword, setKeyword] = useState('')
  const [categoryFilter, setCategoryFilter] = useState('')
  const [sortBy, setSortBy] = useState('score')
  const [page, setPage] = useState(1)
  const [total, setTotal] = useState(0)
  const [selectedSkill, setSelectedSkill] = useState<MarketplaceSkill | null>(null)
  const [skillFiles, setSkillFiles] = useState<SkillFile[]>([])
  const [installing, setInstalling] = useState<string | null>(null)
  const [installProgress, setInstallProgress] = useState<{ status: string; percent: number } | null>(null)

  const pageSize = 24

  useEffect(() => {
    loadInstalledSkills()
  }, [agentId])

  const loadInstalledSkills = async () => {
    try {
      const result: any = await isoGetConfig({ agentId, key: 'skills_enabled' })
      if (result && Array.isArray(result)) {
        setInstalledSlugs(new Set(result.map((s: any) => s.id || s.slug)))
      }
    } catch (e) { console.error(e) }
  }

  const loadSkills = useCallback(async () => {
    setLoading(true)
    setError(null)
    try {
      const result: any = await skillMarketplaceList({
        page,
        pageSize,
        keyword: keyword.trim() || undefined,
        category: categoryFilter || undefined,
        sortBy,
        order: 'desc',
      })
      if (result.code === 0) {
        setSkills(result.data.skills || [])
        setTotal(result.data.total || 0)
      } else {
        setError(result.message || t('panels.skill_marketplace.unknown_error'))
      }
    } catch (e: any) {
      console.error('[Marketplace] Failed to load skills:', e)
      setError(e?.message || e || t('panels.skill_marketplace.load_failed'))
    } finally {
      setLoading(false)
    }
  }, [page, keyword, categoryFilter, sortBy])

  useEffect(() => { loadSkills() }, [loadSkills])

  const loadSkillFiles = async (skill: MarketplaceSkill) => {
    setSelectedSkill(skill)
    setSkillFiles([])
    try {
      const result: any = await skillMarketplaceFiles({ slug: skill.slug })
      if (result.files) setSkillFiles(result.files)
    } catch (e) { console.error(e) }
  }

  const installSkill = async (skill: MarketplaceSkill) => {
    if (!agentId) return
    setInstalling(skill.slug)
    setInstallProgress({ status: t('panels.skill_marketplace.preparing'), percent: 0 })

    try {
      setInstallProgress({ status: t('panels.skill_marketplace.downloading'), percent: 30 })
      const downloadUrl = `${SKILL_DOWNLOAD_BASE}/${skill.slug}/${skill.version}.zip`

      const result: any = await skillInstall({
        agentId,
        slug: skill.slug,
        name: skill.name,
        version: skill.version,
        downloadUrl,
      })

      if (result.success || result.installed) {
        setInstallProgress({ status: t('panels.skill_marketplace.install_complete'), percent: 100 })
        setInstalledSlugs(prev => new Set([...prev, skill.slug]))
        setTimeout(() => {
          setInstalling(null)
          setInstallProgress(null)
          setSelectedSkill(null)
          onInstalled?.()
        }, 1500)
      } else {
        throw new Error(result.message || result.error || 'Installation failed')
      }
    } catch (e: any) {
      console.error('[Marketplace] Install failed:', e)
      setInstallProgress({ status: `${t('panels.skill_marketplace.install_failed_prefix')}${e.message}`, percent: 0 })
      setTimeout(() => { setInstalling(null); setInstallProgress(null) }, 3000)
    }
  }

  const categories = [...new Set(skills.map(s => s.category).filter(Boolean))]
  const displaySkills = skills.filter(s => !installedSlugs.has(s.slug))

  return (
    <div className="space-y-4">
      {/* Header */}
      <div className="flex items-center justify-between">
        <div>
          <h3 className="text-base font-semibold text-dark-text">{t('panels.skill_marketplace.title')}</h3>
          <p className="text-[10px] text-dark-muted mt-0.5">{t('panels.skill_marketplace.subtitle', { total: String(total), installed: installedSlugs.size > 0 ? ` · ${t('panels.skill_marketplace.installed_count', { count: String(installedSlugs.size) })}` : '' })}</p>
        </div>
        <a href="https://clawhub.ai" target="_blank" rel="noopener noreferrer" className="text-[11px] text-primary-400 hover:text-primary-300 transition-colors flex items-center gap-1">
          {t('panels.skill_marketplace.visit_clawhub')} →
        </a>
      </div>

      {/* Search + Filter + Sort */}
      <div className="flex gap-2">
        <div className="relative flex-1">
          <svg className="absolute left-3 top-1/2 -translate-y-1/2 w-4 h-4 text-dark-muted" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={2}>
            <path strokeLinecap="round" strokeLinejoin="round" d="M21 21l-6-6m2-5a7 7 0 11-14 0 7 7 0 0114 0z" />
          </svg>
          <input
            type="text"
            value={keyword}
            onChange={e => setKeyword(e.target.value)}
            onKeyDown={e => e.key === 'Enter' && page !== 1 ? setPage(1) : undefined}
            placeholder={t('panels.skill_marketplace.search_placeholder')}
            className="w-full pl-9 pr-3 py-2 text-xs bg-dark-bg border border-dark-border rounded-lg text-dark-text placeholder:text-dark-muted/50 focus:outline-none focus:border-primary-500/50 transition-colors"
          />
        </div>
        <select
          value={categoryFilter}
          onChange={e => setCategoryFilter(e.target.value)}
          className="px-3 py-2 text-xs bg-dark-bg border border-dark-border rounded-lg text-dark-text focus:outline-none focus:border-primary-500/50"
        >
          <option value="">{t('panels.skill_marketplace.all_categories')}</option>
          {categories.map(c => (
            <option key={c} value={c}>{t(CATEGORY_LABELS[c]?.labelKey || '') || c}</option>
          ))}
        </select>
        <select
          value={sortBy}
          onChange={e => setSortBy(e.target.value)}
          className="px-3 py-2 text-xs bg-dark-bg border border-dark-border rounded-lg text-dark-text focus:outline-none focus:border-primary-500/50"
        >
          <option value="score">{t('panels.skill_marketplace.sort_score')}</option>
          <option value="downloads">{t('panels.skill_marketplace.sort_downloads')}</option>
          <option value="stars">{t('panels.skill_marketplace.sort_stars')}</option>
          <option value="updated_at">{t('panels.skill_marketplace.sort_updated')}</option>
        </select>
      </div>

      {/* Error */}
      {error && (
        <div className="p-3 rounded-lg bg-red-500/10 border border-red-500/20 text-red-400 text-xs flex items-center justify-between">
          <span>⚠️ {error}</span>
          <button onClick={() => { setError(null); loadSkills() }} className="ml-2 underline">{t('panels.skill_marketplace.retry')}</button>
        </div>
      )}

      {/* Loading */}
      {loading ? (
        <div className="flex justify-center py-12">
          <div className="w-8 h-8 border-2 border-primary-500 border-t-transparent rounded-full animate-spin" />
        </div>
      ) : (
        <>
          {/* Skill Grid - 隐藏已安装的 */}
          <div className="grid grid-cols-1 md:grid-cols-2 gap-3 max-h-[480px] overflow-y-auto pr-1 custom-scrollbar">
            {displaySkills.length > 0 ? displaySkills.map((skill) => {
              const cat = CATEGORY_LABELS[skill.category]
              return (
                <div
                  key={skill.slug}
                  onClick={() => !installing && loadSkillFiles(skill)}
                  className={`group p-4 rounded-xl bg-dark-bg/60 border cursor-pointer transition-all duration-200 ${
                    selectedSkill?.slug === skill.slug
                      ? 'border-primary-500/50 bg-primary-500/5 shadow-lg shadow-primary-500/5'
                      : 'border-dark-border hover:border-primary-500/30 hover:bg-dark-bg'
                  }`}
                >
                  {/* Header */}
                  <div className="flex items-start gap-3">
                    <div className="w-10 h-10 rounded-lg bg-gradient-to-br from-primary-600 to-purple-600 flex items-center justify-center text-white font-bold text-sm shrink-0 shadow-md">
                      {(skill.name || '?')[0].toUpperCase()}
                    </div>
                    <div className="min-w-0 flex-1">
                      <div className="flex items-center gap-2">
                        <span className="text-sm font-semibold text-dark-text truncate">{skill.name}</span>
                        {cat && (
                          <span className={`text-[9px] px-1.5 py-0.5 rounded-full border font-medium shrink-0 ${cat.color}`}>
                            {cat.icon} {t(cat.labelKey)}
                          </span>
                        )}
                      </div>
                      <p className="text-[11px] text-dark-muted mt-0.5 line-clamp-2 leading-relaxed">{skill.description_zh || skill.description}</p>
                    </div>
                  </div>

                  {/* Meta */}
                  <div className="flex items-center gap-3 mt-3 text-[10px] text-dark-muted">
                    <span className="flex items-center gap-0.5">⭐ {skill.stars}</span>
                    <span className="flex items-center gap-0.5">📥 {skill.downloads > 1000 ? `${(skill.downloads / 1000).toFixed(1)}k` : skill.downloads}</span>
                    <span className="flex items-center gap-0.5">📦 v{skill.version}</span>
                    <span className="ml-auto text-[9px] text-dark-muted/60">@{skill.ownerName}</span>
                  </div>

                  {/* Install Button */}
                  <div className="mt-3 pt-3 border-t border-dark-border/50 flex justify-end">
                    {installing === skill.slug ? (
                      <div className="flex items-center gap-2 text-[10px] text-primary-400">
                        <div className="w-3.5 h-3.5 border border-primary-400 border-t-transparent rounded-full animate-spin" />
                        {installProgress?.status || t('panels.skill_marketplace.installing')}
                      </div>
                    ) : (
                      <button
                        onClick={(e) => { e.stopPropagation(); installSkill(skill) }}
                        className="px-3 py-1.5 rounded-lg text-[11px] font-medium bg-primary-600 hover:bg-primary-500 text-white transition-all duration-150 hover:shadow-lg hover:shadow-primary-500/20 active:scale-95 flex items-center gap-1"
                      >
                        <svg className="w-3.5 h-3.5" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={2}>
                          <path strokeLinecap="round" strokeLinejoin="round" d="M4 16v1a3 3 0 003 3h10a3 3 0 003-3v-1m-4-4l-4 4m0 0l-4-4m4 4V4" />
                        </svg>
                        {t('panels.skill_marketplace.install')}
                      </button>
                    )}
                  </div>
                </div>
              )
            }) : (
              !loading && (
                <div className="col-span-2 text-center py-12 text-sm text-dark-muted">
                  {installedSlugs.size > 0 ? t('panels.skill_marketplace.all_installed') : t('panels.skill_marketplace.no_match')}
                </div>
              )
            )}
          </div>

          {/* Pagination */}
          {total > pageSize && (
            <div className="flex items-center justify-center gap-3 pt-2">
              <button
                disabled={page <= 1}
                onClick={() => setPage(p => Math.max(1, p - 1))}
                className="px-3 py-1.5 rounded-lg text-xs bg-dark-bg border border-dark-border text-dark-text hover:border-primary-500/30 disabled:opacity-40 disabled:cursor-not-allowed transition-colors"
              >
                {t('panels.skill_marketplace.prev_page')}
              </button>
              <span className="text-xs text-dark-muted">
                {t('panels.skill_marketplace.page_info', { page: String(page), total: String(Math.ceil(total / pageSize)) })}
              </span>
              <button
                disabled={page >= Math.ceil(total / pageSize)}
                onClick={() => setPage(p => p + 1)}
                className="px-3 py-1.5 rounded-lg text-xs bg-dark-bg border border-dark-border text-dark-text hover:border-primary-500/30 disabled:opacity-40 disabled:cursor-not-allowed transition-colors"
              >
                {t('panels.skill_marketplace.next_page')}
              </button>
            </div>
          )}

          {/* Skill Detail Panel (Side Drawer) */}
          {selectedSkill && (
            <div className="fixed inset-0 z-[110] flex justify-end" onClick={() => setSelectedSkill(null)}>
              <div className="absolute inset-0 bg-black/50 backdrop-blur-sm" />
              <div
                className="relative w-full max-w-md h-full bg-dark-surface border-l border-dark-border overflow-y-auto shadow-2xl"
                onClick={e => e.stopPropagation()}
              >
                {/* Detail Header */}
                <div className="sticky top-0 z-10 bg-dark-surface/95 backdrop-blur border-b border-dark-border p-5">
                  <div className="flex items-start justify-between">
                    <div className="min-w-0 flex-1 pr-4">
                      <div className="w-12 h-12 rounded-xl bg-gradient-to-br from-primary-600 to-purple-600 flex items-center justify-center text-white font-bold text-lg mb-3 shadow-lg">
                        {(selectedSkill.name || '?')[0].toUpperCase()}
                      </div>
                      <h2 className="text-lg font-bold text-dark-text">{selectedSkill.name}</h2>
                      <p className="text-xs text-dark-muted mt-1 line-clamp-2">{selectedSkill.description_zh || selectedSkill.description}</p>
                      <div className="flex flex-wrap gap-1.5 mt-3">
                        <span className="text-[10px] px-2 py-0.5 rounded-full bg-dark-bg border border-dark-border text-dark-muted">
                          v{selectedSkill.version}
                        </span>
                        <span className="text-[10px] px-2 py-0.5 rounded-full bg-dark-bg border border-dark-border text-dark-muted">
                          @{selectedSkill.ownerName}
                        </span>
                        {CATEGORY_LABELS[selectedSkill.category] && (
                          <span className={`text-[10px] px-2 py-0.5 rounded-full border font-medium ${CATEGORY_LABELS[selectedSkill.category].color}`}>
                            {CATEGORY_LABELS[selectedSkill.category].icon} {t(CATEGORY_LABELS[selectedSkill.category].labelKey)}
                          </span>
                        )}
                        {installedSlugs.has(selectedSkill.slug) && (
                          <span className="text-[10px] px-2 py-0.5 rounded-full bg-green-500/10 text-green-400 border border-green-500/20">
                            ✓ {t('panels.skill_marketplace.installed')}
                          </span>
                        )}
                      </div>
                    </div>
                    <button
                      onClick={() => setSelectedSkill(null)}
                      className="w-8 h-8 rounded-lg bg-dark-bg border border-dark-border flex items-center justify-center text-dark-muted hover:text-dark-text hover:border-dark-border/80 transition-colors shrink-0"
                    >
                      ✕
                    </button>
                  </div>

                  {/* Stats */}
                  <div className="grid grid-cols-4 gap-2 mt-4">
                    {[{labelKey:'panels.skill_marketplace.statStars', value:selectedSkill.stars, icon:'⭐'}, {labelKey:'panels.skill_marketplace.statDownloads', value:selectedSkill.downloads, icon:'📥'}, {labelKey:'panels.skill_marketplace.statInstalls', value:selectedSkill.installs, icon:'📦'}, {labelKey:'panels.skill_marketplace.statScore', value:Math.round(selectedSkill.score), icon:'🎯'}].map(stat => (
                      <div key={stat.labelKey} className="text-center p-2 rounded-lg bg-dark-bg/60">
                        <div className="text-lg font-bold text-dark-text">{typeof stat.value === 'number' && stat.value > 1000 ? `${(stat.value/1000).toFixed(1)}k` : stat.value}</div>
                        <div className="text-[9px] text-dark-muted">{stat.icon} {t(stat.labelKey)}</div>
                      </div>
                    ))}
                  </div>

                  {/* Install CTA */}
                  <div className="mt-4">
                    {installedSlugs.has(selectedSkill.slug) ? (
                      <div className="w-full py-2.5 rounded-xl bg-green-600/10 border border-green-500/30 text-center text-sm text-green-400">
                        ✓ {t('panels.skill_marketplace.agent_installed')}
                      </div>
                    ) : installing === selectedSkill.slug ? (
                      <div className="w-full py-2.5 rounded-xl bg-primary-600/20 border border-primary-500/30 text-center text-sm text-primary-400 flex items-center justify-center gap-2">
                        <div className="w-4 h-4 border-2 border-primary-400 border-t-transparent rounded-full animate-spin" />
                        {installProgress?.status || t('panels.skill_marketplace.installing')}
                        {installProgress && installProgress.percent > 0 && installProgress.percent < 100 && (
                          <div className="ml-2 flex-1 max-w-[100px] h-1.5 bg-primary-900/30 rounded-full overflow-hidden">
                            <div className="h-full bg-primary-400 rounded-full transition-all duration-300" style={{ width: `${installProgress.percent}%` }} />
                          </div>
                        )}
                      </div>
                    ) : (
                      <button
                        onClick={() => installSkill(selectedSkill)}
                        className="w-full py-2.5 rounded-xl bg-primary-600 hover:bg-primary-500 text-white font-medium text-sm transition-all duration-200 hover:shadow-lg hover:shadow-primary-500/25 active:scale-[0.98] flex items-center justify-center gap-2"
                      >
                        <svg className="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={2}>
                          <path strokeLinecap="round" strokeLinejoin="round" d="M4 16v1a3 3 0 003 3h10a3 3 0 003-3v-1m-4-4l-4 4m0 0l-4-4m4 4V4" />
                        </svg>
                        {t('panels.skill_marketplace.install_to_agent')}
                      </button>
                    )}
                    {selectedSkill.homepage && (
                      <a href={selectedSkill.homepage} target="_blank" rel="noopener noreferrer" className="block mt-2 text-center text-[11px] text-primary-400 hover:text-primary-300 transition-colors">
                        {t('panels.skill_marketplace.view_details')}
                      </a>
                    )}
                  </div>
                </div>

                {/* Files List */}
                <div className="p-5">
                  <h3 className="text-sm font-semibold text-dark-text mb-3 flex items-center gap-2">
                    {t('panels.skill_marketplace.file_list', { count: String(skillFiles.length) })}
                  </h3>
                  {skillFiles.length > 0 ? (
                    <div className="space-y-1">
                      {skillFiles.map(file => (
                        <div key={file.path} className="flex items-center gap-2 px-3 py-2 rounded-lg bg-dark-bg/40 hover:bg-dark-bg/70 transition-colors group">
                          <span className="text-[10px] text-dark-muted shrink-0 w-5 text-center">
                            {file.path.endsWith('.md') ? '📄' : file.path.endsWith('.sh') ? '🐚' : file.path.endsWith('.ts') ? '💜' : file.path.endsWith('.js') ? '🟨' : '📎'}
                          </span>
                          <span className="text-[11px] text-dark-text font-mono truncate flex-1 group-hover:text-primary-400 transition-colors">{file.path}</span>
                          <span className="text-[10px] text-dark-muted/50 shrink-0">{file.size > 1024 ? `${(file.size/1024).toFixed(1)}KB` : `${file.size}B`}</span>
                        </div>
                      ))}
                    </div>
                  ) : (
                    <div className="text-center py-6 text-xs text-dark-muted">{t('panels.skill_marketplace.loading_files')}</div>
                  )}
                </div>
              </div>
            </div>
          )}
        </>
      )}
    </div>
  )
}
