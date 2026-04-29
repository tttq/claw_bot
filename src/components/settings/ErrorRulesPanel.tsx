// Claw Desktop - 错误规则面板 - 管理错误学习规避规则的查看和触发统计
import React, { useState } from 'react'
import { useErrorLearning } from '../../hooks/useErrorLearning'
import { useTranslation } from 'react-i18next'

interface ErrorRulesPanelProps {
  agentId: string
  compact?: boolean
}

const CATEGORY_COLORS: Record<string, string> = {
  api: '#EF4444',
  tool: '#F59E0B',
  logic: '#8B5CF6',
  context: '#06B6D4',
  validation: '#10B981',
  other: '#6B7280',
}

export default function ErrorRulesPanel({ agentId, compact = false }: ErrorRulesPanelProps) {
  const { t } = useTranslation()
  const { rules, loading, stats, captureError, triggerHit } = useErrorLearning(agentId)
  const [showAddRule, setShowAddRule] = useState(false)

  const getCategoryLabel = (category: string) => {
    const labels: Record<string, string> = {
      api: t('errorRules.categoryApi'),
      tool: t('errorRules.categoryTool'),
      logic: t('errorRules.categoryLogic'),
      context: t('errorRules.categoryContext'),
      validation: t('errorRules.categoryValidation'),
      other: t('errorRules.categoryOther'),
    }
    return labels[category] || category
  }

  if (compact) {
    return (
      <div className="error-rules-compact">
        <span className="rule-count" title={t('errorRules.ruleCountTitle')}>
          🛡️ {stats.totalRules} {t('errorRules.rules')}
        </span>
        {stats.activeRules > 0 && (
          <span className="active-count" style={{ color: '#10B981' }}>
            {stats.activeRules} {t('errorRules.active')}
          </span>
        )}
      </div>
    )
  }

  return (
    <div className="error-rules-panel">
      <div className="panel-header">
        <h3>{t('errorRules.title')}</h3>
        <div className="header-stats">
          <span className="stat-badge total">{stats.totalRules} {t('errorRules.total')}</span>
          <span className="stat-badge active">{stats.activeRules} {t('errorRules.activeBadge')}</span>
          {stats.deprecatedRules > 0 && (
            <span className="stat-badge deprecated">{stats.deprecatedRules} {t('errorRules.deprecated')}</span>
          )}
        </div>
      </div>

      {loading ? (
        <div className="loading-state">{t('errorRules.loading')}</div>
      ) : rules.length === 0 ? (
        <div className="empty-state">
          <p>{t('errorRules.noRules')}</p>
          <p className="hint">{t('errorRules.noRulesHint')}</p>
        </div>
      ) : (
        <div className="rules-list">
          {rules.map((rule) => (
            <div key={rule.id} className={`rule-card ${rule.isDeprecated ? 'deprecated' : ''}`}>
              <div className="rule-header">
                <span
                  className="category-dot"
                  style={{ backgroundColor: CATEGORY_COLORS[rule.category] || CATEGORY_COLORS.other }}
                />
                <span className="category-label">{getCategoryLabel(rule.category)}</span>
                <span className="trigger-count">×{rule.triggerCount}</span>
                {rule.isDeprecated && <span className="deprecated-badge">{t('errorRules.deprecatedBadge')}</span>}
              </div>

              <div className="rule-pattern">{rule.pattern}</div>

              {!compact && (
                <>
                  <div className="rule-detail">
                    <div className="detail-item">
                      <span className="detail-label">{t('errorRules.causeLabel')}</span>
                      <span className="detail-value">{rule.cause}</span>
                    </div>
                    <div className="detail-item">
                      <span className="detail-label">{t('errorRules.fixLabel')}</span>
                      <span className="detail-value fix">{rule.fix}</span>
                    </div>
                  </div>

                  <div className="rule-actions">
                    <button
                      className="trigger-btn"
                      onClick={() => triggerHit(rule.id)}
                      title={t('errorRules.triggerTitle')}
                    >
                      {t('errorRules.triggerBtn')}
                    </button>
                  </div>
                </>
              )}
            </div>
          ))}
        </div>
      )}

      <style>{`
        .error-rules-panel {
          background: var(--surface);
          border-radius: 12px;
          border: 1px solid rgba(255,255,255,0.06);
          padding: 16px;
          font-size: 13px;
          color: var(--text-primary);
        }
        .panel-header {
          display: flex;
          justify-content: space-between;
          align-items: center;
          margin-bottom: 12px;
        }
        .panel-header h3 { margin: 0; font-size: 15px; font-weight: 600; }
        .header-stats { display: gap: 6px; }
        .stat-badge {
          padding: 2px 8px; border-radius: 10px; font-size: 11px; font-weight: 500;
        }
        .stat-badge.total { background: rgba(99,102,241,0.15); color: #818CF8; }
        .stat-badge.active { background: rgba(16,185,129,0.15); color: #34D399; }
        .stat-badge.deprecated { background: rgba(107,114,128,0.15); color: #9CA3AF; }

        .rules-list { display: flex; flex-direction: column; gap: 8px; max-height: 400px; overflow-y: auto; }
        .rule-card {
          padding: 10px 12px; border-radius: 8px; background: rgba(255,255,255,0.03);
          border: 1px solid rgba(255,255,255,0.05); transition: all 0.2s;
        }
        .rule-card:hover { border-color: rgba(255,255,255,0.12); background: rgba(255,255,255,0.05); }
        .rule-card.deprecated { opacity: 0.5; }

        .rule-header { display: flex; align-items: center; gap: 6px; margin-bottom: 6px; }
        .category-dot { width: 8px; height: 8px; border-radius: 50%; flex-shrink: 0; }
        .category-label { font-size: 11px; font-weight: 500; text-transform: uppercase; opacity: 0.7; }
        .trigger-count { margin-left: auto; font-size: 11px; opacity: 0.5; }

        .rule-pattern { font-size: 13px; line-height: 1.4; margin-bottom: 6px; }
        .rule-detail { display: flex; flex-direction: column; gap: 4px; margin-top: 8px; padding-top: 8px; border-top: 1px solid rgba(255,255,255,0.06); }
        .detail-item { display: flex; gap: 6px; font-size: 12px; }
        .detail-label { color: var(--text-secondary); white-space: nowrap; font-weight: 500; }
        .detail-value { color: var(--text-primary); opacity: 0.85; }
        .detail-value.fix { color: #34D399; }

        .rule-actions { margin-top: 8px; display: flex; justify-content: flex-end; }
        .trigger-btn {
          padding: 4px 12px; border: none; border-radius: 6px; background: rgba(16,185,129,0.1);
          color: #34D399; cursor: pointer; font-size: 11px; transition: all 0.2s;
        }
        .trigger-btn:hover { background: rgba(16,185,129,0.2); }

        .loading-state, .empty-state { text-align: center; padding: 24px; opacity: 0.5; }
        .empty-state .hint { font-size: 12px; margin-top: 4px; }

        .error-rules-compact { display: flex; align-items: center; gap: 8px; font-size: 12px; }
      `}</style>
    </div>
  )
}
