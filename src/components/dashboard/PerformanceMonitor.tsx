// Claw Desktop - 性能监控面板 - 实时显示系统性能指标、队列状态、健康检查
import React, { useState, useEffect, useCallback } from 'react';
import { useTranslation } from 'react-i18next';
import { getQueueStats, getSystemHealth } from '../../api/system';

interface QueueStats {
  queue: {
    pending: number;
    active: number;
    total_enqueued: number;
    total_completed: number;
    total_failed: number;
    total_cancelled: number;
    avg_wait_time_ms: number;
    peak_queue_size: number;
  };
  semaphore: {
    available_permits: number;
    max_concurrent: number;
  };
  timestamp: string;
}

interface SystemHealth {
  overall_status: string;
  response_time_ms: number;
  timestamp: string;
  components: {
    priority_queue: { status: string; active_tasks?: number; pending_tasks?: number; success_rate?: number };
    database: { status: string };
    memory: { status: string };
  };
  version: string;
  platform: string;
}

interface PerformanceMonitorProps {
  isOpen: boolean;
  onClose: () => void;
}

export const PerformanceMonitor: React.FC<PerformanceMonitorProps> = ({ isOpen, onClose }) => {
  const [queueStats, setQueueStats] = useState<QueueStats | null>(null);
  const [systemHealth, setSystemHealth] = useState<SystemHealth | null>(null);
  const [isMonitoring, setIsMonitoring] = useState(false);
  const { t } = useTranslation()
  const [refreshInterval, setRefreshInterval] = useState(5000)
  const [error, setError] = useState<string | null>(null);
  const [history, setHistory] = useState<Array<{ timestamp: number; pending: number; active: number }>>([]);

  const fetchStats = useCallback(async () => {
    try {
      setError(null);
      const [queueData, healthData] = await Promise.all([
        getQueueStats() as unknown as Record<string, unknown>,
        getSystemHealth() as unknown as Record<string, unknown>
      ]);

      setQueueStats(queueData as any);
      setSystemHealth(healthData as any);

      if (queueData) {
        const qd = queueData as any;
        setHistory(prev => {
          const newEntry = { timestamp: Date.now(), pending: qd.queue?.pending ?? 0, active: qd.queue?.active ?? 0 };
          return [...prev.slice(-59), newEntry];
        });
      }
    } catch (err) {
      console.error('[PerformanceMonitor] Failed to fetch stats:', err);
      setError(err instanceof Error ? err.message : 'Failed to fetch stats');
    }
  }, []);

  useEffect(() => {
    if (!isOpen) {
      setIsMonitoring(false);
      return;
    }

    fetchStats();
    setIsMonitoring(true);

    const interval = setInterval(fetchStats, refreshInterval);
    return () => clearInterval(interval);
  }, [isOpen, refreshInterval, fetchStats]);

  const getStatusColor = (status: string) => {
    switch (status.toLowerCase()) {
      case 'healthy': return '#10b981';
      case 'degraded': return '#f59e0b';
      case 'error': return '#ef4444';
      default: return '#6b7280';
    }
  };

  const getUtilizationPercentage = () => {
    if (!queueStats) return 0;
    const used = queueStats.semaphore.max_concurrent - queueStats.semaphore.available_permits;
    return Math.round((used / queueStats.semaphore.max_concurrent) * 100);
  };

  if (!isOpen) return null;

  return (
    <div className="fixed inset-y-0 right-0 w-[400px] bg-slate-900/98 border-l border-slate-700/50 z-[9999] flex flex-col text-slate-200 text-[13px] font-sans">
      <div className="flex items-center justify-between px-4 py-4 border-b border-slate-700/50 bg-slate-800/80">
        <h2 className="m-0 text-base font-semibold text-slate-100">
          {t('performanceMonitor.title')}
        </h2>
        <button
          onClick={onClose}
          className="w-7 h-7 flex items-center justify-center rounded-lg bg-slate-700/60 hover:bg-red-500/20 text-slate-400 hover:text-red-300 cursor-pointer text-sm transition-all border border-transparent hover:border-red-500/30"
          title="Close"
        >
          ✕
        </button>
      </div>

      <div className="flex-1 overflow-y-auto p-4">
        {error && (
          <div className="bg-red-500/10 border border-red-500/30 rounded-md p-3 mb-4 text-red-300">
            ⚠️ Error: {error}
          </div>
        )}

        <section className="mb-6">
          <h3 className="m-0 mb-3 text-sm font-semibold text-slate-300">
            {t('performanceMonitor.systemHealth')}
          </h3>
          {systemHealth && (
            <div className="rounded-lg p-3 border-2" style={{ background: 'rgba(30, 41, 59, 0.5)', borderColor: getStatusColor(systemHealth.overall_status) }}>
              <div className="flex items-center gap-2 mb-3">
                <span className="w-[10px] h-[10px] rounded-full" style={{ background: getStatusColor(systemHealth.overall_status), animation: isMonitoring ? 'pulse 2s infinite' : 'none' }} />
                <span className="font-semibold capitalize">
                  {systemHealth.overall_status === 'healthy' ? t('performanceMonitor.healthy') : systemHealth.overall_status === 'degraded' ? t('performanceMonitor.degraded') : t('performanceMonitor.error')}
                </span>
                <span className="ml-auto text-slate-400 text-[11px]">
                  {t('performanceMonitor.response', { time: systemHealth.response_time_ms })}
                </span>
              </div>

              <div className="grid grid-cols-2 gap-2 text-[11px]">
                {Object.entries(systemHealth.components).map(([key, value]) => (
                  <div key={key} className="bg-slate-900/50 p-2 rounded">
                    <div className="text-slate-400 mb-1 capitalize">{key.replace('_', ' ')}</div>
                    <div className="flex items-center gap-1.5">
                      <span className="w-2 h-2 rounded-full" style={{ background: getStatusColor(value.status) }} />
                      <span className="capitalize font-medium">{value.status}</span>
                    </div>
                    {'active_tasks' in value && (
                      <div className="mt-1 text-slate-500">
                        Active: {(value as unknown as { active_tasks?: number; pending_tasks?: number }).active_tasks} | Pending: {(value as unknown as { active_tasks?: number; pending_tasks?: number }).pending_tasks}
                      </div>
                    )}
                  </div>
                ))}
              </div>

              <div className="mt-3 pt-3 border-t border-slate-700/30 text-[11px] text-slate-500">
                v{systemHealth.version} • {systemHealth.platform} • {t('performanceMonitor.lastCheck', { time: new Date(systemHealth.timestamp).toLocaleTimeString() })}
              </div>
            </div>
          )}
        </section>

        <section className="mb-6">
          <h3 className="m-0 mb-3 text-sm font-semibold text-slate-300">
            {t('performanceMonitor.queueStatistics')}
          </h3>
          {queueStats && (
            <>
              <div className="bg-slate-800/50 rounded-lg p-3 mb-3">
                <div className="mb-3">
                  <div className="flex justify-between mb-1">
                    <span className="text-slate-400">{t('performanceMonitor.concurrentUsage')}</span>
                    <span className="font-semibold">{getUtilizationPercentage()}%</span>
                  </div>
                  <div className="w-full h-[6px] bg-slate-700/50 rounded overflow-hidden">
                    <div className="h-full rounded-sm transition-all duration-300"
                      style={{
                        width: `${getUtilizationPercentage()}%`,
                        background: getUtilizationPercentage() > 80 ? '#ef4444' : getUtilizationPercentage() > 60 ? '#f59e0b' : '#10b981',
                      }}
                    />
                  </div>
                  <div className="flex justify-between mt-1 text-[11px] text-slate-500">
                    <span>{queueStats.semaphore.max_concurrent - queueStats.semaphore.available_permits} {t('performanceMonitor.active')}</span>
                    <span>{t('performanceMonitor.max')}: {queueStats.semaphore.max_concurrent}</span>
                  </div>
                </div>

                <div className="grid grid-cols-2 gap-2">
                  {[                    { label: t('performanceMonitor.pendingTasks'), value: queueStats.queue.pending, icon: '⏳' },
                    { label: t('performanceMonitor.activeTasksLabel'), value: queueStats.queue.active, icon: '🔄' },
                    { label: t('performanceMonitor.completed'), value: queueStats.queue.total_completed, icon: '✅' },
                    { label: t('performanceMonitor.failed'), value: queueStats.queue.total_failed, icon: '❌' },
                  ].map(({ label, value, icon }) => (
                    <div key={label} className="bg-slate-900/50 p-2 rounded text-center">
                      <div className="text-lg mb-0.5">{icon}</div>
                      <div className="text-base font-bold">{value}</div>
                      <div className="text-[10px] text-slate-500">{label}</div>
                    </div>
                  ))}
                </div>
              </div>

              <div className="bg-slate-800/50 rounded-lg p-3 text-[11px]">
                <div className="grid grid-cols-2 gap-2">
                  <div>
                    <span className="text-slate-500">{t('performanceMonitor.avgWaitTime')}</span>
                    <div className="font-semibold text-sm">{(queueStats.queue.avg_wait_time_ms / 1000).toFixed(2)}s</div>
                  </div>
                  <div>
                    <span className="text-slate-500">{t('performanceMonitor.peakQueueSize')}</span>
                    <div className="font-semibold text-sm">{queueStats.queue.peak_queue_size}</div>
                  </div>
                  <div>
                    <span className="text-slate-500">{t('performanceMonitor.totalEnqueued')}</span>
                    <div className="font-semibold text-sm">{queueStats.queue.total_enqueued}</div>
                  </div>
                  <div>
                    <span className="text-slate-500">{t('performanceMonitor.successRate')}</span>
                    <div className="font-semibold text-sm">
                      {queueStats.queue.total_enqueued > 0
                        ? ((queueStats.queue.total_completed / queueStats.queue.total_enqueued) * 100).toFixed(1)
                        : 100}%
                    </div>
                  </div>
                </div>
              </div>
            </>
          )}
        </section>

        <section className="mb-6">
          <h3 className="m-0 mb-3 text-sm font-semibold text-slate-300">
            {t('performanceMonitor.activityTimeline')}
          </h3>
          <div className="bg-slate-800/50 rounded-lg p-3 h-[120px] relative overflow-hidden">
            {history.length > 1 ? (
              <svg width="100%" height="100%" viewBox={`0 0 ${Math.max(history.length * 4, 376)} 120`} preserveAspectRatio="none">
                <defs>
                  <linearGradient id="pendingGradient" x1="0" y1="0" x2="0" y2="1">
                    <stop offset="0%" stopColor="#3b82f6" stopOpacity="0.3" />
                    <stop offset="100%" stopColor="#3b82f6" stopOpacity="0" />
                  </linearGradient>
                </defs>
                {(() => {
                  const maxVal = Math.max(...history.map(h => h.pending), 1);
                  const points = history.map((h, i) => `${i * (376 / Math.max(history.length - 1, 1))},${120 - (h.pending / maxVal * 100)}`).join(' ');
                  const areaPoints = `0,120 ${points} ${(history.length - 1) * (376 / Math.max(history.length - 1, 1))},120`;

                  return (
                    <>
                      <polygon points={areaPoints} fill="url(#pendingGradient)" />
                      <polyline points={points} fill="none" stroke="#3b82f6" strokeWidth="2" />
                    </>
                  );
                })()}
              </svg>
            ) : (
              <div className="flex items-center justify-center h-full text-slate-500">
                {t('performanceMonitor.collectingData')}
              </div>
            )}
          </div>
        </section>

        <section>
          <h3 className="m-0 mb-3 text-sm font-semibold text-slate-300">
            {t('performanceMonitor.settings')}
          </h3>
          <div className="bg-slate-800/50 rounded-lg p-3">
            <div className="mb-2">
              <label className="block mb-1 text-slate-400 text-[11px]">
                {t('performanceMonitor.refreshInterval', { interval: refreshInterval / 1000 })}
              </label>
              <input
                type="range"
                min="1000"
                max="30000"
                step="1000"
                value={refreshInterval}
                onChange={(e) => setRefreshInterval(Number(e.target.value))}
                className="w-full accent-blue-500"
              />
            </div>
            <div className="flex items-center gap-2 text-[11px] text-slate-400">
              <span className={`w-2 h-2 rounded-full ${isMonitoring ? 'bg-emerald-500 animate-pulse' : 'bg-gray-500'}`} />
              {isMonitoring ? t('performanceMonitor.monitoringActive') : t('performanceMonitor.monitoringPaused')}
            </div>
          </div>
        </section>
      </div>

      <style>{`
        @keyframes pulse {
          0%, 100% { opacity: 1; }
          50% { opacity: 0.5; }
        }
      `}</style>
    </div>
  );
};

export default PerformanceMonitor;
