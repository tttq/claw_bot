// Claw Desktop - @提及输入框组件 - 支持输入@符号弹出Agent列表、选择后插入提及标记
import { useState, useRef, useCallback, useEffect, useMemo } from 'react'
import { useTranslation } from 'react-i18next'
import { getMentionAtPosition, parseMentions } from '../../multiagent/mentionParser'
import type { AgentRegistryEntry } from '../../multiagent/types'
import { BUILT_IN_AGENTS } from '../../multiagent/agentRegistry'
import type { Message } from '../../types'

interface AttachedFile {
  name: string
  type: string
  dataUrl: string
  size: number
}

interface MentionInputProps {
  onSendMessage: (content: string, attachments?: AttachedFile[], mentionedAgentIds?: string[]) => void
  onStopGeneration?: () => void
  isLoading: boolean
  disabled?: boolean
  customAgents?: Array<{ id: string; displayName: string; description?: string; purpose?: string; scope?: string }>
  activeAgentId?: string | null
}

function MentionInput({ onSendMessage, onStopGeneration, isLoading, disabled = false, customAgents = [], activeAgentId }: MentionInputProps) {
  const { t } = useTranslation()
  const [inputValue, setInputValue] = useState('')
  const textareaRef = useRef<HTMLTextAreaElement>(null)
  const [attachedFiles, setAttachedFiles] = useState<AttachedFile[]>([])
  const [isDragging, setIsDragging] = useState(false)
  const containerRef = useRef<HTMLDivElement>(null)
  const [showMentionMenu, setShowMentionMenu] = useState(false)
  const [mentionQuery, setMentionQuery] = useState('')
  const [filteredAgents, setFilteredAgents] = useState<Array<AgentRegistryEntry & { isSidebar?: boolean; recentMessages?: Message[] }>>([])
  const [activeMentionIndex, setActiveMentionIndex] = useState(0)
  const [mentionStartPos, setMentionStartPos] = useState(-1)
  const menuRef = useRef<HTMLDivElement>(null)

  useEffect(() => {
    const handleClickOutside = (e: MouseEvent) => {
      if (menuRef.current && !menuRef.current.contains(e.target as Node)) {
        if (containerRef.current && !containerRef.current.contains(e.target as Node)) {
          setShowMentionMenu(false)
        }
      }
    }
    document.addEventListener('mousedown', handleClickOutside)
    return () => document.removeEventListener('mousedown', handleClickOutside)
  }, [])

  const handleInputChange = (e: React.ChangeEvent<HTMLTextAreaElement>) => {
    const value = e.target.value
    const cursorPos = e.target.selectionStart
    setInputValue(value)

    const el = e.target
    const LINE_H = 20
    const MAX_LINES = 8
    el.style.height = 'auto'
    el.style.height = Math.min(el.scrollHeight, LINE_H * MAX_LINES) + 'px'

    const mentionText = getMentionAtPosition(value, cursorPos)
    if (mentionText !== null) {
      const query = mentionText.toLowerCase()
      const filtered = allMentionableAgents.filter(a =>
        (a.name ?? '').toLowerCase().includes(query) || (a.description ?? '').toLowerCase().includes(query)
      )
      setFilteredAgents(filtered)
      setMentionQuery(mentionText)
      setMentionStartPos(cursorPos - mentionText.length - 1)
      setActiveMentionIndex(0)
      setShowMentionMenu(filtered.length > 0)
      setExpandedAgentId(null)
    } else {
      setShowMentionMenu(false)
    }
  }

  const handleSelectAgent = useCallback((agent: AgentRegistryEntry) => {
    if (mentionStartPos < 0 || !textareaRef.current) return
    const beforeMention = inputValue.slice(0, mentionStartPos)
    const afterCursor = inputValue.slice(textareaRef.current.selectionEnd)
    const newValue = `${beforeMention}@${agent.name} ${afterCursor}`
    setInputValue(newValue)
    setShowMentionMenu(false)
    requestAnimationFrame(() => {
      if (textareaRef.current) {
        const newPos = mentionStartPos + (agent.name ?? '').length + 2
        textareaRef.current.focus()
        textareaRef.current.setSelectionRange(newPos, newPos)
      }
    })
  }, [inputValue, mentionStartPos])

  const insertAtCursor = (text: string) => {
    if (!textareaRef.current) return
    const start = textareaRef.current.selectionStart
    const end = textareaRef.current.selectionEnd
    const newValue = inputValue.slice(0, start) + text + inputValue.slice(end)
    setInputValue(newValue)
    requestAnimationFrame(() => {
      if (textareaRef.current) {
        const newPos = start + text.length
        textareaRef.current.focus()
        textareaRef.current.setSelectionRange(newPos, newPos)
      }
    })
  }
  const safeAgents = Array.isArray(customAgents)
    ? customAgents
    : (customAgents && typeof customAgents === 'object' && Array.isArray((customAgents as any).agents)
      ? (customAgents as any).agents
      : [])
  const sidebarAgents = useMemo(() => {
    if (safeAgents.length === 0) return []
    return safeAgents.map((a: { id: string; displayName: string; description?: string; purpose?: string }) => ({
      id: a.id,
      name: a.displayName,
      description: a.description || a.purpose || '',
      icon: '🤖',
      isSidebar: true,
      conversationId: undefined as string | undefined,
      recentMessages: [] as Message[],
    }))
  }, [customAgents])

  const allMentionableAgents = useMemo(() => {
    const builtIn = BUILT_IN_AGENTS.map(a => ({ ...a, isSidebar: false as const, recentMessages: [] as Message[] }))
    const all = [...builtIn, ...sidebarAgents] as Array<AgentRegistryEntry & { isSidebar?: boolean; recentMessages?: Message[] }>
    return all.filter(a => a.id !== activeAgentId)
  }, [sidebarAgents, activeAgentId])

  const [expandedAgentId, setExpandedAgentId] = useState<string | null>(null)

  const handleKeyDown = (e: React.KeyboardEvent) => {
    if (showMentionMenu) {
      if (e.key === 'ArrowDown') { e.preventDefault(); setActiveMentionIndex(prev => prev < filteredAgents.length - 1 ? prev + 1 : 0) }
      else if (e.key === 'ArrowUp') { e.preventDefault(); setActiveMentionIndex(prev => prev > 0 ? prev - 1 : filteredAgents.length - 1) }
      else if (e.key === 'Enter' || e.key === 'Tab') { e.preventDefault(); if (filteredAgents[activeMentionIndex]) handleSelectAgent(filteredAgents[activeMentionIndex]) }
      else if (e.key === 'Escape') { e.preventDefault(); setShowMentionMenu(false) }
      return
    }
    if (e.key === 'Enter' && !e.shiftKey) {
      e.preventDefault()
      if (isLoading) { onStopGeneration?.() }
      else { handleSend() }
    }
  }

  const handleSend = () => {
    const text = inputValue.trim()
    if ((!text && attachedFiles.length === 0) || disabled) return
    const parsed = parseMentions(text)
    const mentionedIds = parsed.mentions.map(m => m.agentId)
    onSendMessage(text, attachedFiles.length > 0 ? [...attachedFiles] : undefined, mentionedIds.length > 0 ? mentionedIds : undefined)
    setInputValue('')
    setAttachedFiles([])
    setShowMentionMenu(false)
  }

  const handlePaste = useCallback((e: React.ClipboardEvent) => {
    const items = e.clipboardData?.items
    if (!items) return
    for (let i = 0; i < items.length; i++) {
      const item = items[i]
      if (item.type.startsWith('image/')) {
        e.preventDefault()
        const file = item.getAsFile()
        if (!file) continue
        const reader = new FileReader()
        reader.onload = () => {
          setAttachedFiles(prev => [...prev, {
            name: file.name || `image-${Date.now()}.${file.type.split('/')[1] || 'png'}`,
            type: 'image',
            dataUrl: reader.result as string,
            size: file.size,
          }])
        }
        reader.readAsDataURL(file)
        break
      }
    }
  }, [])

  const handleDragOver = useCallback((e: React.DragEvent) => { e.preventDefault(); e.stopPropagation(); setIsDragging(true) }, [])
  const handleDragLeave = useCallback((e: React.DragEvent) => {
    e.preventDefault()
    e.stopPropagation()
    if (containerRef.current && !containerRef.current.contains(e.relatedTarget as Node)) setIsDragging(false)
  }, [])

  const handleDrop = useCallback((e: React.DragEvent) => {
    e.preventDefault()
    e.stopPropagation()
    setIsDragging(false)
    const files = e.dataTransfer?.files
    if (!files || files.length === 0) return
    Array.from(files).forEach(file => {
      const isImage = file.type.startsWith('image/')
      const reader = new FileReader()
      reader.onload = () => {
        setAttachedFiles(prev => [...prev, { name: file.name, type: isImage ? 'image' : 'file', dataUrl: reader.result as string, size: file.size }])
      }
      reader.readAsDataURL(file)
    })
  }, [])

  const removeAttachment = (index: number) => setAttachedFiles(prev => prev.filter((_, i) => i !== index))

  const triggerFileInput = () => {
    const input = document.createElement('input')
    input.type = 'file'
    input.multiple = true
    input.onchange = (e: any) => {
      Array.from(e.target.files || []).forEach((file: unknown) => {
        const f = file as File
        const isImage = f.type.startsWith('image/')
        const reader = new FileReader()
        reader.onload = () => setAttachedFiles(prev => [...prev, { name: f.name, type: isImage ? 'image' : 'file', dataUrl: reader.result as string, size: f.size }])
        reader.readAsDataURL(f)
      })
    }
    input.click()
  }

  return (
    <div
      ref={containerRef}
      className={`px-3 pt-2 pb-3 border-t border-dark-border bg-dark-surface/30 ${disabled ? 'opacity-50' : ''}`}
      onPaste={handlePaste}
      onDragOver={handleDragOver}
      onDragLeave={handleDragLeave}
      onDrop={handleDrop}
    >
      <div className="max-w-5xl mx-auto">
        {isDragging && (
          <div className="flex items-center justify-center h-24 mb-2 rounded-xl border-2 border-dashed border-primary-500 bg-primary-500/10 animate-pulse">
            <div className="text-center">
              <svg className="w-8 h-8 text-primary-400 mx-auto mb-1" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={1.5}><path strokeLinecap="round" strokeLinejoin="round" d="M7 16a4 4 0 01-.88-7.903A5 5 0 1115.9 6L16 6a5 5 0 011 9.9M15 13l-3-3m0 0l-3 3m3-3v12"/></svg>
              <span className="text-xs text-primary-300 font-medium">{t('input.releaseToAttach')}</span>
            </div>
          </div>
        )}

        {attachedFiles.length > 0 && (
          <div className="mb-2 p-2 rounded-lg bg-dark-bg/60 border border-dark-border/40 space-y-1.5 max-h-[160px] overflow-y-auto custom-scrollbar">
            {attachedFiles.map((file, index) => (
              <div key={index} className={`flex items-center gap-2.5 px-2.5 py-2 rounded-lg ${file.type === 'image' ? 'bg-dark-surface/50' : ''}`}>
                {file.type === 'image' ? (
                  <img src={file.dataUrl} alt={file.name} className="w-12 h-12 rounded-md object-cover border border-dark-border/30 shrink-0" />
                ) : (
                  <div className="w-12 h-12 rounded-md bg-dark-surface border border-dark-border/30 flex items-center justify-center shrink-0 text-primary-400">
                    <svg className="w-6 h-6" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={1.5}><path strokeLinecap="round" strokeLinejoin="round" d="M19.5 14.25v-2.625a3.375 3.375 0 00-3.375-3.375h-1.5A1.125 1.125 0 0113.5 7.125v-1.5a3.375 3.375 0 00-3.375-3.375H8.25m0 12.75h7.5m-7.5 3H12M10.5 2.25H5.625c-.621 0-1.125.504-1.125 1.125v17.25c0 .621.504 1.125 1.125 1.125h12.75c.621 0 1.125-.504 1.125-1.125V11.25a9 9 0 00-9-9z"/></svg>
                  </div>
                )}
                <div className="min-w-0 flex-1">
                  <div className="text-xs font-medium text-dark-text truncate">{file.name}</div>
                  <div className="text-[10px] text-dark-muted">{(file.size / 1024).toFixed(1)} KB</div>
                </div>
                <button onClick={() => removeAttachment(index)} className="p-1 rounded-full text-dark-muted/40 hover:text-red-400 hover:bg-red-500/10 transition-all shrink-0">
                  <svg className="w-3.5 h-3.5" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={2}><path strokeLinecap="round" strokeLinejoin="round" d="M6 18L18 6M6 6l12 12"/></svg>
                </button>
              </div>
            ))}
          </div>
        )}

        <div className={`flex items-center gap-1.5 bg-dark-bg rounded-2xl border transition-all shadow-sm ${
          disabled ? 'border-dark-border/30 cursor-not-allowed' : 'border-dark-border focus-within:border-primary-500/40 focus-within:ring-1 focus-within:ring-primary-500/20'
        } ${isDragging ? 'border-primary-500' : ''}`}>
          <div className="relative flex-1 pl-3 pr-1" style={{ minHeight: '32px' }}>
            <textarea
              ref={textareaRef}
              value={inputValue}
              onChange={handleInputChange}
              onKeyDown={handleKeyDown}
              placeholder={disabled ? t('input.placeholderDisabled') : (isLoading ? t('input.placeholderLoading') : t('input.placeholder'))}
              rows={1}
              disabled={disabled}
              className="w-full bg-transparent resize-none outline-none text-[14px] text-dark-text placeholder-dark-muted/35 custom-scrollbar"
              style={{
                height: '32px',
                maxHeight: '160px',
                lineHeight: '20px',
                overflowY: 'auto',
                overflowX: 'hidden',
                paddingTop: '6px',
                paddingBottom: '6px'
              }}
            />
            {showMentionMenu && filteredAgents.length > 0 && (
              <div ref={menuRef} className="absolute bottom-full left-0 mb-2 w-[380px] max-w-[calc(100vw-32px)] max-h-72 overflow-y-auto rounded-xl bg-dark-bg border border-dark-border shadow-2xl z-50 animate-fade-in custom-scrollbar">
                <div className="px-3 py-2 border-b border-white/5 flex items-center gap-2 sticky top-0 bg-dark-bg z-10">
                  <span className="text-[10px] text-dark-muted font-medium uppercase tracking-wider">{t('input.agentsLabel')}</span>
                  {mentionQuery && <span className="text-xs text-primary-400 font-mono">@{mentionQuery}</span>}
                </div>
                {(() => {
                  const builtIn = filteredAgents.filter(a => !a.isSidebar)
                  const sidebar = filteredAgents.filter(a => a.isSidebar)
                  return (
                    <>
                      {builtIn.map((agent, idx) => (
                        <button key={agent.id} className={`w-full flex items-center gap-2.5 px-3 py-2 text-left transition-colors ${idx === activeMentionIndex ? 'bg-primary-500/10 text-primary-300' : 'hover:bg-dark-surface/80 text-dark-text'}`} onClick={() => handleSelectAgent(agent)} onMouseEnter={() => setActiveMentionIndex(idx)}>
                          <span className="text-base w-7 h-7 flex items-center justify-center rounded-md bg-dark-surface border border-dark-border/30 shrink-0">{agent.icon}</span>
                          <div className="min-w-0 flex-1"><div className="text-xs font-semibold truncate">{agent.name}</div><div className="text-[10px] text-dark-muted truncate">{agent.description}</div></div>
                        </button>
                      ))}
                      {sidebar.length > 0 && builtIn.length > 0 && (
                        <div className="mx-3 my-1 border-t border-white/5" />
                      )}
                      {sidebar.map((agent, idx) => {
                        const globalIdx = builtIn.length + idx
                        const isExpanded = expandedAgentId === agent.id
                        return (
                          <div key={agent.id}>
                            <button
                              className={`w-full flex items-center gap-2.5 px-3 py-2 text-left transition-colors ${globalIdx === activeMentionIndex ? 'bg-primary-500/10 text-primary-300' : 'hover:bg-dark-surface/80 text-dark-text'}`}
                              onClick={() => {
                                if (isExpanded) handleSelectAgent(agent)
                                else setExpandedAgentId(isExpanded ? null : String(agent.id))
                              }}
                              onMouseEnter={() => setActiveMentionIndex(globalIdx)}
                            >
                              <span className="text-sm w-7 h-7 flex items-center justify-center rounded-md bg-primary-600/20 border border-primary-500/30 shrink-0 font-bold">{(agent.name ?? '?').charAt(0).toUpperCase()}</span>
                              <div className="min-w-0 flex-1"><div className="text-xs font-semibold truncate">{agent.name ?? ''}</div><div className="text-[10px] text-dark-muted truncate">{agent.description ?? ''}</div></div>
                              {(agent.recentMessages || []).length > 0 && (
                                <svg className={`w-3.5 h-3.5 text-dark-muted/40 shrink-0 transition-transform ${isExpanded ? 'rotate-180' : ''}`} fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={2}><path strokeLinecap="round" strokeLinejoin="round" d="M19 9l-7 7-7-7"/></svg>
                              )}
                            </button>
                            {isExpanded && (agent.recentMessages || []).length > 0 && (
                              <div className="mx-3 mb-2 ml-9 space-y-1">
                                {(agent.recentMessages || []).map((msg: any, mi: any) => (
                                  <div key={mi} className={`rounded-md px-2.5 py-1.5 text-[11px] leading-relaxed ${msg.role === 'user' ? 'bg-primary-500/8 text-primary-200/70' : 'bg-dark-surface/60 text-dark-text/60'}`}>
                                    <span className="text-[9px] font-medium uppercase tracking-wider opacity-50 mr-1.5">{msg.role === 'user' ? t('input.roleYou') : msg.role === 'assistant' ? t('input.roleAI') : msg.role}</span>
                                    {msg.content.slice(0, 120)}{msg.content.length > 120 ? '...' : ''}
                                  </div>
                                ))}
                              </div>
                            )}
                          </div>
                        )
                      })}
                    </>
                  )
                })()}
              </div>
            )}
          </div>

          <button onClick={() => { setInputValue(v => v + '@'); textareaRef.current?.focus() }} className="p-2 text-dark-muted hover:text-primary-400 hover:bg-dark-surface/80 rounded-xl transition-all shrink-0" title={t('input.mentionTitle')}>
            <svg className="w-[18px] h-[18px]" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={2}><circle cx="12" cy="12" r="4"/><path d="M16 8v5a3 3 0 006 0v-1a10 10 0 10-3.92 7.94"/></svg>
          </button>

          {/* <button onClick={() => insertAtCursor('# ')} className="p-2 text-dark-muted hover:text-primary-400 hover:bg-dark-surface/80 rounded-xl transition-all shrink-0" title="# Topic / Tag">
            <svg className="w-[18px] h-[18px]" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={2}><path strokeLinecap="round" strokeLinejoin="round" d="M7 20l4-16m2 16l4-16M6 9h14M4 15h14"/></svg>
          </button> */}

          <button onClick={triggerFileInput} className="p-2 text-dark-muted hover:text-primary-400 hover:bg-dark-surface/80 rounded-xl transition-all shrink-0" title={t('input.attachFileTitle')}>
            <svg className="w-[18px] h-[18px]" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={2}><rect x="3" y="3" width="18" height="18" rx="2" ry="2"/><circle cx="8.5" cy="8.5" r="1.5"/><polyline points="21 15 16 10 5 21"/></svg>
          </button>

          {isLoading ? (
            <button onClick={onStopGeneration} className="p-2 m-1 rounded-xl transition-all shrink-0 bg-red-600 hover:bg-red-500 text-white shadow-sm shadow-red-500/25" title={t('input.stopGenerationTitle')}>
              <svg className="w-[18px] h-[18px]" fill="currentColor" viewBox="0 0 24 24"><rect x="6" y="6" width="12" height="12" rx="2"/></svg>
            </button>
          ) : (
            <button onClick={handleSend} disabled={(!inputValue.trim() && attachedFiles.length === 0) || disabled} className={`p-2 m-1 rounded-xl transition-all shrink-0 ${(inputValue.trim() || attachedFiles.length > 0) && !disabled ? 'bg-primary-600 hover:bg-primary-500 text-white shadow-sm shadow-primary-500/25' : 'text-dark-muted/30 cursor-not-allowed'}`} title={disabled ? t('input.sendDisabledTitle') : t('input.sendTitle')}>
              <svg className="w-[18px] h-[18px]" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={2}><path strokeLinecap="round" strokeLinejoin="round" d="M5 12h14m-7-7v14"/></svg>
            </button>
          )}
        </div>

        <p className="text-center text-[9px] text-dark-muted/30 mt-1.5 leading-tight">{t('input.hint')}</p>
      </div>
    </div>
  )
}

export default MentionInput
