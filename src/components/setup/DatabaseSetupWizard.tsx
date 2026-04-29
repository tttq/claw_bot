// Claw Desktop - 数据库设置向导组件
// 引导用户选择数据库后端（SQLite/PostgreSQL/Qdrant）、配置连接参数、
// 测试连接、初始化数据库表结构，并提供数据库状态面板
import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import {
  getDatabaseStatus,
  initializeDatabase,
  testDatabaseConnection,
  updateDatabaseConfig,
  type DatabaseSettings,
} from '../../api/database'

/** 数据库设置向导属性 */
interface DatabaseSetupWizardProps {
  onComplete: () => void
}

/** 设置步骤类型：选择后端 → 配置参数 → 测试连接 → 初始化 → 完成 */
type Step = 'select' | 'configure' | 'testing' | 'initializing' | 'done'

/** 数据库设置向导组件 — 分步引导用户选择和配置数据库后端 */
export function DatabaseSetupWizard({ onComplete }: DatabaseSetupWizardProps) {
  const { t } = useTranslation()
  const [step, setStep] = useState<Step>('select')
  const [selectedBackend, setSelectedBackend] = useState<string>('sqlite')
  const [error, setError] = useState<string>('')
  const [initResult, setInitResult] = useState<{ tables_created: string[]; vector_support: boolean } | null>(null)

  const [postgresConfig, setPostgresConfig] = useState({
    host: 'localhost',
    port: 5432,
    database: 'claw_desktop',
    username: '',
    password: '',
    pool_size: 5,
  })

  const [qdrantConfig, setQdrantConfig] = useState({
    url: 'http://localhost:6333',
    api_key: '',
    collection: 'claw_vectors',
  })

  const [sqliteConfig, setSqliteConfig] = useState({
    enable_vec: true,
    db_path: '',
  })

  const backends = [
    {
      id: 'sqlite',
      name: 'SQLite + sqlite-vec',
      desc: t('databaseSetup.sqliteDesc'),
      icon: '🗄️',
      recommended: true,
    },
    {
      id: 'postgres',
      name: 'PostgreSQL + pgvector',
      desc: t('databaseSetup.postgresDesc'),
      icon: '🐘',
      recommended: false,
    },
    {
      id: 'qdrant',
      name: 'Qdrant',
      desc: t('databaseSetup.qdrantDesc'),
      icon: '🎯',
      recommended: false,
    },
  ]

  /** 测试数据库连接 */
  const handleTestConnection = async () => {
    setStep('testing')
    setError('')
    try {
      let params: { backend: string; [key: string]: unknown } = { backend: selectedBackend }
      if (selectedBackend === 'postgres') {
        params = { ...params, ...postgresConfig }
      } else if (selectedBackend === 'qdrant') {
        params = { ...params, ...qdrantConfig }
      }
      await testDatabaseConnection(params)
      setStep('configure')
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e))
      setStep('configure')
    }
  }

  /** 初始化数据库 — 更新配置后执行建表操作 */
  const handleInitialize = async () => {
    setStep('initializing')
    setError('')
    try {
      const dbConfig: DatabaseSettings = {
        backend: selectedBackend,
        sqlite: sqliteConfig,
        postgres: postgresConfig,
        qdrant: qdrantConfig,
        initialized: false,
      }

      await updateDatabaseConfig(dbConfig)

      const result = await initializeDatabase()
      setInitResult(result)
      setStep('done')
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e))
      setStep('configure')
    }
  }

  const handleFinish = () => {
    onComplete()
  }

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/70 backdrop-blur-sm">
      <div className="w-full max-w-2xl mx-4 bg-gray-900 border border-gray-700 rounded-2xl shadow-2xl overflow-hidden">
        <div className="px-8 py-6 border-b border-gray-700">
          <h2 className="text-2xl font-bold text-white">{t('databaseSetup.title')}</h2>
          <p className="mt-1 text-sm text-gray-400">{t('databaseSetup.subtitle')}</p>
        </div>

        <div className="px-8 py-6 max-h-[60vh] overflow-y-auto">
          {step === 'select' && (
            <div className="space-y-4">
              <p className="text-gray-300 mb-4">{t('databaseSetup.selectPrompt')}</p>
              {backends.map((b) => (
                <button
                  key={b.id}
                  onClick={() => setSelectedBackend(b.id)}
                  className={`w-full text-left p-4 rounded-xl border-2 transition-all ${
                    selectedBackend === b.id
                      ? 'border-blue-500 bg-blue-500/10'
                      : 'border-gray-700 bg-gray-800 hover:border-gray-600'
                  }`}
                >
                  <div className="flex items-center gap-3">
                    <span className="text-2xl">{b.icon}</span>
                    <div className="flex-1">
                      <div className="flex items-center gap-2">
                        <span className="font-semibold text-white">{b.name}</span>
                        {b.recommended && (
                          <span className="px-2 py-0.5 text-xs font-medium bg-green-500/20 text-green-400 rounded-full">
                            {t('databaseSetup.recommended')}
                          </span>
                        )}
                      </div>
                      <p className="text-sm text-gray-400 mt-0.5">{b.desc}</p>
                    </div>
                    <div
                      className={`w-5 h-5 rounded-full border-2 flex items-center justify-center ${
                        selectedBackend === b.id ? 'border-blue-500' : 'border-gray-600'
                      }`}
                    >
                      {selectedBackend === b.id && (
                        <div className="w-2.5 h-2.5 rounded-full bg-blue-500" />
                      )}
                    </div>
                  </div>
                </button>
              ))}
            </div>
          )}

          {step === 'configure' && selectedBackend === 'sqlite' && (
            <div className="space-y-4">
              <p className="text-gray-300">{t('databaseSetup.sqliteConfigPrompt')}</p>
              <label className="flex items-center gap-3 p-3 rounded-lg bg-gray-800 border border-gray-700">
                <input
                  type="checkbox"
                  checked={sqliteConfig.enable_vec}
                  onChange={(e) => setSqliteConfig({ ...sqliteConfig, enable_vec: e.target.checked })}
                  className="w-4 h-4 rounded border-gray-600 bg-gray-700 text-blue-500 focus:ring-blue-500"
                />
                <div>
                  <span className="text-white font-medium">{t('databaseSetup.enableVec')}</span>
                  <p className="text-xs text-gray-400">{t('databaseSetup.enableVecDesc')}</p>
                </div>
              </label>
              <div>
                <label className="block text-sm font-medium text-gray-300 mb-1">
                  {t('databaseSetup.dbPath')}
                </label>
                <input
                  type="text"
                  value={sqliteConfig.db_path}
                  onChange={(e) => setSqliteConfig({ ...sqliteConfig, db_path: e.target.value })}
                  placeholder={t('databaseSetup.dbPathPlaceholder')}
                  className="w-full px-3 py-2 bg-gray-800 border border-gray-600 rounded-lg text-white placeholder-gray-500 focus:outline-none focus:border-blue-500"
                />
              </div>
            </div>
          )}

          {step === 'configure' && selectedBackend === 'postgres' && (
            <div className="space-y-4">
              <p className="text-gray-300">{t('databaseSetup.postgresConfigPrompt')}</p>
              <div className="grid grid-cols-2 gap-4">
                <div>
                  <label className="block text-sm font-medium text-gray-300 mb-1">{t('databaseSetup.host')}</label>
                  <input
                    type="text"
                    value={postgresConfig.host}
                    onChange={(e) => setPostgresConfig({ ...postgresConfig, host: e.target.value })}
                    className="w-full px-3 py-2 bg-gray-800 border border-gray-600 rounded-lg text-white focus:outline-none focus:border-blue-500"
                  />
                </div>
                <div>
                  <label className="block text-sm font-medium text-gray-300 mb-1">{t('databaseSetup.port')}</label>
                  <input
                    type="number"
                    value={postgresConfig.port}
                    onChange={(e) => setPostgresConfig({ ...postgresConfig, port: parseInt(e.target.value) || 5432 })}
                    className="w-full px-3 py-2 bg-gray-800 border border-gray-600 rounded-lg text-white focus:outline-none focus:border-blue-500"
                  />
                </div>
              </div>
              <div>
                <label className="block text-sm font-medium text-gray-300 mb-1">{t('databaseSetup.database')}</label>
                <input
                  type="text"
                  value={postgresConfig.database}
                  onChange={(e) => setPostgresConfig({ ...postgresConfig, database: e.target.value })}
                  className="w-full px-3 py-2 bg-gray-800 border border-gray-600 rounded-lg text-white focus:outline-none focus:border-blue-500"
                />
              </div>
              <div>
                <label className="block text-sm font-medium text-gray-300 mb-1">{t('databaseSetup.username')}</label>
                <input
                  type="text"
                  value={postgresConfig.username}
                  onChange={(e) => setPostgresConfig({ ...postgresConfig, username: e.target.value })}
                  className="w-full px-3 py-2 bg-gray-800 border border-gray-600 rounded-lg text-white focus:outline-none focus:border-blue-500"
                />
              </div>
              <div>
                <label className="block text-sm font-medium text-gray-300 mb-1">{t('databaseSetup.password')}</label>
                <input
                  type="password"
                  value={postgresConfig.password}
                  onChange={(e) => setPostgresConfig({ ...postgresConfig, password: e.target.value })}
                  className="w-full px-3 py-2 bg-gray-800 border border-gray-600 rounded-lg text-white focus:outline-none focus:border-blue-500"
                />
              </div>
            </div>
          )}

          {step === 'configure' && selectedBackend === 'qdrant' && (
            <div className="space-y-4">
              <p className="text-gray-300">{t('databaseSetup.qdrantConfigPrompt')}</p>
              <div>
                <label className="block text-sm font-medium text-gray-300 mb-1">{t('databaseSetup.qdrantUrl')}</label>
                <input
                  type="text"
                  value={qdrantConfig.url}
                  onChange={(e) => setQdrantConfig({ ...qdrantConfig, url: e.target.value })}
                  className="w-full px-3 py-2 bg-gray-800 border border-gray-600 rounded-lg text-white focus:outline-none focus:border-blue-500"
                />
              </div>
              <div>
                <label className="block text-sm font-medium text-gray-300 mb-1">{t('databaseSetup.apiKey')}</label>
                <input
                  type="password"
                  value={qdrantConfig.api_key}
                  onChange={(e) => setQdrantConfig({ ...qdrantConfig, api_key: e.target.value })}
                  className="w-full px-3 py-2 bg-gray-800 border border-gray-600 rounded-lg text-white focus:outline-none focus:border-blue-500"
                />
              </div>
              <div>
                <label className="block text-sm font-medium text-gray-300 mb-1">{t('databaseSetup.collection')}</label>
                <input
                  type="text"
                  value={qdrantConfig.collection}
                  onChange={(e) => setQdrantConfig({ ...qdrantConfig, collection: e.target.value })}
                  className="w-full px-3 py-2 bg-gray-800 border border-gray-600 rounded-lg text-white focus:outline-none focus:border-blue-500"
                />
              </div>
            </div>
          )}

          {step === 'testing' && (
            <div className="flex flex-col items-center py-8">
              <div className="w-12 h-12 border-4 border-blue-500 border-t-transparent rounded-full animate-spin" />
              <p className="mt-4 text-gray-300">{t('databaseSetup.testing')}</p>
            </div>
          )}

          {step === 'initializing' && (
            <div className="flex flex-col items-center py-8">
              <div className="w-12 h-12 border-4 border-green-500 border-t-transparent rounded-full animate-spin" />
              <p className="mt-4 text-gray-300">{t('databaseSetup.initializing')}</p>
            </div>
          )}

          {step === 'done' && (
            <div className="space-y-4">
              <div className="flex items-center gap-3 p-4 rounded-lg bg-green-500/10 border border-green-500/30">
                <span className="text-2xl">✅</span>
                <div>
                  <p className="font-medium text-green-400">{t('databaseSetup.initSuccess')}</p>
                  <p className="text-sm text-gray-400">
                    {initResult && `${t('databaseSetup.tablesCreated')}: ${initResult.tables_created.length} | ${t('databaseSetup.vectorSupport')}: ${initResult.vector_support ? '✓' : '✗'}`}
                  </p>
                </div>
              </div>
            </div>
          )}

          {error && (
            <div className="mt-4 p-3 rounded-lg bg-red-500/10 border border-red-500/30">
              <p className="text-sm text-red-400">{error}</p>
            </div>
          )}
        </div>

        <div className="px-8 py-4 border-t border-gray-700 flex justify-between">
          {step === 'select' && (
            <button
              onClick={() => setStep('configure')}
              className="px-6 py-2.5 bg-blue-600 hover:bg-blue-700 text-white font-medium rounded-lg transition-colors"
            >
              {t('databaseSetup.next')}
            </button>
          )}

          {step === 'configure' && (
            <div className="flex gap-3">
              <button
                onClick={() => setStep('select')}
                className="px-4 py-2.5 bg-gray-700 hover:bg-gray-600 text-white rounded-lg transition-colors"
              >
                {t('databaseSetup.back')}
              </button>
              {selectedBackend !== 'sqlite' && (
                <button
                  onClick={handleTestConnection}
                  className="px-4 py-2.5 bg-gray-700 hover:bg-gray-600 text-white rounded-lg transition-colors"
                >
                  {t('databaseSetup.testConnection')}
                </button>
              )}
              <button
                onClick={handleInitialize}
                className="px-6 py-2.5 bg-green-600 hover:bg-green-700 text-white font-medium rounded-lg transition-colors"
              >
                {t('databaseSetup.initialize')}
              </button>
            </div>
          )}

          {step === 'done' && (
            <button
              onClick={handleFinish}
              className="px-6 py-2.5 bg-blue-600 hover:bg-blue-700 text-white font-medium rounded-lg transition-colors"
            >
              {t('databaseSetup.start')}
            </button>
          )}
        </div>
      </div>
    </div>
  )
}

/** 数据库状态面板 — 展示当前数据库连接状态、向量支持和表信息 */
export function DatabaseStatusPanel() {
  const { t } = useTranslation()
  const [status, setStatus] = useState<ReturnType<typeof getDatabaseStatus> extends Promise<infer T> ? T : never>()
  const [loading, setLoading] = useState(false)

  const refresh = async () => {
    setLoading(true)
    try {
      const s = await getDatabaseStatus()
      setStatus(s)
    } catch {
      // ignore
    }
    setLoading(false)
  }

  return (
    <div className="p-4 bg-gray-800 rounded-lg border border-gray-700">
      <div className="flex items-center justify-between mb-3">
        <h3 className="text-sm font-medium text-white">{t('databaseSetup.statusTitle')}</h3>
        <button
          onClick={refresh}
          disabled={loading}
          className="px-3 py-1 text-xs bg-gray-700 hover:bg-gray-600 text-gray-300 rounded transition-colors disabled:opacity-50"
        >
          {loading ? '...' : t('databaseSetup.refresh')}
        </button>
      </div>

      {status ? (
        <div className="space-y-2">
          <div className="flex items-center gap-2">
            <span className={`w-2 h-2 rounded-full ${status.connected ? 'bg-green-500' : 'bg-red-500'}`} />
            <span className="text-sm text-gray-300">
              {status.backend.toUpperCase()} - {status.connected ? t('databaseSetup.connected') : t('databaseSetup.disconnected')}
            </span>
          </div>
          <div className="flex items-center gap-2">
            <span className={`w-2 h-2 rounded-full ${status.vector_support ? 'bg-green-500' : 'bg-yellow-500'}`} />
            <span className="text-sm text-gray-300">
              {t('databaseSetup.vectorSupport')}: {status.vector_support ? '✓' : '✗'}
            </span>
          </div>
          {status.tables.length > 0 && (
            <div className="mt-2 max-h-40 overflow-y-auto">
              <table className="w-full text-xs">
                <thead>
                  <tr className="text-gray-400">
                    <th className="text-left py-1">{t('databaseSetup.table')}</th>
                    <th className="text-center py-1">{t('databaseSetup.exists')}</th>
                    <th className="text-right py-1">{t('databaseSetup.rows')}</th>
                  </tr>
                </thead>
                <tbody>
                  {status.tables.map((tbl) => (
                    <tr key={tbl.name} className="border-t border-gray-700">
                      <td className="py-1 text-gray-300">{tbl.name}</td>
                      <td className="text-center py-1">{tbl.exists ? '✓' : '✗'}</td>
                      <td className="text-right py-1 text-gray-400">{tbl.row_count}</td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
          )}
        </div>
      ) : (
        <p className="text-sm text-gray-500">{t('databaseSetup.clickRefresh')}</p>
      )}
    </div>
  )
}
