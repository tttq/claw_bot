// Claw Desktop - 浏览器面板 - CDP浏览器自动化控制（导航、截图、点击、填表、执行JS）

import { useState, useEffect, useCallback } from 'react'
import { useTranslation } from 'react-i18next'
import { browserNavigate, browserClick, browserFillInput, browserReload, browserCloseTab, browserDetect, browserLaunch, browserListTabs, browserCheckPort, browserGetContent, browserExecuteJs, browserScreenshot } from '../../api/browser'
import { isoGetConfig, isoSetConfig } from '../../api/iso'
import { debugLog } from '../../utils/debugLog'

interface BrowserPanelProps {
  onClose?: () => void
}

interface BrowserInfo {
  name: string
  path: string
  version?: string
  is_installed: boolean
}

interface Tab {
  id: string
  url: string
  title: string
}

export default function BrowserPanel({ onClose, agentId }: { onClose?: () => void; agentId?: string }) {
  const { t } = useTranslation()
  const [browsers, setBrowsers] = useState<BrowserInfo[]>([])
  const [selectedBrowser, setSelectedBrowser] = useState<string>('')
  const [port, setPort] = useState(9222)
  const [isConnected, setIsConnected] = useState(false)
  const [tabs, setTabs] = useState<Tab[]>([])
  const [selectedTab, setSelectedTab] = useState<string>('')
  const [loading, setLoading] = useState(false)
  const [error, setError] = useState<string | null>(null)
  const [urlInput, setUrlInput] = useState('')
  const [jsInput, setJsInput] = useState('')
  const [selectorInput, setSelectorInput] = useState('')
  const [valueInput, setValueInput] = useState('')
  const [pageContent, setPageContent] = useState('')
  const [screenshotData, setScreenshotData] = useState<string | null>(null)
  const [activeTab, setActiveTab] = useState<'control' | 'tabs' | 'console' | 'screenshot' | 'execute'>('control')

  useEffect(() => {
    detectBrowsers()
    checkConnection()
    if (agentId) {
      isoGetConfig({ agentId, key: 'browser_config' }).then((resp: any) => {
        if (resp?.value) {
          try {
            const parsed = typeof resp.value === 'string' ? JSON.parse(resp.value) : resp.value
            if (parsed.selectedBrowser) setSelectedBrowser(parsed.selectedBrowser)
            if (parsed.port) setPort(parsed.port)
          } catch (e) { console.error(e) }
        }
      }).catch((e) => { console.error(e) })
    }
    const interval = setInterval(checkConnection, 5000)
    return () => clearInterval(interval)
  }, [])

  const detectBrowsers = async () => {
    try {
      setLoading(true)
      const result = await browserDetect() as unknown as { browsers?: BrowserInfo[] }
      setBrowsers(result.browsers || [])
      if (result.browsers && result.browsers.length > 0) {
        setSelectedBrowser(result.browsers[0].path)
      }
      setError(null)
    } catch (e) {
      const errMsg = e instanceof Error ? e.message : String(e)
      setError(t('panels.browser.detectFailed', { error: errMsg }))
    } finally {
      setLoading(false)
    }
  }

  const launchBrowser = async () => {
    if (!selectedBrowser) { setError(t('panels.browser.selectBrowserFirst')); return }
    try {
      setLoading(true)
      setError(null)
      const result = await browserLaunch({
        browserPath: selectedBrowser,
        port: port,
      }) as { success?: boolean; warning?: string }
      if (result.success) {
        setIsConnected(true)
        await loadTabs()
        if (agentId) {
          isoSetConfig({ agentId, key: 'browser_config', value: JSON.stringify({ selectedBrowser, port }) }).catch((e) => { console.error(e) })
        }
      }
      if (result.warning) {
        console.warn(result.warning)
      }
    } catch (e) {
      const errMsg = e instanceof Error ? e.message : String(e)
      setError(t('panels.browser.launchFailed', { error: errMsg }))
    } finally {
      setLoading(false)
    }
  }

  const checkConnection = async () => {
    try {
      const result = await browserCheckPort({ port }) as { available: boolean }
      setIsConnected(result.available)
      if (result.available) {
        await loadTabs()
      }
    } catch (e) { console.error(e) }
  }

  const loadTabs = async () => {
    try {
      const result = await browserListTabs({ port }) as { tabs?: Tab[] }
      setTabs(result.tabs || [])
      if (result.tabs && result.tabs.length > 0 && !selectedTab) {
        setSelectedTab(result.tabs[0].id)
      }
    } catch (e) {
      console.error(t('panels.browser.loadTabsFailed'), e)
    }
  }

  const navigateToUrl = async () => {
    if (!urlInput.trim() || !selectedTab) return
    try {
      setLoading(true)
      await browserNavigate({ port, tab_id: selectedTab, url: urlInput })
      await new Promise(r => setTimeout(r, 1000))
      await loadTabs()
      await getPageContent()
    } catch (e) {
      const errMsg = e instanceof Error ? e.message : String(e)
      setError(t('panels.browser.navigateFailed', { error: errMsg }))
    } finally {
      setLoading(false)
    }
  }

  const getPageContent = async () => {
    if (!selectedTab) return
    try {
      const result = await browserGetContent({ port, tab_id: selectedTab }) as unknown as { content?: string }
      setPageContent((result.content || '').substring(0, 5000))
    } catch (e) {
      const errMsg = e instanceof Error ? e.message : String(e)
      setError(t('panels.browser.getContentFailed', { error: errMsg }))
    }
  }

  const executeJS = async () => {
    if (!jsInput.trim() || !selectedTab) return
    try {
      setLoading(true)
      const result = await browserExecuteJs({ port, tab_id: selectedTab, script: jsInput })
      debugLog('[BrowserPanel] JS Result:', result)
      alert(t('panels.browser.executeSuccess', { result: JSON.stringify(result, null, 2) }))
    } catch (e) {
      const errMsg = e instanceof Error ? e.message : String(e)
      setError(t('panels.browser.executeJsFailed', { error: errMsg }))
    } finally {
      setLoading(false)
    }
  }

  const takeScreenshot = async () => {
    if (!selectedTab) return
    try {
      setLoading(true)
      const result = await browserScreenshot({ port, tab_id: selectedTab, format: 'png' }) as { data: string }
      setScreenshotData(result.data)
    } catch (e) {
      const errMsg = e instanceof Error ? e.message : String(e)
      setError(t('panels.browser.screenshotFailed', { error: errMsg }))
    } finally {
      setLoading(false)
    }
  }

  const clickElement = async () => {
    if (!selectorInput.trim() || !selectedTab) return
    try {
      setLoading(true)
      await browserClick({ port, tab_id: selectedTab, selector: selectorInput })
      alert(t('panels.browser.clickDone'))
    } catch (e) {
      const errMsg = e instanceof Error ? e.message : String(e)
      setError(t('panels.browser.clickFailed', { error: errMsg }))
    } finally {
      setLoading(false)
    }
  }

  const fillInputElement = async () => {
    if (!selectorInput.trim() || !valueInput.trim() || !selectedTab) return
    try {
      setLoading(true)
      await browserFillInput({ port, tab_id: selectedTab, selector: selectorInput, value: valueInput })
      alert(t('panels.browser.fillDone'))
    } catch (e) {
      const errMsg = e instanceof Error ? e.message : String(e)
      setError(t('panels.browser.fillFailed', { error: errMsg }))
    } finally {
      setLoading(false)
    }
  }

  const reloadPage = async (ignoreCache = false) => {
    if (!selectedTab) return
    try {
      await browserReload({ port, tab_id: selectedTab })
      await new Promise(r => setTimeout(r, 1000))
      await loadTabs()
    } catch (e) {
      const errMsg = e instanceof Error ? e.message : String(e)
      setError(t('panels.browser.refreshFailed', { error: errMsg }))
    }
  }

  const closeTab = async () => {
    if (!selectedTab) return
    try {
      await browserCloseTab({ port, tab_id: selectedTab })
      setSelectedTab('')
      await loadTabs()
    } catch (e) {
      const errMsg = e instanceof Error ? e.message : String(e)
      setError(t('panels.browser.closeTabFailed', { error: errMsg }))
    }
  }

  return (
    <div className="h-full flex flex-col bg-gray-50 dark:bg-gray-900 rounded-lg shadow-lg">
      {/* Title bar */}
      <div className="flex items-center justify-between px-4 py-3 border-b border-gray-200 dark:border-gray-700 bg-white dark:bg-gray-800 rounded-t-lg">
        <div className="flex items-center gap-2">
          <svg className="w-5 h-5 text-green-600" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={2}>
            <path strokeLinecap="round" strokeLinejoin="round" d="M21 12a9 9 0 01-9 9m9-9a9 9 0 00-9-9m9-9H3m9-9a9 9 0 019-9" />
          </svg>
          <h2 className="text-lg font-bold text-gray-900 dark:text-white">🌐 {t('panels.browser.title')}</h2>
        </div>
        <div className="flex items-center gap-2">
          <span className={`px-2 py-1 text-xs rounded-full font-medium ${
            isConnected ? 'bg-green-100 text-green-800 dark:bg-green-900/30 dark:text-green-300' : 'bg-gray-100 text-gray-600'
          }`}>
            {isConnected ? `✓ ${t('panels.browser.connected', { port })}` : `○ ${t('panels.browser.disconnected')}`}
          </span>
          {onClose && (
            <button onClick={onClose} className="p-1 hover:bg-gray-100 dark:hover:bg-gray-700 rounded">
              <svg className="w-5 h-5 text-gray-600 dark:text-gray-300" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={2}>
                <path strokeLinecap="round" strokeLinejoin="round" d="M6 18L18 6M6 6l12 12" />
              </svg>
            </button>
          )}
        </div>
      </div>

      {/* Error notification */}
      {error && (
        <div className="mx-4 mt-2 p-3 bg-red-50 dark:bg-red-900/20 border border-red-200 dark:border-red-700 text-red-700 dark:text-red-300 rounded text-sm flex justify-between">
          ⚠️ {error}
          <button onClick={() => setError(null)} className="underline">{t('panels.browser.close')}</button>
        </div>
      )}

      {/* Content area */}
      <div className="flex-1 overflow-y-auto p-4 space-y-4">

        {/* Browser selection and launch */}
        {!isConnected ? (
          <div className="space-y-3 animate-fade-in">
            <h3 className="font-semibold text-gray-900 dark:text-white">🔍 {t('panels.browser.detectAndLaunch')}</h3>

            <div className="grid gap-2">
              {browsers.map((browser, idx) => (
                <label key={idx} className={`flex items-center p-3 rounded-lg cursor-pointer transition-all ${
                  selectedBrowser === browser.path
                    ? 'bg-blue-50 dark:bg-blue-900/30 ring-2 ring-blue-500'
                    : 'bg-white dark:bg-gray-800 hover:bg-gray-50 dark:hover:bg-gray-750'
                }`}>
                  <input type="radio" name="browser" checked={selectedBrowser === browser.path}
                    onChange={() => setSelectedBrowser(browser.path)} className="sr-only" />
                  <div className="ml-2 flex-1 min-w-0">
                    <div className="font-medium text-gray-900 dark:text-white">{browser.name}</div>
                    <div className="text-xs text-gray-500 truncate">{browser.path}</div>
                  </div>
                  {browser.version && (
                    <span className="text-xs text-gray-400 ml-2">v{browser.version}</span>
                  )}
                </label>
              ))}
            </div>

            {browsers.length === 0 && !loading && (
              <div className="text-center py-8 text-gray-400">
                <div className="text-4xl mb-2">🌐</div>
                <div>{t('panels.browser.noBrowserDetected')}</div>
                <div className="text-xs mt-1">{t('panels.browser.noBrowserHint')}</div>
                <button onClick={detectBrowsers} className="mt-3 px-4 py-2 bg-blue-600 text-white rounded-lg text-sm">{t('panels.browser.redetect')}</button>
              </div>
            )}

            <div className="flex gap-2 items-end">
              <div className="flex-1">
                <label className="block text-xs text-gray-600 dark:text-gray-400 mb-1">{t('panels.browser.debugPort')}</label>
                <input type="number" value={port} onChange={(e) => setPort(Number(e.target.value))}
                  className="w-full px-3 py-2 border rounded-lg bg-white dark:bg-gray-800 text-gray-900 dark:text-white" />
              </div>
              <button onClick={launchBrowser} disabled={loading || !selectedBrowser}
                className="px-6 py-2.5 bg-green-600 text-white rounded-lg hover:bg-green-700 disabled:opacity-50 transition-colors font-medium">
                {loading ? '⏳ ' + t('panels.browser.launching') : '🚀 ' + t('panels.browser.launchBrowser')}
              </button>
            </div>

            <div className="p-3 bg-yellow-50 dark:bg-yellow-900/20 rounded-lg text-xs text-yellow-800 dark:text-yellow-200 leading-relaxed">
              💡 **{t('panels.browser.tip')}**
              <br />{t('panels.browser.tipManual')}
            </div>
          </div>
        ) : (
          <>
            {/* Connected state - Tab navigation */}
            <div className="flex gap-1 mb-3 overflow-x-auto pb-1">
              {[{ id: 'control', label: '🎮 ' + t('panels.browser.tabControl') }, { id: 'tabs', label: '📑 ' + t('panels.browser.tabTabs') }, { id: 'screenshot', label: '📸 ' + t('panels.browser.tabScreenshot') }, { id: 'execute', label: '⚡ ' + t('panels.browser.tabExecute') }].map(tab => (
                <button key={tab.id} onClick={() => setActiveTab(tab.id as typeof activeTab)}
                  className={`px-3 py-1.5 text-xs rounded whitespace-nowrap transition-all ${
                    activeTab === tab.id ? 'bg-blue-600 text-white' : 'hover:bg-gray-200 dark:hover:bg-gray-700 text-gray-700 dark:text-gray-300'
                  }`}>{tab.label}</button>
              ))}
            </div>

            {/* Console Tab */}
            {activeTab === 'control' && (
              <div className="space-y-3 animate-fade-in">
                <div className="grid grid-cols-2 gap-3">
                  <button onClick={() => window.open(`http://localhost:${port}`, '_blank')}
                    className="p-3 bg-white dark:bg-gray-800 rounded-lg hover:shadow-md transition-shadow text-left">
                    <div className="text-sm font-medium text-gray-900 dark:text-white">🔗 {t('panels.browser.openDevTools')}</div>
                    <div className="text-xs text-gray-400">{t('panels.browser.openDevToolsDesc')}</div>
                  </button>
                  <button onClick={() => reloadPage()}
                    className="p-3 bg-white dark:bg-gray-800 rounded-lg hover:shadow-md transition-shadow text-left">
                    <div className="text-sm font-medium text-gray-900 dark:text-white">🔄 {t('panels.browser.refreshPage')}</div>
                    <div className="text-xs text-gray-400">{t('panels.browser.refreshPageDesc')}</div>
                  </button>
                  <button onClick={() => reloadPage(true)}
                    className="p-3 bg-white dark:bg-gray-800 rounded-lg hover:shadow-md transition-shadow text-left">
                    <div className="text-sm font-medium text-gray-900 dark:text-white">⚡ {t('panels.browser.hardRefresh')}</div>
                    <div className="text-xs text-gray-400">{t('panels.browser.hardRefreshDesc')}</div>
                  </button>
                  <button onClick={closeTab}
                    className="p-3 bg-red-50 dark:bg-red-900/20 rounded-lg hover:shadow-md transition-shadow text-left">
                    <div className="text-sm font-medium text-red-800 dark:text-red-300">❌ {t('panels.browser.closeTab')}</div>
                    <div className="text-xs text-red-400">{t('panels.browser.closeTabDesc')}</div>
                  </button>
                </div>

                <div>
                  <label className="block text-xs font-medium text-gray-700 dark:text-gray-300 mb-1">{t('panels.browser.quickNavigate')}</label>
                  <div className="flex gap-2">
                    <input type="text" value={urlInput} onChange={(e) => setUrlInput(e.target.value)}
                      onKeyPress={(e) => e.key === 'Enter' && navigateToUrl()}
                      placeholder="https://example.com"
                      className="flex-1 px-3 py-2 border rounded-lg bg-white dark:bg-gray-800 text-gray-900 dark:text-white text-sm" />
                    <button onClick={navigateToUrl} disabled={!urlInput.trim()} className="px-4 py-2 bg-blue-600 text-white rounded-lg text-sm">{t('panels.browser.go')}</button>
                  </div>
                </div>

                <div>
                  <label className="block text-xs font-medium text-gray-700 dark:text-gray-300 mb-1">{t('panels.browser.getPageContent')}</label>
                  <div className="flex gap-2">
                    <button onClick={getPageContent} className="px-4 py-2 bg-purple-600 text-white rounded-lg text-sm">{t('panels.browser.getContent')}</button>
                  </div>
                  {pageContent && (
                    <pre className="mt-2 p-3 bg-gray-100 dark:bg-gray-800 rounded-lg text-xs max-h-48 overflow-auto text-gray-800 dark:text-gray-200">{pageContent}</pre>
                  )}
                </div>
              </div>
            )}

            {/* Tabs list */}
            {activeTab === 'tabs' && (
              <div className="space-y-2 animate-fade-in">
                <div className="flex items-center justify-between">
                  <h3 className="font-semibold text-gray-900 dark:text-white">{t('panels.browser.openTabs')}</h3>
                  <button onClick={loadTabs} className="text-xs text-blue-600 hover:underline">{t('panels.browser.refreshList')}</button>
                </div>
                {tabs.map(tab => (
                  <div key={tab.id} onClick={() => setSelectedTab(tab.id)}
                    className={`p-3 rounded-lg cursor-pointer transition-all ${
                      selectedTab === tab.id ? 'bg-blue-50 dark:bg-blue-900/30 ring-2 ring-blue-500' : 'bg-white dark:bg-gray-800 hover:bg-gray-50'
                    }`}>
                    <div className="flex items-start gap-3">
                      <div className={`w-2 h-2 mt-2 rounded-full ${selectedTab === tab.id ? 'bg-blue-600' : 'bg-gray-300'}`} />
                      <div className="flex-1 min-w-0">
                        <div className="font-medium text-gray-900 dark:text-white truncate">{tab.title || 'Untitled'}</div>
                        <div className="text-xs text-gray-500 truncate mt-0.5">{tab.url}</div>
                        <div className="text-[10px] text-gray-400 font-mono mt-1">{tab.id.slice(0, 8)}...</div>
                      </div>
                    </div>
                  </div>
                ))}
                {tabs.length === 0 && <div className="text-center py-8 text-gray-400">{t('panels.browser.noOpenTabs')}</div>}
              </div>
            )}

            {/* Screenshot */}
            {activeTab === 'screenshot' && (
              <div className="space-y-3 animate-fade-in">
                <button onClick={takeScreenshot} disabled={loading}
                  className="w-full py-3 bg-indigo-600 text-white rounded-lg hover:bg-indigo-700 disabled:opacity-50 font-medium">
                  📸 {t('panels.browser.takeScreenshot')}
                </button>
                {screenshotData && (
                  <div className="border-2 border-dashed border-gray-300 dark:border-gray-600 rounded-lg p-2 bg-white">
                    <img src={`data:image/png;base64,${screenshotData}`} alt="Screenshot" className="w-full rounded" />
                  </div>
                )}
              </div>
            )}

            {/* Execute JS */}
            {activeTab === 'execute' && (
              <div className="space-y-3 animate-fade-in">
                <div>
                  <label className="block text-xs font-medium text-gray-700 dark:text-gray-300 mb-1">{t('panels.browser.javascriptCode')}</label>
                  <textarea value={jsInput} onChange={(e) => setJsInput(e.target.value)} rows={6}
                    placeholder="document.title"
                    className="w-full px-3 py-2 border rounded-lg bg-white dark:bg-gray-800 text-gray-900 dark:text-white text-sm font-mono" />
                  <button onClick={executeJS} disabled={loading || !jsInput.trim()}
                    className="mt-2 w-full py-2 bg-orange-600 text-white rounded-lg hover:bg-orange-700 disabled:opacity-50 text-sm font-medium">
                    ▶ {t('panels.browser.executeJavascript')}
                  </button>
                </div>

                <hr className="border-gray-200 dark:border-gray-700" />

                <h4 className="font-medium text-sm text-gray-900 dark:text-white">{t('panels.browser.domShortcuts')}</h4>

                <div className="grid grid-cols-2 gap-3">
                  <div>
                    <label className="block text-xs text-gray-600 dark:text-gray-400 mb-1">{t('panels.browser.cssSelector')}</label>
                    <input type="text" value={selectorInput} onChange={(e) => setSelectorInput(e.target.value)}
                      placeholder="#submit-btn" className="w-full px-3 py-2 border rounded-lg bg-white dark:bg-gray-800 text-sm" />
                  </div>
                  <div>
                    <label className="block text-xs text-gray-600 dark:text-gray-400 mb-1">{t('panels.browser.value')}</label>
                    <input type="text" value={valueInput} onChange={(e) => setValueInput(e.target.value)}
                      placeholder="Hello World" className="w-full px-3 py-2 border rounded-lg bg-white dark:bg-gray-800 text-sm" />
                  </div>
                  <button onClick={clickElement} disabled={!selectorInput.trim()}
                    className="py-2 bg-cyan-600 text-white rounded-lg hover:bg-cyan-700 disabled:opacity-50 text-sm">🖱 {t('panels.browser.clickElement')}</button>
                  <button onClick={fillInputElement} disabled={!selectorInput.trim() || !valueInput.trim()}
                    className="py-2 bg-teal-600 text-white rounded-lg hover:bg-teal-700 disabled:opacity-50 text-sm">✏️ {t('panels.browser.fillInput')}</button>
                </div>
              </div>
            )}
          </>
        )}
      </div>
    </div>
  )
}
