// Claw Desktop - 设置面板组件（全屏模态弹窗）
// 功能：5 个配置标签页（通用/模型/API/外观/高级）、本地编辑状态管理、测试连接按钮
// 布局：左侧 Tab 导航栏 + 右侧内容区 + 底部保存栏

import { useState, useEffect } from 'react'
import { useTranslation } from 'react-i18next'
import { testConnection } from '../../api/system'
import type { AppConfig, ModelProvider, ModelProviderModel } from '../../types'

interface SettingsPanelProps {
  config: AppConfig | null     // 当前应用配置（从父组件传入）
  onSave: (config: AppConfig) => void   // 保存配置的回调
  onClose: () => void          // 关闭面板的回调
}

type TabId = 'general' | 'model' | 'tools' | 'appearance' | 'advanced'

let _systemThemeHandler: ((e: MediaQueryListEvent) => void) | null = null
let _systemMq: MediaQueryList | null = null

function applyTheme(theme: 'dark' | 'light' | 'system') {
  if (_systemThemeHandler && _systemMq) {
    _systemMq.removeEventListener('change', _systemThemeHandler)
    _systemThemeHandler = null
    _systemMq = null
  }
  localStorage.setItem('claw-theme', theme)
  const root = document.documentElement
  if (theme === 'light') {
    root.classList.add('light')
  } else if (theme === 'dark') {
    root.classList.remove('light')
  } else {
    const mq = window.matchMedia('(prefers-color-scheme: light)')
    if (mq.matches) root.classList.add('light')
    else root.classList.remove('light')
    const handler = (e: MediaQueryListEvent) => {
      if (e.matches) root.classList.add('light')
      else root.classList.remove('light')
    }
    mq.addEventListener('change', handler)
    _systemThemeHandler = handler
    _systemMq = mq
  }
}

export default function SettingsPanel({ config, onSave, onClose }: SettingsPanelProps) {
  const { t } = useTranslation()
  const [activeTab, setActiveTab] = useState<TabId>('general')           // 当前激活的标签页
  const [localConfig, setLocalConfig] = useState<AppConfig | null>(null) // 本地编辑副本（深拷贝，不直接修改原配置）
  const [hasChanges, setHasChanges] = useState(false)                    // 是否有未保存的修改

  // 当外部 config 加载后，深拷贝到 localConfig 作为编辑基准
  useEffect(() => {
    if (config && !localConfig) {
      setLocalConfig(JSON.parse(JSON.stringify(config)))
    }
  }, [config])

  // ==================== 标签页定义（左侧导航） ====================
  
  const tabs: { id: TabId; label: string; icon: JSX.Element }[] = [
    { id: 'general', label: t('settings.tabs.general'), icon: <svg className="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={2}><path strokeLinecap="round" strokeLinejoin="round" d="M3 12l2-2m0 0l7-7 7 7M5 10v10a1 1 0 001 1h3m10-11l2 2m-2-2v10a1 1 0 01-1 1h-3m-6 0a1 1 0 001-1v-4a1 1 0 011-1h2a1 1 0 011 1v4a1 1 0 001 1m-6 0h6" /></svg> },
    { id: 'model', label: t('settings.tabs.model'), icon: <svg className="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={2}><path strokeLinecap="round" strokeLinejoin="round" d="M9.75 17L9 20l-1 1h8l-1-1-.75-3M3 13h18M5 17h14a2 2 0 002-2V5a2 2 0 00-2-2H5a2 2 0 00-2 2v10a2 2 0 002 2z" /></svg> },
    { id: 'tools', label: t('settings.tabs.tools'), icon: <svg className="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={2}><path strokeLinecap="round" strokeLinejoin="round" d="M11.42 15.17l-5.66-5.66a8 8 0 1111.31 0l-5.65 5.66zm0 0L12 21m-4.24-7.07l1.41-1.42m5.66 0l1.41 1.42" /></svg> },
    { id: 'appearance', label: t('settings.tabs.appearance'), icon: <svg className="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={2}><path strokeLinecap="round" strokeLinejoin="round" d="M7 21a4 4 0 01-4-4V5a2 2 0 012-2h4a2 2 0 012 2v12a4 4 0 01-4 4zm0 0h12a2 2 0 002-2v-4a2 2 0 00-2-2h-2.343M11 7.343l1.657-1.657a2 2 0 012.828 0l2.829 2.829a2 2 0 010 2.828l-8.486 8.485M7 17h.01" /></svg> },
    { id: 'advanced', label: t('settings.tabs.advanced'), icon: <svg className="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={2}><path strokeLinecap="round" strokeLinejoin="round" d="M10.325 4.317c.426-1.756 2.924-1.756 3.35 0a1.724 1.724 0 002.573 1.066c1.543-.94 3.31.826 2.37 2.37a1.724 1.724 0 001.066 2.573c1.756.426 1.756 2.924 0 3.35a1.724 1.724 0 00-1.066 2.573c.94 1.543-.826 3.31-2.37 2.37a1.724 1.724 0 00-2.573 1.066c-.426 1.756-2.924 1.756-3.35 0a1.724 1.724 0 00-2.573-1.066c-1.543.94-3.31-.826-2.37-2.37a1.724 1.724 0 00-1.066-2.573c-1.756-.426-1.756-2.924 0-3.35a1.724 1.724 0 001.066-2.573c-.94-1.543.826-3.31 2.37-2.37.996.608 2.296.07 2.572-1.065z" /><path strokeLinecap="round" strokeLinejoin="round" d="M15 12a3 3 0 11-6 0 3 3 0 016 0z" /></svg> },
  ]

  /// 更新指定配置分组的字段值，同时标记为有修改
  const updateField = (section: keyof AppConfig, field: string, value: unknown) => {
    setLocalConfig(prev => {
      if (!prev) return prev
      const updated = { ...prev, [section]: { ...prev[section], [field]: value } }
      setHasChanges(true)
      return updated
    })
  }

  /// 调用保存回调
  const handleSave = () => {
    if (localConfig) onSave(localConfig)
  }

  // 配置未加载完成时显示加载动画
  if (!localConfig) {
    return (
      <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/50 backdrop-blur-sm">
        <div className="bg-dark-surface rounded-2xl p-8">
          <div className="w-8 h-8 border-2 border-primary-500 border-t-transparent rounded-full animate-spin"></div>
        </div>
      </div>
    )
  }

  // ==================== 主渲染：全屏模态弹窗 ====================

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/50 backdrop-blur-sm animate-fade-in" onClick={onClose}>
      {/* 弹窗容器（点击内部区域阻止关闭） */}
      <div className="bg-dark-surface border border-dark-border rounded-2xl shadow-2xl w-[900px] max-h-[85vh] flex flex-col overflow-hidden animate-fade-in" onClick={e => e.stopPropagation()}>
        {/* ===== 头部：标题 + 关闭按钮 ===== */}
        <div className="flex items-center justify-between px-6 py-3 border-b border-dark-border shrink-0">
          <h2 className="text-base font-bold text-dark-text">{t('settings.title')}</h2>
          <button onClick={onClose} className="p-1.5 rounded-lg hover:bg-dark-border/50 text-dark-muted hover:text-dark-text transition-colors">
            <svg className="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={2}>
              <path strokeLinecap="round" strokeLinejoin="round" d="M6 18L18 6M6 6l12 12" />
            </svg>
          </button>
        </div>

        {/* ===== 主体：左侧 Tab 栏 + 右侧内容区 ===== */}
        <div className="flex flex-1 overflow-hidden">
          {/* 左侧导航栏 */}
          <div className="w-44 border-r border-dark-border py-2 shrink-0">
            {tabs.map(tab => (
              <button
                key={tab.id}
                onClick={() => setActiveTab(tab.id)}
                className={`w-full flex items-center gap-3 px-4 py-2.5 text-sm transition-colors ${
                  activeTab === tab.id
                    ? 'bg-primary-600/10 text-primary-400 border-r-2 border-primary-500'   // 激活态高亮
                    : 'text-dark-muted hover:text-dark-text hover:bg-dark-border/30'            // 普通态
                }`}
              >
                {tab.icon}
                {tab.label}
              </button>
            ))}
          </div>

          {/* 右侧内容区（根据 activeTab 渲染对应子组件） */}
          <div className="flex-1 overflow-y-auto p-6">
            {activeTab === 'general' && (<GeneralTab config={localConfig} updateField={updateField} />)}
            {activeTab === 'model' && (<ModelTab config={localConfig} updateField={updateField} />)}
            {activeTab === 'tools' && (<ToolsTab config={localConfig} updateField={updateField} />)}
            {activeTab === 'appearance' && (<AppearanceTab config={localConfig} updateField={updateField} />)}
            {activeTab === 'advanced' && (<AdvancedTab config={localConfig} updateField={updateField} />)}
          </div>
        </div>

        {/* ===== 底部：未保存提示 + 保存按钮 ===== */}
        {hasChanges && (
          <div className="flex items-center justify-end gap-3 px-6 py-3 border-t border-dark-border shrink-0">
            <span className="text-xs text-yellow-400">{t('settings.unsavedChanges')}</span>
            <button
              onClick={handleSave}
              className="px-4 py-2 rounded-lg bg-primary-600 hover:bg-primary-500 text-white text-sm font-medium transition-colors shadow-lg shadow-primary-600/20"
            >
              {t('settings.saveChanges')}
            </button>
          </div>
        )}
      </div>
    </div>
  )
}

// ==================== 各标签页子组件 ====================

/** 通用设置标签页：语言、主题、自动更新、托盘、启动行为 */
function GeneralTab({ config, updateField }: { config: AppConfig; updateField: (s: keyof AppConfig, f: string, v: unknown) => void }) {
  const { t } = useTranslation()
  return (
    <div className="space-y-6">
      <h3 className="text-base font-semibold text-dark-text pb-2 border-b border-dark-border">{t('settings.general.title')}</h3>

      <SettingField label={t('settings.general.language')} description={t('settings.general.languageDesc')}>
        <select value={config.app.language} onChange={e => updateField('app', 'language', e.target.value)} className="w-full bg-dark-bg border border-dark-border rounded-lg px-3 py-2 text-sm text-dark-text focus:outline-none focus:border-primary-500">
          <option value="zh-CN">{t('settings.general.languageOptions.zh-CN')}</option><option value="en-US">{t('settings.general.languageOptions.en')}</option><option value="ja-JP">{t('settings.general.languageOptions.ja-JP')}</option>
        </select>
      </SettingField>

      <SettingField label={t('settings.general.theme')} description={t('settings.general.themeDesc')}>
        <div className="flex gap-2">
          {(['dark', 'light', 'system'] as const).map(theme => (
            <button key={theme} onClick={() => { updateField('app', 'theme', theme); applyTheme(theme) }} className={`px-4 py-2 rounded-lg text-sm transition-all ${config.app.theme === theme ? 'bg-primary-600 text-white' : 'bg-dark-bg border border-dark-border text-dark-muted hover:text-dark-text'}`}>{t(`settings.general.themeOptions.${theme}`)}</button>
          ))}
        </div>
      </SettingField>

      <ToggleField label={t('settings.general.autoUpdate')} description={t('settings.general.autoUpdateDesc')} checked={config.app.auto_update} onChange={v => updateField('app', 'auto_update', v)} />
      <ToggleField label={t('settings.general.minimizeToTray')} description={t('settings.general.minimizeToTrayDesc')} checked={config.app.minimize_to_tray} onChange={v => updateField('app', 'minimize_to_tray', v)} />

      <SettingField label={t('settings.general.startupBehavior')} description={t('settings.general.startupBehaviorDesc')}>
        <select value={config.app.startup_behavior} onChange={e => updateField('app', 'startup_behavior', e.target.value)} className="w-full bg-dark-bg border border-dark-border rounded-lg px-3 py-2 text-sm text-dark-text focus:outline-none focus:border-primary-500">
          <option value="normal">{t('settings.general.startupOptions.normal')}</option><option value="maximized">{t('settings.general.startupOptions.maximized')}</option><option value="minimized">{t('settings.general.startupOptions.minimized')}</option>
        </select>
      </SettingField>
    </div>
  )
}

/** 模型设置标签页：从 model_providers.json 动态加载提供商，分类展示，三字段配置 */
function ModelTab({ config, updateField }: { config: AppConfig; updateField: (s: keyof AppConfig, f: string, v: unknown) => void }) {
  const { t } = useTranslation()
  const [providers, setProviders] = useState<ModelProvider[] | null>(null)
  const [categories, setCategories] = useState<Array<{ id: string; name: string }>>([])
  const [activeCategory, setActiveCategory] = useState<string>('international')
  const [selectedProviderId, setSelectedProviderId] = useState<string>('')
  const [testStatus, setTestStatus] = useState<'idle' | 'loading' | 'success' | 'error'>('idle')
  const [testMessage, setTestMessage] = useState('')
  const [testLogs, setTestLogs] = useState<Array<{ timestamp: string; level: string; phase: string; detail: string }>>([])
  const [showLogs, setShowLogs] = useState(false)
  const [testResponse, setTestResponse] = useState<Record<string, unknown> | null>(null)

  useEffect(() => {
    fetch('/model_providers.json').then(r => r.json()).then((data: { providers?: ModelProvider[]; categories?: Array<{ id: string; name: string }> }) => {
      setProviders(data.providers || [])
      setCategories(data.categories || [])
      if (data.categories?.[0]) setActiveCategory(data.categories[0].id)
    }).catch((e) => { console.error(e) })
  }, [])

  const currentProvider = providers?.find((p: ModelProvider) => p.id === selectedProviderId)
  const currentModels = currentProvider?.availableModels || currentProvider?.models || []
  const providersInCategory = providers?.filter((p: ModelProvider) => p.category === activeCategory) || []

  const handleSelectProvider = (provider: ModelProvider) => {
    setSelectedProviderId(provider.id)
    updateField('model', 'provider', 'custom')
    const url = provider.defaultBaseUrl || ''
    updateField('model', 'custom_url', url)
    if (provider.availableModels?.[0]) {
      updateField('model', 'custom_model_name', provider.availableModels[0].id)
      updateField('model', 'default_model', provider.availableModels[0].id)
    }
    const isAnthropic = ['anthropic', 'amazon-bedrock'].includes(provider.id)
    updateField('model', 'api_format', isAnthropic ? 'anthropic' : 'openai')
    setTestStatus('idle'); setTestMessage('')
  }

  const isEditableProvider = !selectedProviderId ||
    currentProvider?.category === 'local' ||
    currentProvider?.category === 'proxy' ||
    currentProvider?.category === 'gateway' ||
    currentProvider?.category === 'custom'
  const isAnthropicProvider = ['anthropic', 'amazon-bedrock'].includes(currentProvider?.id ?? '')

  const handleTestConnection = async () => {
    setTestStatus('loading'); setTestMessage(''); setTestLogs([]); setShowLogs(false); setTestResponse(null)
    try {
      const result = await testConnection({ config }) as { success: boolean; message: string; logs?: Array<{ timestamp: string; level: string; phase: string; detail: string }>; response?: Record<string, unknown> }
      if (result.logs && Array.isArray(result.logs)) { setTestLogs(result.logs); setShowLogs(true) }
      if (result.response) setTestResponse(result.response)
      setTestStatus(result.success ? 'success' : 'error')
      setTestMessage(result.message || '')
    } catch (e: unknown) {
      setTestStatus('error'); setTestMessage(String(e))
      setTestLogs([{ timestamp: new Date().toLocaleTimeString(), level: 'ERROR', phase: 'Exception', detail: String(e) }])
      setShowLogs(true)
    }
  }

  const CATEGORY_ICONS: Record<string, string> = {
    international: '🌍', chinese: '🇨🇳', aggregator: '🔗', local: '🏠',
    gateway: '🚪', proxy: '🔀', oauth: '🔐', fast: '⚡',
    search: '🔍', privacy: '🛡️', coding: '💻', transcription: '🎙️',
    tts: '🔊', media: '🎬',
  }
  const CATEGORY_COLORS: Record<string, string> = {
    international: 'from-blue-600/15 to-blue-500/5', chinese: 'from-red-600/15 to-red-500/5',
    aggregator: 'from-purple-600/15 to-purple-500/5', local: 'from-green-600/15 to-green-500/5',
    gateway: 'from-cyan-600/15 to-cyan-500/5', proxy: 'from-gray-600/15 to-gray-500/5',
    oauth: 'from-yellow-600/15 to-yellow-500/5', fast: 'from-orange-600/15 to-orange-500/5',
    search: 'from-indigo-600/15 to-indigo-500/5', privacy: 'from-pink-600/15 to-pink-500/5',
    coding: 'from-emerald-600/15 to-emerald-500/5', transcription: 'from-teal-600/15 to-teal-500/5',
    tts: 'from-violet-600/15 to-violet-500/5', media: 'from-fuchsia-600/15 to-fuchsia-500/5',
  }

  const LEVEL_COLORS: Record<string, string> = {
    INFO: 'text-blue-400', DEBUG: 'text-gray-400', ERROR: 'text-red-400', SUCCESS: 'text-green-400', WARN: 'text-yellow-400',
  }
  const PHASE_ICONS: Record<string, string> = {
    Init: '🚀', Config: '⚙️', APIKey: '🔑', URL: '🌐', Model: '🤖',
    HTTPClient: '📡', Request: '📤', Network: '🌍', Response: '📥',
    Usage: '📊', Result: '✅', Exception: '💥',
  }

  return (
    <div className="space-y-5">
      <h3 className="text-base font-semibold text-dark-text pb-2 border-b border-dark-border">{t('settings.model.title')}</h3>

      {!providers ? (
        <div className="flex justify-center py-8"><div className="w-6 h-6 border-2 border-primary-500 border-t-transparent rounded-full animate-spin"></div></div>
      ) : (
        <>
          {/* ===== 分类标签栏 ===== */}
          <div className="flex gap-1 flex-wrap p-1 rounded-lg bg-dark-bg border border-dark-border">
            {categories.map((cat: { id: string; name: string }) => (
              <button key={cat.id} onClick={() => setActiveCategory(cat.id)}
                className={`flex items-center gap-1.5 px-3 py-1.5 rounded-md text-[11px] transition-all ${
                  activeCategory === cat.id ? 'bg-primary-600 text-white shadow-sm' : 'text-dark-muted hover:text-dark-text hover:bg-dark-surface'
                }`}>
                <span>{CATEGORY_ICONS[cat.id] || '📦'}</span>
                <span>{cat.name}</span>
                <span className={`text-[9px] px-1 rounded-full ${activeCategory === cat.id ? 'bg-white/20' : 'bg-dark-border text-dark-muted'}`}>
                  {providersInCategory.length}
                </span>
              </button>
            ))}
          </div>

          {/* ===== 提供商网格（当前分类下） ===== */}
          <div>
            <div className="flex items-center justify-between mb-2">
              <span className="text-xs font-medium text-dark-muted">
                {categories.find((c: { id: string; name: string }) => c.id === activeCategory)?.name || t('settings.model.providers')}
              </span>
              {selectedProviderId && (
                <span className="text-[10px] px-2 py-0.5 rounded-full bg-primary-600/10 text-primary-300">
                  {currentProvider?.name}
                </span>
              )}
            </div>
            <div className="grid grid-cols-3 sm:grid-cols-4 gap-1.5 max-h-[200px] overflow-y-auto p-1 rounded-lg border border-dark-border bg-dark-bg/50">
              {providersInCategory.map((p: ModelProvider) => (
                <button key={p.id} onClick={() => handleSelectProvider(p)}
                  className={`px-2.5 py-2 rounded-lg text-left transition-all group ${
                    selectedProviderId === p.id
                      ? `bg-gradient-to-br ${CATEGORY_COLORS[p.category] || CATEGORY_COLORS.international} border border-primary-500/30 shadow-sm`
                      : 'border border-transparent hover:border-dark-border hover:bg-dark-surface'
                  }`}>
                  <div className={`text-[11px] font-medium truncate ${selectedProviderId === p.id ? 'text-dark-text' : 'text-dark-text group-hover:text-primary-300'}`}>{p.name}</div>
                  {(p.models?.length ?? 0) > 0 && (
                    <div className={`text-[9px] mt-0.5 ${selectedProviderId === p.id ? 'text-primary-400/70' : 'text-dark-muted'}`}>
                      {p.models!.length} {t('settings.model.modelCount', { count: String(p.models!.length) })}
                    </div>
                  )}
                </button>
              ))}
              {providersInCategory.length === 0 && (
                <div className="col-span-full text-center py-4 text-xs text-dark-muted">{t('settings.model.noProvidersInCategory')}</div>
              )}
            </div>
          </div>

          {/* ===== 三字段配置面板（始终显示） ===== */}
          <div className="space-y-4 p-4 rounded-xl border border-dark-border bg-dark-bg">
            <div className="flex items-center gap-2 mb-1">
              <h4 className="text-sm font-semibold text-dark-text">{t('settings.model.apiConfig')}</h4>
              {currentProvider && (
                <span className="text-[10px] px-2 py-0.5 rounded-full bg-primary-600/10 text-primary-300">
                  via {currentProvider.name}
                </span>
              )}
            </div>

            {/* API Format 选择：自定义/本地/代理/网关 可编辑，其余锁定显示 */}
            <div className="space-y-1.5">
              <div className="flex items-center justify-between">
                <label className="text-[11px] font-medium text-dark-muted">{t('settings.model.apiFormat')}</label>
                {!isEditableProvider && (
                  <span className="text-[9px] px-1.5 py-0.5 rounded bg-dark-surface text-dark-muted border border-dark-border">{t('settings.model.locked')}</span>
                )}
              </div>
              {isEditableProvider ? (
                <div className="flex gap-2">
                  {(['openai', 'anthropic'] as const).map(fmt => (
                    <button key={fmt} onClick={() => updateField('model', 'api_format', fmt)}
                      className={`px-3 py-1.5 rounded-lg text-xs transition-all flex items-center gap-1.5 ${
                        config.model.api_format === fmt ? 'bg-primary-600 text-white' : 'bg-dark-surface border border-dark-border text-dark-muted hover:text-dark-text'
                      }`}>
                      {fmt === 'openai' ? t('settings.model.openai') : t('settings.model.anthropic')}
                    </button>
                  ))}
                </div>
              ) : (
                <div className={`inline-flex items-center gap-1.5 px-3 py-1.5 rounded-lg text-xs ${
                  isAnthropicProvider
                    ? 'bg-orange-500/10 text-orange-400 border border-orange-500/20'
                    : 'bg-blue-500/10 text-blue-400 border border-blue-500/20'
                }`}>
                  {isAnthropicProvider ? t('settings.model.anthropic') : t('settings.model.openai')}
                  <span className="text-[9px] opacity-60">({t('settings.model.autoFill')})</span>
                </div>
              )}
            </div>

            {/* 字段1: Model URL — 仅自定义/本地/代理/网关可编辑 */}
            <div>
              <div className="flex items-center justify-between mb-1">
                <label className="text-[11px] font-medium text-dark-text">{t('settings.model.modelUrl')} <span className="text-dark-muted font-normal">{t('settings.model.modelUrlDesc')}</span></label>
                {!isEditableProvider && (
                  <span className="text-[9px] px-1.5 py-0.5 rounded bg-dark-surface text-dark-muted border border-dark-border">{t('settings.model.readOnly')}</span>
                )}
              </div>
              {isEditableProvider ? (
                <input type="text" value={config.model.custom_url} onChange={e => updateField('model', 'custom_url', e.target.value)}
                placeholder={t('settings.model.urlPlaceholder')}
                className="w-full bg-dark-surface border border-dark-border rounded-lg px-3 py-2 text-sm text-dark-text placeholder-dark-muted/30 focus:outline-none focus:border-primary-500 font-mono" />
              ) : (
                <div className="w-full bg-dark-bg/50 border border-dark-border/50 rounded-lg px-3 py-2 text-sm text-dark-muted font-mono flex items-center gap-2 cursor-not-allowed select-all">
                  <svg className="w-3 h-3 shrink-0 opacity-40" fill="none" viewBox="0 0 24 24" stroke="currentColor"><path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M12 15v2m-6 4h12a2 2 0 002-2v-6a2 2 0 00-2-2H6a2 2 0 00-2 2v6a2 2 0 002 2zm10-10V7a4 4 0 00-8 0v4h8z" /></svg>
                  <span className="truncate">{config.model.custom_url || `(${t('settings.model.autoFill')})`}</span>
                </div>
              )}
            </div>

            {/* 字段2: API Key — 始终可编辑（用户必须输入自己的密钥） */}
            <div>
              <label className="block text-[11px] font-medium text-dark-text mb-1">{t('settings.model.apiKey')} <span className="text-dark-muted font-normal">{t('settings.model.apiKeyDesc')}</span></label>
              <input type="password" value={config.model.custom_api_key} onChange={e => updateField('model', 'custom_api_key', e.target.value)}
                placeholder={t('settings.model.apiKeyPlaceholder')}
                className="w-full bg-dark-surface border border-dark-border rounded-lg px-3 py-2 text-sm text-dark-text placeholder-dark-muted/30 focus:outline-none focus:border-primary-500 font-mono" />
            </div>

            {/* 字段3: Model Name（有模型列表时显示下拉，否则显示输入框） */}
            <div>
              <label className="block text-[11px] font-medium text-dark-text mb-1">{t('settings.model.modelName')} <span className="text-dark-muted font-normal">{t('settings.model.modelNameDesc')}</span></label>
              {currentModels.length > 0 ? (
                <select value={config.model.custom_model_name} onChange={e => { updateField('model', 'custom_model_name', e.target.value); updateField('model', 'default_model', e.target.value) }}
                  className="w-full bg-dark-surface border border-dark-border rounded-lg px-3 py-2 text-sm text-dark-text focus:outline-none focus:border-primary-500">
                  {currentModels.map((m: any) => (
                    <option key={m.id} value={m.id}>
                      {m.name}{m.reasoning ? ' 🧠' : ''}{m.input?.includes('image') ? ' 🖼️' : ''}
                    </option>
                  ))}
                </select>
              ) : (
                <input type="text" value={config.model.custom_model_name} onChange={e => { updateField('model', 'custom_model_name', e.target.value); updateField('model', 'default_model', e.target.value) }}
                  placeholder={t('settings.model.modelNamePlaceholder')}
                  className="w-full bg-dark-surface border border-dark-border rounded-lg px-3 py-2 text-sm text-dark-text placeholder-dark-muted/30 focus:outline-none focus:border-primary-500 font-mono" />
              )}
            </div>
          </div>
        </>
      )}

      {/* ===== 测试连接 + 日志区域 ===== */}
      <div className="space-y-3">
        <div className="flex items-center gap-3 flex-wrap">
          <button onClick={handleTestConnection} disabled={testStatus === 'loading'}
            className={`px-4 py-2 rounded-lg text-sm font-medium transition-all flex items-center gap-2 ${
              testStatus === 'loading' ? 'bg-primary-600/50 text-white/70 cursor-wait'
              : testStatus === 'success' ? 'bg-green-600/20 text-green-400 border border-green-500/30 hover:bg-green-600/30'
              : testStatus === 'error' ? 'bg-red-600/20 text-red-400 border border-red-500/30 hover:bg-red-600/30'
              : 'bg-primary-600 hover:bg-primary-500 text-white shadow-lg shadow-primary-600/20'
            }`}>
            {testStatus === 'loading' && <svg className="animate-spin w-4 h-4" fill="none" viewBox="0 0 24 24"><circle className="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" strokeWidth="4"/><path className="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4z"/></svg>}
            {testStatus === 'success' && <svg className="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={2}><path strokeLinecap="round" strokeLinejoin="round" d="M5 13l4 4L19 7"/></svg>}
            {testStatus === 'error' && <svg className="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={2}><path strokeLinecap="round" strokeLinejoin="round" d="M6 18L18 6M6 6l12 12"/></svg>}
            {testStatus === 'loading' ? t('settings.model.testing') : testStatus === 'success' ? t('settings.model.connectSuccess') : testStatus === 'error' ? t('settings.model.connectFailed') : t('settings.model.testConnection')}
          </button>
          {testMessage && <span className={`text-xs ${testStatus === 'success' ? 'text-green-400' : 'text-red-400'}`}>{testMessage}</span>}
          {testLogs.length > 0 && (
            <button onClick={() => setShowLogs(!showLogs)} className="px-2.5 py-1 rounded text-[11px] border border-dark-border text-dark-muted hover:text-primary-400 hover:border-primary-500/30 transition-colors">
              {showLogs ? t('settings.model.hideLogs') : t('settings.model.showLogs', { count: String(testLogs.length) })}
            </button>
          )}
        </div>

        {/* 详细日志显示区域 */}
        {showLogs && testLogs.length > 0 && (
          <div className="rounded-xl border border-dark-border overflow-hidden bg-dark-bg">
            {/* 日志头部统计 */}
            <div className="flex items-center justify-between px-3 py-1.5 bg-dark-surface border-b border-dark-border">
              <span className="text-[10px] font-medium text-dark-muted">
                {(() => {
                  const timeRange = (() => {
                    if (testLogs.length >= 2) {
                      const first = testLogs[0].timestamp
                      const last = testLogs[testLogs.length - 1].timestamp
                      return `${first} → ${last}`
                    }
                    return ''
                  })()
                  return t('settings.model.testLogTitle', { count: String(testLogs.length), time: timeRange })
                })()}
              </span>
              <div className="flex items-center gap-2">
                {['INFO', 'DEBUG', 'ERROR', 'SUCCESS'].filter(l => testLogs.some(log => log.level === l)).map(l => (
                  <span key={l} className={`text-[9px] px-1.5 py-0.5 rounded ${LEVEL_COLORS[l] || 'text-gray-400'} bg-current/10`}>{l}</span>
                ))}
              </div>
            </div>

            {/* 日志列表 */}
            <div className="max-h-[320px] overflow-y-auto divide-y divide-dark-border/50 p-1">
              {testLogs.map((log, i) => (
                <div key={i} className={`flex items-start gap-2 px-2 py-1.5 hover:bg-dark-surface/50 transition-colors ${log.level === 'ERROR' ? 'bg-red-500/5' : ''}`}>
                  <span className="text-[10px] text-dark-muted shrink-0 pt-0.5 font-mono w-16">{log.timestamp.split('.')[0]}</span>
                  <span className="shrink-0 w-5">{PHASE_ICONS[log.phase] || '📝'}</span>
                  <span className={`text-[10px] font-semibold shrink-0 w-14 ${LEVEL_COLORS[log.level] || 'text-gray-400'}`}>{log.level}</span>
                  <span className="text-[10px] font-medium text-primary-300/70 shrink-0 w-16 truncate">{log.phase}</span>
                  <span className={`text-[11px] break-all ${log.level === 'ERROR' ? 'text-red-300' : log.level === 'SUCCESS' ? 'text-green-300' : 'text-dark-text'}`}>
                    {log.detail}
                  </span>
                </div>
              ))}
            </div>

            {/* 响应数据展示（如果成功且有响应体） */}
            {testResponse && (
              <div className="border-t border-dark-border p-3 space-y-2">
                <span className="text-[10px] font-semibold text-green-400">{t('settings.model.apiResponseData')}</span>
                <pre className="text-[10px] font-mono text-dark-text bg-black/20 rounded-lg p-3 max-h-[200px] overflow-auto whitespace-pre-wrap break-words">
                  {JSON.stringify(testResponse, null, 2)}
                </pre>
              </div>
            )}
          </div>
        )}
      </div>

      {/* 模型参数滑块：Temperature / Max Tokens / Top P / Thinking Budget */}
      <SliderField label={t('settings.model.temperature')} value={config.model.temperature} min={0} max={2} step={0.1} onChange={v => updateField('model', 'temperature', v)} description={t('settings.model.temperatureDesc')} />
      <SliderField label={t('settings.model.maxTokens')} value={config.model.max_tokens} min={256} max={128000} step={256} onChange={v => updateField('model', 'max_tokens', v)} description={t('settings.model.maxTokensDesc')} />
      <SliderField label={t('settings.model.topP')} value={config.model.top_p} min={0} max={1} step={0.05} onChange={v => updateField('model', 'top_p', v)} description={t('settings.model.topPDesc')} />
      <SliderField label={t('settings.model.thinkingBudget')} value={config.model.thinking_budget} min={0} max={100000} step={1000} onChange={v => updateField('model', 'thinking_budget', v)} description={t('settings.model.thinkingBudgetDesc')} />

      <ToggleField label={t('settings.model.streamMode')} description={t('settings.model.streamModeDesc')} checked={config.model.stream_mode} onChange={v => updateField('model', 'stream_mode', v)} />
    </div>
  )
}

/** 工具设置标签页：控制各类工具的启用/禁用 */
function ToolsTab({ config, updateField }: { config: AppConfig; updateField: (s: keyof AppConfig, f: string, v: any) => void }) {
  const { t } = useTranslation()
  const tools = config.tools || { file_access: true, file_write: true, shell: true, search: true, web: true, git: true, browser: true, automation: false, agent: true }

  const updateTool = (field: string, value: boolean) => {
    updateField('tools', field, value)
  }

  return (
    <div className="space-y-6">
      <h3 className="text-base font-semibold text-dark-text pb-2 border-b border-dark-border">{t('settings.tools.title')}</h3>
      <p className="text-xs text-dark-muted">{t('settings.tools.description')}</p>

      <div className="space-y-1">
        <ToggleField label={t('settings.tools.fileAccess')} description={t('settings.tools.fileAccessDesc')} checked={tools.file_access} onChange={v => updateTool('file_access', v)} />
        <ToggleField label={t('settings.tools.fileWrite')} description={t('settings.tools.fileWriteDesc')} checked={tools.file_write} onChange={v => updateTool('file_write', v)} />
        <ToggleField label={t('settings.tools.shell')} description={t('settings.tools.shellDesc')} checked={tools.shell} onChange={v => updateTool('shell', v)} />
        <ToggleField label={t('settings.tools.search')} description={t('settings.tools.searchDesc')} checked={tools.search} onChange={v => updateTool('search', v)} />
        <ToggleField label={t('settings.tools.web')} description={t('settings.tools.webDesc')} checked={tools.web} onChange={v => updateTool('web', v)} />
        <ToggleField label={t('settings.tools.git')} description={t('settings.tools.gitDesc')} checked={tools.git} onChange={v => updateTool('git', v)} />
        <ToggleField label={t('settings.tools.browser')} description={t('settings.tools.browserDesc')} checked={tools.browser} onChange={v => updateTool('browser', v)} />
        <ToggleField label={t('settings.tools.automation')} description={t('settings.tools.automationDesc')} checked={tools.automation} onChange={v => updateTool('automation', v)} />
        <ToggleField label={t('settings.tools.agent')} description={t('settings.tools.agentDesc')} checked={tools.agent} onChange={v => updateTool('agent', v)} />
      </div>
    </div>
  )
}

/** 外观设置标签页：字体大小/族、侧边栏宽度、行号、代码主题、消息风格 */
function AppearanceTab({ config, updateField }: { config: AppConfig; updateField: (s: keyof AppConfig, f: string, v: any) => void }) {
  const { t } = useTranslation()
  return (
    <div className="space-y-6">
      <h3 className="text-base font-semibold text-dark-text pb-2 border-b border-dark-border">{t('settings.appearance.title')}</h3>
      <SliderField label={t('settings.appearance.fontSize')} value={config.ui.font_size} min={12} max={24} step={1} onChange={v => updateField('ui', 'font_size', v)} description={t('settings.appearance.fontSizeDesc')} />
      <SettingField label={t('settings.appearance.fontFamily')} description={t('settings.appearance.fontFamilyDesc')}>
        <select value={config.ui.font_family} onChange={e => updateField('ui', 'font_family', e.target.value)} className="w-full bg-dark-bg border border-dark-border rounded-lg px-3 py-2 text-sm text-dark-text focus:outline-none focus:border-primary-500">
          <option value="Inter">Inter</option><option value="system-ui">System UI</option><option value="JetBrains Mono">JetBrains Mono</option><option value="Fira Code">Fira Code</option>
        </select>
      </SettingField>
      <SliderField label={t('settings.appearance.sidebarWidth')} value={config.ui.sidebar_width} min={200} max={350} step={10} onChange={v => updateField('ui', 'sidebar_width', v)} description={t('settings.appearance.sidebarWidthDesc')} />
      <ToggleField label={t('settings.appearance.showLineNumbers')} description={t('settings.appearance.showLineNumbersDesc')} checked={config.ui.show_line_numbers} onChange={v => updateField('ui', 'show_line_numbers', v)} />
      <SettingField label={t('settings.appearance.codeTheme')} description={t('settings.appearance.codeThemeDesc')}>
        <select value={config.ui.code_theme} onChange={e => updateField('ui', 'code_theme', e.target.value)} className="w-full bg-dark-bg border border-dark-border rounded-lg px-3 py-2 text-sm text-dark-text focus:outline-none focus:border-primary-500">
          <option value="oneDark">One Dark</option><option value="oneLight">One Light</option><option value="vsDark">VS Code Dark</option><option value="githubDark">GitHub Dark</option>
        </select>
      </SettingField>
      <SettingField label={t('settings.appearance.messageStyle')} description={t('settings.appearance.messageStyleDesc')}>
        <div className="flex gap-2">
          {(['bubble', 'plain'] as const).map(style => (
            <button key={style} onClick={() => updateField('ui', 'message_style', style)} className={`px-4 py-2 rounded-lg text-sm capitalize transition-all ${config.ui.message_style === style ? 'bg-primary-600 text-white' : 'bg-dark-bg border border-dark-border text-dark-muted hover:text-dark-text'}`}>{style}</button>
          ))}
        </div>
      </SettingField>
    </div>
  )
}

/** 高级设置标签页：数据目录（只读）、日志级别、历史上限、压缩阈值、代理、遥测、危险操作 */
function AdvancedTab({ config, updateField }: { config: AppConfig; updateField: (s: keyof AppConfig, f: string, v: any) => void }) {
  const { t } = useTranslation()
  return (
    <div className="space-y-6">
      <h3 className="text-base font-semibold text-dark-text pb-2 border-b border-dark-border">{t('settings.advanced.title')}</h3>
      <SettingField label={t('settings.advanced.dataDir')} description={t('settings.advanced.dataDirDesc')}>
        <input type="text" value={config.advanced.data_dir} onChange={e => updateField('advanced', 'data_dir', e.target.value)} readOnly
          className="w-full bg-dark-bg border border-dark-border rounded-lg px-3 py-2 text-sm text-dark-muted font-mono cursor-not-allowed opacity-70" />
      </SettingField>
      <SettingField label={t('settings.advanced.logLevel')} description={t('settings.advanced.logLevelDesc')}>
        <select value={config.advanced.log_level} onChange={e => updateField('advanced', 'log_level', e.target.value)} className="w-full bg-dark-bg border border-dark-border rounded-lg px-3 py-2 text-sm text-dark-text focus:outline-none focus:border-primary-500">
          <option value="trace">Trace</option><option value="debug">Debug</option><option value="info">Info</option><option value="warn">Warning</option><option value="error">Error</option>
        </select>
      </SettingField>
      <SliderField label={t('settings.advanced.maxHistory')} value={config.advanced.max_conversation_history} min={10} max={500} step={10} onChange={v => updateField('advanced', 'max_conversation_history', v)} description={t('settings.advanced.maxHistoryDesc')} />
      <SliderField label={t('settings.advanced.autoCompactThreshold')} value={config.advanced.auto_compact_tokens} min={50000} max={300000} step={10000} onChange={v => updateField('advanced', 'auto_compact_tokens', v)} description={t('settings.advanced.autoCompactThresholdDesc')} />
      <SettingField label={t('settings.advanced.proxyUrl')} description={t('settings.advanced.proxyUrlDesc')}>
        <input type="text" value={config.advanced.proxy_url} onChange={e => updateField('advanced', 'proxy_url', e.target.value)} placeholder="http://proxy:port"
          className="w-full bg-dark-bg border border-dark-border rounded-lg px-3 py-2 text-sm text-dark-text placeholder-dark-muted/30 focus:outline-none focus:border-primary-500 font-mono" />
      </SettingField>
      <ToggleField label={t('settings.advanced.enableTelemetry')} description={t('settings.advanced.enableTelemetryDesc')} checked={config.advanced.enable_telemetry} onChange={v => updateField('advanced', 'enable_telemetry', v)} />

      {/* 危险操作区 */}
      <div className="pt-4 border-t border-dark-border/50">
        <h4 className="text-sm font-semibold text-red-400 mb-3">{t('settings.advanced.dangerZone')}</h4>
        <div className="space-y-3">
          <button className="px-4 py-2 rounded-lg border border-red-500/30 text-red-400 text-sm hover:bg-red-500/10 transition-colors">{t('settings.advanced.clearAllData')}</button>
          <button className="px-4 py-2 ml-3 rounded-lg border border-orange-500/30 text-orange-400 text-sm hover:bg-orange-500/10 transition-colors">{t('settings.advanced.exportAllData')}</button>
          <button className="px-4 py-2 ml-3 rounded-lg border border-blue-500/30 text-blue-400 text-sm hover:bg-blue-500/10 transition-colors">{t('settings.advanced.importData')}</button>
        </div>
      </div>
    </div>
  )
}

// ==================== 可复用 UI 子组件 ====================

/** 设置字段容器：标签 + 描述 + 内容插槽 */
function SettingField({ label, description, children }: { label: string; description?: string; children: React.ReactNode }) {
  return (
    <div>
      <label className="block text-sm font-medium text-dark-text mb-1.5">{label}</label>
      {description && <p className="text-xs text-dark-muted mb-2">{description}</p>}
      {children}
    </div>
  )
}

/** 开关切换控件 */
function ToggleField({ label, description, checked, onChange }: { label: string; description?: string; checked: boolean; onChange: (v: boolean) => void }) {
  return (
    <div className="flex items-center justify-between py-2">
      <div>
        <label className="text-sm font-medium text-dark-text">{label}</label>
        {description && <p className="text-xs text-dark-muted mt-0.5">{description}</p>}
      </div>
      <button onClick={() => onChange(!checked)} role="switch" aria-checked={checked}
        className="relative w-11 h-6 shrink-0 rounded-full transition-colors duration-200 border-none outline-none cursor-pointer appearance-none bg-dark-border data-[state=checked]:bg-primary-600 focus-visible:ring-2 focus-visible:ring-primary-500/50"
        data-state={checked ? 'checked' : 'unchecked'}>
        <span className={`absolute top-1 left-0 w-4 h-4 rounded-full bg-white shadow-md transition-transform duration-200 ${checked ? 'translate-x-[22px]' : 'translate-x-[4px]'} pointer-events-none`} />
      </button>
    </div>
  )
}

/** 滑块数值控件（带当前值显示） */
function SliderField({ label, value, min, max, step, onChange, description }: { label: string; value: number; min: number; max: number; step: number; onChange: (v: number) => void; description?: string }) {
  return (
    <div>
      <div className="flex items-center justify-between mb-1.5">
        <label className="text-sm font-medium text-dark-text">{label}</label>
        <span className="text-xs text-primary-400 font-mono bg-primary-600/10 px-2 py-0.5 rounded">{value}</span>
      </div>
      {description && <p className="text-xs text-dark-muted mb-2">{description}</p>}
      <input type="range" min={min} max={max} step={step} value={value} onChange={e => onChange(Number(e.target.value))}
        className="w-full h-1.5 bg-dark-border rounded-full appearance-none cursor-pointer accent-primary-500 [&::-webkit-slider-thumb]:appearance-none [&::-webkit-slider-thumb]:w-4 [&::-webkit-slider-thumb]:h-4 [&::-webkit-slider-thumb]:rounded-full [&::-webkit-slider-thumb]:bg-primary-500 [&::-webkit-slider-thumb]:shadow [&::-webkit-slider-thumb]:cursor-pointer" />
    </div>
  )
}
