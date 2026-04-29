// Claw Desktop - i18n国际化配置 - 使用i18next + react-i18next实现多语言支持
// 支持语言：zh-CN(简体中文，默认)、zh-TW(繁体中文)、en(英语)
import i18n from 'i18next'
import { initReactI18next } from 'react-i18next'
import LanguageDetector from 'i18next-browser-languagedetector'
import zhCN from './locales/zh-CN.json'
import en from './locales/en.json'
import zhTW from './locales/zh-TW.json'

const resources = {
  'zh-CN': { translation: zhCN },
  'en': { translation: en },
  'zh-TW': { translation: zhTW },
}

i18n
  .use(LanguageDetector)
  .use(initReactI18next)
  .init({
    resources,
    fallbackLng: 'zh-CN',
    supportedLngs: ['zh-CN', 'en', 'zh-TW'],
    interpolation: {
      escapeValue: false,
    },
    detection: {
      order: ['localStorage', 'navigator'],
      caches: ['localStorage'],
      lookupLocalStorage: 'claw-language',
    },
  })

export default i18n
