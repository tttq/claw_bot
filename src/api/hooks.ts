// Claw Desktop - Hook 钩子管理 API
// 提供事件钩子的增删查接口，用于在特定事件触发时执行自定义处理逻辑
import { httpGet, httpPost } from '../ws/http';

/** Hook 钩子定义 */
export interface HookDefinition {
  id: string;                          // 钩子唯一 ID
  name: string;                        // 钩子名称
  event: string;                       // 监听的事件类型
  pattern?: string;                    // 匹配模式（可选，正则表达式）
  handler_type: string;                // 处理器类型（如 "prompt", "transform"）
  handler_config: Record<string, unknown>; // 处理器配置参数
  priority: number;                    // 优先级（数值越小越先执行）
  enabled: boolean;                    // 是否启用
}

/** Hook 钩子 API 集合 */
export const hookApi = {
  /** 获取所有已注册的钩子列表 */
  list: () => httpGet<HookDefinition[]>('/api/hooks'),
  /** 创建新的钩子 */
  create: (hook: Partial<HookDefinition>) => httpPost<HookDefinition>('/api/hooks', hook),
  /** 删除指定钩子 */
  delete: (id: string) => httpPost<{ deleted: boolean }>('/api/hooks/delete', { id }),
};
