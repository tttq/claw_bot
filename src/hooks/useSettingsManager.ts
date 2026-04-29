// Claw Desktop - 设置管理 Hook
// 封装配置保存、会话压缩/清空、数据导出/导入、系统诊断等设置相关操作
import { useCallback } from 'react'
import { useTranslation } from 'react-i18next'
import { useConversationStore } from '../stores/conversationStore'
import { useConfigStore } from '../stores/configStore'
import { useUIStore } from '../stores/uiStore'
import { sendMessageStreaming, compactConversation, clearConversationMessages, saveConfig, exportWithDialog, importData as importDataApi, runDoctorCheck } from '../api'
import type { AppConfig } from '../types'

/** 设置管理 Hook：提供配置保存、会话压缩/清空、数据导入导出、诊断等操作 */
export function useSettingsManager() {
  const { t } = useTranslation()
  const { config, showSettings, setConfig, setShowSettings } = useConfigStore()
  const { setToast } = useUIStore()
  const { activeConversationId, convState, setConvState, initConvState, getActiveConv } = useConversationStore()

  /** 保存应用配置到后端 */
  const handleSaveConfig = useCallback(
    async (newConfig: AppConfig) => {
      try {
        await saveConfig(newConfig)
        setConfig(newConfig)
        setShowSettings(false)
        setToast(t('errors.settingsSaved'))
      } catch (e) {
        setToast(t('errors.saveFailed', { error: String(e) }))
      }
    },
    [setConfig, setShowSettings, setToast, t],
  )

  /** 压缩当前会话历史（RAG 摘要压缩，减少 token 占用） */
  const handleCompact = useCallback(async () => {
    if (!activeConversationId) return
    try {
      await compactConversation({ conversationId: activeConversationId })
      const { getMessages: fetchMsgs } = await import('../api')
      const msgs = (await fetchMsgs({ conversationId: activeConversationId })) as import('../types').Message[]
      const current = convState[activeConversationId]
      setConvState(activeConversationId!, {
        messages: msgs,
        isLoading: false,
        multiAgentMessages: (current as unknown as { multiAgentMessages?: import('../multiagent/types').MultiAgentMessageContent[] })?.multiAgentMessages || [],
        toolExecutions: [],
      })
      setToast(t('errors.compacted'))
    } catch (e) {
      setToast(t('errors.compactFailed', { error: String(e) }))
    }
  }, [activeConversationId, convState, setConvState, setToast, t])

  /** 清空当前会话的所有消息 */
  const handleClear = useCallback(async () => {
    if (!activeConversationId) return
    try {
      await clearConversationMessages({ conversationId: activeConversationId })
      initConvState(activeConversationId!)
      setToast(t('errors.cleared'))
    } catch (e) {
      setToast(t('errors.clearFailed', { error: String(e) }))
    }
  }, [activeConversationId, initConvState, setToast, t])

  /** 通过系统文件对话框导出数据 */
  const handleExport = useCallback(async () => {
    try {
      const filePath = await exportWithDialog()
      setToast(t('errors.exported', { path: String(filePath).split(/[/\\]/).pop() }))
    } catch (e) {
      setToast(t('errors.exportFailed', { error: String(e) }))
    }
  }, [setToast, t])

  /** 从指定路径导入数据 */
  const handleImport = useCallback(async () => {
    const path = prompt(t('errors.enterPath'))
    if (path) {
      try {
        await importDataApi({ path })
        const { listConversations: listConvs } = await import('../api')
        const convs = (await listConvs()) as import('../types').Conversation[]
        useConversationStore.getState().setConversations(convs)
        setToast(t('errors.imported'))
      } catch (e) {
        setToast(t('errors.importFailed', { error: String(e) }))
      }
    }
  }, [setToast, t])

  /** 运行系统诊断检查 */
  const handleDoctor = useCallback(async () => {
    try {
      const r = (await runDoctorCheck()) as unknown as { status: string }[]
      setToast(
        t('errors.doctorResult', {
          passed: String(r.filter((x: any) => x.status === 'ok').length),
          total: String(r.length),
        }),
      )
    } catch (e) {
      setToast(t('errors.doctorFailed', { error: String(e) }))
    }
  }, [setToast, t])

  return {
    config,
    showSettings,
    handleSaveConfig,
    handleCompact,
    handleClear,
    handleExport,
    handleImport,
    handleDoctor,
  }
}
