// Claw Desktop - 调试日志工具 - 提供开发环境下的调试日志输出
let _debugEnabled = false

export function setDebug(enabled: boolean): void {
  _debugEnabled = enabled
}

export function isDebug(): boolean {
  return _debugEnabled || (typeof globalThis !== 'undefined' && (globalThis as Record<string, unknown>)?.__DEV__ === true)
}

export function debugLog(...args: unknown[]): void {
  if (_debugEnabled) {
    console.log('[Debug]', ...args)
  }
}

export function clearDebugLogs(): void {
  if (!_debugEnabled) return
}
