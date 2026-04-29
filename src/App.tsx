// Claw Desktop - 应用主组件（根组件）
// 使用 Zustand 状态管理 + 自定义 Hooks 拆分业务逻辑
// 职责：布局编排、全局初始化、面板/弹窗状态管理、错误边界兜底
import { useState, useEffect, Component, useRef } from 'react'
import { useTranslation } from 'react-i18next'
import { wsClient } from './ws/bridge'
import Sidebar from './components/layout/Sidebar'
import ChatArea from './components/chat/ChatArea'
import MentionInput from './components/chat/MentionInput'
import StatusBar from './components/layout/StatusBar'
import AgentVisualization3D from './components/visualization/AgentVisualization3D'
import SettingsPanel from './components/settings/SettingsPanel'
import MemoryPanel from './components/panels/MemoryPanel'
import BrowserPanel from './components/panels/BrowserPanel'
import AgentConfigModal from './components/config/AgentConfigModal'
import PerformanceMonitor from './components/dashboard/PerformanceMonitor'
import HarnessDashboard from './components/dashboard/HarnessDashboard'
import LanguageSwitcher from './components/LanguageSwitcher'
import { DatabaseSetupWizard } from './components/setup/DatabaseSetupWizard'
import { checkDatabaseInitialized } from './api/database'

import { useConversationStore } from './stores/conversationStore'
import { useAgentStore } from './stores/agentStore'
import { useConfigStore } from './stores/configStore'
import { useUIStore } from './stores/uiStore'
import { useWSStore } from './stores/wsStore'

import { useConversationManager } from './hooks/useConversationManager'
import { useStreamingRenderer } from './hooks/useStreamingRenderer'
import { useMultiAgentCoordinator } from './hooks/useMultiAgentCoordinator'
import { useSettingsManager } from './hooks/useSettingsManager'
import { useWebSocketEvents } from './hooks/useWebSocketEvents'

import type { Message } from './types'

interface ErrorBoundaryProps {
  children: React.ReactNode
  t?: (key: string) => string
}

interface ErrorBoundaryState {
  hasError: boolean
  error: Error | null
}

/** 应用级错误边界组件：捕获子组件渲染异常，展示友好错误提示并提供重试按钮 */
class AppErrorBoundary extends Component<ErrorBoundaryProps, ErrorBoundaryState> {
  /** 构造函数：初始化错误状态 */
  constructor(props: ErrorBoundaryProps) {
    super(props)
    this.state = { hasError: false, error: null }
  }

  /** 静态生命周期：捕获渲染错误并更新状态，触发降级 UI 展示 */
  static getDerivedStateFromError(error: Error) {
    return { hasError: true, error }
  }

  /** 渲染：出错时展示降级 UI，正常时渲染子组件 */
  render() {
    if (this.state.hasError) {
      return (
        <div className="flex h-screen w-screen items-center justify-center bg-dark-bg">
          <div className="text-center space-y-4 p-8">
            <div className="text-5xl">💥</div>
            <h2 className="text-lg font-bold text-dark-text">{this.props.t?.('app.errorBoundary.title') || 'Something went wrong'}</h2>
            <p className="text-sm text-dark-muted max-w-md">{this.state.error?.message}</p>
            <button
              onClick={() => this.setState({ hasError: false, error: null })}
              className="px-4 py-2 rounded-lg bg-primary-600 hover:bg-primary-500 text-white text-sm"
            >
              {this.props.t?.('app.errorBoundary.retry') || 'Try Again'}
            </button>
          </div>
        </div>
      )
    }
    return this.props.children
  }
}

/** 自动消失的 Toast 提示组件：4秒后自动调用 onDismiss 关闭自身 */
function ToastAutoDismiss({ message, onDismiss }: { message: string; onDismiss: () => void }) {
  /** 设置4秒定时器，到期后自动关闭提示 */
  useEffect(() => {
    const timer = setTimeout(onDismiss, 4000)
    return () => clearTimeout(timer)
  }, [onDismiss])
  return (
    <div className="fixed bottom-20 left-1/2 -translate-x-1/2 z-50 animate-fade-in">
      <div className="px-4 py-2 rounded-lg bg-dark-surface border border-dark-border shadow-xl text-xs text-dark-text">{message}</div>
    </div>
  )
}

/** 应用根组件：整合所有子组件、状态管理、WebSocket 通信和面板弹窗 */
function App() {
  const { t } = useTranslation()

  // ---- Zustand 全局状态 ----
  const conversations = useConversationStore((s) => s.conversations)           // 会话列表
  const activeConversationId = useConversationStore((s) => s.activeConversationId) // 当前活跃会话ID
  const activeAgentId = useAgentStore((s) => s.activeAgentId)                 // 当前活跃Agent ID
  const agents = useAgentStore((s) => s.agents)                               // Agent列表
  const wsReady = useWSStore((s) => s.wsReady)                                // WebSocket连接状态
  const convState = useConversationStore((s) => s.convState)                  // 各会话的消息/加载/流式状态映射
  const { activeTab, showSettings, showMemoryPanel, showBrowserPanel, showPerformanceMonitor, toast, configuringAgent, pendingConfirmation,
    setActiveTab, setShowSettings, setConfiguringAgent, setPendingConfirmation } = useUIStore() // UI面板/弹窗状态
  const { config } = useConfigStore()                                         // 应用配置

  // ---- 当前会话数据派生 ----
  const currentConv = activeConversationId ? (convState[activeConversationId] ?? { messages: [], isLoading: false, streamingText: undefined, thinkingText: undefined, multiAgentMessages: [], toolExecutions: [], toolExecutionDetails: [] }) : { messages: [], isLoading: false, streamingText: undefined, thinkingText: undefined, multiAgentMessages: [], toolExecutions: [], toolExecutionDetails: [] }
  const currentMessages = currentConv.messages                                // 当前会话消息列表
  const currentIsLoading = currentConv.isLoading                             // 当前会话是否正在加载
  const currentMultiAgentMessages = currentConv.multiAgentMessages            // 多Agent消息列表
  const currentToolExecutions = currentConv.toolExecutions                    // 工具执行记录列表

  const safeConversations = Array.isArray(conversations) ? conversations : []  // 防御性处理：确保会话列表为数组
  const activeConversation = safeConversations.find(c => c.id === activeConversationId) // 当前活跃会话对象

  const messagesEndRef = useRef<HTMLDivElement>(null)                          // 消息列表底部锚点，用于自动滚动
  const [showDbSetup, setShowDbSetup] = useState(false)
  const [dbCheckDone, setDbCheckDone] = useState(false)

  // ---- 自定义 Hooks ----
  const conversationManager = useConversationManager()                         // 会话管理（增删改查、选择）
  const { scheduleStreamingRender, cleanupRaf } = useStreamingRenderer()       // 流式渲染调度（RAF节流）
  const settingsManager = useSettingsManager()                                // 设置管理（保存/导出/导入/诊断）
  const wsEvents = useWebSocketEvents(scheduleStreamingRender)                 // WebSocket事件监听与分发
  const { handleSendMessage, handleStopGeneration, handleConfirm } = useMultiAgentCoordinator() // 多Agent协调器

  /** 新消息到达时自动滚动到底部 */
  useEffect(() => {
    if (currentMessages.length > 0) {
      messagesEndRef.current?.scrollIntoView({ behavior: 'smooth' })
    }
  }, [currentMessages])

  /** 应用初始化：认证、加载配置/会话/Agent，注册窗口关闭清理逻辑 */
  useEffect(() => {
    const init = async () => {
      await wsEvents.initAuth()               // WebSocket 认证握手

      try {
        const result = await checkDatabaseInitialized()
        if (!result?.initialized) {
          setShowDbSetup(true)
          setDbCheckDone(true)
          return
        }
      } catch {
        // 如果检查失败，继续正常初始化
      }
      setDbCheckDone(true)

      conversationManager.loadConfig()         // 加载应用配置
      conversationManager.loadConversations()  // 加载会话列表
      conversationManager.loadAgents()         // 加载 Agent 列表
    }
    init()

    /** 窗口关闭前断开 WebSocket 并清除令牌 */
    const handleBeforeUnload = () => {
      wsClient.disconnect()
      wsClient.invalidateToken()
    }
    window.addEventListener('beforeunload', handleBeforeUnload)

    return () => {
      window.removeEventListener('beforeunload', handleBeforeUnload)
      wsClient.disconnect()
      cleanupRaf()
    }
  }, [])

  return (
    <AppErrorBoundary t={t}>
    {showDbSetup && (
      <DatabaseSetupWizard
        onComplete={() => {
          setShowDbSetup(false)
          conversationManager.loadConfig()
          conversationManager.loadConversations()
          conversationManager.loadAgents()
        }}
      />
    )}
    {!showDbSetup && dbCheckDone && (
    <div className="flex h-screen w-screen overflow-hidden bg-dark-bg">
      <Sidebar
        conversations={conversations} activeId={activeConversationId}
        onSelectConversation={conversationManager.handleSelectConversation}
        onNewConversation={conversationManager.handleNewConversation}
        onDeleteConversation={conversationManager.handleDeleteConversation}
        onRenameConversation={conversationManager.handleRenameConversation}
        onSelectAgent={(id) => useAgentStore.getState().setActiveAgentId(id)}
        activeAgentId={activeAgentId}
        onAgentsLoaded={(agents) => useAgentStore.getState().setAgents(agents)}
        wsReady={wsReady}
        onConfigureAgent={(aid, name) => setConfiguringAgent({ agentId: aid, agentName: name })}
        onOpenSettings={() => setShowSettings(true)}
        onOpenMemory={() => useUIStore.getState().setShowMemoryPanel(true)}
        onOpenBrowser={() => useUIStore.getState().setShowBrowserPanel(true)}
        onOpenPerformanceMonitor={() => useUIStore.getState().setShowPerformanceMonitor(true)}
        onExport={settingsManager.handleExport}
        onImport={settingsManager.handleImport}
        onDoctor={settingsManager.handleDoctor}
      />

      <div className="flex-1 flex flex-col min-w-0">
        <div className="flex items-center h-10 bg-dark-surface/80 border-b border-dark-border/50 px-2 shrink-0">
          {(['chat', '3d', 'harness'] as const).map(tab => (
            <button key={tab}
              onClick={() => setActiveTab(tab)}
              className={`relative px-4 py-1.5 text-xs font-medium transition-colors rounded-md ${
                activeTab === tab ? 'text-primary-400 bg-primary-500/10' : 'text-dark-muted hover:text-dark-text'
              }`}
            >
              {t(`tabs.${tab}`)}
              {activeTab === tab && <span className="absolute bottom-0 left-1/2 -translate-x-1/2 w-6 h-0.5 bg-primary-500 rounded-full" />}
            </button>
          ))}
          <div className="flex-1" />
          <LanguageSwitcher />
        </div>

        {activeTab === 'chat' ? (
          <>
            <ChatArea
              messages={currentMessages} isLoading={currentIsLoading}
              conversationTitle={activeConversation?.title} messagesEndRef={messagesEndRef}
              conversationId={activeConversationId} activeAgentId={activeAgentId}
              showToolExecutions={config?.ui.show_tool_executions ?? true}
              streamingText={currentConv.streamingText}
              thinkingText={currentConv.thinkingText}
              multiAgentMessages={currentMultiAgentMessages}
              toolExecutions={currentToolExecutions}
              toolExecutionDetails={currentConv.toolExecutionDetails}
              pendingConfirmation={activeConversationId === pendingConfirmation?.conversationId ? pendingConfirmation : null}
              onConfirm={handleConfirm}
              onDismissConfirm={() => setPendingConfirmation(null)}
              onSuggestionClick={(text) => handleSendMessage(text)}
            />
            <MentionInput
              onSendMessage={handleSendMessage}
              onStopGeneration={handleStopGeneration}
              isLoading={currentIsLoading}
              disabled={!activeAgentId}
              customAgents={agents}
              activeAgentId={activeAgentId}
            />
            <StatusBar
              conversationId={activeConversationId}
              messageCount={currentMessages.length}
              isLoading={currentIsLoading}
              onCompact={settingsManager.handleCompact}
              onClear={settingsManager.handleClear}
            />
          </>
        ) : activeTab === '3d' ? (
          <AgentVisualization3D agents={agents} convState={convState} isActive={activeTab === '3d'} />
        ) : (
          <div className="flex-1 overflow-y-auto p-4"><HarnessDashboard /></div>
        )}
      </div>

      {showSettings && (<SettingsPanel config={config} onSave={settingsManager.handleSaveConfig} onClose={() => setShowSettings(false)} />)}

      {showMemoryPanel && (
        <div className="fixed inset-0 z-40 bg-black/50 flex items-center justify-center p-4">
          <div className="w-full max-w-4xl h-[80vh] flex flex-col bg-dark-surface rounded-xl shadow-2xl overflow-hidden border border-dark-border">
            <MemoryPanel agentId={activeAgentId || 'default'} onClose={() => useUIStore.getState().setShowMemoryPanel(false)} />
          </div>
        </div>
      )}

      {showBrowserPanel && (
        <div className="fixed inset-0 z-40 bg-black/50 flex items-center justify-center p-4">
          <div className="w-full max-w-5xl h-[85vh] flex flex-col bg-dark-surface rounded-xl shadow-2xl overflow-hidden border border-dark-border">
            <BrowserPanel onClose={() => useUIStore.getState().setShowBrowserPanel(false)} />
          </div>
        </div>
      )}

      {toast && (
        <ToastAutoDismiss message={toast} onDismiss={() => useUIStore.getState().setToast(null)} />
      )}

      {configuringAgent && (
        <AgentConfigModal agentId={configuringAgent.agentId} agentName={configuringAgent.agentName} onClose={() => setConfiguringAgent(null)} />
      )}

      <PerformanceMonitor isOpen={showPerformanceMonitor} onClose={() => useUIStore.getState().setShowPerformanceMonitor(false)} />
    </div>
    )}
    </AppErrorBoundary>
  )
}

export default App
