// Claw Desktop - 定时任务（Cron）API
// 提供定时任务的增删改查、手动触发等接口
import { httpGet, httpPost } from '../ws/http';

/** 定时任务定义 */
export interface CronJob {
  id: string;                      // 任务唯一 ID
  name: string;                    // 任务名称
  schedule: string;                // Cron 表达式（如 "0 9 * * 1-5"）
  prompt: string;                  // 要执行的提示词/任务描述
  agent_id?: string;               // 关联 Agent ID（可选）
  delivery_channel_id?: string;    // 结果投递渠道 ID（可选）
  delivery_chat_id?: string;       // 结果投递聊天 ID（可选）
  enabled: boolean;                // 是否启用
  silent_on_empty: boolean;        // 无结果时是否静默（不发送通知）
  last_run_at?: number;            // 上次执行时间戳
  next_run_at?: number;            // 下次执行时间戳
  run_count: number;               // 累计执行次数
  last_result?: string;            // 上次执行结果
  created_at: number;              // 创建时间戳
  updated_at: number;              // 更新时间戳
}

/** 定时任务 API 集合 */
export const cronApi = {
  /** 获取所有定时任务列表 */
  list: () => httpGet<CronJob[]>('/api/cron'),
  /** 创建新的定时任务 */
  create: (job: Partial<CronJob>) => httpPost<CronJob>('/api/cron', job),
  /** 更新定时任务配置 */
  update: (job: Partial<CronJob>) => httpPost<CronJob>('/api/cron/update', job),
  /** 删除定时任务 */
  delete: (id: string) => httpPost<{ deleted: boolean }>('/api/cron/delete', { id }),
  /** 手动触发定时任务执行 */
  trigger: (id: string) => httpPost<{ triggered: boolean }>('/api/cron/trigger', { id }),
};
