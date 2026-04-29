// Claw Desktop - Harness仪表盘 - Agent管理、可观测性统计、事件流监控
import React, { useState, useEffect } from 'react'
import { useTranslation } from 'react-i18next'
import { harnessObservabilityGetStats, harnessObservabilityGetEvents } from '../../api/harness'

interface HarnessEvent {
  id: string
  eventType: string
  agentId: string
  correlationId?: string | null
  payload?: string | null
  timestamp: number
  durationMs?: number | null
}

interface HarnessStats {
  totalEvents: number
  eventsByType: [string, number][]
  eventsByAgent: [string, number][]
  errorCount: number
  completionCount: number
  failureCount: number
  successRate: number
  averageTaskDurationMs: number
}

export default function HarnessDashboard() {
  const { t } = useTranslation()
  const [stats, setStats] = useState<HarnessStats | null>(null)
  const [recentEvents, setRecentEvents] = useState<HarnessEvent[]>([])
  const [activeTab, setActiveTab] = useState<'overview' | 'events' | 'agents'>('overview')
  const [loading, setLoading] = useState(true)

  useEffect(() => {
    loadDashboardData()
    const interval = setInterval(loadDashboardData, 30000)
    return () => clearInterval(interval)
  }, [])

  async function loadDashboardData() {
    try {
      const [statsResult, eventsResult] = await Promise.all([
        harnessObservabilityGetStats() as unknown as Record<string, unknown>,
        harnessObservabilityGetEvents({ limit: 50 }) as unknown as Record<string, unknown>
      ])
      const mapped: HarnessStats = {
        totalEvents: (statsResult as any).total_events ?? (statsResult as any).totalEvents ?? 0,
        eventsByType: Array.isArray((statsResult as any).events_by_type) ? (statsResult as any).events_by_type : Array.isArray((statsResult as any).eventsByType) ? (statsResult as any).eventsByType : [],
        eventsByAgent: Array.isArray((statsResult as any).events_by_agent) ? (statsResult as any).events_by_agent : Array.isArray((statsResult as any).eventsByAgent) ? (statsResult as any).eventsByAgent : [],
        errorCount: (statsResult as any).errors ?? (statsResult as any).errorCount ?? 0,
        completionCount: (statsResult as any).tasks_completed ?? (statsResult as any).completionCount ?? 0,
        failureCount: (statsResult as any).tasks_failed ?? (statsResult as any).failureCount ?? 0,
        successRate: (statsResult as any).success_rate ?? (statsResult as any).successRate ?? 0,
        averageTaskDurationMs: (statsResult as any).average_task_duration_ms ?? (statsResult as any).averageTaskDurationMs ?? 0,
      }
      setStats(mapped)
      setRecentEvents((eventsResult as any).events || [])
    } catch (err) {
      console.error('Failed to load dashboard data:', err)
    } finally {
      setLoading(false)
    }
  }

  const eventTypeLabels: Record<string, string> = {
    agent_started: t('panels.harness.eventAgentStarted'),
    agent_stopped: t('panels.harness.eventAgentStopped'),
    session_created: t('panels.harness.eventSessionCreated'),
    task_decomposed: t('panels.harness.eventTaskDecomposed'),
    task_assigned: t('panels.harness.eventTaskAssigned'),
    task_started: t('panels.harness.eventTaskStarted'),
    task_completed: t('panels.harness.eventTaskCompleted'),
    task_failed: t('panels.harness.eventTaskFailed'),
    error_occurred: t('panels.harness.eventErrorOccurred'),
    error_rule_generated: t('panels.harness.eventRuleGenerated'),
    mention_detected: t('panels.harness.eventMentionDetected'),
    validation_performed: t('panels.harness.eventValidationPerformed'),
    memory_stored: t('panels.harness.eventMemoryStored'),
    cross_memory_accessed: t('panels.harness.eventCrossMemoryAccessed'),
  }

  if (loading) {
    return (
      <div className="harness-dashboard">
        <div className="dashboard-loading">{t('panels.harness.loadingData')}</div>
      </div>
    )
  }

  return (
    <div className="harness-dashboard">
      <div className="dashboard-header">
        <h2>{t('panels.harness.title')}</h2>
        <div className="header-actions">
          <button className="refresh-btn" onClick={loadDashboardData}>🔄 {t('panels.harness.refresh')}</button>
        </div>
      </div>

      <div className="tab-bar">
        <button className={activeTab === 'overview' ? 'active' : ''} onClick={() => setActiveTab('overview')}>
          {t('panels.harness.tabOverview')}
        </button>
        <button className={activeTab === 'events' ? 'active' : ''} onClick={() => setActiveTab('events')}>
          {t('panels.harness.tabEvents')} ({recentEvents.length})
        </button>
        <button className={activeTab === 'agents' ? 'active' : ''} onClick={() => setActiveTab('agents')}>
          {t('panels.harness.tabAgents')}
        </button>
      </div>

      {activeTab === 'overview' && stats && (
        <div className="overview-grid">
          <div className="stat-card success-rate">
            <div className="stat-value">{(stats.successRate * 100).toFixed(1)}%</div>
            <div className="stat-label">{t('panels.harness.successRate')}</div>
            <div className="stat-bar">
              <div className="stat-bar-fill" style={{ width: `${stats.successRate * 100}%` }} />
            </div>
          </div>

          <div className="stat-card completions">
            <div className="stat-value">{stats.completionCount}</div>
            <div className="stat-label">{t('panels.harness.completedTasks')}</div>
          </div>

          <div className="stat-card failures">
            <div className="stat-value error">{stats.failureCount}</div>
            <div className="stat-label">{t('panels.harness.failedTasks')}</div>
          </div>

          <div className="stat-card errors">
            <div className="stat-value warn">{stats.errorCount}</div>
            <div className="stat-label">{t('panels.harness.errorEvents')}</div>
          </div>

          <div className="stat-card duration">
            <div className="stat-value">{stats.averageTaskDurationMs > 0 ? `${(stats.averageTaskDurationMs / 1000).toFixed(1)}s` : '-'}</div>
            <div className="stat-label">{t('panels.harness.avgDuration')}</div>
          </div>

          <div className="stat-card total-events">
            <div className="stat-value">{stats.totalEvents}</div>
            <div className="stat-label">{t('panels.harness.totalEvents')}</div>
          </div>

          <div className="chart-section type-distribution">
            <h4>{t('panels.harness.eventTypeDistribution')}</h4>
            <div className="bar-chart">
              {stats.eventsByType.map(([type, count]) => (
                <div key={type} className="chart-row">
                  <span className="row-label">{eventTypeLabels[type] || type}</span>
                  <div className="row-bar-track">
                    <div
                      className="row-bar-fill"
                      style={{
                        width: `${stats.totalEvents > 0 ? (count / stats.totalEvents) * 100 : 0}%`,
                        backgroundColor: getEventTypeColor(type),
                      }}
                    />
                  </div>
                  <span className="row-count">{count}</span>
                </div>
              ))}
            </div>
          </div>
        </div>
      )}

      {activeTab === 'events' && (
        <div className="events-timeline">
          {recentEvents.length === 0 ? (
            <div className="empty-events">{t('panels.harness.noEvents')}</div>
          ) : (
            recentEvents.map((event) => (
              <div key={event.id} className="event-item">
                <span className="event-icon">{eventTypeLabels[event.eventType]?.split(' ')[0] || '•'}</span>
                <div className="event-body">
                  <div className="event-meta">
                    <span className="event-type">{eventTypeLabels[event.eventType]?.slice(2) || event.eventType}</span>
                    <span className="event-agent">@{event.agentId}</span>
                    {event.durationMs != null && (
                      <span className="event-duration">{event.durationMs}ms</span>
                    )}
                  </div>
                  <div className="event-time">
                    {new Date(event.timestamp).toLocaleTimeString()}
                  </div>
                </div>
              </div>
            ))
          )}
        </div>
      )}

      {activeTab === 'agents' && stats && (
        <div className="agent-distribution">
          {stats.eventsByAgent.map(([agentId, count]) => (
            <div key={agentId} className="agent-row">
              <span className="agent-name">{agentId}</span>
              <div className="agent-bar-track">
                <div
                  className="agent-bar-fill"
                  style={{
                    width: `${stats.totalEvents > 0 ? (count / stats.totalEvents) * 100 : 0}%`,
                  }}
                />
              </div>
              <span className="agent-count">{t('panels.harness.eventsCount', { count })}</span>
            </div>
          ))}
        </div>
      )}

      <style>{`
        .harness-dashboard {
          background: var(--surface); border-radius: 12px;
          border: 1px solid rgba(255,255,255,0.06); padding: 20px;
          color: var(--text-primary); font-family: inherit;
        }
        .dashboard-header { display: flex; justify-content: space-between; align-items: center; margin-bottom: 16px; }
        .dashboard-header h2 { margin: 0; font-size: 18px; font-weight: 700; }
        .refresh-btn {
          padding: 6px 14px; border: 1px solid rgba(255,255,255,0.1); border-radius: 8px;
          background: transparent; color: var(--text-secondary); cursor: pointer; font-size: 12px;
          transition: all 0.2s;
        }
        .refresh-btn:hover { background: rgba(255,255,255,0.05); color: var(--text-primary); }

        .tab-bar { display: flex; gap: 4px; margin-bottom: 20px; border-bottom: 1px solid rgba(255,255,255,0.06); padding-bottom: 0; }
        .tab-bar button {
          padding: 8px 16px; border: none; background: none; color: var(--text-secondary);
          cursor: pointer; font-size: 13px; border-bottom: 2px solid transparent;
          transition: all 0.2s;
        }
        .tab-bar button.active { color: var(--text-primary); border-bottom-color: #818CF8; }
        .tab-bar button:hover:not(.active) { color: var(--text-primary); opacity: 0.7; }

        .overview-grid {
          display: grid; grid-template-columns: repeat(auto-fill, minmax(160px, 1fr)); gap: 12px;
        }
        .stat-card {
          background: rgba(255,255,255,0.03); border: 1px solid rgba(255,255,255,0.06);
          border-radius: 10px; padding: 14px; position: relative; overflow: hidden;
        }
        .stat-value { font-size: 24px; font-weight: 700; line-height: 1.2; }
        .stat-value.error { color: #F87171; }
        .stat-value.warn { color: #FBBF24; }
        .stat-label { font-size: 11px; color: var(--text-secondary); margin-top: 4px; }
        .stat-bar { height: 3px; background: rgba(255,255,255,0.08); border-radius: 2px; margin-top: 8px; overflow: hidden; }
        .stat-bar-fill { height: 100%; background: linear-gradient(90deg, #34D399, #10B981); border-radius: 2px; transition: width 0.5s ease; }

        .chart-section { grid-column: 1 / -1; padding: 14px; }
        .chart-section h4 { margin: 0 0 12px; font-size: 13px; font-weight: 600; }
        .bar-chart { display: flex; flex-direction: column; gap: 6px; }
        .chart-row { display: flex; align-items: center; gap: 8px; font-size: 12px; }
        .row-label { width: 120px; flex-shrink: 0; opacity: 0.8; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
        .row-bar-track { flex: 1; height: 16px; background: rgba(255,255,255,0.04); border-radius: 4px; overflow: hidden; }
        .row-bar-fill { height: 100%; border-radius: 4px; min-width: 2px; transition: width 0.3s ease; }
        .row-count { width: 32px; text-align: right; font-weight: 600; font-variant-numeric: tabular-nums; }

        .events-timeline { display: flex; flex-direction: column; gap: 2px; max-height: 500px; overflow-y: auto; }
        .event-item { display: flex; gap: 10px; padding: 8px 10px; border-radius: 6px; transition: background 0.15s; }
        .event-item:hover { background: rgba(255,255,255,0.03); }
        .event-icon { font-size: 16px; line-height: 1; flex-shrink: 0; }
        .event-body { flex: 1; min-width: 0; }
        .event-meta { display: flex; gap: 10px; align-items: center; font-size: 12px; }
        .event-type { color: #818CF8; font-weight: 500; }
        .event-agent { color: var(--text-secondary); }
        .event-duration { color: var(--text-muted); margin-left: auto; }
        .event-time { font-size: 11px; color: var(--text-muted); }

        .empty-events, .dashboard-loading { text-align: center; padding: 40px; opacity: 0.5; }

        .agent-distribution { display: flex; flex-direction: column; gap: 8px; }
        .agent-row { display: flex; align-items: center; gap: 10px; font-size: 13px; }
        .agent-name { width: 120px; flex-shrink: 0; font-family: monospace; font-size: 12px; }
        .agent-bar-track { flex: 1; height: 20px; background: rgba(255,255,255,0.04); border-radius: 4px; overflow: hidden; }
        .agent-bar-fill { height: 100%; background: linear-gradient(90deg, #818CF8, #A78BFA); border-radius: 4px; min-width: 2px; transition: width 0.3s ease; }
        .agent-count { width: 50px; text-align: right; font-variant-numeric: tabular-nums; color: var(--text-secondary); }
      `}</style>
    </div>
  )
}

function getEventTypeColor(type: string): string {
  const colors: Record<string, string> = {
    task_completed: '#10B981', task_failed: '#EF4444', error_occurred: '#F59E0B',
    error_rule_generated: '#8B5CF6', task_decomposed: '#06B6D4', mention_detected: '#EC4899',
  }
  return colors[type] || '#6366F1'
}
