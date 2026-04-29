// Claw Desktop - Agent配置弹窗组件 - 编辑Agent系统提示词、模型、温度、工具、技能等完整配置
import { useState, useEffect, useCallback } from 'react'
import { useTranslation } from 'react-i18next'
import {
  isoGetConfig, isoSetConfig, isoSetSkillsEnabled, isoAgentGet, isoAgentUpdateConfig,
  testConnection, toolTaskCreate, toolTodoWrite, toolScheduleCron,
  toolListAll, toolTaskList, toolTodoGet, toolScheduleList
} from '../../api'
import { createPortal } from 'react-dom'
import modelProvidersData from '../../model_providers.json'
import GitPanel from './panels/GitPanel'
import FileExplorer from './panels/FileExplorer'
import CostPanel from './panels/CostPanel'
import PlanEditor from './panels/PlanEditor'
import McpConfig from './panels/McpConfig'
import TagManager from './panels/TagManager'
import EnvViewer from './panels/EnvViewer'
import CodeReview from './panels/CodeReview'
import QuickActions from './panels/QuickActions'
import AboutPanel from './panels/AboutPanel'
import NotePanel from './panels/NotePanel'
import WebSearchPanel from './panels/WebSearchPanel'
import SkillMarketplace from './panels/SkillMarketplace'

import MemoryPanel from '../panels/MemoryPanel'
import BrowserPanel from '../panels/BrowserPanel'
import AgentPersonaEditor from './panels/AgentPersonaEditor'
import { CronPanel } from '../panels/CronPanel'
import { HookPanel } from '../panels/HookPanel'
import { WeixinLoginPanel } from '../panels/WeixinLoginPanel'
import ChannelPanel from '../settings/ChannelPanel'

type ConfigTab = 'profile' | 'model' | 'skills' | 'tools' | 'tasks' | 'git' | 'files' | 'cost' | 'plan' | 'mcp' | 'tags' | 'env' | 'review' | 'quick' | 'notes' | 'web' | 'memory' | 'browser' | 'about' | 'cron' | 'hooks' | 'weixin' | 'automation' | 'channels'

interface AgentConfigModalProps {
  agentId: string
  agentName: string
  onClose: () => void
}

interface AgentModelConfig {
  provider: string
  api_format: string
  custom_url: string
  custom_api_key: string
  custom_model_name: string
  default_model: string
  temperature: number
  max_tokens: number
  top_p: number
  thinking_budget: number
  stream_mode: boolean
}

export default function AgentConfigModal({ agentId, agentName, onClose }: AgentConfigModalProps) {
  const { t } = useTranslation()
  const [activeTab, setActiveTab] = useState<ConfigTab>('profile')
  const [modelConfig, setModelConfig] = useState<AgentModelConfig>({
    provider: 'custom', api_format: 'openai', custom_url: '', custom_api_key: '',
    custom_model_name: '', default_model: '', temperature: 0.7, max_tokens: 4096,
    top_p: 1, thinking_budget: 0, stream_mode: true,
  })
  const staticProviders = modelProvidersData.providers || []
  const staticCategories = modelProvidersData.categories || []
  const [providers, setProviders] = useState<any[]>(staticProviders)
  const [categories, setCategories] = useState<any[]>(staticCategories)
  const [activeCategory, setActiveCategory] = useState<string>(staticCategories.find((c: any) => c.id === 'international') ? 'international' : (staticCategories[0]?.id || 'international'))
  const [selectedProviderId, setSelectedProviderId] = useState<string>('')
  const [hasChanges, setHasChanges] = useState(false)
  const [saving, setSaving] = useState(false)
  const [profileData, setProfileData] = useState<{systemPrompt:string;purpose:string;scope:string;maxTurns:number}>({systemPrompt:'',purpose:'',scope:'',maxTurns:20})
  const [skills, setSkills] = useState<any[]>([])

  useEffect(() => { loadAgentConfig() }, [agentId])

  const loadAgentConfig = async () => {
    try {
      const keys = ['agent_model_provider','agent_model_format','agent_model_url','agent_model_key','agent_model_name','agent_model_default','agent_temperature','agent_max_tokens','agent_top_p','agent_thinking_budget','agent_stream_mode']
      const results: any = {}
      for (const key of keys) {
        try {
          const resp: any = await isoGetConfig({ agentId, key })
          if (resp && typeof resp === 'object' && 'value' in resp) {
            results[key] = resp.value
          } else {
            results[key] = resp ?? null
          }
        } catch { results[key] = null }
      }
      if (results.agent_model_provider) {
        const numVal = (v: any, def: number) => { const n = Number(v); return Number.isFinite(n) ? n : def }
        const boolVal = (v: any, def: boolean) => { if (typeof v === 'boolean') return v; if (v === 'true') return true; if (v === 'false') return false; return def }
        setModelConfig(prev => ({
          ...prev, provider: results.agent_model_provider||'custom', api_format: results.agent_model_format||'openai',
          custom_url: results.agent_model_url||'', custom_api_key: results.agent_model_key||'', custom_model_name: results.agent_model_name||'',
          default_model: results.agent_model_default||'', temperature: numVal(results.agent_temperature, 0.7),
          max_tokens: numVal(results.agent_max_tokens, 4096), top_p: numVal(results.agent_top_p, 1),
          thinking_budget: numVal(results.agent_thinking_budget, 0), stream_mode: boolVal(results.agent_stream_mode, true),
        }))
        if (results.agent_model_provider) {
          setSelectedProviderId(results.agent_model_provider)
          if (results.agent_model_provider === 'custom') setActiveCategory('custom')
        }
      }
      try {
        const detail: any = await isoAgentGet({ agentId: agentId })
        setProfileData({
          systemPrompt: detail?.systemPrompt || '',
          purpose: detail?.purpose || '',
          scope: detail?.scope || '',
          maxTurns: detail?.maxTurns || 20,
        })
        const defaultSkills = [
          {id:'file_read',name:t('agentConfig.skills.defaultFileRead'),desc:t('agentConfig.skills.defaultFileReadDesc'),enabled:true},{id:'file_write',name:t('agentConfig.skills.defaultFileWrite'),desc:t('agentConfig.skills.defaultFileWriteDesc'),enabled:true},
          {id:'file_edit',name:t('agentConfig.skills.defaultFileEdit'),desc:t('agentConfig.skills.defaultFileEditDesc'),enabled:true},{id:'web_search',name:t('agentConfig.skills.defaultWebSearch'),desc:t('agentConfig.skills.defaultWebSearchDesc'),enabled:false},
          {id:'code_exec',name:t('agentConfig.skills.defaultCodeExec'),desc:t('agentConfig.skills.defaultCodeExecDesc'),enabled:false},{id:'git_ops',name:t('agentConfig.skills.defaultGitOps'),desc:t('agentConfig.skills.defaultGitOpsDesc'),enabled:true},
          {id:'desktop_automation',name:t('agentConfig.skills.defaultDesktopAutomation'),desc:t('agentConfig.skills.defaultDesktopAutomationDesc'),enabled:true},
        ]
        let savedSkills: string[] | null = null
        if (detail?.skillsEnabled) {
          try { savedSkills = typeof detail.skillsEnabled === 'string' ? JSON.parse(detail.skillsEnabled) : detail.skillsEnabled } catch (e) { console.error('[AgentConfig] Parse skillsEnabled:', e) }
        }
        if (savedSkills && Array.isArray(savedSkills)) {
          setSkills(defaultSkills.map(s => ({ ...s, enabled: savedSkills!.includes(s.id) })))
        } else {
          setSkills(defaultSkills)
        }
      } catch (e) { console.error(e) }
    } catch (e) { /* silently handle loadAgentConfig error */ }
  }

  const updateModelField = (field: keyof AgentModelConfig, value: any) => { setModelConfig(prev => ({ ...prev, [field]: value })); setHasChanges(true) }

  const toggleSkill = async(skillId:string)=>{
    const updated=skills.map(s=>s.id===skillId?{...s,enabled:!s.enabled}:s)
    setSkills(updated); setHasChanges(true)
    try{await isoSetSkillsEnabled({agentId,enabled:updated.filter(s=>s.enabled).map(s=>s.id)})}catch(e){console.error(e)}
  }

  const handleSaveAndClose = async () => {
    setSaving(true)
    try {
      await isoSetConfig({agentId,key:'agent_model_provider',value:modelConfig.provider})
      await isoSetConfig({agentId,key:'agent_model_format',value:modelConfig.api_format})
      await isoSetConfig({agentId,key:'agent_model_url',value:modelConfig.custom_url})
      await isoSetConfig({agentId,key:'agent_model_key',value:modelConfig.custom_api_key})
      await isoSetConfig({agentId,key:'agent_model_name',value:modelConfig.custom_model_name})
      await isoSetConfig({agentId,key:'agent_model_default',value:modelConfig.default_model})
      await isoSetConfig({agentId,key:'agent_temperature',value:String(modelConfig.temperature)})
      await isoSetConfig({agentId,key:'agent_max_tokens',value:String(modelConfig.max_tokens)})
      await isoSetConfig({agentId,key:'agent_top_p',value:String(modelConfig.top_p)})
      await isoSetConfig({agentId,key:'agent_thinking_budget',value:String(modelConfig.thinking_budget)})
      await isoSetConfig({agentId,key:'agent_stream_mode',value:String(modelConfig.stream_mode)})
      await isoAgentUpdateConfig({
        agentId,
        systemPrompt: profileData.systemPrompt || undefined,
        purpose: profileData.purpose || undefined,
        scope: profileData.scope || undefined,
        modelOverride: modelConfig.default_model || modelConfig.custom_model_name || undefined,
        maxTurns: profileData.maxTurns,
        temperature: modelConfig.temperature,
      })
      setHasChanges(false); onClose()
    } catch (e) { console.error('Save failed:', e) } finally { setSaving(false) }
  }

  const currentProvider = providers?.find((p: any) => p.id === selectedProviderId)
  const currentModels = currentProvider?.models || []
  const providersInCategory = providers?.filter((p: any) => p.category === activeCategory) || []

  const handleSelectProvider = (provider: any) => {
    setSelectedProviderId(provider.id); updateModelField('provider', provider.id)
    const url = provider.baseUrl || provider.baseUrlCN || ''
    updateModelField('custom_url', url)
    if (provider.models?.[0]) { updateModelField('custom_model_name', provider.models[0].id); updateModelField('default_model', provider.models[0].id) }
    updateModelField('api_format',['anthropic','amazon-bedrock'].includes(provider.id)?'anthropic':'openai')
  }

  const isEditableProvider = !selectedProviderId || currentProvider?.category==='local'||currentProvider?.category==='proxy'||currentProvider?.category==='gateway'||currentProvider?.category==='custom'
  const isAnthropicProvider = ['anthropic','amazon-bedrock'].includes(currentProvider?.id)

  const CATEGORY_ICONS: Record<string,string>={international:'🌍',chinese:'🇨🇳',aggregator:'🔗',local:'🏠',gateway:'🚪',proxy:'🔀',oauth:'🔐',fast:'⚡',search:'🔍',privacy:'🛡️',coding:'💻',transcription:'🎙️',tts:'🔊',media:'🎬'}
  const CATEGORY_COLORS: Record<string,string>={international:'from-blue-600/15 to-blue-500/5',chinese:'from-red-600/15 to-red-500/5',aggregator:'from-purple-600/15 to-purple-500/5',local:'from-green-600/15 to-green-500/5',gateway:'from-cyan-600/15 to-cyan-500/5',proxy:'from-gray-600/15 to-gray-500/5',oauth:'from-yellow-600/15 to-yellow-500/5',fast:'from-orange-600/15 to-orange-500/5',search:'from-indigo-600/15 to-indigo-500/5',privacy:'from-pink-600/15 to-pink-500/5',coding:'from-emerald-600/15 to-emerald-500/5',transcription:'from-teal-600/15 to-teal-500/5',tts:'from-violet-600/15 to-violet-500/5',media:'from-fuchsia-600/15 to-fuchsia-500/5'}

  const tabs: { id: ConfigTab; labelKey: string; icon: JSX.Element; group: string }[] = [
    { id: 'profile', labelKey: 'agentConfig.tabs.profile', icon: <svg className="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={2}><path strokeLinecap="round" strokeLinejoin="round" d="M16 7a4 4 0 11-8 0 4 4 0 018 0zM12 14a7 7 0 00-7 7h14a7 7 0 00-7-7z"/></svg>, group: 'core' },
    { id: 'model', labelKey: 'agentConfig.tabs.model', icon: <svg className="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={2}><path strokeLinecap="round" strokeLinejoin="round" d="M9.75 17L9 20l-1 1h8l-1-1-.75-3M3 13h18M5 17h14a2 2 0 002-2V5a2 2 0 00-2-2H5a2 2 0 00-2 2v10a2 2 0 002 2z"/></svg>, group: 'core' },
    { id: 'skills', labelKey: 'agentConfig.tabs.skills', icon: <svg className="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={2}><path strokeLinecap="round" strokeLinejoin="round" d="M9.663 17h4.673M12 3v1m6.364 1.636l-.707.707M21 12h-1M4 12H3m3.343-5.657l-.707-.707m2.828 9.9a5 5 0 117.072 0l-.548.547A3.374 3.374 0 0014 18.469V19a2 2 0 11-4 0v-.531c0-.895-.356-1.754-.988-2.386l-.548-.547z"/></svg>, group: 'core' },
    { id: 'tools', labelKey: 'agentConfig.tabs.tools', icon: <svg className="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={2}><path strokeLinecap="round" strokeLinejoin="round" d="M10.325 4.317c.426-1.756 2.924-1.756 3.35 0a1.724 1.724 0 002.573 1.066c1.543-.94 3.31.826 2.37 2.37a1.724 1.724 0 001.066 2.573c1.756.426 1.756 2.924 0 3.35a1.724 1.724 0 00-1.066 2.573c.94 1.543-.826 3.31-2.37 2.37a1.724 1.724 0 00-2.573 1.066c-.426 1.756-2.924 1.756-3.35 0a1.724 1.724 0 00-2.573-1.066c-1.543.94-3.31-.826-2.37-2.37a1.724 1.724 0 00-1.066-2.573c-1.756-.426-1.756-2.924 0-3.35a1.724 1.724 0 001.066-2.573c-.94-1.543.826-3.31 2.37-2.37.996.608 2.296.07 2.572-1.065z"/><path strokeLinecap="round" strokeLinejoin="round" d="M15 12a3 3 0 11-6 0 3 3 0 016 0z"/></svg>, group: 'core' },
    { id: 'mcp', labelKey: 'agentConfig.tabs.mcp', icon: <svg className="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={2}><path strokeLinecap="round" strokeLinejoin="round" d="M8 9l3 3-3 3m5 0h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z"/></svg>, group: 'core' },
    { id: 'memory', labelKey: 'agentConfig.tabs.memory', icon: <svg className="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={2}><path strokeLinecap="round" strokeLinejoin="round" d="M9.663 17h4.673M12 3v1m6.364 1.636l-.707.707M21 12h-1M4 12H3m3.343-5.657l-.707-.707m2.828 9.9a5 5 0 117.072 0l-.548.547A3.374 3.374 0 0014 18.469V19a2 2 0 11-4 0v-.531c0-.895-.356-1.754-.988-2.386l-.548-.547z"/></svg>, group: 'core' },
    { id: 'browser', labelKey: 'agentConfig.tabs.browser', icon: <svg className="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={2}><path strokeLinecap="round" strokeLinejoin="round" d="M21 12a9 9 0 01-9 9m9-9a9 9 0 00-9-9m9 9H3m9 9a9 9 0 01-9-9m9 9c1.657 0 3-4.03 3-9s-1.343-9-3-9m0 18c-1.657 0-3-4.03-3-9s1.343-9 3-9m-9 9a9 9 0 019-9"/></svg>, group: 'core' },
    { id: 'tasks', labelKey: 'agentConfig.tabs.tasks', icon: <svg className="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={2}><path strokeLinecap="round" strokeLinejoin="round" d="M9 5H7a2 2 0 00-2 2v12a2 2 0 002 2h10a2 2 0 002-2V7a2 2 0 00-2-2h-2M9 5a2 2 0 002 2h2a2 2 0 002-2M9 5a2 2 0 012-2h2a2 2 0 012 2m-6 9l2 2 4-4"/></svg>, group: 'ops' },
    { id: 'cron', labelKey: 'agentConfig.tabs.cron', icon: <svg className="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={2}><path strokeLinecap="round" strokeLinejoin="round" d="M12 8v4l3 3m6-3a9 9 0 11-18 0 9 9 0 0118 0z"/></svg>, group: 'automation' },
    { id: 'git', labelKey: 'agentConfig.tabs.git', icon: <svg className="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={2}><path strokeLinecap="round" strokeLinejoin="round" d="M13 7h8m0 0v8m0-8l-8 8-4-4-6 6"/></svg>, group: 'dev' },
    { id: 'files', labelKey: 'agentConfig.tabs.files', icon: <svg className="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={2}><path strokeLinecap="round" strokeLinejoin="round" d="M3 7v10a2 2 0 002 2h14a2 2 0 002-2V9a2 2 0 00-2-2h-6l-2-2H5a2 2 0 00-2 2z"/></svg>, group: 'dev' },
    { id: 'review', labelKey: 'agentConfig.tabs.review', icon: <svg className="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={2}><path strokeLinecap="round" strokeLinejoin="round" d="M9 12l2 2 4-4m6 2a9 9 0 11-18 0 9 9 0 0118 0z"/></svg>, group: 'dev' },
    { id: 'plan', labelKey: 'agentConfig.tabs.plan', icon: <svg className="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={2}><path strokeLinecap="round" strokeLinejoin="round" d="M9 5H7a2 2 0 00-2 2v12a2 2 0 002 2h10a2 2 0 002-2V7a2 2 0 00-2-2h-2M9 5a2 2 0 002 2h2a2 2 0 002-2M9 5a2 2 0 012-2h2a2 2 0 012 2m-6 9l2 2 4-4"/></svg>, group: 'dev' },
    { id: 'cost', labelKey: 'agentConfig.tabs.cost', icon: <svg className="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={2}><path strokeLinecap="round" strokeLinejoin="round" d="M12 8c-1.657 0-3 .895-3 2s1.343 2 3 2 3 .895 3 2-1.343 2-3 2m0-8c1.11 0 2.08.402 2.599 1M12 21V3m0 0l-3 3m3-3l3 3"/></svg>, group: 'dev' },
    { id: 'tags', labelKey: 'agentConfig.tabs.tags', icon: <svg className="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={2}><path strokeLinecap="round" strokeLinejoin="round" d="M7 7h.01M7 3h5c.512 0 1.024.195 1.414.586l7 7a2 2 0 010 2.828l-7 7a2 2 0 01-2.828 0l-7-7A1.994 1.994 0 013 12V7a4 4 0 014-4z"/></svg>, group: 'extra' },
    { id: 'env', labelKey: 'agentConfig.tabs.env', icon: <svg className="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={2}><path strokeLinecap="round" strokeLinejoin="round" d="M9.75 17L9 20l-1 1h8l-1-1-.75-3M3 13h18M5 17h14a2 2 0 002-2V5a2 2 0 00-2-2H5a2 2 0 00-2 2v10a2 2 0 002 2z"/></svg>, group: 'extra' },
    { id: 'quick', labelKey: 'agentConfig.tabs.quick', icon: <svg className="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={2}><path strokeLinecap="round" strokeLinejoin="round" d="M13 10V3L4 14h7v7l9-11h-7z"/></svg>, group: 'extra' },
    { id: 'notes', labelKey: 'agentConfig.tabs.notes', icon: <svg className="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={2}><path strokeLinecap="round" strokeLinejoin="round" d="M11 5H6a2 2 0 00-2 2v11a2 2 0 002 2h11a2 2 0 002-2v-5m-1.414-9.414a2 2 0 112.828 2.828L11.828 15H9v-2.828l8.586-8.586z"/></svg>, group: 'extra' },
    { id: 'web', labelKey: 'agentConfig.tabs.web', icon: <svg className="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={2}><path strokeLinecap="round" strokeLinejoin="round" d="M21 12a9 9 0 01-9 9m9-9a9 9 0 00-9-9m9 9H3m9 9a9 9 0 01-9-9m9 9c1.657 0 3-4.03 3-9s-1.343-9-3-9m0 18c-1.657 0-3-4.03-3-9s1.343-9 3-9m-9 9a9 9 0 019-9"/></svg>, group: 'extra' },
    { id: 'about', labelKey: 'agentConfig.tabs.about', icon: <svg className="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={2}><path strokeLinecap="round" strokeLinejoin="round" d="M13 16h-1v-4h-1m1-4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z"/></svg>, group: 'extra' },
    { id: 'hooks', labelKey: 'agentConfig.tabs.hooks', icon: <svg className="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={2}><path strokeLinecap="round" strokeLinejoin="round" d="M13.828 10.172a4 4 0 00-5.656 0l-4 4a4 4 0 105.656 5.656l1.102-1.101m-.758-4.899a4 4 0 005.656 0l4-4a4 4 0 00-5.656-5.656l-1.1 1.1"/></svg>, group: 'automation' },
    { id: 'automation', labelKey: 'agentConfig.tabs.automation', icon: <svg className="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={2}><path strokeLinecap="round" strokeLinejoin="round" d="M15 15l-2 5L9 9l11 4-5 2zm0 0l5 5M7.188 2.239l.777 2.897M5.136 7.965l-2.898-.777M13.95 4.05l-2.122 2.122m-5.657 5.656l-2.12 2.122"/></svg>, group: 'automation' },
    { id: 'weixin', labelKey: 'agentConfig.tabs.weixin', icon: <svg className="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={2}><path strokeLinecap="round" strokeLinejoin="round" d="M8 12h.01M12 12h.01M16 12h.01M21 12c0 4.418-4.03 8-9 8a9.863 9.863 0 01-4.255-.949L3 20l1.395-3.72C3.512 15.042 3 13.574 3 12c0-4.418 4.03-8 9-8s9 3.582 9 8z"/></svg>, group: 'channels' },
    { id: 'channels', labelKey: 'agentConfig.tabs.channels', icon: <svg className="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={2}><path strokeLinecap="round" strokeLinejoin="round" d="M6 5c7.18 0 13 5.82 13 13M6 11a7 7 0 017 7m-6 0a1 1 0 11-2 0 1 1 0 012 0z"/></svg>, group: 'channels' },
  ]

  return createPortal(
    <div className="fixed inset-0 z-[70] flex items-center justify-center bg-black/50 backdrop-blur-sm animate-fade-in" onClick={handleSaveAndClose}>
      <div className="bg-dark-surface border border-dark-border rounded-2xl shadow-2xl w-[900px] max-h-[88vh] flex flex-col overflow-hidden animate-fade-in" onClick={e => e.stopPropagation()}>
        {/* Header */}
        <div className="flex items-center justify-between px-5 py-2.5 border-b border-dark-border shrink-0">
          <div className="flex items-center gap-2.5 min-w-0">
            <div className="w-7 h-7 rounded-lg bg-gradient-to-br from-primary-500 to-primary-700 flex items-center justify-center shrink-0">
              <svg className="w-4 h-4 text-white" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={2}><path strokeLinecap="round" strokeLinejoin="round" d="M10.325 4.317c.426-1.756 2.924-1.756 3.35 0a1.724 1.724 0 002.573 1.066c1.543-.94 3.31.826 2.37 2.37a1.724 1.724 0 001.066 2.573c1.756.426 1.756 2.924 0 3.35a1.724 1.724 0 00-1.066 2.573c.94 1.543-.826 3.31-2.37 2.37a1.724 1.724 0 00-2.573 1.066c-.426 1.756-2.924 1.756-3.35 0a1.724 1.724 0 00-2.573-1.066c-1.543.94-3.31-.826-2.37-2.37a1.724 1.724 0 00-1.066-2.573c-1.756-.426-1.756-2.924 0-3.35a1.724 1.724 0 001.066-2.573c-.94-1.543.826-3.31 2.37-2.37.996.608 2.296.07 2.572-1.065z"/><path strokeLinecap="round" strokeLinejoin="round" d="M15 12a3 3 0 11-6 0 3 3 0 016 0z"/></svg>
            </div>
            <div className="min-w-0">
              <h2 className="text-base font-bold text-dark-text truncate">{agentName} — {t('agentConfig.title')}</h2>
              <p className="text-[10px] text-dark-muted truncate">{t('agentConfig.subtitle')}</p>
            </div>
          </div>
          <button onClick={handleSaveAndClose} disabled={saving} className="p-1.5 rounded-lg hover:bg-dark-border/50 text-dark-muted hover:text-dark-text transition-colors shrink-0 disabled:opacity-50">
            <svg className="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={2}><path strokeLinecap="round" strokeLinejoin="round" d="M6 18L18 6M6 6l12 12" /></svg>
          </button>
        </div>

        {/* Body: Left tabs + Right content */}
        <div className="flex flex-1 overflow-hidden">
          <div className="w-40 border-r border-dark-border py-2 shrink-0 overflow-y-auto custom-scrollbar">
            {['core','ops','automation','dev','extra','channels'].map(group => (
              <div key={group} className="mb-1">
                <div className="px-3 py-1 text-[9px] font-semibold text-dark-muted/40 uppercase tracking-wider">{{'core':t('agentConfig.groups.core'),'ops':t('agentConfig.groups.ops'),'automation':t('agentConfig.groups.automation'),'dev':t('agentConfig.groups.dev'),'extra':t('agentConfig.groups.extra'),'channels':t('agentConfig.groups.channels')}[group]}</div>
                {tabs.filter(tb=>tb.group===group).map(tab=>(
                  <button key={tab.id} onClick={()=>setActiveTab(tab.id)} className={`w-full flex items-center gap-2 px-4 py-1.5 text-xs transition-colors ${activeTab===tab.id?'bg-primary-600/10 text-primary-400 border-r-2 border-primary-500':'text-dark-muted hover:text-dark-text hover:bg-dark-border/30'}`}>
                    {tab.icon}{t(tab.labelKey)}
                  </button>
                ))}
              </div>
            ))}
          </div>

          <div className="flex-1 overflow-y-auto p-4">
            {activeTab==='profile'&&<ProfileConfigContent profileData={profileData} setProfileData={setProfileData} setHasChanges={setHasChanges} agentId={agentId} agentName={agentName} />}
            {activeTab==='model'&&<ModelConfigContent modelConfig={modelConfig} updateField={updateModelField} providers={providers} categories={categories} activeCategory={activeCategory} setActiveCategory={setActiveCategory} selectedProviderId={selectedProviderId} onSelectProvider={handleSelectProvider} providersInCategory={providersInCategory} currentProvider={currentProvider} currentModels={currentModels} isEditableProvider={isEditableProvider} isAnthropicProvider={isAnthropicProvider} CATEGORY_ICONS={CATEGORY_ICONS} CATEGORY_COLORS={CATEGORY_COLORS} />}
            {activeTab==='skills'&&<SkillsConfigContent agentId={agentId} skills={skills} onToggleSkill={toggleSkill} onRefresh={loadAgentConfig} />}
            {activeTab==='mcp'&&<McpConfig agentId={agentId} />}
            {activeTab==='tools'&&<ToolsConfigContent agentId={agentId} />}
            {activeTab==='tasks'&&<TasksConfigContent agentId={agentId} />}
            {activeTab==='cron'&&<CronPanel agentId={agentId} />}
            {activeTab==='git'&&<GitPanel agentId={agentId} />}
            {activeTab==='files'&&<FileExplorer agentId={agentId} />}
            {activeTab==='cost'&&<CostPanel agentId={agentId} />}
            {activeTab==='plan'&&<PlanEditor agentId={agentId} />}
            {activeTab==='tags'&&<TagManager agentId={agentId} />}
            {activeTab==='env'&&<EnvViewer agentId={agentId} />}
            {activeTab==='review'&&<CodeReview agentId={agentId} />}
            {activeTab==='quick'&&<QuickActions agentId={agentId} />}
            {activeTab==='notes'&&<NotePanel agentId={agentId} />}
            {activeTab==='web'&&<WebSearchPanel agentId={agentId} />}
            {activeTab==='memory'&&<MemoryPanel agentId={agentId} />}
            {activeTab==='browser'&&<BrowserPanel agentId={agentId} />}
            {activeTab==='about'&&<AboutPanel />}
            {activeTab==='hooks'&&<HookPanel agentId={agentId} />}
            {activeTab==='automation'&&<AutomationConfigContent agentId={agentId} />}
            {activeTab==='weixin'&&<WeixinLoginPanel agentId={agentId} />}
            {activeTab==='channels'&&<ChannelPanel />}
          </div>
        </div>

        {/* Footer save bar (only show save button on Model tab) */}
        {(activeTab==='model'||activeTab==='profile')&&(
          <div className="flex items-center justify-end gap-3 px-5 py-2.5 border-t border-dark-border shrink-0">
            {hasChanges&&<span className="text-xs text-yellow-400">{t('agentConfig.unsavedChanges')}</span>}
            <button onClick={onClose} className="px-4 py-1.5 rounded-lg border border-dark-border text-xs text-dark-muted hover:text-dark-text transition-colors">{t('agentConfig.cancel')}</button>
            <button onClick={handleSaveAndClose} disabled={saving||!hasChanges} className={`px-4 py-1.5 rounded-lg text-xs font-medium transition-all ${saving?'bg-primary-600/50 text-white/70 cursor-wait':hasChanges?'bg-primary-600 hover:bg-primary-500 text-white shadow-lg shadow-primary-600/20':'bg-dark-border text-dark-muted cursor-not-allowed'}`}>{saving?t('agentConfig.saving'):t('agentConfig.saveConfig')}</button>
          </div>
        )}
      </div>
    </div>,
    document.body
  )
}

// ==================== Profile Config Content ====================

function ProfileConfigContent({profileData,setProfileData,setHasChanges,agentId,agentName}:{profileData:{systemPrompt:string;purpose:string;scope:string;maxTurns:number};setProfileData:(d:{systemPrompt:string;purpose:string;scope:string;maxTurns:number})=>void;setHasChanges:(v:boolean)=>void;agentId:string;agentName:string}){
  const { t } = useTranslation()
  const update=(field:string,value:any)=>{setProfileData({...profileData,[field]:value});setHasChanges(true)}
  return(
    <div className="space-y-4">
      <div><h3 className="text-base font-semibold text-dark-text pb-2 border-b border-dark-border">{t('agentConfig.profile.title')}</h3><p className="text-[10px] text-dark-muted mt-1">{t('agentConfig.profile.desc')}</p></div>

      {/* Persona Editor */}
      <AgentPersonaEditor agentId={agentId} agentName={agentName} />

      <div className="border-t border-dark-border pt-4">
        <h4 className="text-sm font-semibold text-dark-text mb-3">{t('agentConfig.profile.advancedConfig')}</h4>
      </div>
      <div><label className="block text-[11px] font-medium text-dark-text mb-1">{t('agentConfig.profile.systemPrompt')}</label>
        <textarea value={profileData.systemPrompt} onChange={e=>update('systemPrompt',e.target.value)} rows={8}
          placeholder={t('agentConfig.profile.systemPromptPlaceholder')}
          className="w-full px-3 py-2 rounded-lg bg-dark-bg border border-dark-border text-dark-text text-xs focus:outline-none focus:border-primary-500 resize-none font-mono leading-relaxed" />
      </div>
      <div><label className="block text-[11px] font-medium text-dark-text mb-1">{t('agentConfig.profile.purpose')}</label>
        <input value={profileData.purpose} onChange={e=>update('purpose',e.target.value)} placeholder={t('agentConfig.profile.purposePlaceholder')}
          className="w-full px-3 py-2 rounded-lg bg-dark-bg border border-dark-border text-dark-text text-sm focus:outline-none focus:border-primary-500" />
        <p className="text-[9px] text-dark-muted/50 mt-1">{t('agentConfig.profile.purposeDesc')}</p>
      </div>
      <div><label className="block text-[11px] font-medium text-dark-text mb-1">{t('agentConfig.profile.scope')}</label>
        <input value={profileData.scope} onChange={e=>update('scope',e.target.value)} placeholder={t('agentConfig.profile.scopePlaceholder')}
          className="w-full px-3 py-2 rounded-lg bg-dark-bg border border-dark-border text-dark-text text-sm focus:outline-none focus:border-primary-500" />
        <p className="text-[9px] text-dark-muted/50 mt-1">{t('agentConfig.profile.scopeDesc')}</p>
      </div>
      <div className="p-4 rounded-xl border border-dark-border bg-dark-bg">
        <label className="block text-[11px] font-medium text-dark-text mb-2">{t('agentConfig.profile.maxTurns')}</label>
        <div className="flex items-center gap-3">
          <input type="range" min={1} max={50} step={1} value={profileData.maxTurns} onChange={e=>update('maxTurns',parseInt(e.target.value))}
            className="flex-1 h-1.5 bg-dark-border rounded-full appearance-none cursor-pointer accent-primary-500 [&::-webkit-slider-thumb]:appearance-none [&::-webkit-slider-thumb]:w-4 [&::-webkit-slider-thumb]:h-4 [&::-webkit-slider-thumb]:rounded-full [&::-webkit-slider-thumb]:bg-primary-500" />
          <span className="text-xs text-primary-400 font-mono bg-primary-600/10 px-2 py-0.5 rounded min-w-[2rem] text-center">{profileData.maxTurns}</span>
        </div>
        <p className="text-[9px] text-dark-muted/50 mt-1">{t('agentConfig.profile.maxTurnsDesc')}</p>
      </div>
    </div>
  )
}

// ==================== Model Config Content ====================

function ModelConfigContent({ modelConfig,updateField,providers,categories,activeCategory,setActiveCategory,selectedProviderId,onSelectProvider,providersInCategory,currentProvider,currentModels,isEditableProvider,isAnthropicProvider,CATEGORY_ICONS,CATEGORY_COLORS }:any){
  const { t } = useTranslation()
  const [testStatus, setTestStatus] = useState<'idle'|'loading'|'success'|'error'>('idle')
  const [testMsg, setTestMsg] = useState('')
  const [testDetail, setTestDetail] = useState('')

  const handleTestConnection = async () => {
    if (!modelConfig.custom_url || !modelConfig.custom_api_key || !modelConfig.custom_model_name) {
      setTestStatus('error')
      setTestMsg(t('agentConfig.modelConfig.fillRequiredFields'))
      return
    }
    setTestStatus('loading')
    setTestMsg(t('agentConfig.modelConfig.connecting'))
    setTestDetail('')
    try {
      const result: any = await testConnection({
        config: {
          app: { language: 'zh-CN', theme: 'dark' },
          api: { base_url: modelConfig.custom_url, api_key: modelConfig.custom_api_key },
          ui: {},
          advanced: {},
          harness: {},
          model: {
            custom_url: modelConfig.custom_url,
            custom_api_key: modelConfig.custom_api_key,
            custom_model_name: modelConfig.custom_model_name || modelConfig.default_model,
            default_model: modelConfig.custom_model_name || modelConfig.default_model,
            provider: modelConfig.api_format === 'anthropic' ? 'anthropic' : 'openai',
            api_format: modelConfig.api_format || 'openai',
          }
        }
      })
      if (result?.success) {
        setTestStatus('success')
        setTestMsg(result.message || t('agentConfig.modelConfig.connectSuccess'))
      } else {
        setTestStatus('error')
        setTestMsg(result?.message || t('agentConfig.modelConfig.connectFailed'))
        if (result?.logs) {
          const errLogs = (result.logs as unknown as Array<{ level: string; phase: string; detail: string }>).filter(l => l.level === 'ERROR').map(l => `[${l.phase}] ${l.detail}`)
          setTestDetail(errLogs.join('\n') || '')
        }
      }
    } catch (e: any) {
      setTestStatus('error')
      setTestMsg(`${t('agentConfig.modelConfig.requestError')}: ${e?.message || e}`)
    }
  }

  return(
    <div className="space-y-4">
      <div><h3 className="text-base font-semibold text-dark-text pb-2 border-b border-dark-border">{t('agentConfig.modelConfig.title')}</h3><p className="text-[10px] text-dark-muted mt-1">{t('agentConfig.modelConfig.desc')}</p></div>

      <div className="flex gap-1 flex-wrap p-1 rounded-lg bg-dark-bg border border-dark-border">
        {categories.map((cat:any)=>(
          <button key={cat.id} onClick={()=>setActiveCategory(cat.id)} className={`flex items-center gap-1 px-2.5 py-1 rounded-md text-[10px] transition-all ${activeCategory===cat.id?'bg-primary-600 text-white':'text-dark-muted hover:text-dark-text hover:bg-dark-surface'}`}>
            <span>{CATEGORY_ICONS[cat.id]||'📦'}</span><span>{cat.name}</span>
            <span className={`text-[9px] px-1 rounded-full ${activeCategory===cat.id?'bg-white/20':'bg-dark-border text-dark-muted'}`}>{providersInCategory.length}</span>
          </button>
        ))}
      </div>

      <div>
        <div className="flex items-center justify-between mb-2"><span className="text-xs font-medium text-dark-muted">{categories.find((c:any)=>c.id===activeCategory)?.name}</span>{selectedProviderId&&<span className="text-[10px] px-2 py-0.5 rounded-full bg-primary-600/10 text-primary-300">{currentProvider?.name}</span>}</div>
        <div className="grid grid-cols-3 sm:grid-cols-4 gap-1.5 max-h-[160px] overflow-y-auto p-1 rounded-lg border border-dark-border bg-dark-bg/50">
          {providersInCategory.map((p:any)=>(
            <button key={p.id} onClick={()=>onSelectProvider(p)} className={`px-2.5 py-1.5 rounded-lg text-left transition-all ${selectedProviderId===p.id?`bg-gradient-to-br ${CATEGORY_COLORS[p.category]||CATEGORY_COLORS.international} border border-primary-500/30`:'border-transparent hover:border-dark-border hover:bg-dark-surface'}`}>
              <div className={`text-[11px] font-medium truncate ${selectedProviderId===p.id?'text-dark-text':'text-dark-text group-hover:text-primary-300'}`}>{p.name}</div>
              {p.models?.length>0&&<div className={`text-[9px] mt-0.5 ${selectedProviderId===p.id?'text-primary-400/70':'text-dark-muted'}`}>{p.models.length} {t('agentConfig.modelConfig.modelsUnit')}</div>}
            </button>
          ))}
          {providersInCategory.length===0&&<div className="col-span-full text-center py-4 text-xs text-dark-muted">{t('agentConfig.modelConfig.noProviders')}</div>}
        </div>
      </div>

      <div className="space-y-3 p-4 rounded-xl border border-dark-border bg-dark-bg">
        <div className="flex items-center justify-between mb-1"><h4 className="text-sm font-semibold text-dark-text">{t('agentConfig.modelConfig.apiConfig')}</h4>{currentProvider&&<span className="text-[10px] px-2 py-0.5 rounded-full bg-primary-600/10 text-primary-300">via {currentProvider.name}</span>}</div>
        <div className="space-y-1.5"><label className="block text-[11px] font-medium text-dark-muted">{t('agentConfig.modelConfig.apiFormat')}</label><div className="flex gap-2">{(['openai','anthropic'] as const).map(fmt=><button key={fmt} onClick={()=>updateField('api_format',fmt)} className={`px-3 py-1 rounded-lg text-xs ${modelConfig.api_format===fmt?'bg-primary-600 text-white':'bg-dark-surface border border-dark-border text-dark-muted'}`}>{fmt==='openai'?'OpenAI':'Anthropic'}</button>)}</div></div>
        <div><label className="block text-[11px] font-medium text-dark-text mb-1">{t('agentConfig.modelConfig.modelUrl')}</label>{isEditableProvider?<input type="text" value={modelConfig.custom_url} onChange={e=>updateField('custom_url',e.target.value)} placeholder={t('agentConfig.modelConfig.modelUrlPlaceholder')} className="w-full bg-dark-surface border border-dark-border rounded-lg px-3 py-2 text-sm text-dark-text placeholder-dark-muted/30 focus:outline-none focus:border-primary-500 font-mono"/>:<div className="w-full bg-dark-bg/50 border border-dark-border/50 rounded-lg px-3 py-2 text-sm text-dark-muted font-mono truncate">{modelConfig.custom_url||t('agentConfig.modelConfig.autoFilled')}</div>}</div>
        <div><label className="block text-[11px] font-medium text-dark-text mb-1">{t('agentConfig.modelConfig.apiKey')}</label><input type="password" value={modelConfig.custom_api_key} onChange={e=>updateField('custom_api_key',e.target.value)} placeholder={t('agentConfig.modelConfig.apiKeyPlaceholder')} className="w-full bg-dark-surface border border-dark-border rounded-lg px-3 py-2 text-sm text-dark-text placeholder-dark-muted/30 focus:outline-none focus:border-primary-500 font-mono"/></div>
        <div><label className="block text-[11px] font-medium text-dark-text mb-1">{t('agentConfig.modelConfig.modelName')}</label>{currentModels.length>0?<select value={modelConfig.custom_model_name} onChange={e=>{updateField('custom_model_name',e.target.value);updateField('default_model',e.target.value)}} className="w-full bg-dark-surface border border-dark-border rounded-lg px-3 py-2 text-sm text-dark-text focus:outline-none focus:border-primary-500">{currentModels.map((m:any)=><option key={m.id} value={m.id}>{m.name}{m.reasoning?' 🧠':''}</option>)}</select>:<input type="text" value={modelConfig.custom_model_name} onChange={e=>{updateField('custom_model_name',e.target.value);updateField('default_model',e.target.value)}} placeholder={t('agentConfig.modelConfig.modelNamePlaceholder')} className="w-full bg-dark-surface border border-dark-border rounded-lg px-3 py-2 text-sm text-dark-text placeholder-dark-muted/30 focus:outline-none focus:border-primary-500 font-mono"/>}</div>

        <div className="flex items-center gap-2">
          <button onClick={handleTestConnection} disabled={testStatus==='loading'} className={`px-4 py-1.5 rounded-lg text-xs font-medium transition-all flex items-center gap-1.5 ${testStatus==='loading'?'bg-yellow-600/20 text-yellow-400 cursor-wait':testStatus==='success'?'bg-green-600/20 text-green-400':testStatus==='error'?'bg-red-600/20 text-red-400':'bg-primary-600/15 text-primary-300 hover:bg-primary-600/25 border border-primary-500/20'}`}>
            {testStatus==='loading'&&<svg className="w-3.5 h-3.5 animate-spin" fill="none" viewBox="0 0 24 24"><circle className="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" strokeWidth="4"/><path className="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4z"/></svg>}
            {testStatus==='success'&&<svg className="w-3.5 h-3.5" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={3}><path strokeLinecap="round" strokeLinejoin="round" d="M5 13l4 4L19 7"/></svg>}
            {testStatus==='error'&&<svg className="w-3.5 h-3.5" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={2.5}><path strokeLinecap="round" strokeLinejoin="round" d="M6 18L18 6M6 6l12 12"/></svg>}
            {testStatus==='idle' && <svg className="w-3.5 h-3.5" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={2}><path strokeLinecap="round" strokeLinejoin="round" d="M13 10V3L4 14h7v7l9-11h-7z"/></svg>}
            {testStatus==='loading'?t('agentConfig.modelConfig.testing'):testStatus==='success'?t('agentConfig.modelConfig.connectSuccess'):testStatus==='error'?t('agentConfig.modelConfig.connectFailed'):t('agentConfig.modelConfig.testConnection')}
          </button>
          {testStatus!=='idle' && <span className="text-[10px] truncate">{testMsg}</span>}
        </div>
        {testDetail && <pre className="text-[10px] text-red-400/70 bg-red-500/5 border border-red-500/10 rounded-lg p-2 mt-1 whitespace-pre-wrap font-mono max-h-20 overflow-y-auto">{testDetail}</pre>}
      </div>

      <SliderField label={t('agentConfig.modelConfig.temperature')} value={modelConfig.temperature} min={0} max={2} step={0.1} onChange={v=>updateField('temperature',v)} />
      <SliderField label={t('agentConfig.modelConfig.maxTokens')} value={modelConfig.max_tokens} min={256} max={128000} step={256} onChange={v=>updateField('max_tokens',v)} />
      <SliderField label={t('agentConfig.modelConfig.topP')} value={modelConfig.top_p} min={0} max={1} step={0.05} onChange={v=>updateField('top_p',v)} />
      <SliderField label={t('agentConfig.modelConfig.thinkingBudget')} value={modelConfig.thinking_budget} min={0} max={100000} step={1000} onChange={v=>updateField('thinking_budget',v)} />
      <ToggleField label={t('agentConfig.modelConfig.streamMode')} checked={modelConfig.stream_mode} onChange={v=>updateField('stream_mode',v)} />
    </div>
  )
}

// ==================== Skills Config Content ====================

const SKILL_ICONS: Record<string, { icon: string; color: string }> = {
  'file_read':   { icon: '📖', color: 'from-blue-500 to-cyan-500' },
  'file_write':  { icon: '✏️', color: 'from-emerald-500 to-green-500' },
  'file_edit':   { icon: '🔧', color: 'from-amber-500 to-orange-500' },
  'web_search':  { icon: '🔍', color: 'from-violet-500 to-purple-500' },
  'code_exec':   { icon: '⚡', color: 'from-yellow-500 to-red-500' },
  'git_ops':     { icon: '🔀', color: 'from-orange-500 to-red-500' },
}

function SkillsConfigContent({agentId, skills, onToggleSkill, onRefresh}:{agentId:string; skills:any[]; onToggleSkill:(id:string)=>void; onRefresh:()=>void}){
  const { t } = useTranslation()
  const [showMarketplace, setShowMarketplace] = useState(false)
  const loading = skills.length === 0

  if(loading)return<div className="flex justify-center py-8"><div className="w-6 h-6 border-2 border-primary-500 border-t-transparent rounded-full animate-spin"></div></div>

  return (
    <div className="space-y-4">
      <div className="flex items-center justify-between">
        <div>
          <h3 className="text-base font-semibold text-dark-text">{t('agentConfig.skills.title')}</h3>
          <p className="text-[10px] text-dark-muted mt-0.5">{t('agentConfig.skills.desc')}</p>
        </div>
        <button
          onClick={() => setShowMarketplace(true)}
          className="px-3 py-1.5 rounded-lg text-[11px] font-medium bg-primary-600 text-white hover:bg-primary-500 transition-all duration-150 flex items-center gap-1.5 active:scale-95 shadow-lg shadow-primary-600/20"
        >
          <svg className="w-3.5 h-3.5" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={2}>
            <path strokeLinecap="round" strokeLinejoin="round" d="M12 4v16m8-8H4" />
          </svg>
          {t('agentConfig.skills.browseMarketplace')}
        </button>
      </div>

      <div className="space-y-1.5">{skills.map((skill:any)=>{
        const meta=SKILL_ICONS[skill.id]||{icon:'📦',color:'from-gray-500 to-gray-400'}
        return(
          <div key={skill.id} className={`group flex items-center gap-3 p-3 rounded-xl border transition-all duration-200 ${skill.enabled?'bg-primary-500/5 border-primary-500/20':'bg-dark-bg/40 border-dark-border hover:border-dark-border/80'}`}>
            <div className={`w-9 h-9 rounded-lg bg-gradient-to-br ${meta.color} flex items-center justify-center text-white text-sm shrink-0 shadow-md ${skill.enabled?'shadow-primary-500/20':''}`}>
              {meta.icon}
            </div>
            <div className="min-w-0 flex-1">
              <div className="flex items-center gap-2">
                <span className={`text-[13px] font-semibold ${skill.enabled?'text-dark-text':'text-dark-muted'}`}>{skill.name}</span>
                <code className="text-[9px] text-primary-400/40 font-mono px-1.5 py-0.5 rounded-md bg-primary-500/5 border border-primary-500/10">{skill.id}</code>
                {skill.enabled && (
                  <span className="text-[8px] px-1.5 py-0.5 rounded-full bg-primary-500/15 text-primary-400 font-medium">{t('agentConfig.skills.enabled')}</span>
                )}
              </div>
              <p className={`text-[11px] mt-0.5 leading-snug ${skill.enabled?'text-dark-muted/80':'text-dark-muted/50'}`}>{skill.desc}</p>
            </div>
            <button
              onClick={()=>onToggleSkill(skill.id)}
              className={`relative w-11 h-6 rounded-full transition-all duration-300 ease-in-out shrink-0 focus:outline-none focus-visible:ring-2 focus-visible:ring-primary-400 focus-visible:ring-offset-2 focus-visible:ring-offset-transparent ${skill.enabled?'bg-primary-600 shadow-lg shadow-primary-500/30':'bg-dark-border'}`}
              role="switch"
              aria-checked={skill.enabled}
            >
              <span className={`absolute top-0.5 left-0.5 w-5 h-5 rounded-full bg-white shadow-md transition-transform duration-300 ease-in-out ${skill.enabled?'translate-x-[22px]':'translate-x-0'}`}>
                {skill.enabled && (
                  <svg className="w-3 h-3 text-primary-600 absolute top-1 left-1" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={3}><path strokeLinecap="round" strokeLinejoin="round" d="M5 13l4 4L19 7" /></svg>
                )}
              </span>
            </button>
          </div>
        )
      })}</div>

      {/* Fullscreen marketplace modal */}
      {showMarketplace && createPortal(
        <div className="fixed inset-0 z-[100] flex items-center justify-center" onClick={() => setShowMarketplace(false)}>
          <div className="absolute inset-0 bg-black/60 backdrop-blur-sm" />
          <div className="relative w-full max-w-5xl max-h-[90vh] bg-dark-surface border border-dark-border rounded-2xl shadow-2xl overflow-hidden flex flex-col" onClick={e => e.stopPropagation()}>
            <div className="flex items-center justify-between p-5 border-b border-dark-border">
              <div>
                <h2 className="text-lg font-bold text-dark-text flex items-center gap-2">
                  🛒 {t('agentConfig.skills.skillShopTitle')}
                  <span className="text-[10px] font-normal text-dark-muted bg-dark-bg px-2 py-0.5 rounded-full">ClawHub</span>
                </h2>
                <p className="text-[10px] text-dark-muted mt-0.5">{t('agentConfig.skills.skillShopSubtitle')}</p>
              </div>
              <button onClick={() => setShowMarketplace(false)} className="w-8 h-8 rounded-lg bg-dark-bg border border-dark-border flex items-center justify-center text-dark-muted hover:text-dark-text transition-colors">✕</button>
            </div>
            <div className="flex-1 overflow-y-auto p-5 custom-scrollbar">
              <SkillMarketplace agentId={agentId} onInstalled={() => { onRefresh(); setShowMarketplace(false) }} />
            </div>
          </div>
        </div>,
        document.body
      )}
    </div>
  )
}

// ==================== MCP Config Content ====================

function McpConfigContent({agentId}:{agentId:string}){
  const { t } = useTranslation()
  const [servers,setServers]=useState<Array<{name:string;command:string;args:string;env:string;enabled:boolean}>>([]),[loading,setLoading]=useState(true)
  useEffect(()=>{loadMcpConfig()},[agentId])
  const loadMcpConfig=async()=>{
    setLoading(true)
    try{
      const result:any=await isoGetConfig({agentId,key:'mcp_servers'})
      if(result&&Array.isArray(result)) setServers(result)
      else setServers([{name:'filesystem',command:'npx',args:'-y @modelcontextprotocol/server-filesystem /path/to/dir',env:'',enabled:false},{name:'github',command:'npx',args:'-y @modelcontextprotocol/server-github',env:'GITHUB_TOKEN=xxx',enabled:false},{name:'postgres',command:'npx',args:'-y @modelcontextprotocol/server-postgres postgres://...',env:'',enabled:false}])
    }catch(e){console.error('[McpConfig] Failed to load MCP servers:', e)}finally{setLoading(false)}
  }
  const toggleServer=async(idx:number)=>{
    const updated=servers.map((s,i)=>i===idx?{...s,enabled:!s.enabled}:s)
    setServers(updated)
    try{await isoSetConfig({agentId,key:'mcp_servers',value:JSON.stringify(updated)})}catch(e){console.error(e)}
  }
  if(loading)return<div className="flex justify-center py-8"><div className="w-6 h-6 border-2 border-primary-500 border-t-transparent rounded-full animate-spin"></div></div>
  return(
    <div className="space-y-4">
      <div><h3 className="text-base font-semibold text-dark-text pb-2 border-b border-dark-border">{t('agentConfig.mcp.title')}</h3><p className="text-[10px] text-dark-muted mt-1">{t('agentConfig.mcp.desc')}</p></div>
      <div className="space-y-2">{servers.map((server,idx)=>(<div key={idx} className="p-3.5 rounded-xl bg-dark-bg border border-dark-border space-y-2"><div className="flex items-center justify-between"><div className="flex items-center gap-2"><div className="w-6 h-6 rounded-md bg-cyan-500/10 flex items-center justify-center text-[10px] font-bold text-cyan-400">{server.name.charAt(0).toUpperCase()}</div><span className="text-sm font-medium text-dark-text">{server.name}</span></div><button onClick={()=>toggleServer(idx)} className={`relative w-9 h-5 rounded-full transition-colors ${server.enabled?'bg-primary-600':'bg-dark-border'}`}><span className={`absolute top-0.5 w-3.5 h-3.5 rounded-full bg-white shadow transition-transform ${server.enabled?'translate-x-4':'translate-x-0.5'}`}/></button></div><div className="grid grid-cols-2 gap-2 pl-8"><div><span className="text-[9px] text-dark-muted block">{t('agentConfig.mcp.command')}</span><code className="text-[10px] text-dark-text font-mono break-all">{server.command}</code></div><div><span className="text-[9px] text-dark-muted block">{t('agentConfig.mcp.args')}</span><code className="text-[10px] text-dark-text font-mono break-all">{server.args.slice(0,60)}{server.args.length>60?'...':''}</code></div>{server.env&&<div className="col-span-2"><span className="text-[9px] text-dark-muted block">{t('agentConfig.mcp.env')}</span><code className="text-[10px] text-yellow-400/70 font-mono break-all">{server.env}</code></div>}</div></div>))}{servers.length===0&&<div className="text-center py-8 text-xs text-dark-muted"><div className="text-2xl mb-2">🔌</div>{t('agentConfig.mcp.noMcpServers')}</div>}</div>
    </div>
  )
}

// ==================== Tools Config Content (extracted from ToolPanel) ====================

function ToolsConfigContent({agentId}:{agentId:string}){
  const { t } = useTranslation()
  const [tools,setTools]=useState<any[]>([]),[loading,setLoading]=useState(true),[selectedTool,setSelectedTool]=useState<any>(null)
  useEffect(()=>{
    toolListAll().then(data=>{
      try{const parsed=typeof data==='string'?JSON.parse(data):data;setTools(getAllTools())}
      catch{setTools(getAllTools())}
    }).catch(()=>setTools(getAllTools())).finally(()=>setLoading(false))
  },[])
  const categories=[{key:'file',labelKey:'agentConfig.tools.toolCategories.file',color:'from-blue-500 to-blue-700'},{key:'shell',labelKey:'agentConfig.tools.toolCategories.shell',color:'from-orange-500 to-orange-700'},{key:'search',labelKey:'agentConfig.tools.toolCategories.search',color:'from-purple-500 to-purple-700'},{key:'web',labelKey:'agentConfig.tools.toolCategories.web',color:'from-cyan-500 to-cyan-700'},{key:'agent',labelKey:'agentConfig.tools.toolCategories.agent',color:'from-pink-500 to-pink-700'},{key:'misc',labelKey:'agentConfig.tools.toolCategories.misc',color:'from-gray-500 to-gray-700'}]
  const getCategory=(n:string)=>{if(['Read','Edit','Write'].includes(n))return'file';if(['Bash'].includes(n))return'shell';if(['Glob','Grep'].includes(n))return'search';if(['WebFetch','WebSearch'].includes(n))return'web';if(['Agent','TodoWrite','TaskCreate','TaskList','Workflow','Skill','EnterPlanMode','ExitPlanMode'].includes(n))return'agent';return'misc'}
  if(loading)return<div className="flex justify-center py-12"><div className="w-7 h-7 border-2 border-primary-500 border-t-transparent rounded-full animate-spin"></div></div>
  return(
    <div className="space-y-4">
      <div><h3 className="text-base font-semibold text-dark-text">{t('agentConfig.tools.title')}</h3><p className="text-xs text-dark-muted mt-0.5">{t('agentConfig.tools.desc')}</p></div>
      <div className="space-y-2">{tools.map(tool=>{const cat=getCategory(tool.name);const info=categories.find(c=>c.key===cat)!;return(<div key={tool.name} onClick={()=>setSelectedTool(tool)} className="p-3 rounded-xl border border-dark-border bg-dark-bg hover:border-primary-500/30 cursor-pointer transition-all group"><div className="flex items-start gap-3"><div className={`w-8 h-8 rounded-lg bg-gradient-to-br ${info.color} flex items-center justify-center shrink-0 opacity-80 group-hover:opacity-100`}>{ToolIconMap[tool.name]||<span className="text-white text-xs font-bold">?</span>}</div><div className="flex-1 min-w-0"><div className="flex items-center gap-2 mb-0.5"><code className="text-sm font-semibold text-primary-300">{tool.name}</code><span className="px-1.5 py-0.5 rounded text-[10px] bg-dark-border/50 text-dark-muted">{t(info.labelKey)}</span></div><p className="text-xs text-dark-muted line-clamp-1">{tool.description}</p></div></div></div>)})}</div>
      {selectedTool&&createPortal(<div className="fixed inset-0 z-[80] flex items-center justify-center bg-black/50 backdrop-blur-sm" onClick={()=>setSelectedTool(null)}><div className="bg-dark-surface border border-dark-border rounded-2xl shadow-2xl w-[650px] max-h-[80vh] overflow-y-auto p-5 animate-fade-in" onClick={e=>e.stopPropagation()}><div className="flex items-center justify-between mb-4"><div className="flex items-center gap-3"><code className="text-lg font-bold text-primary-300">{selectedTool.name}</code><span className="px-2 py-0.5 rounded bg-dark-border text-xs text-dark-muted">{t(categories.find(c=>c.key===getCategory(selectedTool.name))?.labelKey||'')}</span></div><button onClick={()=>setSelectedTool(null)} className="p-1.5 rounded-lg hover:bg-dark-border/50 text-dark-muted"><svg className="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={2}><path strokeLinecap="round" strokeLinejoin="round" d="M6 18L18 6M6 6l12 12"/></svg></button></div><p className="text-sm text-dark-text mb-4">{selectedTool.description}</p><h4 className="text-xs font-semibold uppercase tracking-wider mb-2">{t('agentConfig.tools.paramSchema')}</h4><pre className="p-4 rounded-lg bg-dark-bg border border-dark-border text-xs text-dark-muted overflow-auto font-mono leading-relaxed">{JSON.stringify(selectedTool.input_schema,null,2)}</pre></div></div>,document.body)}
    </div>
  )
}

// ==================== Tasks Config Content ====================

function TasksConfigContent({agentId}:{agentId:string}){
  const { t } = useTranslation()
  const [tasks,setTasks]=useState<any[]>([]),[todos,setTodos]=useState<any[]>([]),[loading,setLoading]=useState(true),[toast,setToast]=useState<string|null>(null)
  const loadData=useCallback(async()=>{
    try{const[tR,tR2]=await Promise.all([toolTaskList({statusFilter:''}).catch(()=>null),toolTodoGet().catch(()=>null)]);if(Array.isArray(tR))setTasks(tR);if((tR2 as unknown as {todos?:unknown[]})?.todos)setTodos((tR2 as unknown as {todos:unknown[]}).todos)}catch(e){console.error('[TasksConfig] Failed to load tasks/todos:', e)}finally{setLoading(false)}
  },[])
  useEffect(()=>{loadData()},[loadData])
  const showToast=(msg:string)=>{setToast(msg);setTimeout(()=>setToast(null),2000)}
  const handleCreateTask=async()=>{const prompt=window.prompt(t('agentConfig.tasks.enterTaskDesc'));if(!prompt?.trim())return;try{await toolTaskCreate({prompt:prompt.trim()});showToast(t('agentConfig.tasks.created'));loadData()}catch{showToast(t('agentConfig.tasks.createFailed'))}}
  const handleUpdateTodos=async(newTodos:any[])=>{setTodos(newTodos);try{await toolTodoWrite({todos:newTodos})}catch{showToast(t('agentConfig.tasks.saveFailed'))}}
  const todoCls=(todo:any)=>'flex-1 text-xs '+(todo.status==='completed'?'line-through text-dark-muted':'text-dark-text')
  const priCls=(todo:any)=>'px-1.5 py-0.5 rounded text-[10px] '+(todo.priority==='high'?'bg-red-500/10 text-red-400':todo.priority==='medium'?'bg-yellow-500/10 text-yellow-400':'bg-dark-border text-dark-muted')
  const stCls=(task:any)=>'px-1.5 py-0.5 rounded text-[10px] font-medium '+(task.status==='running'?'text-blue-400 bg-blue-500/10':task.status==='completed'?'text-green-400 bg-green-500/10':task.status==='failed'?'text-red-400 bg-red-500/10':'text-dark-muted bg-dark-border')

  if(loading)return<div className="flex justify-center py-12"><div className="w-7 h-7 border-2 border-primary-500 border-t-transparent rounded-full animate-spin"></div></div>
  return(
    <div className="space-y-4">
      <div className="flex items-center justify-between"><div><h3 className="text-base font-semibold text-dark-text">{t('agentConfig.tasks.title')}</h3><p className="text-xs text-dark-muted mt-0.5">{t('agentConfig.tasks.desc')}</p></div><button onClick={handleCreateTask} className="px-3 py-1.5 rounded-lg bg-primary-600 hover:bg-primary-500 text-white text-xs font-medium flex items-center gap-1.5"><svg className="w-3.5 h-3.5" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={2}><path strokeLinecap="round" strokeLinejoin="round" d="M12 4v16m8-8H4"/></svg>{t('agentConfig.tasks.createNew')}</button></div>
      {toast&&<div className="px-3 py-2 rounded-lg bg-primary-600/10 border border-primary-500/20 text-xs text-primary-300">{toast}</div>}

      {/* Todo list */}
      <div className="p-4 rounded-xl border border-dark-border bg-dark-bg space-y-3">
        <h4 className="text-xs font-semibold uppercase tracking-wider flex items-center gap-2">{t('agentConfig.tasks.todoList')} <span className="text-dark-muted font-normal">{todos.length}{t('agentConfig.tasks.todoItems')}</span></h4>
        {todos.length===0?<p className="text-xs text-dark-muted text-center py-4">{t('agentConfig.tasks.noTodo')}</p>:(
          <div className="space-y-1.5">{todos.map((todo:any,i:number)=>(<div key={i} className="flex items-center gap-2.5 p-2 rounded-lg hover:bg-dark-surface/50 group">
            <select value={todo.status} onChange={e=>{const u=[...todos];u[i]={...todo,status:e.target.value};handleUpdateTodos(u)}} className="text-base bg-transparent outline-none"><option value="pending">⬜</option><option value="in_progress">🔄</option><option value="completed">✅</option></select>
            <span className={todoCls(todo)}>{todo.content}</span>
            <span className={priCls(todo)}>{todo.priority}</span>
            <button onClick={()=>handleUpdateTodos(todos.filter((_,j:number)=>j!==i))} className="opacity-0 group-hover:opacity-100 text-dark-muted hover:text-red-400"><svg className="w-3.5 h-3.5" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={2}><path strokeLinecap="round" strokeLinejoin="round" d="M6 18L18 6M6 6l12 12"/></svg></button>
          </div>))}</div>
        )}
      </div>

      {/* Background tasks */}
      <div className="p-4 rounded-xl border border-dark-border bg-dark-bg space-y-3">
        <h4 className="text-xs font-semibold uppercase tracking-wider flex items-center gap-2">{t('agentConfig.tasks.backgroundTasks')} <span className="text-dark-muted font-normal">{tasks.length}{t('agentConfig.tasks.taskCount')}</span></h4>
        {tasks.length===0?<p className="text-xs text-dark-muted text-center py-4">{t('agentConfig.tasks.noBackgroundTasks')}</p>:(
          <div className="space-y-2">{tasks.map((task:any)=>(<div key={task.id} className="p-3 rounded-lg bg-dark-surface/30 border border-dark-border/50">
            <div className="flex items-center justify-between mb-1">
              <span className="text-xs font-medium text-dark-text truncate flex-1 mr-2">{task.prompt.slice(0,80)}</span>
              <span className={stCls(task)}>{task.status}</span>
            </div>
            {task.result&&<pre className="mt-1.5 text-[11px] text-dark-muted bg-dark-bg p-2 rounded max-h-20 overflow-auto font-mono">{String(task.result).slice(0,300)}</pre>}
          </div>))}</div>
        )}
      </div>
    </div>
  )
}

// ==================== Automation Config Content ====================

function AutomationConfigContent({agentId}:{agentId:string}){
  const { t } = useTranslation()
  const [autoConfig, setAutoConfig] = useState({
    enabled: false,
    ocrLanguage: 'eng',
    captureScreen: true,
    mouseControl: true,
    keyboardControl: true,
    maxRetries: 3,
    retryDelayMs: 1000,
  })
  const [saving, setSaving] = useState(false)
  const [toast, setToast] = useState<string|null>(null)

  useEffect(()=>{
    isoGetConfig({agentId, key:'automation_config'}).then((resp:any)=>{
      if(resp?.value){
        try{
          const parsed = typeof resp.value === 'string' ? JSON.parse(resp.value) : resp.value
          setAutoConfig(prev=>({...prev,...parsed}))
        }catch(e){console.error('[AutomationConfig] Failed to parse automation config:', e)}
      }
    }).catch((e) => { console.error(e) })
  },[agentId])

  const showToast=(msg:string)=>{setToast(msg);setTimeout(()=>setToast(null),2000)}

  const handleSave=async()=>{
    setSaving(true)
    try{
      await isoSetConfig({agentId, key:'automation_config', value:JSON.stringify(autoConfig)})
      showToast(t('agentConfig.automation.saveSuccess'))
    }catch(e){
      showToast(t('agentConfig.automation.saveFailed'))
    }finally{setSaving(false)}
  }

  const ToggleField=({label,description,checked,onChange}:{label:string;description?:string;checked:boolean;onChange:(v:boolean)=>void})=>(
    <div className="flex items-center justify-between py-2.5">
      <div><div className="text-sm text-dark-text">{label}</div>{description&&<div className="text-xs text-dark-muted mt-0.5">{description}</div>}</div>
      <button onClick={()=>onChange(!checked)} className={`relative w-11 h-6 rounded-full transition-all duration-300 ease-in-out shrink-0 ${checked?'bg-primary-600 shadow-lg shadow-primary-500/30':'bg-dark-border'}`} role="switch" aria-checked={checked}>
        <span className={`absolute top-0.5 left-0.5 w-5 h-5 rounded-full bg-white shadow-md transition-transform duration-300 ease-in-out ${checked?'translate-x-[22px]':'translate-x-0'}`}/>
      </button>
    </div>
  )

  return(
    <div className="p-5 space-y-5">
      {toast&&<div className="fixed top-4 right-4 z-50 px-4 py-2 rounded-lg bg-dark-card border border-dark-border text-sm text-dark-text shadow-xl animate-fade-in">{toast}</div>}
      <div className="flex items-center justify-between">
        <div>
          <h3 className="text-base font-semibold text-dark-text">{t('agentConfig.automation.title')}</h3>
          <p className="text-xs text-dark-muted mt-1">{t('agentConfig.automation.desc')}</p>
        </div>
        <button onClick={handleSave} disabled={saving} className="px-4 py-1.5 text-sm rounded-lg bg-primary-600 text-white hover:bg-primary-700 disabled:opacity-50 transition-colors">
          {saving?t('agentConfig.saving'):t('agentConfig.saveConfig')}
        </button>
      </div>

      <div className="bg-dark-card/50 rounded-xl border border-dark-border p-4 space-y-1">
        <ToggleField label={t('agentConfig.automation.enabled')} description={t('agentConfig.automation.enabledDesc')} checked={autoConfig.enabled} onChange={v=>setAutoConfig({...autoConfig,enabled:v})}/>
        <ToggleField label={t('agentConfig.automation.captureScreen')} description={t('agentConfig.automation.captureScreenDesc')} checked={autoConfig.captureScreen} onChange={v=>setAutoConfig({...autoConfig,captureScreen:v})}/>
        <ToggleField label={t('agentConfig.automation.mouseControl')} description={t('agentConfig.automation.mouseControlDesc')} checked={autoConfig.mouseControl} onChange={v=>setAutoConfig({...autoConfig,mouseControl:v})}/>
        <ToggleField label={t('agentConfig.automation.keyboardControl')} description={t('agentConfig.automation.keyboardControlDesc')} checked={autoConfig.keyboardControl} onChange={v=>setAutoConfig({...autoConfig,keyboardControl:v})}/>
      </div>

      <div className="bg-dark-card/50 rounded-xl border border-dark-border p-4 space-y-3">
        <div>
          <label className="text-sm text-dark-text">{t('agentConfig.automation.ocrLanguage')}</label>
          <select value={autoConfig.ocrLanguage} onChange={e=>setAutoConfig({...autoConfig,ocrLanguage:e.target.value})} className="mt-1 w-full px-3 py-2 rounded-lg bg-dark-bg border border-dark-border text-dark-text text-sm focus:border-primary-500 focus:outline-none">
            <option value="eng">English</option>
            <option value="chi_sim">简体中文</option>
            <option value="chi_tra">繁體中文</option>
            <option value="jpn">日本語</option>
            <option value="kor">한국어</option>
          </select>
        </div>
        <div>
          <label className="text-sm text-dark-text">{t('agentConfig.automation.maxRetries')}</label>
          <input type="number" min={1} max={10} value={autoConfig.maxRetries} onChange={e=>setAutoConfig({...autoConfig,maxRetries:Number(e.target.value)})} className="mt-1 w-full px-3 py-2 rounded-lg bg-dark-bg border border-dark-border text-dark-text text-sm focus:border-primary-500 focus:outline-none"/>
        </div>
        <div>
          <label className="text-sm text-dark-text">{t('agentConfig.automation.retryDelay')}</label>
          <input type="number" min={100} max={10000} step={100} value={autoConfig.retryDelayMs} onChange={e=>setAutoConfig({...autoConfig,retryDelayMs:Number(e.target.value)})} className="mt-1 w-full px-3 py-2 rounded-lg bg-dark-bg border border-dark-border text-dark-text text-sm focus:border-primary-500 focus:outline-none"/>
        </div>
      </div>

      <div className="bg-dark-card/50 rounded-xl border border-dark-border p-4">
        <h4 className="text-sm font-medium text-dark-text mb-2">{t('agentConfig.automation.capabilities')}</h4>
        <div className="grid grid-cols-2 gap-2 text-xs text-dark-muted">
          <div className="flex items-center gap-2"><span className="w-1.5 h-1.5 rounded-full bg-green-500"></span>{t('agentConfig.automation.capScreen')}</div>
          <div className="flex items-center gap-2"><span className="w-1.5 h-1.5 rounded-full bg-blue-500"></span>{t('agentConfig.automation.capOcr')}</div>
          <div className="flex items-center gap-2"><span className="w-1.5 h-1.5 rounded-full bg-yellow-500"></span>{t('agentConfig.automation.capSpatial')}</div>
          <div className="flex items-center gap-2"><span className="w-1.5 h-1.5 rounded-full bg-purple-500"></span>{t('agentConfig.automation.capLlm')}</div>
          <div className="flex items-center gap-2"><span className="w-1.5 h-1.5 rounded-full bg-red-500"></span>{t('agentConfig.automation.capMouse')}</div>
          <div className="flex items-center gap-2"><span className="w-1.5 h-1.5 rounded-full bg-orange-500"></span>{t('agentConfig.automation.capKeyboard')}</div>
        </div>
      </div>
    </div>
  )
}

// ==================== Cron Config Content ====================

function CronConfigContent({agentId}:{agentId:string}){
  const { t } = useTranslation()
  const [crons,setCrons]=useState<any[]>([]),[loading,setLoading]=useState(true),[showCreate,setShowCreate]=useState(false),[toast,setToast]=useState<string|null>(null)
  useEffect(()=>{toolScheduleList().then((d:any)=>{if(d?.output)try{const p=JSON.parse(d.output);if(Array.isArray(p))setCrons(p)}catch(e){console.error('[CronConfig] Failed to parse cron list:', e)}}).catch((e) => { console.error(e) }).finally(()=>setLoading(false))},[])
  const showToast=(msg:string)=>{setToast(msg);setTimeout(()=>setToast(null),2000)}
  const handleRegister=async(job:any)=>{try{await toolScheduleCron({name:job.name,schedule:job.schedule,task:job.task,enabled:job.enabled});setCrons([...crons,job]);setShowCreate(false);showToast(t('agentConfig.cron.registered', { name: job.name }))}catch{showToast(t('agentConfig.cron.registerFailed'))}}
  if(loading)return<div className="flex justify-center py-12"><div className="w-7 h-7 border-2 border-primary-500 border-t-transparent rounded-full animate-spin"></div></div>
  return(
    <div className="space-y-4">
      <div className="flex items-center justify-between"><div><h3 className="text-base font-semibold text-dark-text">{t('agentConfig.cron.title')}</h3><p className="text-xs text-dark-muted mt-0.5">{t('agentConfig.cron.desc')}</p></div><button onClick={()=>setShowCreate(!showCreate)} className="px-3 py-1.5 rounded-lg bg-primary-600 hover:bg-primary-500 text-white text-xs font-medium flex items-center gap-1.5"><svg className="w-3.5 h-3.5" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={2}><path strokeLinecap="round" strokeLinejoin="round" d="M12 4v16m8-8H4"/></svg>{t('agentConfig.cron.createNew')}</button></div>
      {toast&&<div className="px-3 py-2 rounded-lg bg-primary-600/10 border border-primary-500/20 text-xs text-primary-300">{toast}</div>}
      {showCreate&&(<div className="p-4 rounded-xl border border-primary-500/20 bg-primary-600/5 space-y-3"><h4 className="text-xs font-semibold text-primary-400">{t('agentConfig.cron.createNewCronJob')}</h4><CronForm onSubmit={handleRegister} onCancel={()=>setShowCreate(false)}/></div>)}
      <div className="space-y-2">{crons.length===0?<div className="text-center py-12 text-sm text-dark-muted">{t('agentConfig.cron.noCronJobs')}</div>:crons.map((job:any,i:number)=>(
        <div key={i} className="p-3.5 rounded-xl border border-dark-border bg-dark-bg flex items-center gap-4">
          <div className={'w-2 h-2 rounded-full '+(job.enabled?'bg-green-400 shadow-sm shadow-green-400/50':'bg-dark-border')} />
          <div className="flex-1 min-w-0"><div className="flex items-center gap-2"><span className="text-sm font-medium text-dark-text">{job.name}</span><code className="text-[11px] text-primary-300 bg-primary-600/10 px-1.5 py-0.5 rounded font-mono">{job.schedule}</code></div><p className="text-xs text-dark-muted mt-0.5 truncate">{job.task}</p></div>
          <span className={'px-1.5 py-0.5 rounded text-[10px] '+(job.enabled?'bg-green-500/10 text-green-400':'bg-dark-border text-dark-muted')}>{job.enabled?t('agentConfig.cron.running'):t('agentConfig.cron.paused')}</span>
        </div>
      ))}</div>
    </div>
  )
}

function CronForm({onSubmit,onCancel}:{onSubmit:(j:any)=>void;onCancel:()=>void}){
  const { t } = useTranslation()
  const [form,setForm]=useState<any>({name:'',schedule:'0 * * * *',task:'',enabled:true})
  return(
    <div className="grid grid-cols-2 gap-3">
      <div><label className="block text-[11px] font-medium text-dark-text mb-1">{t('agentConfig.cron.cronNameLabel')} *</label><input value={form.name} onChange={e=>setForm({...form,name:e.target.value})} placeholder="my-daily-backup" className="w-full bg-dark-bg border border-dark-border rounded-lg px-3 py-1.5 text-xs text-dark-text focus:outline-none focus:border-primary-500 font-mono"/></div>
      <div><label className="block text-[11px] font-medium text-dark-text mb-1">{t('agentConfig.cron.cronExprLabel')} *</label><input value={form.schedule} onChange={e=>setForm({...form,schedule:e.target.value})} placeholder="0 0 * * *" className="w-full bg-dark-bg border border-dark-border rounded-lg px-3 py-1.5 text-xs text-dark-text focus:outline-none focus:border-primary-500 font-mono"/></div>
      <div className="col-span-2"><label className="block text-[11px] font-medium text-dark-text mb-1">{t('agentConfig.cron.taskCmdLabel')} *</label><input value={form.task} onChange={e=>setForm({...form,task:e.target.value})} placeholder={t('agentConfig.cron.taskCmdPlaceholder')} className="w-full bg-dark-bg border border-dark-border rounded-lg px-3 py-1.5 text-xs text-dark-text focus:outline-none focus:border-primary-500 font-mono"/></div>
      <div className="col-span-2 flex justify-end gap-2 pt-1"><button onClick={onCancel} className="px-3 py-1.5 rounded-lg border border-dark-border text-xs text-dark-muted">{t('agentConfig.cron.cancel')}</button><button onClick={()=>{if(form.name&&form.schedule&&form.task)onSubmit(form)}} disabled={!form.name||!form.schedule||!form.task} className="px-3 py-1.5 rounded-lg bg-primary-600 hover:bg-primary-500 text-white text-xs disabled:opacity-40">{t('agentConfig.cron.register')}</button></div>
    </div>
  )
}

// ==================== Built-in Tool Data ====================

function getAllTools():any[]{
  return[
    {name:'Read',description:'Read file contents',input_schema:{type:"object",properties:{file_path:{type:"string"},offset:{type:"integer"},limit:{type:"integer"}},required:["file_path"]}},
    {name:'Edit',description:'Make edits via string replacement',input_schema:{type:"object",properties:{file_path:{type:"string"},edits:{type:"array",items:{type:"object",properties:{old_string:{type:"string"},new_string:{type:"string"}},required:["old_string","new_string"]}},dry_run:{type:"boolean"}},required:["file_path","edits"]}},
    {name:'Write',description:'Write content to a file',input_schema:{type:"object",properties:{file_path:{type:"string"},content:{type:"string"},create_dirs:{type:"boolean"}},required:["file_path","content"]}},
    {name:'Bash',description:'Execute shell command',input_schema:{type:"object",properties:{command:{type:"string"},working_dir:{type:"string"},timeout_secs:{type:"integer"}},required:["command"]}},
    {name:'Glob',description:'Find files matching glob pattern',input_schema:{type:"object",properties:{pattern:{type:"string"},path:{type:"string"},exclude_patterns:{type:"array",items:{type:"string"}}},required:["pattern"]}},
    {name:'Grep',description:'Search file contents using regex',input_schema:{type:"object",properties:{pattern:{type:"string"},path:{type:"string"},include_pattern:{type:"string"},exclude_pattern:{type:"string"}},required:["pattern"]}},
    {name:'WebFetch',description:'Fetch URL content',input_schema:{type:"object",properties:{url:{type:"string"},max_length:{type:"integer"}},required:["url"]}},
    {name:'WebSearch',description:'Search internet for information',input_schema:{type:"object",properties:{query:{type:"string"},num_results:{type:"integer"}},required:["query"]}},
    {name:'Agent',description:'Spawn sub-agent',input_schema:{type:"object",properties:{prompt:{type:"string"},mode:{type:"string"},model_override:{type:"string"}},required:["prompt"]}},
    {name:'TodoWrite',description:'Create/update todo list',input_schema:{type:"object",properties:{todos:{type:"array",items:{type:"object",properties:{content:{type:"string"},status:{type:"string"},priority:{type:"string"}},required:["content","status"]}}},required:["todos"]}},
    {name:'TaskCreate',description:'Create background task',input_schema:{type:"object",properties:{prompt:{type:"string"},description:{type:"string"}},required:["prompt"]}},
    {name:'Skill',description:'Invoke named skill',input_schema:{type:"object",properties:{skill_name:{type:"string"},args:{type:"object"}},required:["skill_name"]}},
    {name:'EnterPlanMode',description:'Enter plan mode',input_schema:{type:"object",properties:{}}},
    {name:'ExitPlanMode',description:'Exit plan mode',input_schema:{type:"object",properties:{}}},
  ]
}

const ToolIconMap:Record<string,JSX.Element>={
  Read:<svg className="w-4 h-4 text-white" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={2}><path strokeLinecap="round" strokeLinejoin="round" d="M9 12h6m-6 4h6m2 5H7a2 2 0 01-2-2V5a2 2 0 012-2h5.586a1 1 0 01.707.293l5.414 5.414a1 1 0 01.293.707V19a2 2 0 01-2 2z"/></svg>,
  Edit:<svg className="w-4 h-4 text-white" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={2}><path strokeLinecap="round" strokeLinejoin="round" d="M11 5H6a2 2 0 00-2 2v11a2 2 0 002 2h11a2 2 0 002-2v-5m-1.414-9.414a2 2 0 112.828 2.828L11.828 15H9v-2.828l8.586-8.586z"/></svg>,
  Write:<svg className="w-4 h-4 text-white" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={2}><path strokeLinecap="round" strokeLinejoin="round" d="M12 4v16m8-8H4"/></svg>,
  Bash:<svg className="w-4 h-4 text-white" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={2}><path strokeLinecap="round" strokeLinejoin="round" d="M8 9l3 3-3 3m5 0h3M5 20h14a2 2 0 002-2V6a2 2 0 00-2-2H5a2 2 0 00-2 2v12a2 2 0 002 2z"/></svg>,
  Glob:<svg className="w-4 h-4 text-white" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={2}><path strokeLinecap="round" strokeLinejoin="round" d="M3 7v10a2 2 0 002 2h14a2 2 0 002-2V9a2 2 0 00-2-2h-6l-2-2H5a2 2 0 00-2 2z"/></svg>,
  Grep:<svg className="w-4 h-4 text-white" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={2}><path strokeLinecap="round" strokeLinejoin="round" d="M21 21l-6-6m2-5a7 7 0 11-14 0 7 7 0 0114 0z"/></svg>,
  WebFetch:<svg className="w-4 h-4 text-white" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={2}><path strokeLinecap="round" strokeLinejoin="round" d="M21 12a9 9 0 01-9 9m9-9a9 9 0 00-9-9m9 9H3m9 9a9 9 0 01-9-9m9 9c1.657 0 3-4.03 3-9s-1.343-9-3-9m0 18c-1.657 0-3-4.03-3-9s1.343-9 3-9m-9 9a9 9 0 019-9"/></svg>,
  WebSearch:<svg className="w-4 h-4 text-white" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={2}><path strokeLinecap="round" strokeLinejoin="round" d="M21 21l-6-6m2-5a7 7 0 11-14 0 7 7 0 0114 0z"/><path strokeLinecap="round" strokeLinejoin="round" d="M10 7v6m3-3H7"/></svg>,
  Agent:<svg className="w-4 h-4 text-white" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={2}><path strokeLinecap="round" strokeLinejoin="round" d="M9.75 17L9 20l-1 1h8l-1-1-.75-3M3 13h18M5 17h14a2 2 0 002-2V5a2 2 0 00-2-2H5a2 2 0 00-2 2v10a2 2 0 002 2z"/></svg>,
  TodoWrite:<svg className="w-4 h-4 text-white" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={2}><path strokeLinecap="round" strokeLinejoin="round" d="M9 5H7a2 2 0 00-2 2v12a2 2 0 002 2h10a2 2 0 002-2V7a2 2 0 00-2-2h-2M9 5a2 2 0 002 2h2a2 2 0 002-2M9 5a2 2 0 012-2h2a2 2 0 012 2m-6 9l2 2 4-4"/></svg>,
  TaskCreate:<svg className="w-4 h-4 text-white" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={2}><path strokeLinecap="round" strokeLinejoin="round" d="M12 4v16m8-8H4"/></svg>,
  Skill:<svg className="w-4 h-4 text-white" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={2}><path strokeLinecap="round" strokeLinejoin="round" d="M9.663 17h4.673M12 3v1m6.364 1.636l-.707.707M21 12h-1M4 12H3m3.343-5.657l-.707-.707m2.828 9.9a5 5 0 117.072 0l-.548.547A3.374 3.374 0 0014 18.469V19a2 2 0 11-4 0v-.531c0-.895-.356-1.754-.988-2.386l-.548-.547z"/></svg>,
  EnterPlanMode:<svg className="w-4 h-4 text-white" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={2}><path strokeLinecap="round" strokeLinejoin="round" d="M9 5H7a2 2 0 00-2 2v12a2 2 0 002 2h10a2 2 0 002-2V7a2 2 0 00-2-2h-2M9 5a2 2 0 002 2h2a2 2 0 002-2M9 5a2 2 0 012-2h2a2 2 0 012 2m-6 9l2 2 4-4"/></svg>,
  ExitPlanMode:<svg className="w-4 h-4 text-white" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={2}><path strokeLinecap="round" strokeLinejoin="round" d="M11 16l-4-4m0 0l4-4m-4 4h14m-5 4v1a3 3 0 01-3 3H6a3 3 0 01-3-3V7a3 3 0 013-3h7a3 3 0 013 3v1"/></svg>,
}

// ==================== UI Helper Components ====================

function SliderField({label,value,min,max,step,onChange}:{label:string;value:number;min:number;max:number;step:number;onChange:(v:number)=>void}){
  return(<div><div className="flex items-center justify-between mb-1.5"><label className="text-sm font-medium text-dark-text">{label}</label><span className="text-xs text-primary-400 font-mono bg-primary-600/10 px-2 py-0.5 rounded">{value}</span></div><input type="range" min={min} max={max} step={step} value={value} onChange={e=>onChange(Number(e.target.value))} className="w-full h-1.5 bg-dark-border rounded-full appearance-none cursor-pointer accent-primary-500 [&::-webkit-slider-thumb]:appearance-none [&::-webkit-slider-thumb]:w-4 [&::-webkit-slider-thumb]:h-4 [&::-webkit-slider-thumb]:rounded-full [&::-webkit-slider-thumb]:bg-primary-500"/></div>)
}

function ToggleField({label,description,checked,onChange}:{label:string;description?:string;checked:boolean;onChange:(v:boolean)=>void}){
  return(<div className="flex items-center justify-between py-2"><div><label className="text-sm font-medium text-dark-text">{label}</label>{description&&<p className="text-xs text-dark-muted mt-0.5">{description}</p>}</div><button onClick={()=>onChange(!checked)} className={`relative w-11 h-6 rounded-full transition-colors duration-200 ${checked?'bg-primary-600':'bg-dark-border'}`}><span className={`absolute top-1 w-4 h-4 rounded-full bg-white shadow transition-transform duration-200 ${checked?'translate-x-6':'translate-x-1'}`}/></button></div>)
}
