// Claw Desktop - WebSocket 连接状态管理模块
// 管理 WebSocket 连接的就绪状态
import { create } from 'zustand'

/** WebSocket 状态管理接口：管理 WebSocket 连接的就绪状态 */
interface WSStore {
  wsReady: boolean              // WebSocket 是否已连接就绪

  setWsReady: (ready: boolean) => void  // 设置 WebSocket 连接就绪状态
}

/** 创建 WebSocket 状态管理 Store，使用 Zustand 管理 WebSocket 连接状态 */
export const useWSStore = create<WSStore>((set) => ({
  wsReady: false,               // 初始 WebSocket 未连接
  setWsReady: (ready) => set({ wsReady: ready }),
}))
