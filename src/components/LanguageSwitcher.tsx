// Claw Desktop - 语言切换器 - 切换界面显示语言（中文简体/繁体/英语）
import { useState, useRef, useEffect } from 'react'
import { useTranslation } from 'react-i18next'

const LANGUAGES = [
  { code: 'zh-CN', label: '简体中文', flag: '🇨🇳' },
  { code: 'en', label: 'English', flag: '🇺🇸' },
  { code: 'zh-TW', label: '繁體中文', flag: '🇹🇼' },
]

export default function LanguageSwitcher() {
  const { i18n } = useTranslation()
  const [isOpen, setIsOpen] = useState(false)
  const ref = useRef<HTMLDivElement>(null)

  const currentLang = LANGUAGES.find(l => l.code === i18n.language) || LANGUAGES[0]

  useEffect(() => {
    const handleClickOutside = (e: MouseEvent) => {
      if (ref.current && !ref.current.contains(e.target as Node)) {
        setIsOpen(false)
      }
    }
    document.addEventListener('mousedown', handleClickOutside)
    return () => document.removeEventListener('mousedown', handleClickOutside)
  }, [])

  const handleChangeLanguage = (code: string) => {
    i18n.changeLanguage(code)
    setIsOpen(false)
  }

  return (
    <div ref={ref} className="relative">
      <button
        onClick={() => setIsOpen(!isOpen)}
        className="flex items-center gap-1.5 px-2.5 py-1.5 rounded-lg text-xs text-dark-muted hover:text-dark-text hover:bg-dark-surface/80 border border-transparent hover:border-dark-border/50 transition-all duration-150"
        title={i18n.t('languageSwitcher.title')}
      >
        <span>{currentLang.flag}</span>
        <span className="hidden sm:inline">{currentLang.code.toUpperCase()}</span>
        <svg className={`w-3 h-3 transition-transform ${isOpen ? 'rotate-180' : ''}`} fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={2}>
          <path strokeLinecap="round" strokeLinejoin="round" d="M19 9l-7 7-7-7" />
        </svg>
      </button>

      {isOpen && (
        <div className="absolute right-0 top-full mt-1 w-40 bg-dark-bg border border-dark-border rounded-xl shadow-xl z-50 py-1 animate-fade-in overflow-hidden">
          {LANGUAGES.map(lang => (
            <button
              key={lang.code}
              onClick={() => handleChangeLanguage(lang.code)}
              className={`w-full flex items-center gap-2.5 px-3 py-2 text-left text-xs transition-colors ${
                i18n.language === lang.code
                  ? 'bg-primary-600/10 text-primary-400 font-medium'
                  : 'text-dark-muted hover:text-dark-text hover:bg-dark-surface/60'
              }`}
            >
              <span className="text-base">{lang.flag}</span>
              <span>{lang.label}</span>
              {i18n.language === lang.code && (
                <svg className="w-3.5 h-3.5 ml-auto text-primary-400" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={3}>
                  <path strokeLinecap="round" strokeLinejoin="round" d="M5 13l4 4L19 7" />
                </svg>
              )}
            </button>
          ))}
        </div>
      )}
    </div>
  )
}
