// Claw Desktop - 多Agent协作API
// 提供子Agent执行和Agent间协调消息发送的HTTP接口
import { httpRequest } from '../ws/http'

/** 执行子Agent任务 */
export async function executeSubAgent(data: unknown): Promise<{ success: boolean; output?: string; error?: string }> {
  return httpRequest<{ success: boolean; output?: string; error?: string }>('/api/multi-agent/execute', {
    method: 'POST',
    body: JSON.stringify(data),
  })
}

export async function coordinationMessage(data: unknown): Promise<{ success: boolean }> {
  return httpRequest<{ success: boolean }>('/api/multi-agent/coordination', {
    method: 'POST',
    body: JSON.stringify(data),
  })
}
