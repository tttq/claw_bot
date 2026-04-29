// Claw Desktop - Web搜索面板 - 配置搜索引擎参数和搜索策略
// 对应 WebSearchTool + WebFetch，提供可视化搜索界面

import { useState, useEffect } from 'react'
import { useTranslation } from 'react-i18next'
import { toolWebSearch, toolWebFetch } from '../../../api/tools'
import { isoGetConfig, isoSetConfig } from '../../../api/iso'

interface SearchResult { title: string; url: string; snippet: string }

const SEARCH_ENGINES = [
  { id: 'google', name: 'Google', icon: '🔍' },
  { id: 'bing', name: 'Bing', icon: '🔎' },
  { id: 'duckduckgo', name: 'DuckDuckGo', icon: '🦆' },
  { id: 'brave', name: 'Brave', icon: '🦁' },
]

export default function WebSearchPanel({ agentId }: { agentId?: string }) {
  const { t } = useTranslation()
  const [query, setQuery] = useState('')
  const [engine, setEngine] = useState('google')
  const [results, setResults] = useState<SearchResult[]>([])
  const [loading, setLoading] = useState(false)
  const [fetchUrl, setFetchUrl] = useState('')
  const [fetchedContent, setFetchedContent] = useState('')
  const [activeTab, setActiveTab] = useState<'search' | 'fetch'>('search')
  const [history, setHistory] = useState<Array<{ query: string; time: number; count: number }>>([])

  useEffect(()=>{
    if(agentId){
      isoGetConfig({agentId,key:'web_search_config'}).then((resp:any)=>{
        if(resp?.value){
          try{
            const parsed=typeof resp.value==='string'?JSON.parse(resp.value):resp.value
            if(parsed.engine) setEngine(parsed.engine)
          }catch(e){console.error('[WebSearchPanel] Failed to parse web search config:', e)}
        }
      }).catch((e) => { console.error(e) })
    }
  },[agentId])

  const saveEngine=async(eng:string)=>{
    setEngine(eng)
    if(agentId){
      isoSetConfig({agentId,key:'web_search_config',value:JSON.stringify({engine:eng})}).catch((e) => { console.error(e) })
    }
  }

  const handleSearch = async () => {
    if (!query.trim()) return
    setLoading(true); setResults([])
    try {
      const data = await toolWebSearch({ query }) as unknown as { output?: string }
      if (data.output) {
        const parsed: SearchResult[] = []
        const lines = data.output.split('\n')
        let current: Partial<SearchResult> = {}
        for (const line of lines) {
          if (line.startsWith('Title:')) current.title = line.replace('Title:', '').trim()
          else if (line.startsWith('URL:')) current.url = line.replace('URL:', '').trim()
          else if (line.startsWith('Snippet:') || line.trim()) {
            current.snippet = (current.snippet || '') + ' ' + (line.replace('Snippet:', '').trim())
            if (current.title && current.url && current.snippet) {
              parsed.push(current as SearchResult)
              current = {}
            }
          }
        }
        setResults(parsed.length > 0 ? parsed : [{ title: query, url: '', snippet: data.output.substring(0, 500) }])
        setHistory(prev => [{ query, time: Date.now(), count: results.length }, ...prev.slice(0, 9)])
      }
    } catch (e) { setResults([{ title: 'Error', url: '', snippet: String(e) }]) }
    finally { setLoading(false) }
  }

  const handleFetch = async () => {
    if (!fetchUrl.trim()) return
    setLoading(true); setFetchedContent('')
    try {
      const data = await toolWebFetch({ url: fetchUrl }) as unknown as { output?: string }
      setFetchedContent(data.output || 'No content retrieved')
    } catch (e) { setFetchedContent(`Error: ${e}`) }
    finally { setLoading(false) }
  }

  return (
    <div className="space-y-4">
      {/* Tab switcher */}
      <div className="flex gap-1 p-1 rounded-lg bg-dark-bg border border-dark-border w-fit">
        <button onClick={() => setActiveTab('search')} className={`px-3 py-1.5 rounded-md text-xs transition-colors flex items-center gap-1.5 ${activeTab === 'search' ? 'bg-primary-600 text-white' : 'text-dark-muted hover:text-dark-text'}`}>
          🔍 {t('panels.web_search.search')}
        </button>
        <button onClick={() => setActiveTab('fetch')} className={`px-3 py-1.5 rounded-md text-xs transition-colors flex items-center gap-1.5 ${activeTab === 'fetch' ? 'bg-primary-600 text-white' : 'text-dark-muted hover:text-dark-text'}`}>
          🌐 {t('panels.web_search.fetch')}
        </button>
      </div>

      {/* Search tab */}
      {activeTab === 'search' && (
        <>
          {/* Search input */}
          <div className="space-y-2">
            <div className="flex gap-2">
              <input value={query} onChange={e => setQuery(e.target.value)} onKeyDown={e => e.key === 'Enter' && handleSearch()}
                placeholder={t('panels.web_search.search_placeholder')} className="flex-1 bg-dark-bg border border-dark-border rounded-lg px-3 py-2 text-sm text-dark-text outline-none focus:border-primary-500" />
              <button onClick={handleSearch} disabled={loading || !query.trim()} className="px-4 py-2 rounded-lg bg-primary-600 hover:bg-primary-500 text-white text-sm font-medium disabled:opacity-40 transition-colors flex items-center gap-1.5">
                {loading ? <div className="w-4 h-4 border-2 border-white/30 border-t-transparent rounded-full animate-spin" /> : t('panels.web_search.search_btn')}
              </button>
            </div>

            {/* Engine selector */}
            <div className="flex gap-1">
              {SEARCH_ENGINES.map(e => (
                <button key={e.id} onClick={() => saveEngine(e.id)} className={`px-2 py-1 rounded text-[11px] transition-colors flex items-center gap-1 ${engine === e.id ? 'bg-primary-600/10 text-primary-300 border border-primary-500/20' : 'border border-dark-border text-dark-muted hover:text-dark-text'}`}>
                  {e.icon} {e.name}
                </button>
              ))}
            </div>
          </div>

          {/* Search history */}
          {history.length > 0 && (
            <div className="flex items-center gap-2">
              <span className="text-[10px] text-dark-muted">{t('panels.web_search.recent')}:</span>
              <div className="flex gap-1 flex-wrap">{history.slice(0, 5).map((h, i) => (
                <button key={i} onClick={() => { setQuery(h.query); handleSearch() }} className="px-1.5 py-0.5 rounded bg-dark-border/50 text-[10px] text-dark-muted hover:text-primary-300 hover:bg-primary-600/10 transition-colors truncate max-w-[120px]">{h.query}</button>
              ))}</div>
            </div>
          )}

          {/* Results */}
          <div className="rounded-xl border border-dark-border overflow-hidden">
            {loading ? (
              <div className="flex justify-center py-8"><div className="w-6 h-6 border-2 border-primary-500 border-t-transparent rounded-full animate-spin"></div></div>
            ) : results.length === 0 ? (
              <div className="text-center py-8 text-sm text-dark-muted">{t('panels.web_search.start_search')}</div>
            ) : (
              <div className="divide-y divide-dark-border max-h-[340px] overflow-y-auto">
                {results.map((r, i) => (
                  <a key={i} href={r.url || '#'} target="_blank" rel="noopener noreferrer" className="block px-4 py-3 hover:bg-dark-bg transition-colors group">
                    <p className="text-sm font-medium text-blue-400 group-hover:underline truncate">{r.title || t('panels.web_search.no_title')}</p>
                    {r.url && <p className="text-[10px] text-green-400/60 font-mono mt-0.5 truncate">{r.url}</p>}
                    <p className="text-xs text-dark-muted mt-1 line-clamp-2">{r.snippet || ''}</p>
                  </a>
                ))}
              </div>
            )}
          </div>
        </>
      )}

      {/* Fetch tab */}
      {activeTab === 'fetch' && (
        <div className="space-y-3">
          <div className="flex gap-2">
            <input value={fetchUrl} onChange={e => setFetchUrl(e.target.value)} onKeyDown={e => e.key === 'Enter' && handleFetch()}
              placeholder={t('panels.web_search.fetch_url_placeholder')} className="flex-1 bg-dark-bg border border-dark-border rounded-lg px-3 py-2 text-sm text-dark-text outline-none focus:border-primary-500 font-mono text-xs" />
            <button onClick={handleFetch} disabled={loading || !fetchUrl.trim()} className="px-4 py-2 rounded-lg bg-blue-600 hover:bg-blue-500 text-white text-sm font-medium disabled:opacity-40 transition-colors">{t('panels.web_search.fetch_btn')}</button>
          </div>

          {/* Fetched content */}
          <div className="rounded-xl border border-dark-border overflow-hidden">
            {loading ? (
              <div className="flex justify-center py-8"><div className="w-6 h-6 border-2 border-blue-500 border-t-transparent rounded-full animate-spin"></div></div>
            ) : fetchedContent ? (
              <div className="max-h-[380px] overflow-y-auto p-4">
                {fetchUrl && <a href={fetchUrl} target="_blank" rel="noopener noreferrer" className="text-xs text-blue-400 hover:underline font-mono mb-2 block break-all">{fetchUrl}</a>}
                <pre className="text-[11px] text-dark-text whitespace-pre-wrap break-words leading-relaxed">{fetchedContent}</pre>
              </div>
            ) : (
              <div className="text-center py-8 text-sm text-dark-muted">{t('panels.web_search.fetch_content_placeholder')}</div>
            )}
          </div>
        </div>
      )}
    </div>
  )
}
