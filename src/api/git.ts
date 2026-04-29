// Claw Desktop - Git操作API
// 提供仓库状态查询、差异对比、提交、分支管理、暂存等HTTP接口
import { httpRequest } from '../ws/http'

/** Git状态结果 — 当前分支、暂存/未暂存/未跟踪文件列表 */
export interface GitStatusResult {
  branch: string
  staged: string[]
  unstaged: string[]
  untracked: string[]
  [key: string]: unknown
}

export interface GitDiffResult {
  diff: string
  files: string[]
  [key: string]: unknown
}

export interface GitLogEntry {
  hash: string
  message: string
  author: string
  date: string
  [key: string]: unknown
}

export async function gitStatus(data?: unknown): Promise<GitStatusResult> {
  return httpRequest<GitStatusResult>('/api/git/status', {
    method: 'POST',
    ...(data ? { body: JSON.stringify(data) } : {}),
  })
}

export async function gitDiff(data?: unknown): Promise<GitDiffResult> {
  return httpRequest<GitDiffResult>('/api/git/diff', {
    method: 'POST',
    ...(data ? { body: JSON.stringify(data) } : {}),
  })
}

export async function gitCommit(data: unknown): Promise<{ success: boolean; hash?: string }> {
  return httpRequest<{ success: boolean; hash?: string }>('/api/git/commit', {
    method: 'POST',
    body: JSON.stringify(data),
  })
}

export async function gitLog(data?: unknown): Promise<{ entries: GitLogEntry[] }> {
  return httpRequest<{ entries: GitLogEntry[] }>('/api/git/log', {
    method: 'POST',
    ...(data ? { body: JSON.stringify(data) } : {}),
  })
}

export async function gitBranchList(data?: unknown): Promise<{ branches: string[]; current: string }> {
  return httpRequest<{ branches: string[]; current: string }>('/api/git/branch-list', {
    method: 'POST',
    ...(data ? { body: JSON.stringify(data) } : {}),
  })
}

export async function gitCreateBranch(data: unknown): Promise<{ success: boolean }> {
  return httpRequest<{ success: boolean }>('/api/git/create-branch', {
    method: 'POST',
    body: JSON.stringify(data),
  })
}

export async function gitCheckoutBranch(data: unknown): Promise<{ success: boolean }> {
  return httpRequest<{ success: boolean }>('/api/git/checkout-branch', {
    method: 'POST',
    body: JSON.stringify(data),
  })
}

export async function gitStash(data?: unknown): Promise<{ success: boolean }> {
  return httpRequest<{ success: boolean }>('/api/git/stash', {
    method: 'POST',
    ...(data ? { body: JSON.stringify(data) } : {}),
  })
}

export async function gitStashPop(data?: unknown): Promise<{ success: boolean }> {
  return httpRequest<{ success: boolean }>('/api/git/stash-pop', {
    method: 'POST',
    ...(data ? { body: JSON.stringify(data) } : {}),
  })
}

export async function gitAdd(data: unknown): Promise<{ success: boolean }> {
  return httpRequest<{ success: boolean }>('/api/git/add', {
    method: 'POST',
    body: JSON.stringify(data),
  })
}

export async function gitReset(data: unknown): Promise<{ success: boolean }> {
  return httpRequest<{ success: boolean }>('/api/git/reset', {
    method: 'POST',
    body: JSON.stringify(data),
  })
}

export async function gitIsRepository(data?: unknown): Promise<{ is_repo: boolean }> {
  return httpRequest<{ is_repo: boolean }>('/api/git/is-repository', {
    method: 'POST',
    ...(data ? { body: JSON.stringify(data) } : {}),
  })
}
