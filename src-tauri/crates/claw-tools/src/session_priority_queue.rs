// Claw Desktop - 会话优先队列 - 按优先级调度Agent会话
use std::collections::BinaryHeap;
use std::sync::{Arc, Mutex};
use std::time::Instant;
use serde::{Serialize, Deserialize};

/// 会话任务 — 优先队列中的任务单元
#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct SessionTask {
    pub conversation_id: String,
    pub priority: u64,
    pub created_at_instant: Instant,
    pub created_at_timestamp: u64,
    pub content: String,
    pub status: TaskStatus,
}

/// 任务状态枚举
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum TaskStatus {
    Pending,
    Processing,
    Completed,
    Failed,
    Cancelled,
}

impl Eq for SessionTask {}

impl PartialEq for SessionTask {
    fn eq(&self, other: &Self) -> bool {
        self.conversation_id == other.conversation_id
    }
}

impl Ord for SessionTask {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        other.priority.cmp(&self.priority)
            .then_with(|| self.created_at_instant.cmp(&other.created_at_instant))
    }
}

impl PartialOrd for SessionTask {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

/// 优先队列 — 按优先级调度Agent会话任务，控制最大并发数
#[allow(dead_code)]
pub struct PriorityQueue {
    heap: BinaryHeap<SessionTask>,
    active_tasks: Vec<String>,
    max_concurrent: usize,
    stats: QueueStats,
}

/// 队列统计信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueueStats {
    pub total_enqueued: u64,
    pub total_completed: u64,
    pub total_failed: u64,
    pub total_cancelled: u64,
    pub avg_wait_time_ms: u64,
    pub peak_queue_size: usize,
    pub current_queue_size: usize,
    pub active_count: usize,
}

impl Default for QueueStats {
    fn default() -> Self {
        Self {
            total_enqueued: 0,
            total_completed: 0,
            total_failed: 0,
            total_cancelled: 0,
            avg_wait_time_ms: 0,
            peak_queue_size: 0,
            current_queue_size: 0,
            active_count: 0,
        }
    }
}

#[allow(dead_code)]
impl PriorityQueue {
    /// 创建优先队列，指定最大并发数
    pub fn new(max_concurrent: usize) -> Self {
        Self {
            heap: BinaryHeap::new(),
            active_tasks: Vec::new(),
            max_concurrent,
            stats: QueueStats::default(),
        }
    }

    /// 入队任务 — 将会话任务加入优先队列
    pub fn enqueue(&mut self, conversation_id: String, content: String) -> Result<(), String> {
        let now = Instant::now();
        let task = SessionTask {
            conversation_id: conversation_id.clone(),
            priority: now.elapsed().as_nanos() as u64,
            created_at_instant: now,
            created_at_timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as u64,
            content,
            status: TaskStatus::Pending,
        };

        let conv_id_for_log = claw_types::truncate_str_safe(&conversation_id, 16).to_string();
        self.heap.push(task.clone());
        self.stats.total_enqueued += 1;
        self.stats.current_queue_size = self.heap.len();
        
        if self.stats.current_queue_size > self.stats.peak_queue_size {
            self.stats.peak_queue_size = self.stats.current_queue_size;
        }

        log::info!("[PriorityQueue] Enqueued task for conv={}, queue_size={}, priority={}",
            conv_id_for_log,
            self.stats.current_queue_size,
            task.priority
        );

        Ok(())
    }

    /// 出队任务 — 取出优先级最高的待处理任务，受最大并发数限制
    pub fn dequeue(&mut self) -> Option<SessionTask> {
        if self.active_tasks.len() >= self.max_concurrent {
            log::warn!("[PriorityQueue] Max concurrent reached ({}), cannot dequeue", self.max_concurrent);
            return None;
        }

        while let Some(mut task) = self.heap.pop() {
            if !self.active_tasks.contains(&task.conversation_id) {
                task.status = TaskStatus::Processing;
                let conv_id = task.conversation_id.clone();
                self.active_tasks.push(conv_id);
                self.stats.active_count = self.active_tasks.len();
                self.stats.current_queue_size = self.heap.len();

                let wait_time = task.created_at_instant.elapsed().as_millis() as u64;
                self.update_avg_wait_time(wait_time);

                log::info!("[PriorityQueue] Dequeued task for conv={}, wait={}ms, active={}",
                    claw_types::truncate_str_safe(&task.conversation_id, 16),
                    wait_time,
                    self.stats.active_count
                );

                return Some(task);
            }
        }

        None
    }

    /// 标记任务完成 — 从活跃列表中移除
    pub fn complete_task(&mut self, conversation_id: &str) {
        if let Some(pos) = self.active_tasks.iter().position(|id| id == conversation_id) {
            self.active_tasks.remove(pos);
            self.stats.total_completed += 1;
            self.stats.active_count = self.active_tasks.len();

            log::info!("[PriorityQueue] Completed conv={}, remaining_active={}",
                claw_types::truncate_str_safe(&conversation_id, 16),
                self.stats.active_count
            );
        }
    }

    /// 标记任务失败 — 从活跃列表中移除并记录失败统计
    pub fn fail_task(&mut self, conversation_id: &str) {
        if let Some(pos) = self.active_tasks.iter().position(|id| id == conversation_id) {
            self.active_tasks.remove(pos);
            self.stats.total_failed += 1;
            self.stats.active_count = self.active_tasks.len();

            log::warn!("[PriorityQueue] Failed conv={}, remaining_active={}",
                claw_types::truncate_str_safe(&conversation_id, 16),
                self.stats.active_count
            );
        }
    }

    /// 取消任务 — 从队列和活跃列表中移除
    pub fn cancel_task(&mut self, conversation_id: &str) -> bool {
        let initial_len = self.heap.len();
        self.heap.retain(|task| {
            task.conversation_id != conversation_id || task.status != TaskStatus::Pending
        });
        
        let removed_from_heap = self.heap.len() < initial_len;

        if let Some(pos) = self.active_tasks.iter().position(|id| id == conversation_id) {
            self.active_tasks.remove(pos);
            self.stats.total_cancelled += 1;
            self.stats.active_count = self.active_tasks.len();
            true
        } else {
            removed_from_heap
        }
    }

    /// 获取队列统计信息
    pub fn get_stats(&self) -> QueueStats {
        self.stats.clone()
    }

    /// 获取待处理任务数量
    pub fn get_pending_count(&self) -> usize {
        self.heap.len()
    }

    /// 获取活跃任务数量
    pub fn get_active_count(&self) -> usize {
        self.active_tasks.len()
    }

    /// 更新平均等待时间 — 增量计算
    fn update_avg_wait_time(&mut self, new_wait: u64) {
        let total = self.stats.avg_wait_time_ms * self.stats.total_completed;
        self.stats.avg_wait_time_ms = (total + new_wait) / (self.stats.total_completed + 1).max(1);
    }

    /// 获取指定会话在队列中的位置（从1开始）
    pub fn get_position_in_queue(&self, conversation_id: &str) -> Option<usize> {
        self.heap.iter()
            .position(|t| t.conversation_id == conversation_id)
            .map(|pos| pos + 1)
    }

    /// 清空队列和活跃列表
    pub fn clear_all(&mut self) {
        self.heap.clear();
        self.active_tasks.clear();
        self.stats.active_count = 0;
        self.stats.current_queue_size = 0;
    }
}

/// 共享优先队列类型 — 线程安全的Arc<Mutex>包装
pub type SharedPriorityQueue = Arc<Mutex<PriorityQueue>>;

/// 创建共享优先队列 — 返回Arc<Mutex>包装的PriorityQueue实例
pub fn create_shared_queue(max_concurrent: usize) -> SharedPriorityQueue {
    Arc::new(Mutex::new(PriorityQueue::new(max_concurrent)))
}
