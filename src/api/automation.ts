// Claw Desktop - 桌面自动化API
// 提供CUA指令执行、屏幕截图/OCR、鼠标键盘控制、窗口管理、应用启动等HTTP接口
import { httpRequest } from '../ws/http'

/** CUA单步执行结果 */
export interface CuaStepResult {
  step: number
  action: string
  success: boolean
  screenshot_before?: string
  screenshot_after?: string
  reasoning?: string
  error?: string
}

export interface CuaExecutionResult {
  success: boolean
  instruction: string
  total_steps: number
  elapsed_ms: number
  steps: CuaStepResult[]
  error?: string
}

export interface WindowInfo {
  title: string
  process_id: number
  window_id: number
  rect?: {
    x: number
    y: number
    width: number
    height: number
  }
}

export interface AppInfo {
  name: string
  executable_path: string
  description?: string
  publisher?: string
  version?: string
  launch_command?: string
}

/** 执行CUA指令 — 多步骤桌面自动化 */
export async function executeCuaInstruction(instruction: string): Promise<CuaExecutionResult> {
  return httpRequest<CuaExecutionResult>('/api/automation/cua-execute', {
    method: 'POST',
    body: JSON.stringify({ instruction }),
  })
}

/** 执行自动化指令 — 单步桌面操作 */
export async function executeAutomationInstruction(instruction: string): Promise<Record<string, unknown>> {
  return httpRequest<Record<string, unknown>>('/api/automation/execute', {
    method: 'POST',
    body: JSON.stringify({ instruction }),
  })
}

/** 截取屏幕截图 — 返回Base64编码的截图 */
export async function captureScreen(): Promise<string> {
  const result = await httpRequest<{ data: string }>('/api/automation/capture-screen', { method: 'GET' })
  return result.data
}

/** OCR识别屏幕 — 返回识别结果 */
export async function ocrRecognizeScreen(language?: string): Promise<Record<string, unknown>> {
  return httpRequest<Record<string, unknown>>('/api/automation/ocr', {
    method: 'POST',
    body: JSON.stringify({ language }),
  })
}

/** 鼠标左键单击 */
export async function mouseClick(x: number, y: number): Promise<void> {
  await httpRequest<{ success: boolean }>('/api/automation/mouse/click', {
    method: 'POST',
    body: JSON.stringify({ x, y }),
  })
}

/** 鼠标左键双击 */
export async function mouseDoubleClick(x: number, y: number): Promise<void> {
  await httpRequest<{ success: boolean }>('/api/automation/mouse/double-click', {
    method: 'POST',
    body: JSON.stringify({ x, y }),
  })
}

/** 鼠标右键单击 */
export async function mouseRightClick(x: number, y: number): Promise<void> {
  await httpRequest<{ success: boolean }>('/api/automation/mouse/right-click', {
    method: 'POST',
    body: JSON.stringify({ x, y }),
  })
}

/** 鼠标滚轮滚动 */
export async function mouseScroll(amount: number): Promise<void> {
  await httpRequest<{ success: boolean }>('/api/automation/mouse/scroll', {
    method: 'POST',
    body: JSON.stringify({ amount }),
  })
}

/** 鼠标拖拽 */
export async function mouseDrag(fromX: number, fromY: number, toX: number, toY: number): Promise<void> {
  await httpRequest<{ success: boolean }>('/api/automation/mouse/drag', {
    method: 'POST',
    body: JSON.stringify({ fromX, fromY, toX, toY }),
  })
}

/** 键盘输入文本 */
export async function keyboardType(text: string): Promise<void> {
  await httpRequest<{ success: boolean }>('/api/automation/keyboard/type', {
    method: 'POST',
    body: JSON.stringify({ text }),
  })
}

/** 键盘按键 */
export async function keyboardPress(key: string): Promise<void> {
  await httpRequest<{ success: boolean }>('/api/automation/keyboard/press', {
    method: 'POST',
    body: JSON.stringify({ key }),
  })
}

/** 获取活动窗口信息 */
export async function getActiveWindow(): Promise<Record<string, unknown>> {
  return httpRequest<Record<string, unknown>>('/api/automation/window/active', { method: 'GET' })
}

/** 获取窗口标题 */
export async function getWindowTitle(): Promise<Record<string, unknown>> {
  return httpRequest<Record<string, unknown>>('/api/automation/window/title', { method: 'GET' })
}

/** 列出所有窗口 */
export async function listWindows(): Promise<Record<string, unknown>> {
  return httpRequest<Record<string, unknown>>('/api/automation/window/list', { method: 'GET' })
}

/** 聚焦窗口 */
export async function focusWindow(titleContains: string): Promise<Record<string, unknown>> {
  return httpRequest<Record<string, unknown>>('/api/automation/window/focus', {
    method: 'POST',
    body: JSON.stringify({ titleContains }),
  })
}

/** 获取屏幕尺寸 */
export async function getScreenSize(): Promise<Record<string, unknown>> {
  return httpRequest<Record<string, unknown>>('/api/automation/screen/size', { method: 'GET' })
}

/** 列出已安装应用 */
export async function listInstalledApps(filter?: string): Promise<Record<string, unknown>> {
  const url = filter
    ? `/api/automation/apps/list?filter=${encodeURIComponent(filter)}`
    : '/api/automation/apps/list'
  return httpRequest<Record<string, unknown>>(url, { method: 'GET' })
}

/** 启动应用 */
export async function launchApplication(name: string): Promise<Record<string, unknown>> {
  return httpRequest<Record<string, unknown>>('/api/automation/apps/launch', {
    method: 'POST',
    body: JSON.stringify({ name }),
  })
}

/** 获取自动化配置 */
export async function getAutomationConfig(): Promise<Record<string, unknown>> {
  return httpRequest<Record<string, unknown>>('/api/automation/config', { method: 'GET' })
}

/** 初始化ManoP模型 */
export async function initManoPModel(): Promise<Record<string, unknown>> {
  return httpRequest<Record<string, unknown>>('/api/automation/manop/init', { method: 'POST' })
}

/** 获取ManoP状态 */
export async function getManoPStatus(): Promise<Record<string, unknown>> {
  return httpRequest<Record<string, unknown>>('/api/automation/manop/status', { method: 'GET' })
}

/** 下载ManoP模型 */
export async function downloadManoPModel(version?: string): Promise<Record<string, unknown>> {
  return httpRequest<Record<string, unknown>>('/api/automation/manop/download', {
    method: 'POST',
    body: JSON.stringify({ version }),
  })
}

/** 执行ManoP指令 */
export async function executeManoPInstruction(instruction: string): Promise<Record<string, unknown>> {
  return httpRequest<Record<string, unknown>>('/api/automation/manop/execute', {
    method: 'POST',
    body: JSON.stringify({ instruction }),
  })
}

/** 配置ManoP云端 */
export async function configureManoPCloud(apiUrl: string, apiKey?: string): Promise<Record<string, unknown>> {
  return httpRequest<Record<string, unknown>>('/api/automation/manop/configure-cloud', {
    method: 'POST',
    body: JSON.stringify({ apiUrl, apiKey }),
  })
}

/** 搜索应用 */
export async function searchApps(query: string): Promise<Record<string, unknown>> {
  return httpRequest<Record<string, unknown>>(`/api/automation/apps/search?query=${encodeURIComponent(query)}`, { method: 'GET' })
}

/** 查找应用 */
export async function findApp(name: string): Promise<Record<string, unknown>> {
  return httpRequest<Record<string, unknown>>(`/api/automation/apps/find?name=${encodeURIComponent(name)}`, { method: 'GET' })
}

/** 刷新应用索引 */
export async function refreshAppIndex(): Promise<Record<string, unknown>> {
  return httpRequest<Record<string, unknown>>('/api/automation/apps/refresh-index', { method: 'POST' })
}
