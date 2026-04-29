// Claw Desktop - 流式渲染调度 Hook
// 使用 requestAnimationFrame 节流机制，将高频的流式文本更新合并为低频的 React 状态更新，
// 避免每个 token 都触发重渲染，提升流式输出性能
import { useCallback, useEffect } from 'react'
import { useStreamingStore } from '../stores/streamingStore'
import { useConversationStore } from '../stores/conversationStore'

/** 流式渲染调度 Hook：通过 RAF 节流批量更新流式文本到会话状态 */
export function useStreamingRenderer() {
  const { streamingTextRef, rafIdRef, setStreamingText, clearStreamingText, getAllStreamingTexts, setRafId } =
    useStreamingStore()
  const { setConvState } = useConversationStore()

  /** 调度一次 RAF 渲染：将 streamingTextRef 快照批量写入 convState */
  const scheduleStreamingRender = useCallback(() => {
    if (rafIdRef !== null) return
    const id = requestAnimationFrame(() => {
      setRafId(null)
      const snapshot = { ...streamingTextRef }
      setConvStateBatch(snapshot)
    })
    setRafId(id)
  }, [rafIdRef, streamingTextRef, setRafId])

  /** 批量更新会话流式文本状态 */
  const setConvStateBatch = useCallback(
    (snapshot: Record<string, string>) => {
      const state = useConversationStore.getState()
      const next = { ...state.convState }
      for (const [cid, text] of Object.entries(snapshot)) {
        if (next[cid]) {
          next[cid] = { ...next[cid], streamingText: text }
        }
      }
      useConversationStore.setState({ convState: next })
    },
    [],
  )

  /** 清理未完成的 RAF 定时器（组件卸载时调用） */
  const cleanupRaf = useCallback(() => {
    if (rafIdRef !== null) {
      cancelAnimationFrame(rafIdRef)
      setRafId(null)
    }
  }, [rafIdRef, setRafId])

  return {
    streamingTextRef,
    rafIdRef,
    scheduleStreamingRender,
    cleanupRaf,
    setStreamingText,
    clearStreamingText,
    getAllStreamingTexts,
  }
}
