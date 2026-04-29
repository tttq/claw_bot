// Claw Desktop - MCP配置面板 - 管理Model Context Protocol服务端注册和配置

import { useState, useEffect } from 'react'
import { useTranslation } from 'react-i18next'
import { toolConfig } from '../../../api/tools'

interface McpServer {
  id: string
  name: string
  type: 'stdio' | 'sse' | 'streamable-http'
  command?: string
  args?: string[]
  url?: string
  headers?: Record<string, string>
  enabled: boolean
  env?: Record<string, string>
}

const DEFAULT_SERVERS: McpServer[] = [
  { id: 'filesystem', name: 'Filesystem MCP', type: 'stdio', command: 'npx', args: ['-y', '@modelcontextprotocol/server-filesystem', '/path/to/allowed/dir'], enabled: false },
  { id: 'memory', name: 'Knowledge Graph MCP', type: 'stdio', command: 'npx', args: ['-y', '@modelcontextprotocol/server-memory'], enabled: false },
  { id: 'brave-search', name: 'Brave Search MCP', type: 'stdio', command: 'npx', args: ['-y', '@anthropic-ai/mcp-server-brave-search'], env: { BRAVE_API_KEY: '' }, enabled: false },
]

export default function McpConfig({ agentId }: { agentId?: string }) {
  const { t } = useTranslation()
  const [servers, setServers] = useState<McpServer[]>(DEFAULT_SERVERS)
  const [editingServer, setEditingServer] = useState<McpServer | null>(null)
  const [isCreating, setIsCreating] = useState(false)
  const [activeTab, setActiveTab] = useState<'servers' | 'tools'>('servers')
  const [toast, setToast] = useState<string | null>(null)

  useEffect(() => { loadServers() }, [])
  const showToast = (msg: string) => { setToast(msg); setTimeout(() => setToast(null), 2500) }

  const loadServers = async () => {
    try {
      const result = await toolConfig({ action: 'get', key: 'mcp_servers' }) as unknown as string
      if (result && result !== 'Available:\n...') { try { setServers(JSON.parse(result)) } catch (e) { console.error(e) } }
    } catch (e) { console.error(e) }
  }

  const handleSave = async () => {
    if (!editingServer) return
    const updated = editingServer.id ? servers.map(s => s.id === editingServer.id ? editingServer : editingServer) : [...servers, { ...editingServer, id: crypto.randomUUID() }]
    setServers(updated as McpServer[])
    try { await toolConfig({ action: 'set', key: 'mcp_servers', value: JSON.stringify(updated) }); showToast(editingServer.id ? t('panels.mcp.server_updated') : t('panels.mcp.server_added')) }
    catch { showToast(t('panels.mcp.save_failed')) }
    setEditingServer(null); setIsCreating(false)
  }

  const handleToggle = async (id: string) => {
    const updated = servers.map(s => s.id === id ? { ...s, enabled: !s.enabled } : s)
    setServers(updated)
    try { await toolConfig({ action: 'set', key: 'mcp_servers', value: JSON.stringify(updated) }) } catch (e) { console.error(e) }
  }

  const handleDelete = async (id: string) => {
    const updated = servers.filter(s => s.id !== id)
    setServers(updated)
    try { await toolConfig({ action: 'set', key: 'mcp_servers', value: JSON.stringify(updated) }); showToast(t('panels.mcp.deleted')) } catch (e) { console.error(e) }
  }

  return (
    <div className="space-y-5">
      <div className="flex items-center justify-between">
        <div>
          <h3 className="text-base font-semibold text-dark-text flex items-center gap-2">
            <svg className="w-5 h-5 text-cyan-400" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={2}><path strokeLinecap="round" strokeLinejoin="round" d="M8 9l3 3-3 3m5 0h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z"/></svg>
            {t('panels.mcp.title')}
          </h3>
          <p className="text-xs text-dark-muted mt-0.5">{t('panels.mcp.description')}</p>
        </div>
        <button onClick={() => { setIsCreating(true); setEditingServer({ id: '', name: '', type: 'stdio', command: '', args: [], enabled: true }) }} className="px-3 py-1.5 rounded-lg bg-primary-600 hover:bg-primary-500 text-white text-xs font-medium transition-colors flex items-center gap-1.5">
          <svg className="w-3.5 h-3.5" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={2}><path strokeLinecap="round" strokeLinejoin="round" d="M12 4v16m8-8H4"/></svg>{t('panels.mcp.add_server')}
        </button>
      </div>

      {toast && <div className="px-3 py-2 rounded-lg bg-primary-600/10 border border-primary-500/20 text-xs text-primary-300">{toast}</div>}

      <div className="flex gap-1 p-1 rounded-lg bg-dark-bg border border-dark-border w-fit">
        <button onClick={() => setActiveTab('servers')} className={`px-3 py-1.5 rounded-md text-xs transition-colors ${activeTab === 'servers' ? 'bg-primary-600 text-white' : 'text-dark-muted hover:text-dark-text'}`}>{t('panels.mcp.tab_servers')}</button>
        <button onClick={() => setActiveTab('tools')} className={`px-3 py-1.5 rounded-md text-xs transition-colors ${activeTab === 'tools' ? 'bg-primary-600 text-white' : 'text-dark-muted hover:text-dark-text'}`}>{t('panels.mcp.tab_tools')}</button>
      </div>

      {activeTab === 'servers' && (
        <div className="space-y-2">
          {servers.map(server => (
            <div key={server.id} className={`p-4 rounded-xl border transition-all ${server.enabled ? 'bg-dark-bg border-dark-border' : 'bg-dark-bg/50 border-dark-border/50 opacity-60'}`}>
              <div className="flex items-start justify-between mb-2">
                <div className="flex-1 min-w-0">
                  <div className="flex items-center gap-2 mb-1">
                    <span className={`w-7 h-7 rounded-lg flex items-center justify-center text-xs font-bold ${server.type === 'stdio' ? 'bg-blue-500/10 text-blue-400' : server.type === 'sse' ? 'bg-green-500/10 text-green-400' : 'bg-purple-500/10 text-purple-400'}`}>{server.type === 'stdio' ? 'CLI' : server.type === 'sse' ? 'SSE' : 'HTTP'}</span>
                    <span className="text-sm font-semibold text-dark-text truncate">{server.name}</span>
                    <span className={`px-1.5 py-0.5 rounded text-[10px] ${server.enabled ? 'bg-green-500/10 text-green-400' : 'bg-dark-border text-dark-muted'}`}>{server.enabled ? t('panels.mcp.running') : t('panels.mcp.stopped')}</span>
                  </div>
                  <div className="text-[11px] text-dark-muted font-mono truncate">{server.type === 'stdio' ? `${server.command || ''} ${(server.args || []).join(' ')}` : server.url}</div>
                </div>
                <div className="flex items-center gap-1.5 ml-3 shrink-0">
                  <button onClick={() => { setEditingServer({ ...server }); setIsCreating(false) }} className="p-1.5 rounded-lg hover:bg-dark-surface text-dark-muted hover:text-primary-400 transition-colors" title={t('panels.mcp.edit')}>
                    <svg className="w-3.5 h-3.5" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={2}><path strokeLinecap="round" strokeLinejoin="round" d="M11 5H6a2 2 0 00-2 2v11a2 2 0 002 2h11a2 2 0 002-2v-5m-1.414-9.414a2 2 0 112.828 2.828L11.828 15H9v-2.828l8.586-8.586z"/></svg>
                  </button>
                  <button onClick={() => handleToggle(server.id)} className={`p-1.5 rounded-lg transition-colors ${server.enabled ? 'hover:bg-red-500/10 text-dark-muted hover:text-red-400' : 'hover:bg-green-500/10 text-dark-muted hover:text-green-400'}`} title={server.enabled ? t('panels.mcp.stop') : t('panels.mcp.start')}>
                    <svg className="w-3.5 h-3.5" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={2}><path strokeLinecap="round" strokeLinejoin="round" d={server.enabled ? 'M18.364 18.364A9 9 0 005.636 5.636m12.728 12.728A9 9 0 015.636 5.636m12.728 12.728L5.636 5.636' : 'M5.636 5.636a9 9 0 0112.728 12.728m0-12.728l-12.728 12.728'} /></svg>
                  </button>
                  <button onClick={() => handleDelete(server.id)} className="p-1.5 rounded-lg hover:bg-red-500/10 text-dark-muted hover:text-red-400 transition-colors" title={t('panels.mcp.delete')}>
                    <svg className="w-3.5 h-3.5" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={2}><path strokeLinecap="round" strokeLinejoin="round" d="M19 7l-.867 12.142A2 2 0 0116.138 21H7.862a2 2 0 01-1.995-1.858L5 7m5 4v6m4-6v6m1-10V4a1 1 0 00-1-1h-4a1 1 0 00-1 1v3M4 7h16"/></svg>
                  </button>
                </div>
              </div>
            </div>
          ))}
        </div>
      )}

      {activeTab === 'tools' && (
        <div className="rounded-xl border border-dark-border p-4 space-y-3">
          <p className="text-sm text-dark-text">{t('panels.mcp.tools_info')}</p>
          <div className="grid grid-cols-2 gap-2">
            {['filesystem.read_file', 'filesystem.write_file', 'filesystem.list_directory', 'memory.search_graph', 'memory.add_knowledge', 'brave.web_search'].map(tool => (
              <div key={tool} className="p-3 rounded-lg bg-dark-bg border border-dashed border-dark-border text-center"><span className="text-xs text-dark-muted font-mono">{tool}</span></div>
            ))}
          </div>
        </div>
      )}

      {editingServer && (
        <div className="fixed inset-0 z-[60] flex items-center justify-center bg-black/50 backdrop-blur-sm" onClick={() => { setEditingServer(null); setIsCreating(false) }}>
          <div className="bg-dark-surface border border-dark-border rounded-2xl shadow-2xl w-[580px] max-h-[85vh] overflow-y-auto p-6 animate-fade-in" onClick={e => e.stopPropagation()}>
            <h3 className="text-base font-bold text-dark-text mb-4">{isCreating ? t('panels.mcp.add_modal_title') : t('panels.mcp.edit_modal_title')}</h3>
            <div className="space-y-4">
              <div><label className="block text-xs font-medium text-dark-text mb-1">{t('panels.mcp.name_label')} *</label><input value={editingServer.name} onChange={e => setEditingServer(s => s ? { ...s, name: e.target.value } : null)} placeholder={t('panels.mcp.name_placeholder')} className="w-full bg-dark-bg border border-dark-border rounded-lg px-3 py-2 text-sm text-dark-text focus:outline-none focus:border-primary-500" /></div>
              <div className="grid grid-cols-2 gap-3">
                <div><label className="block text-xs font-medium text-dark-text mb-1">{t('panels.mcp.type_label')} *</label>
                  <select value={editingServer.type} onChange={e => setEditingServer(s => s ? { ...s, type: e.target.value as 'stdio' | 'sse' | 'streamable-http' } : null)} className="w-full bg-dark-bg border border-dark-border rounded-lg px-3 py-2 text-sm text-dark-text focus:outline-none focus:border-primary-500">
                    <option value="stdio">STDIO</option><option value="sse">SSE</option><option value="streamable-http">Streamable HTTP</option>
                  </select>
                </div>
                <div className="flex items-end pb-2"><label className="flex items-center gap-2 cursor-pointer"><div onClick={() => setEditingServer(s => s ? { ...s, enabled: !s.enabled } : null)} className={`relative w-9 h-5 rounded-full transition-colors ${editingServer?.enabled ? 'bg-primary-600' : 'bg-dark-border'}`}><span className={`absolute top-0.5 w-4 h-4 rounded-full bg-white shadow transition-transform ${editingServer?.enabled ? 'translate-x-4' : 'translate-x-0.5'}`} /></div><span className="text-xs text-dark-text">{t('panels.mcp.enable_label')}</span></label></div>
              </div>
              {editingServer.type === 'stdio' ? (
                <><div><label className="block text-xs font-medium text-dark-text mb-1">{t('panels.mcp.command_label')} *</label><input value={editingServer.command || ''} onChange={e => setEditingServer(s => s ? { ...s, command: e.target.value } : null)} placeholder="npx / node / python" className="w-full bg-dark-bg border border-dark-border rounded-lg px-3 py-2 text-sm text-dark-text focus:outline-none focus:border-primary-500 font-mono" /></div>
                <div><label className="block text-xs font-medium text-dark-text mb-1">{t('panels.mcp.args_label')}</label><input value={(editingServer.args || []).join(' ')} onChange={e => setEditingServer(s => s ? { ...s, args: e.target.value.split(/\s+/).filter(Boolean) } : null)} placeholder="--arg1 --arg2" className="w-full bg-dark-bg border border-dark-border rounded-lg px-3 py-2 text-sm text-dark-text focus:outline-none focus:border-primary-500 font-mono" /></div></>
              ) : (<div><label className="block text-xs font-medium text-dark-text mb-1">{t('panels.mcp.url_label')} *</label><input value={editingServer.url || ''} onChange={e => setEditingServer(s => s ? { ...s, url: e.target.value } : null)} placeholder={t('panels.mcp.url_placeholder')} className="w-full bg-dark-bg border border-dark-border rounded-lg px-3 py-2 text-sm text-dark-text focus:outline-none focus:border-primary-500 font-mono" /></div>)}
              <div className="flex justify-end gap-2 pt-3 border-t border-dark-border/50">
                <button onClick={() => { setEditingServer(null); setIsCreating(false) }} className="px-4 py-2 rounded-lg border border-dark-border text-sm text-dark-muted hover:text-dark-text hover:bg-dark-border/30 transition-colors">{t('panels.mcp.cancel_btn')}</button>
                <button onClick={handleSave} disabled={!editingServer.name.trim()} className="px-4 py-2 rounded-lg bg-primary-600 hover:bg-primary-500 text-white text-sm font-medium transition-colors disabled:opacity-40">{t('panels.mcp.save_btn')}</button>
              </div>
            </div>
          </div>
        </div>
      )}
    </div>
  )
}
