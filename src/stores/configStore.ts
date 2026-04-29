// Claw Desktop - 配置状态管理模块
// 管理应用全局配置及设置面板显示状态
import { create } from 'zustand'
import type { AppConfig } from '../types'

/** 配置状态管理接口：管理应用配置和设置面板的显示状态 */
interface ConfigStore {
  config: AppConfig | null      // 应用全局配置对象
  showSettings: boolean         // 设置面板是否可见

  setConfig: (config: AppConfig) => void     // 更新应用配置
  setShowSettings: (show: boolean) => void   // 切换设置面板显示状态
}

/** 创建配置状态管理 Store，使用 Zustand 管理全局配置和设置面板状态 */
export const useConfigStore = create<ConfigStore>((set) => ({
  config: null,                 // 初始配置为空，需从后端加载
  showSettings: false,          // 初始设置面板不可见

  setConfig: (config) => set({ config }),
  setShowSettings: (show) => set({ showSettings: show }),
}))
