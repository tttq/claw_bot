// Claw Desktop - 浏览器控制API
// 提供浏览器检测/启动、标签页管理、页面导航/内容获取/截图/点击/填入/JS执行等HTTP接口
import { httpRequest } from '../ws/http'

/** 浏览器信息 — 名称、路径、版本 */
export interface BrowserInfo {
  name: string
  path: string
  version: string
  [key: string]: unknown
}

export interface TabInfo {
  id: string
  title: string
  url: string
  [key: string]: unknown
}

export interface PageContent {
  html: string
  text: string
  [key: string]: unknown
}

export interface ScreenshotResult {
  data: string
  format: string
  [key: string]: unknown
}

export async function browserDetect(): Promise<{ browsers: BrowserInfo[] }> {
  return httpRequest<{ browsers: BrowserInfo[] }>('/api/browser/detect', { method: 'GET' })
}

export async function browserLaunch(data?: unknown): Promise<{ success: boolean; port?: number }> {
  return httpRequest<{ success: boolean; port?: number }>('/api/browser/launch', {
    method: 'GET',
  })
}

export async function browserCheckPort(data: { port: number }): Promise<{ available: boolean }> {
  return httpRequest<{ available: boolean }>(`/api/browser/check-port/${data.port}`, { method: 'GET' })
}

export async function browserListTabs(data: { port: number }): Promise<{ tabs: TabInfo[] }> {
  return httpRequest<{ tabs: TabInfo[] }>(`/api/browser/tabs/${data.port}`, { method: 'GET' })
}

export async function browserNavigate(data: { port: number; tab_id: string; url?: string }): Promise<{ success: boolean }> {
  return httpRequest<{ success: boolean }>(`/api/browser/navigate/${data.port}/${data.tab_id}`, {
    method: 'POST',
    body: JSON.stringify(data),
  })
}

export async function browserGetContent(data: { port: number; tab_id: string }): Promise<PageContent> {
  return httpRequest<PageContent>(`/api/browser/content/${data.port}/${data.tab_id}`, { method: 'GET' })
}

export async function browserScreenshot(data: { port: number; tab_id: string; format?: string }): Promise<ScreenshotResult> {
  const fmt = data.format || 'png'
  return httpRequest<ScreenshotResult>(`/api/browser/screenshot/${data.port}/${data.tab_id}?format=${fmt}`, {
    method: 'GET',
  })
}

export async function browserClick(data: { port: number; tab_id: string; selector?: string }): Promise<{ success: boolean }> {
  return httpRequest<{ success: boolean }>(`/api/browser/click/${data.port}/${data.tab_id}`, {
    method: 'POST',
    body: JSON.stringify(data),
  })
}

export async function browserFillInput(data: { port: number; tab_id: string; selector?: string; value?: string }): Promise<{ success: boolean }> {
  return httpRequest<{ success: boolean }>(`/api/browser/fill-input/${data.port}/${data.tab_id}`, {
    method: 'POST',
    body: JSON.stringify(data),
  })
}

export async function browserExecuteJs(data: { port: number; tab_id: string; script?: string }): Promise<{ result: unknown }> {
  return httpRequest<{ result: unknown }>(`/api/browser/execute-js/${data.port}/${data.tab_id}`, {
    method: 'POST',
    body: JSON.stringify(data),
  })
}

export async function browserGetInfo(data: { port: number; tab_id: string }): Promise<{ url: string; title: string }> {
  return httpRequest<{ url: string; title: string }>(`/api/browser/info/${data.port}/${data.tab_id}`, { method: 'GET' })
}

export async function browserReload(data: { port: number; tab_id: string }): Promise<{ success: boolean }> {
  return httpRequest<{ success: boolean }>(`/api/browser/reload/${data.port}/${data.tab_id}`, { method: 'GET' })
}

export async function browserCloseTab(data: { port: number; tab_id: string }): Promise<{ success: boolean }> {
  return httpRequest<{ success: boolean }>(`/api/browser/close-tab/${data.port}/${data.tab_id}`, { method: 'POST' })
}
