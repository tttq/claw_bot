// Claw Desktop - 数据库管理API
// 提供数据库状态查询、初始化、连接测试、配置读写等HTTP接口
import { httpRequest } from '../ws/http'

/** 数据库配置 — 后端类型及各后端连接参数 */
export interface DatabaseSettings {
  backend: string
  sqlite: {
    enable_vec: boolean
    db_path: string
  }
  postgres: {
    host: string
    port: number
    database: string
    username: string
    password: string
    pool_size: number
  }
  qdrant: {
    url: string
    api_key: string
    collection: string
  }
  initialized: boolean
}

export interface DatabaseStatus {
  backend: string
  connected: boolean
  vector_support: boolean
  tables: Array<{
    name: string
    exists: boolean
    row_count: number
    columns_valid: boolean
    needs_repair: boolean
  }>
  total_rows: Record<string, number>
}

export interface DatabaseInitResult {
  backend: string
  success: boolean
  tables_created: string[]
  tables_repaired: string[]
  vector_support: boolean
  message: string
}

export async function getDatabaseStatus() {
  return httpRequest<DatabaseStatus>('/api/database/status', { method: 'GET' })
}

export async function initializeDatabase() {
  return httpRequest<DatabaseInitResult>('/api/database/initialize', { method: 'POST' })
}

export async function testDatabaseConnection(params: { backend: string; [key: string]: unknown }) {
  return httpRequest<{ success: boolean }>('/api/database/test-connection', {
    method: 'POST',
    body: JSON.stringify(params),
  })
}

export async function getDatabaseConfig() {
  return httpRequest<DatabaseSettings>('/api/database/config', { method: 'GET' })
}

export async function updateDatabaseConfig(config: DatabaseSettings) {
  return httpRequest<{ saved: boolean }>('/api/database/config', {
    method: 'POST',
    body: JSON.stringify(config),
  })
}

export async function checkDatabaseInitialized() {
  return httpRequest<{ initialized: boolean }>('/api/database/is-initialized', { method: 'GET' })
}
