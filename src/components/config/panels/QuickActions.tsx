// Claw Desktop - 快捷操作面板 - 提供常用操作的快捷入口
// 快速模式/清空会话/语音输入/主题切换

import { useState, useEffect } from 'react'
import { useTranslation } from 'react-i18next'

type Theme = 'dark' | 'light' | 'system'
type FastMode = boolean

interface QuickActionsProps {
  onClearConversation?: () => void
  onToggleFastMode?: (enabled: FastMode) => void
  fastMode?: FastMode
  onThemeChange?: (theme: Theme) => void
  currentTheme?: Theme
  agentId?: string
}

export default function QuickActions({ onClearConversation, onToggleFastMode, fastMode = false, onThemeChange, currentTheme = 'dark', agentId }: QuickActionsProps) {
  const { t } = useTranslation()
  const [voiceEnabled, setVoiceEnabled] = useState(false)
  const [voiceStatus, setVoiceStatus] = useState<'idle' | 'listening' | 'processing'>('idle')

  useEffect(() => {
    const saved = localStorage.getItem('claw-theme') as Theme || 'dark'
    if (onThemeChange) onThemeChange(saved)
  }, [])

  const handleThemeChange = (theme: Theme) => {
    localStorage.setItem('claw-theme', theme)
    if (onThemeChange) onThemeChange(theme)
    const root = document.documentElement
    if (theme === 'light') {
      root.classList.add('light')
    } else if (theme === 'dark') {
      root.classList.remove('light')
    } else {
      const mq = window.matchMedia('(prefers-color-scheme: light)')
      if (mq.matches) root.classList.add('light')
      else root.classList.remove('light')
    }
  }

  const toggleVoice = () => {
    if (!voiceEnabled) return
    setVoiceStatus('listening')
    if ('webkitSpeechRecognition' in window || 'SpeechRecognition' in window) {
      const SpeechRecognition = (window as any).SpeechRecognition || (window as any).webkitSpeechRecognition
      const recognition = new SpeechRecognition()
      recognition.continuous = false
      recognition.interimResults = false
      recognition.lang = 'zh-CN'
      recognition.onresult = (event: any) => {
        const text = event.results[0][0].transcript
        setVoiceStatus('idle')
        window.dispatchEvent(new CustomEvent('voice-result', { detail: text }))
      }
      recognition.onerror = () => { setVoiceStatus('idle') }
      recognition.onend = () => { setVoiceStatus('idle') }
      recognition.start()
    } else {
      setVoiceStatus('idle')
    }
  }

  const themes: { id: Theme; label: string; icon: string }[] = [
    { id: 'dark', label: 'Dark', icon: '🌙' },
    { id: 'light', label: 'Light', icon: '☀️' },
    { id: 'system', label: 'System', icon: '💻' },
  ]

  return (
    <div className="space-y-4">
      <h3 className="text-base font-semibold text-dark-text flex items-center gap-2">
        <svg className="w-5 h-5 text-amber-400" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={2}><path strokeLinecap="round" strokeLinejoin="round" d="M13 10V3L4 14h7v7l9-11h-7z"/></svg>
        {t('panels.quickActions.title')}
      </h3>

      {/* Theme switcher */}
      <div className="space-y-2">
        <span className="text-xs font-medium text-dark-muted">{t('panels.quickActions.themeLabel')}</span>
        <div className="flex gap-1.5 p-1 rounded-lg bg-dark-bg border border-dark-border w-fit">
          {themes.map(th => (
            <button key={th.id} onClick={() => handleThemeChange(th.id)} className={`px-3 py-1.5 rounded-md text-[11px] transition-colors flex items-center gap-1.5 ${currentTheme === th.id ? 'bg-primary-600 text-white' : 'text-dark-muted hover:text-dark-text'}`}>
              <span>{th.icon}</span> {th.label}
            </button>
          ))}
        </div>
      </div>

      {/* Action buttons grid */}
      <div className="grid grid-cols-2 gap-2">
        {/* Fast Mode */}
        <button onClick={() => onToggleFastMode?.(!fastMode)} className={`p-3 rounded-xl border transition-all text-left ${fastMode ? 'bg-orange-500/10 border-orange-500/30' : 'border border-dark-border hover:bg-dark-bg'}`}>
          <div className="flex items-center gap-2 mb-1">
            <span className={`w-6 h-6 rounded-lg flex items-center justify-center text-xs ${fastMode ? 'bg-orange-500/20 text-orange-400' : 'bg-dark-surface text-dark-muted'}`}>⚡</span>
            <span className={`text-xs font-semibold ${fastMode ? 'text-orange-400' : 'text-dark-text'}`}>{t('panels.quickActions.fastMode')}</span>
          </div>
          <span className="text-[10px] text-dark-muted">{fastMode ? t('panels.quickActions.fastModeOn') : t('panels.quickActions.fastModeOff')}</span>
        </button>

        {/* Clear Conversation */}
        <button onClick={() => { if (confirm(t('panels.quickActions.clearConfirm'))) onClearConversation?.() }} className="p-3 rounded-xl border border-dark-border hover:bg-red-500/5 hover:border-red-500/20 transition-all text-left">
          <div className="flex items-center gap-2 mb-1">
            <span className="w-6 h-6 rounded-lg bg-red-500/10 text-red-400 flex items-center justify-center text-xs">🗑</span>
            <span className="text-xs font-semibold text-dark-text">{t('panels.quickActions.clearConversation')}</span>
          </div>
          <span className="text-[10px] text-dark-muted">{t('panels.quickActions.clearConversationDesc')}</span>
        </button>

        {/* Voice Input */}
        <button onClick={() => { setVoiceEnabled(!voiceEnabled); if (!voiceEnabled) toggleVoice() }} className={`p-3 rounded-xl border transition-all text-left ${voiceEnabled ? 'bg-purple-500/10 border-purple-500/30' : 'border border-dark-border hover:bg-dark-bg'}`}>
          <div className="flex items-center gap-2 mb-1">
            <span className={`w-6 h-6 rounded-lg flex items-center justify-center text-xs ${voiceStatus === 'listening' ? 'bg-purple-400 animate-pulse text-white' : voiceEnabled ? 'bg-purple-500/20 text-purple-400' : 'bg-dark-surface text-dark-muted'}`}>🎤</span>
            <span className={`text-xs font-semibold ${voiceEnabled ? 'text-purple-400' : 'text-dark-text'}`}>{t('panels.quickActions.voiceInput')}</span>
          </div>
          <span className="text-[10px] text-dark-muted">
            {voiceStatus === 'listening' ? t('panels.quickActions.voiceListening') : voiceEnabled ? t('panels.quickActions.voiceClickStart') : t('panels.quickActions.voiceDisabled')}
          </span>
        </button>

        {/* Compact Mode placeholder */}
        <button className="p-3 rounded-xl border border-dark-border hover:bg-dark-bg transition-all text-left opacity-60 cursor-not-allowed">
          <div className="flex items-center gap-2 mb-1">
            <span className="w-6 h-6 rounded-lg bg-dark-surface text-dark-muted flex items-center justify-center text-xs">📐</span>
            <span className="text-xs font-semibold text-dark-text">{t('panels.quickActions.compactMode')}</span>
          </div>
          <span className="text-[10px] text-dark-muted">{t('panels.quickActions.comingSoon')}</span>
        </button>
      </div>

      {/* Voice status indicator */}
      {voiceStatus === 'listening' && (
        <div className="flex items-center gap-2 px-3 py-2 rounded-lg bg-purple-600/10 border border-purple-500/20">
          <div className="relative">
            <div className="w-2 h-2 bg-purple-400 rounded-full animate-ping"></div>
            <div className="absolute top-0 w-2 h-2 bg-purple-400 rounded-full"></div>
          </div>
          <span className="text-xs text-purple-300">{t('panels.quickActions.voiceListeningHint')}</span>
          <button onClick={() => setVoiceStatus('idle')} className="ml-auto text-[10px] text-purple-300/50 hover:text-purple-300">{t('panels.quickActions.cancel')}</button>
        </div>
      )}
    </div>
  )
}
