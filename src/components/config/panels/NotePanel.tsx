// Claw Desktop - 笔记面板 - 管理Agent工作区笔记的创建和编辑
// AI辅助笔记管理：创建/编辑/搜索/分类/Markdown预览

import { useState, useEffect } from 'react'
import { useTranslation } from 'react-i18next'
import { isoSetConfig, isoGetConfig } from '../../../api/iso'

interface Note {
  id: string
  title: string
  content: string
  tags: string[]
  createdAt: number
  updatedAt: number
}

export default function NotePanel({ agentId }: { agentId?: string }) {
  const { t } = useTranslation()
  const [notes, setNotes] = useState<Note[]>([])
  const [selectedNote, setSelectedNote] = useState<Note | null>(null)
  const [editingContent, setEditingContent] = useState('')
  const [searchQuery, setSearchQuery] = useState('')
  const [showPreview, setShowPreview] = useState(false)

  useEffect(() => { loadNotes() }, [agentId])

  const loadNotes = async () => {
    try {
      const configKey = agentId ? `agent_notes_${agentId}` : 'notes'
      const result = await isoGetConfig({ agentId: agentId || '', key: configKey }) as unknown as string
      let loadedNotes: Note[] = []
      if (result && typeof result === 'string' && !result.startsWith('Available') && result !== 'null' && result !== 'undefined') {
        try { loadedNotes = JSON.parse(result) } catch (e) { console.error(e) }
      }
      if (loadedNotes.length > 0) { setNotes(loadedNotes) }
      else {
        const defaultTitle = agentId ? `${agentId} Notebook` : 'Welcome to Claw Desktop'
        const defaults: Note[] = [
          { id: '1', title: defaultTitle, content: `# Claw Desktop Notebook\n\nThis is an AI-assisted note management system.\n\n## Features\n- Markdown editing and preview\n- Tag classification\n- Full-text search\n- AI summary`, tags: ['welcome'], createdAt: Date.now(), updatedAt: Date.now() },
          { id: '2', title: 'Project Config Memo', content: '## API Config\n\n- Provider: anthropic\n- Model: claude-sonnet-4-20250514\n- Base URL: https://api.anthropic.com\n\n## Shortcuts\n`Ctrl+Enter` send message', tags: ['config'], createdAt: Date.now(), updatedAt: Date.now() },
        ]
        setNotes(defaults)
      }
    } catch (e) { console.error(e) }
  }

  const handleSaveNote = async () => {
    if (!selectedNote) return
    const updated = { ...selectedNote, content: editingContent, updatedAt: Date.now() }
    setNotes(prev => prev.map(n => n.id === updated.id ? updated : n))
    setSelectedNote(updated)
    try {
      const configKey = agentId ? `agent_notes_${agentId}` : 'notes'
      await isoSetConfig({ agentId: agentId || '', key: configKey, value: JSON.stringify(notes.map(n => n.id === updated.id ? updated : n)) })
    } catch (e) { console.error(e) }
  }

  const handleNewNote = () => {
    const newNote: Note = { id: crypto.randomUUID(), title: t('panels.note_panel.default_title'), content: '', tags: [], createdAt: Date.now(), updatedAt: Date.now() }
    setNotes([newNote, ...notes]); setSelectedNote(newNote); setEditingContent('')
  }

  const handleDeleteNote = (id: string) => {
    setNotes(prev => prev.filter(n => n.id !== id))
    if (selectedNote?.id === id) { setSelectedNote(null); setEditingContent('') }
  }

  const filteredNotes = searchQuery ? notes.filter(n =>
    n.title.toLowerCase().includes(searchQuery.toLowerCase()) ||
    n.content.toLowerCase().includes(searchQuery.toLowerCase()) ||
    n.tags.some(t => t.toLowerCase().includes(searchQuery.toLowerCase()))
  ) : notes

  return (
    <div className="flex gap-3 h-[520px]">
      <div className="w-[220px] shrink-0 flex flex-col rounded-xl border border-dark-border overflow-hidden">
        <div className="p-2 border-b border-dark-border space-y-1.5">
          <button onClick={handleNewNote} className="w-full px-2.5 py-1.5 rounded-lg bg-primary-600 hover:bg-primary-500 text-white text-xs font-medium transition-colors flex items-center justify-center gap-1.5">
            <svg className="w-3.5 h-3.5" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={2}><path strokeLinecap="round" strokeLinejoin="round" d="M12 4v16m8-8H4"/></svg>{t('panels.note_panel.new_note')}
          </button>
          <input value={searchQuery} onChange={e => setSearchQuery(e.target.value)} placeholder={t('panels.note_panel.search_placeholder')} className="w-full bg-dark-bg border border-dark-border rounded-lg px-2.5 py-1 text-[11px] text-dark-text outline-none focus:border-primary-500" />
        </div>
        <div className="flex-1 overflow-y-auto divide-y divide-dark-border/50 p-1">
          {filteredNotes.map(note => (
            <button key={note.id} onClick={() => { setSelectedNote(note); setEditingContent(note.content) }} className={`w-full text-left px-2 py-2 rounded-lg transition-colors group ${selectedNote?.id === note.id ? 'bg-primary-600/10 border border-primary-500/20' : 'hover:bg-dark-bg'}`}>
              <p className="text-xs font-medium text-dark-text truncate">{note.title}</p>
              <div className="flex items-center gap-1 mt-1">
                {(note.tags || []).slice(0, 2).map(tg => <span key={tg} className="text-[9px] px-1 py-px rounded bg-primary-600/10 text-primary-300">{tg}</span>)}
                <span className="text-[9px] text-dark-muted ml-auto">{new Date(note.updatedAt).toLocaleDateString()}</span>
              </div>
            </button>
          ))}
          {filteredNotes.length === 0 && <div className="text-center py-6 text-[11px] text-dark-muted">{t('panels.note_panel.no_match')}</div>}
        </div>
      </div>

      <div className="flex-1 flex flex-col rounded-xl border border-dark-border overflow-hidden">
        {selectedNote ? (
          <>
            <div className="flex items-center justify-between px-4 py-2 border-b border-dark-border bg-dark-surface">
              <input value={selectedNote.title} onChange={e => setSelectedNote({ ...selectedNote, title: e.target.value })} className="bg-transparent text-sm font-semibold text-dark-text outline-none" />
              <div className="flex items-center gap-1.5">
                <button onClick={() => setShowPreview(!showPreview)} className={`px-2 py-1 rounded text-[10px] transition-colors ${showPreview ? 'bg-primary-600 text-white' : 'border border-dark-border text-dark-muted hover:text-dark-text'}`}>
                  {showPreview ? t('panels.note_panel.edit') : t('panels.note_panel.preview')}
                </button>
                <button onClick={handleSaveNote} className="px-2 py-1 rounded bg-green-600/10 text-green-400 text-[10px] hover:bg-green-600/20 transition-colors">{t('panels.note_panel.save')}</button>
                <button onClick={() => handleDeleteNote(selectedNote.id)} className="p-1 rounded hover:bg-red-500/10 text-dark-muted hover:text-red-400 transition-colors"><svg className="w-3.5 h-3.5" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={2}><path strokeLinecap="round" strokeLinejoin="round" d="M19 7l-.867 12.142A2 2 0 0116.138 21H7.862a2 2 0 01-1.995-1.858L5 7m5 4v6m4-6v6m1-10V4a1 1 0 00-1-1h-4a1 1 0 00-1 1v3M4 7h16"/></svg></button>
              </div>
            </div>
            {!showPreview ? (
              <textarea value={editingContent} onChange={e => setEditingContent(e.target.value)}
                placeholder={t('panels.note_panel.write_placeholder')}
                className="flex-1 w-full bg-transparent text-sm text-dark-text outline-none resize-none p-4 placeholder-dark-muted/30 leading-relaxed font-mono" />
            ) : (
              <div className="flex-1 overflow-y-auto p-4 prose prose-invert prose-sm max-w-none text-sm text-dark-text leading-relaxed">
                {editingContent.split('\n').map((line, i) => {
                  if (line.startsWith('# ')) return <h1 key={i} className="text-lg font-bold">{line.slice(2)}</h1>
                  if (line.startsWith('## ')) return <h2 key={i} className="text-base font-bold mt-3 mb-1">{line.slice(3)}</h2>
                  if (line.startsWith('- ')) return <li key={i} className="ml-4 list-disc">{line.slice(2)}</li>
                  if (line.startsWith('`')) return <code key={i} className="px-1.5 py-0.5 rounded bg-dark-surface text-primary-300 text-xs font-mono">{line.replace(/`/g, '')}</code>
                  if (!line) return <br key={i} />
                  return <p key={i}>{line}</p>
                })}
              </div>
            )}
          </>
        ) : (
          <div className="flex-1 flex items-center justify-center">
            <div className="text-center">
              <div className="w-12 h-12 rounded-xl bg-dark-bg flex items-center justify-center mx-auto mb-3">
                <svg className="w-6 h-6 text-dark-muted" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={1.5}><path strokeLinecap="round" strokeLinejoin="round" d="M11 5H6a2 2 0 00-2 2v11a2 2 0 002 2h11a2 2 0 002-2v-5m-1.414-9.414a2 2 0 112.828 2.828L11.828 15H9v-2.828l8.586-8.586z"/></svg>
              </div>
              <p className="text-sm text-dark-text">{t('panels.note_panel.select_or_create')}</p>
              <p className="text-xs text-dark-muted mt-1">{t('panels.note_panel.markdown_support')}</p>
            </div>
          </div>
        )}
      </div>
    </div>
  )
}
