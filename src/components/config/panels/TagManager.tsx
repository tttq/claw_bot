// Claw Desktop - 标签管理器 - 管理会话和消息的标签分类与颜色
// 标签增删查改、搜索过滤

import { useState, useEffect } from 'react'
import { useTranslation } from 'react-i18next'
import { toolTagAdd, toolTagDelete, toolTagList } from '../../../api/tools'

interface Tag {
  id: string
  name: string
  color: string
  count?: number
}

const TAG_COLORS = [
  '#ef4444', '#f97316', '#eab308', '#22c55e',
  '#14b8a6', '#3b82f6', '#8b5cf6', '#ec4899',
  '#6366f1', '#06b6d4',
]

export default function TagManager({ agentId }: { agentId?: string }) {
  const { t } = useTranslation()
  const [tags, setTags] = useState<Tag[]>([])
  const [newTagName, setNewTagName] = useState('')
  const [selectedColor, setSelectedColor] = useState(TAG_COLORS[0])
  const [filter, setFilter] = useState('')
  const [loading, setLoading] = useState(false)

  useEffect(() => { loadTags() }, [])

  const loadTags = async () => { try { const data = await toolTagList() as unknown as { tags?: Array<{ name: string; color?: string }> }; if (Array.isArray(data?.tags)) setTags(data.tags.map((t, i) => ({ id: t.name || String(i), name: t.name, color: t.color || '' }))) } catch (e) { console.error(e) } }

  const handleAddTag = async () => {
    if (!newTagName.trim()) return; setLoading(true)
    try { await toolTagAdd({ name: newTagName.trim(), color: selectedColor }); setNewTagName(''); loadTags() }
    catch (e) { console.error('[TagManager] Failed to add tag:', e) } finally { setLoading(false) }
  }

  const handleDeleteTag = async (name: string) => {
    try { await toolTagDelete({ name }); loadTags() }
    catch (e) { console.error('[TagManager] Failed to delete tag:', e) }
  }

  const filteredTags = filter ? tags.filter(tg => tg.name.toLowerCase().includes(filter.toLowerCase())) : tags

  return (
    <div className="space-y-4">
      {/* Header */}
      <div className="flex items-center justify-between">
        <h3 className="text-base font-semibold text-dark-text flex items-center gap-2">
          <svg className="w-5 h-5 text-pink-400" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={2}><path strokeLinecap="round" strokeLinejoin="round" d="M7 7h.01M7 3h5c.512 0 1.024.195 1.414.586l7 7a2 2 0 010 2.828l-7 7a2 2 0 01-2.828 0l-7-7A1.994 1.994 0 013 12V7a4 4 0 014-4z"/></svg>
          {t('panels.tag_manager.title')}
        </h3>
        <span className="text-xs px-2 py-0.5 rounded-full bg-dark-bg border border-dark-border text-dark-muted">{tags.length} {t('panels.tag_manager.tags_count')}</span>
      </div>

      {/* Add new */}
      <div className="flex gap-2">
        <input value={newTagName} onChange={e => setNewTagName(e.target.value)} onKeyDown={e => e.key === 'Enter' && handleAddTag()} placeholder={t('panels.tag_manager.new_tag_placeholder')} className="flex-1 bg-dark-bg border border-dark-border rounded-lg px-3 py-2 text-xs text-dark-text outline-none focus:border-pink-500" />
        <button onClick={handleAddTag} disabled={!newTagName.trim() || loading} className="px-3 py-2 rounded-lg bg-pink-600 hover:bg-pink-500 text-white text-xs font-medium disabled:opacity-40 transition-colors flex items-center gap-1">
          {loading ? <div className="w-3 h-3 border-2 border-white/30 border-t-transparent rounded-full animate-spin" /> : t('panels.tag_manager.add')}
        </button>
      </div>

      {/* Color picker */}
      <div className="flex gap-1.5 flex-wrap">{TAG_COLORS.map(c => (
        <button key={c} onClick={() => setSelectedColor(c)} className={`w-6 h-6 rounded-full border-2 transition-all ${selectedColor === c ? 'border-white scale-110' : 'border-transparent'}`} style={{ backgroundColor: c }} title={`Select color ${c}`} />
      ))}</div>

      {/* Filter */}
      <input value={filter} onChange={e => setFilter(e.target.value)} placeholder={t('panels.tag_manager.search_placeholder')} className="w-full bg-dark-bg border border-dark-border rounded-lg px-3 py-1.5 text-xs text-dark-text outline-none focus:border-primary-500" />

      {/* Tags list */}
      <div className="space-y-1.5 max-h-[300px] overflow-y-auto">
        {filteredTags.map(tag => (
          <div key={tag.id} className="group flex items-center justify-between p-2.5 rounded-lg bg-dark-bg border border-dark-border hover:border-pink-500/30 transition-colors">
            <div className="flex items-center gap-2.5 min-w-0">
              <span className="w-3 h-3 rounded-full shrink-0" style={{ backgroundColor: tag.color || TAG_COLORS[0] }} />
              <span className="text-sm font-medium text-dark-text truncate">{tag.name}</span>
              {tag.count !== undefined && <span className="text-[10px] text-dark-muted">({tag.count})</span>}
            </div>
            <button onClick={() => handleDeleteTag(tag.name)} className="opacity-0 group-hover:opacity-100 p-1 rounded hover:bg-red-500/10 text-dark-muted hover:text-red-400 transition-all shrink-0">
              <svg className="w-3.5 h-3.5" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={2}><path strokeLinecap="round" strokeLinejoin="round" d="M19 7l-.867 12.142A2 2 0 0116.138 21H7.862a2 2 0 01-1.995-1.858L5 7m5 4v6m4-6v6m1-10V4a1 1 0 00-1-1h-4a1 1 0 00-1 1v3M4 7h16"/></svg>
            </button>
          </div>
        ))}
        {filteredTags.length === 0 && <div className="text-center py-8 text-sm text-dark-muted">{tags.length === 0 ? t('panels.tag_manager.no_tags') : t('panels.tag_manager.no_results')}</div>}
      </div>
    </div>
  )
}
