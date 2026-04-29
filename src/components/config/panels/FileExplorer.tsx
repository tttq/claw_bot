// Claw Desktop - 文件浏览器 - 浏览和编辑Agent工作区文件
// 可视化项目目录树，支持点击查看文件内容、搜索过滤

import { useState, useEffect, useCallback } from 'react'
import { useTranslation } from 'react-i18next'
import { toolGlob } from '../../../api/tools'

interface FileNode {
  name: string
  path: string
  isDir: boolean
  children?: FileNode[]
  size?: number
}

const IGNORE_DIRS = new Set(['node_modules', '.git', 'target', 'dist', '.next', '__pycache__', '.cache', 'build', '.idea', '.vscode', 'vendor', 'env'])

export default function FileExplorer({ onFileSelect, workingDir, agentId }: { onFileSelect?: (path: string) => void; workingDir?: string; agentId?: string }) {
  const { t } = useTranslation()
  const [rootNodes, setRootNodes] = useState<FileNode[]>([])
  const [expanded, setExpanded] = useState<Set<string>>(new Set())
  const [searchQuery, setSearchQuery] = useState('')
  const [loading, setLoading] = useState(false)
  const [selectedPath, setSelectedPath] = useState<string | null>(null)

  useEffect(() => { loadRoot() }, [])

  const loadRoot = async () => {
    setLoading(true)
    try {
      const result = await toolGlob({ pattern: '*', path: workingDir || '.', excludePatterns: Array.from(IGNORE_DIRS) }) as { output: string }
      if (result.output) {
        const lines = result.output.split('\n').filter(l => l.trim()).map(l => l.trim().replace(/^Found \d+ matches.*\n?/, '')).filter(l => l)
        const nodes: FileNode[] = lines.map(l => ({
          name: l.split(/[/\\]/).pop() || l,
          path: l,
          isDir: !l.includes('.'),
        }))
        setRootNodes(nodes)
      }
    } catch (e) { console.error(e) }
    finally { setLoading(false) }
  }

  const toggleExpand = async (path: string, isDir: boolean) => {
    if (!isDir) {
      setSelectedPath(path); onFileSelect?.(path); return
    }
    setExpanded(prev => { const s = new Set(prev); if (s.has(path)) s.delete(path); else s.add(path); return s })
    if (!expanded.has(path)) {
      try {
        const pattern = `${path.replace(/\\/g, '/')}/*`
        const result = await toolGlob({ pattern, excludePatterns: Array.from(IGNORE_DIRS) }) as { output: string }
        if (result.output && !result.output.startsWith('No')) {
          const lines = result.output.split('\n').filter(l => l.trim())
          setRootNodes(prev => updateChildren(prev, path, lines.map(l => ({ name: l.split(/[/\\]/).pop() || l, path: l, isDir: !l.includes('.') }))))
        }
      } catch (e) { console.error(e) }
    }
  }

  function updateChildren(nodes: FileNode[], targetPath: string, children: FileNode[]): FileNode[] {
    return nodes.map(n => n.path === targetPath ? { ...n, children } : n.children ? { ...n, children: updateChildren(n.children, targetPath, children) } : n)
  }

  const filteredNodes = searchQuery ? filterNodes(rootNodes, searchQuery.toLowerCase()) : rootNodes

  return (
    <div className="h-full flex flex-col">
      {/* Search */}
      <div className="relative mb-2">
        <svg className="absolute left-2.5 top-1/2 -translate-y-1/2 w-3.5 h-3.5 text-dark-muted" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={2}><path strokeLinecap="round" strokeLinejoin="round" d="M21 21l-6-6m2-5a7 7 0 11-14 0 7 7 0 0114 0z"/></svg>
        <input value={searchQuery} onChange={e => setSearchQuery(e.target.value)} placeholder={t('panels.fileExplorer.searchPlaceholder')} className="w-full bg-dark-bg border border-dark-border rounded-lg pl-9 pr-3 py-1.5 text-xs text-dark-text focus:outline-none focus:border-primary-500 placeholder-dark-muted/30" />
      </div>

      {/* Tree */}
      <div className="flex-1 overflow-y-auto rounded-lg border border-dark-border bg-dark-bg min-h-[200px]">
        {loading ? (
          <div className="flex justify-center py-8"><div className="w-6 h-6 border-2 border-primary-500 border-t-transparent rounded-full animate-spin"></div></div>
        ) : filteredNodes.length === 0 ? (
          <div className="text-center py-8 text-xs text-dark-muted">{t('panels.fileExplorer.noFiles')}</div>
        ) : (
          <div className="py-1">
            {filteredNodes.map((node, i) => renderNode(node, i * 10))}
          </div>
        )}
      </div>
    </div>
  )

  function renderNode(node: FileNode, indent: number) {
    const isExpanded = expanded.has(node.path)
    const isSelected = selectedPath === node.path

    return (
      <div key={node.path}>
        <div onClick={() => toggleExpand(node.path, node.isDir)}
          className={`flex items-center gap-1.5 px-2 py-1 cursor-pointer transition-colors text-[11px] group ${isSelected ? 'bg-primary-600/15 text-primary-300' : 'hover:bg-dark-surface text-dark-text'}`}
          style={{ paddingLeft: `${indent + 8}px` }}
        >
          <span className="w-3.5 shrink-0 text-center">{node.isDir ? (isExpanded ? '▼' : '▶') : ''}</span>
          {node.isDir ? (
            <svg className="w-3.5 h-3.5 text-yellow-400 shrink-0" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={2}><path strokeLinecap="round" strokeLinejoin="round" d="M3 7v10a2 2 0 002 2h14a2 2 0 002-2V9a2 2 0 00-2-2h-6l-2-2H5a2 2 0 00-2 2z" /></svg>
          ) : (
            <svg className="w-3.5 h-3.5 text-dark-muted shrink-0 opacity-60" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={2}><path strokeLinecap="round" strokeLinejoin="round" d="M9 12h6m-6 4h6m2 5H7a2 2 0 01-2-2V5a2 2 0 012-2h5.586a1 1 0 01.707.293l5.414 5.414a1 1 0 01.293.707V19a2 2 0 01-2 2z" /></svg>
          )}
          <span className="truncate font-mono">{node.name}</span>
        </div>
        {isExpanded && node.children && node.children.map(child => renderNode(child, indent + 12))}
      </div>
    )
  }

  function filterNodes(nodes: FileNode[], query: string): FileNode[] {
    return nodes.reduce((acc, node) => {
      if (node.name.toLowerCase().includes(query)) acc.push(node)
      else if (node.children) {
        const filtered = filterNodes(node.children, query)
        if (filtered.length > 0) acc.push({ ...node, children: filtered })
      }
      return acc
    }, [] as FileNode[])
  }
}
