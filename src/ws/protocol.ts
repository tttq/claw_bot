// Claw Desktop - WebSocket 通信协议类型定义
// 定义前端与后端之间 WebSocket 通信的三种核心消息类型：WsRequest(请求)、WsResponse(响应)、WsEvent(事件)

/** WebSocket请求消息 */
export interface WsRequest {
  id: string                    // 请求唯一ID
  type: string                  // 消息类型 (request)
  method: string                // 调用方法名
  params: Record<string, unknown> // 请求参数
  token: string                 // 认证令牌
}

/** WebSocket响应消息 */
export interface WsResponse {
  id: string                    // 对应请求的ID
  type: string                  // 消息类型 (response)
  method: string                // 调用方法名
  success: boolean              // 是否成功
  data: unknown                 // 响应数据
  error: string                 // 错误信息
}

/** WebSocket流式事件消息 */
export interface WsEvent {
  id: string                    // 事件ID
  type: string                  // 消息类型 (stream)
  method: string                // 关联的方法名
  event: string                 // 事件名称
  data: unknown                 // 事件数据
}
