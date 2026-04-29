// Claw Desktop - 应用配置 API
// 提供应用全局配置的读取和保存接口
import { httpRequest } from '../ws/http'
import type { AppConfig } from '../types'

/** 获取当前应用配置 */
export async function getConfig() {
  return httpRequest<AppConfig>('/api/config', { method: 'GET' })
}

/** 保存应用配置（整体覆盖） */
export async function saveConfig(config: AppConfig) {
  return httpRequest<{ saved: boolean }>('/api/config', {
    method: 'POST',
    body: JSON.stringify({ config }),
  })
}
