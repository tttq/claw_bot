// Claw Desktop - 流式输出状态管理模块
// 管理 AI 流式响应文本的缓存及 requestAnimationFrame ID 引用
import { create } from 'zustand'

/** 流式输出状态管理接口：管理各会话的流式文本缓存和渲染帧 ID */
interface StreamingStore {
  streamingTextRef: Record<string, string>  // 各会话的流式文本缓存（key 为会话 ID）
  rafIdRef: number | null                   // requestAnimationFrame 的回调 ID，用于取消渲染帧

  setStreamingText: (convId: string, text: string) => void      // 设置指定会话的流式文本
  clearStreamingText: (convId: string) => void                   // 清除指定会话的流式文本
  getAllStreamingTexts: () => Record<string, string>             // 获取所有会话的流式文本
  setRafId: (id: number | null) => void                          // 设置 requestAnimationFrame ID
}

/** 创建流式输出状态管理 Store，使用 Zustand 管理流式文本缓存和渲染帧引用 */
export const useStreamingStore = create<StreamingStore>((set, get) => ({
  streamingTextRef: {},         // 初始无流式文本缓存
  rafIdRef: null,               // 初始无渲染帧 ID

  /** 更新指定会话的流式文本 */
  setStreamingText: (convId, text) =>
    set((s) => ({
      streamingTextRef: { ...s.streamingTextRef, [convId]: text },
    })),

  /** 删除指定会话的流式文本缓存 */
  clearStreamingText: (convId) =>
    set((s) => {
      const next = { ...s.streamingTextRef }
      delete next[convId]
      return { streamingTextRef: next }
    }),

  /** 返回所有流式文本的引用 */
  getAllStreamingTexts: () => get().streamingTextRef,
  setRafId: (id) => set({ rafIdRef: id }),
}))
