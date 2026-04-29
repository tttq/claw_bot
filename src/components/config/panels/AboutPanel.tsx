// Claw Desktop - 关于面板 - 显示应用版本、系统信息和许可证
// 键盘快捷键、功能描述

import { useState } from 'react'
import { useTranslation } from 'react-i18next'

interface Shortcut { keys: string; descriptionKey: string }

export default function AboutPanel() {
  const { t } = useTranslation()
  const [activeSection, setActiveSection] = useState<'about' | 'shortcuts' | 'tools'>('about')

  const SHORTCUTS: Shortcut[] = [
    { keys: 'Ctrl + Enter', descriptionKey: 'panels.about.shortcutSendMessage' },
    { keys: 'Ctrl + N', descriptionKey: 'panels.about.shortcutNewSession' },
    { keys: 'Ctrl + Shift + N', descriptionKey: 'panels.about.shortcutNewTaggedSession' },
    { keys: 'Ctrl + /', descriptionKey: 'panels.about.shortcutToggleSidebar' },
    { keys: 'Ctrl + ,', descriptionKey: 'panels.about.shortcutOpenSettings' },
    { keys: 'Ctrl + K', descriptionKey: 'panels.about.shortcutQuickToolSearch' },
    { keys: 'Escape', descriptionKey: 'panels.about.shortcutClosePopup' },
    { keys: '↑ / ↓', descriptionKey: 'panels.about.shortcutNavigateHistory' },
    { keys: 'Tab', descriptionKey: 'panels.about.shortcutAutoComplete' },
  ]

  const FEATURES = [
    { icon: '🤖', titleKey: 'panels.about.featureAIChatEngine', descKey: 'panels.about.featureAIChatEngineDesc' },
    { icon: '📁', titleKey: 'panels.about.featureFileOperations', descKey: 'panels.about.featureFileOperationsDesc' },
    { icon: '🔧', titleKey: 'panels.about.featureShellExecution', descKey: 'panels.about.featureShellExecutionDesc' },
    { icon: '🌐', titleKey: 'panels.about.featureNetworkTools', descKey: 'panels.about.featureNetworkToolsDesc' },
    { icon: '🤖', titleKey: 'panels.about.featureAgentSubAgents', descKey: 'panels.about.featureAgentSubAgentsDesc' },
    { icon: '📋', titleKey: 'panels.about.featureGitIntegration', descKey: 'panels.about.featureGitIntegrationDesc' },
    { icon: '💰', titleKey: 'panels.about.featureUsageTracking', descKey: 'panels.about.featureUsageTrackingDesc' },
    { icon: '📝', titleKey: 'panels.about.featurePlanMode', descKey: 'panels.about.featurePlanModeDesc' },
    { icon: '🔌', titleKey: 'panels.about.featureMCPSupport', descKey: 'panels.about.featureMCPSupportDesc' },
    { icon: '🏷️', titleKey: 'panels.about.featureTagSystem', descKey: 'panels.about.featureTagSystemDesc' },
    { icon: '🔍', titleKey: 'panels.about.featureCodeReview', descKey: 'panels.about.featureCodeReviewDesc' },
    { icon: '⚡', titleKey: 'panels.about.featureQuickActions', descKey: 'panels.about.featureQuickActionsDesc' },
  ]

  const TOOL_LIST: [string, string][] = [
    ['Read', 'panels.about.toolRead'], ['Write', 'panels.about.toolWrite'], ['Edit', 'panels.about.toolEdit'],
    ['Bash', 'panels.about.toolBash'], ['Glob', 'panels.about.toolGlob'], ['Grep', 'panels.about.toolGrep'],
    ['WebFetch', 'panels.about.toolWebFetch'], ['WebSearch', 'panels.about.toolWebSearch'],
    ['Agent', 'panels.about.toolAgent'], ['TodoWrite', 'panels.about.toolTodoWrite'],
    ['TaskCreate', 'panels.about.toolTaskCreate'], ['TaskGet', 'panels.about.toolTaskGet'],
    ['TaskUpdate', 'panels.about.toolTaskUpdate'], ['TaskList', 'panels.about.toolTaskList'],
    ['Workflow', 'panels.about.toolWorkflow'], ['Skill', 'panels.about.toolSkill'],
    ['PlanMode', 'panels.about.toolPlanMode'], ['Brief', 'panels.about.toolBrief'],
    ['Config', 'panels.about.toolConfig'], ['NotebookEdit', 'panels.about.toolNotebookEdit'],
    ['ScheduleCron', 'panels.about.toolScheduleCron'], ['ScheduleList', 'panels.about.toolScheduleList'],
    ['AskUserQuestion', 'panels.about.toolAskUserQuestion'], ['ToolSearch', 'panels.about.toolToolSearch'],
    ['GitStatus', 'panels.about.toolGitStatus'], ['GitDiff', 'panels.about.toolGitDiff'],
    ['GitCommit', 'panels.about.toolGitCommit'], ['GitLog', 'panels.about.toolGitLog'],
    ['GitBranch', 'panels.about.toolGitBranch'], ['GitCheckout', 'panels.about.toolGitCheckout'],
    ['GitStash', 'panels.about.toolGitStash'], ['GitStashPop', 'panels.about.toolGitStashPop'],
    ['GitAdd', 'panels.about.toolGitAdd'], ['GitReset', 'panels.about.toolGitReset'],
    ['GitIsRepo', 'panels.about.toolGitIsRepo'],
    ['EnvVars', 'panels.about.toolEnvVars'], ['CodeChanges', 'panels.about.toolCodeChanges'],
    ['CodeReview', 'panels.about.toolCodeReview'], ['FastMode', 'panels.about.toolFastMode'],
  ]

  return (
    <div className="space-y-5">
      {/* App branding */}
      <div className="text-center py-4">
        <div className="w-16 h-16 rounded-2xl bg-gradient-to-br from-primary-500 to-purple-600 flex items-center justify-center mx-auto mb-3 shadow-lg shadow-primary-500/20">
          <span className="text-2xl font-bold text-white">CD</span>
        </div>
        <h2 className="text-xl font-bold text-dark-text">{t('panels.about.appName')}</h2>
        <p className="text-xs text-dark-muted mt-1">{t('panels.about.appSubtitle')}</p>
        <p className="text-[10px] text-dark-muted/50 mt-0.5">v1.0.0 · Built with Tauri 2 + React</p>
      </div>

      {/* Section tabs */}
      <div className="flex gap-1 p-1 rounded-lg bg-dark-bg border border-dark-border w-fit mx-auto">
        {[
          { id: 'about' as const, labelKey: 'panels.about.tabAbout' },
          { id: 'shortcuts' as const, labelKey: 'panels.about.tabShortcuts' },
          { id: 'tools' as const, labelKey: 'panels.about.tabTools' },
        ].map(s => (
          <button key={s.id} onClick={() => setActiveSection(s.id)} className={`px-3 py-1 rounded-md text-xs transition-colors ${activeSection === s.id ? 'bg-primary-600 text-white' : 'text-dark-muted hover:text-dark-text'}`}>{t(s.labelKey)}</button>
        ))}
      </div>

      {/* About section */}
      {activeSection === 'about' && (
        <div className="space-y-3">
          {FEATURES.map((item, i) => (
            <div key={i} className="flex items-start gap-3 px-3 py-2.5 rounded-lg hover:bg-dark-bg transition-colors group">
              <span className="text-lg shrink-0 mt-0.5">{item.icon}</span>
              <div>
                <span className="text-sm font-medium text-dark-text group-hover:text-primary-300 transition-colors">{t(item.titleKey)}</span>
                <p className="text-[11px] text-dark-muted mt-0.5 leading-relaxed">{t(item.descKey)}</p>
              </div>
            </div>
          ))}
        </div>
      )}

      {/* Shortcuts section */}
      {activeSection === 'shortcuts' && (
        <div className="rounded-xl border border-dark-border overflow-hidden">
          {SHORTCUTS.map((sc, i) => (
            <div key={i} className="flex items-center justify-between px-4 py-2.5 hover:bg-dark-bg transition-colors border-b border-dark-border/50 last:border-b-0">
              <kbd className="px-2 py-1 rounded-md bg-dark-surface border border-dark-border text-[11px] font-mono text-primary-300">{sc.keys}</kbd>
              <span className="text-xs text-dark-muted">{t(sc.descriptionKey)}</span>
            </div>
          ))}
        </div>
      )}

      {/* Tools list section */}
      {activeSection === 'tools' && (
        <div className="rounded-xl border border-dark-border overflow-hidden max-h-[400px] overflow-y-auto">
          <div className="px-4 py-2 bg-dark-surface border-b border-dark-border sticky top-0 z-10">
            <span className="text-xs font-semibold text-dark-text">{t('panels.about.registeredTools')} ({TOOL_LIST.length})</span>
          </div>
          <div className="grid grid-cols-2 gap-px bg-dark-border divide-y divide-dark-border">
            {TOOL_LIST.map(([name, descKey], i) => (
              <div key={i} className="bg-dark-bg px-3 py-2 hover:bg-dark-surface transition-colors">
                <code className="text-[11px] font-mono text-primary-300">{name}</code>
                <p className="text-[10px] text-dark-muted mt-0.5 truncate">{t(descKey)}</p>
              </div>
            ))}
          </div>
        </div>
      )}
    </div>
  )
}
