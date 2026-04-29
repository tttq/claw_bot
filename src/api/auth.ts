// Claw Desktop - 认证API
// 提供RSA密钥握手和Token验证的HTTP接口
import { httpRequest } from '../ws/http'

/** RSA握手认证 — 用加密的会话密钥换取访问Token */
export async function authHandshake(encryptedKey: string): Promise<{ token: string; expiresAt: number }> {
  return httpRequest<{ token: string; expiresAt: number }>('/api/auth/handshake', {
    method: 'POST',
    body: JSON.stringify({ encryptedSessionKey: encryptedKey }),
  })
}

export async function authValidate(token: string): Promise<boolean> {
  return httpRequest<boolean>('/api/auth/validate', {
    method: 'POST',
    body: JSON.stringify({ token }),
  })
}
