// Claw Desktop - 会话与消息 API
// 提供会话的增删改查、消息收发、流式聊天、RAG 检索/压缩等核心通信接口
import { httpRequest } from '../ws/http'
import type { Conversation, Message } from '../types'

/** 发送消息请求参数 */
export interface SendMessageRequest {
  conversationId: string            // 目标会话 ID
  content: string                   // 消息内容
  agentId?: string                  // 指定 Agent ID（可选）
  modelOverride?: string            // 覆盖模型名称（可选）
  stream?: boolean                  // 是否流式输出（可选）
}

/** 创建会话请求参数 */
export interface CreateConversationRequest {
  title?: string                   // 会话标题（可选，默认取首条消息前50字符）
  agentId?: string                 // 关联 Agent ID（可选）
}

/** 重命名会话请求参数 */
export interface RenameConversationRequest {
  newTitle: string
}

/** RAG 检索请求参数 */
export interface RagSearchRequest {
  query: string
  agentId?: string
  limit?: number
}

/** 会话压缩请求参数 */
export interface CompactRequest {
  conversationId?: string
  modelName?: string
}

/** 获取所有会话列表 */
export async function listConversations() {
  return httpRequest<Conversation[]>('/api/conversations', { method: 'GET' })
}

/** 创建新会话 */
export async function createConversation(data?: CreateConversationRequest) {
  return httpRequest<Conversation>('/api/conversations', {
    method: 'POST',
    ...(data ? { body: JSON.stringify(data) } : {}),
  })
}

/** 获取指定会话的消息列表 */
export async function getMessages(data: { conversationId: string }) {
  return httpRequest<Message[]>(`/api/conversations/${data.conversationId}/messages`, { method: 'GET' })
}

/** 发送消息响应 */
export interface SendMessageResponse {
  text: string
  usage?: {
    input_tokens: number
    output_tokens: number
    cache_read?: number
    model: string
    streamed: boolean
  }
  streamed: boolean
}

/** 发送消息（非流式，等待完整响应） */
export async function sendMessage(data: SendMessageRequest): Promise<SendMessageResponse> {
  return httpRequest<SendMessageResponse>('/api/conversations/send', {
    method: 'POST',
    body: JSON.stringify(data),
  })
}

/** 发送消息（流式，通过 WebSocket 逐步推送 token） */
export async function sendMessageStreaming(data: SendMessageRequest): Promise<SendMessageResponse> {
  return httpRequest<SendMessageResponse>('/api/conversations/streaming', {
    method: 'POST',
    body: JSON.stringify(data),
  })
}

/** 删除指定会话 */
export async function deleteConversation(data: { conversationId: string }) {
  return httpRequest<{ deleted: boolean }>(`/api/conversations/${data.conversationId}`, { method: 'DELETE' })
}

/** 重命名指定会话 */
export async function renameConversation(id: string, data?: RenameConversationRequest) {
  return httpRequest<Conversation>(`/api/conversations/${id}/rename`, {
    method: 'PUT',
    ...(data ? { body: JSON.stringify(data) } : {}),
  })
}

/** RAG 检索结果 */
export interface RagSearchResult {
  id: string
  text: string
  fact_type: string
  source_type: string
  semantic_score: number
  bm25_score: number
  temporal_score: number
  final_score: number
}

/** RAG 语义检索：在记忆库中搜索相关内容 */
export async function ragSearch(query: RagSearchRequest): Promise<{ results: RagSearchResult[] }> {
  return httpRequest<{ results: RagSearchResult[] }>('/api/rag/search', {
    method: 'POST',
    body: JSON.stringify(query),
  })
}

/** 清空指定会话的所有消息 */
export async function clearConversationMessages(data: { conversationId: string }) {
  return httpRequest<{ cleared: boolean }>(`/api/conversations/${data.conversationId}/clear`, { method: 'POST' })
}

/** 压缩会话历史（保留摘要，减少 token 占用） */
export async function compactConversation(data?: CompactRequest): Promise<{ success: boolean; original_count?: number; compacted_count?: number }> {
  return httpRequest<{ success: boolean; original_count?: number; compacted_count?: number }>('/api/conversations/compact', {
    method: 'POST',
    ...(data ? { body: JSON.stringify(data) } : {}),
  })
}

export async function ragCompact(data?: CompactRequest): Promise<{ success: boolean; units_before?: number; units_after?: number }> {
  return httpRequest<{ success: boolean; units_before?: number; units_after?: number }>('/api/rag/compact', {
    method: 'POST',
    ...(data ? { body: JSON.stringify(data) } : {}),
  })
}

/** 取消正在进行的流式生成 */
export async function cancelStream(data: { conversationId: string }) {
  return httpRequest<{ cancelled: boolean }>(`/api/conversations/${data.conversationId}/cancel-stream`, { method: 'POST' })
}

import { fetchStreamChat, type ChatEvent } from '../ws/http'

export { fetchStreamChat, type ChatEvent }

/** 流式聊天生成器：逐事件产出 ChatEvent，支持 AbortSignal 取消 */
export async function* chatStream(data: { conversationId: string; content: string }, signal?: AbortSignal): AsyncGenerator<ChatEvent> {
  yield* fetchStreamChat(data.conversationId, data.content, { signal })
}
