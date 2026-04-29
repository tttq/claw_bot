// Claw Desktop - 渠道面板 - 管理Discord/Telegram/微信等消息渠道账号
// 布局：左侧渠道列表 + 右侧配置表单 + 顶部操作栏

import { useState, useEffect, useCallback } from 'react'
import { channelCreateAccount, channelUpdateAccount, channelDeleteAccount, channelToggle, channelSendMessage, channelList, channelStatus, channelTestConnection, channelGetSchema } from '../../api/channels'

// ====== 类型定义 ======

interface ChannelMeta {
    id: string
    label: string
    description: string
    icon: string
}

interface ChannelAccount {
    id: string
    channel_id: string
    name: string
    enabled: boolean
    config_json: Record<string, any>
    status: string
    last_error: string | null
}

interface ConfigFieldMeta {
    key: string
    label: string
    field_type: string
    required: boolean
    sensitive: boolean
    placeholder?: string
    help_text?: string
}

interface ChannelSchema {
    fields: ConfigFieldMeta[]
}

interface ChannelListData {
    channels: ChannelMeta[]
    accounts: ChannelAccount[]
}

export default function ChannelPanel() {
    const [data, setData] = useState<ChannelListData | null>(null)
    const [selectedAccountId, setSelectedAccountId] = useState<string | null>(null)
    const [loading, setLoading] = useState(true)
    const [saving, setSaving] = useState(false)
    const [testing, setTesting] = useState<string | null>(null)
    const [formValues, setFormValues] = useState<Record<string, string>>({})
    const [schema, setSchema] = useState<ChannelSchema | null>(null)
    const [error, setError] = useState<string | null>(null)

    // 加载渠道列表
    const loadChannels = useCallback(async () => {
        setLoading(true)
        setError(null)
        try {
            const result = await channelList() as unknown as Record<string, unknown>
            setData(result as any)
            if (result.accounts && Array.isArray(result.accounts) && result.accounts.length > 0) {
                setSelectedAccountId(result.accounts[0].id)
            }
        } catch (e) {
            setError(e instanceof Error ? e.message : String(e))
        } finally {
            setLoading(false)
        }
    }, [])

    useEffect(() => {
        loadChannels()
    }, [loadChannels])

    // 加载指定渠道的配置 Schema
    const loadSchema = async (channelType: string) => {
        try {
            const result = await channelGetSchema({ channel_type: channelType })
            setSchema(result as any)
        } catch (e) {
            console.error('Failed to load schema:', e)
        }
    }

    // 当选择账户时加载其配置
    useEffect(() => {
        if (!selectedAccountId || !data) return

        const account = data.accounts.find(a => a.id === selectedAccountId)
        if (account) {
            setFormValues(account.config_json || {})
            loadSchema(account.channel_id)
        }
    }, [selectedAccountId, data])

    // 创建新账户
    const handleCreate = async (channelType: string) => {
        setSaving(true)
        try {
            await channelCreateAccount({
                channel_type: channelType,
                name: `New ${channelType} Account`,
                config: {}
            })
            await loadChannels()
        } catch (e) {
            setError(e instanceof Error ? e.message : String(e))
        } finally {
            setSaving(false)
        }
    }

    // 更新账户配置
    const handleSave = async () => {
        if (!selectedAccountId) return

        setSaving(true)
        try {
            await channelUpdateAccount({
                account_id: selectedAccountId,
                name: formValues.name || '',
                config: formValues,
                dm_policy: { allow_from: { type: 'everyone' }, require_mention: false },
                group_policy: { allowed_groups: [], require_mention: true }
            })
            await loadChannels()
        } catch (e) {
            setError(e instanceof Error ? e.message : String(e))
        } finally {
            setSaving(false)
        }
    }

    // 删除账户
    const handleDelete = async () => {
        if (!selectedAccountId) return
        if (!confirm('Are you sure you want to delete this account?')) return

        setSaving(true)
        try {
            await channelDeleteAccount({ account_id: selectedAccountId })
            setSelectedAccountId(null)
            setFormValues({})
            await loadChannels()
        } catch (e) {
            setError(e instanceof Error ? e.message : String(e))
        } finally {
            setSaving(false)
        }
    }

    // 切换启用/禁用
    const handleToggle = async (accountId: string, enabled: boolean) => {
        try {
            await channelToggle({ account_id: accountId, enabled })
            await loadChannels()
        } catch (e) {
            setError(e instanceof Error ? e.message : String(e))
        }
    }

    // 测试连接
    const handleTestConnection = async (accountId: string) => {
        setTesting(accountId)
        try {
            const result = await channelTestConnection({ account_id: accountId }) as unknown as { connected: boolean }
            alert(result.connected ? '✅ Connection successful!' : '❌ Connection failed')
        } catch (e) {
            alert(`Connection test failed: ${e instanceof Error ? e.message : String(e)}`)
        } finally {
            setTesting(null)
        }
    }

    // 发送测试消息
    const handleSendMessage = async () => {
        if (!selectedAccountId || !formValues.test_target) return

        try {
            await channelSendMessage({
                account_id: selectedAccountId,
                target_id: formValues.test_target,
                text: formValues.test_message || 'Hello from qclaw-desktop! 🚀',
                chat_type: 'direct'
            })
            alert('✅ Message sent!')
        } catch (e) {
            alert(`Failed to send: ${e instanceof Error ? e.message : String(e)}`)
        }
    }

    // 渲染状态指示器
    const StatusBadge = ({ status }: { status: string }) => {
        const colors: Record<string, string> = {
            connected: 'bg-green-500',
            configured: 'bg-yellow-500',
            connecting: 'bg-blue-500 animate-pulse',
            error: 'bg-red-500',
            disabled: 'bg-gray-500'
        }
        return (
            <span className={`inline-flex items-center px-2 py-0.5 rounded-full text-xs font-medium text-white ${colors[status] || 'bg-gray-500'}`}>
                {status}
            </span>
        )
    }

    // 渲染动态表单字段
    const renderField = (field: ConfigFieldMeta) => {
        const value = formValues[field.key] || ''

        if (field.field_type === 'password') {
            return (
                <div key={field.key} className="space-y-1">
                    <label className="block text-sm font-medium text-dark-text">
                        {field.label} {field.required && <span className="text-red-500">*</span>}
                    </label>
                    <input
                        type="password"
                        value={value}
                        onChange={e => setFormValues(prev => ({ ...prev, [field.key]: e.target.value }))}
                        placeholder={field.placeholder}
                        className="w-full px-3 py-2 bg-dark-bg border border-dark-border rounded-lg text-dark-text text-sm focus:border-primary-500 focus:outline-none"
                    />
                    {field.help_text && <p className="text-xs text-dark-muted">{field.help_text}</p>}
                </div>
            )
        }

        if (field.field_type === 'select') {
            return (
                <div key={field.key} className="space-y-1">
                    <label className="block text-sm font-medium text-dark-text">{field.label}</label>
                    <select
                        value={value}
                        onChange={e => setFormValues(prev => ({ ...prev, [field.key]: e.target.value }))}
                        className="w-full px-3 py-2 bg-dark-bg border border-dark-border rounded-lg text-dark-text text-sm focus:border-primary-500 focus:outline-none"
                    >
                        <option value="">Select...</option>
                    </select>
                </div>
            )
        }

        return (
            <div key={field.key} className="space-y-1">
                <label className="block text-sm font-medium text-dark-text">
                    {field.label} {field.required && <span className="text-red-500">*</span>}
                </label>
                <input
                    type={field.field_type === 'number' ? 'number' : 'text'}
                    value={value}
                    onChange={e => setFormValues(prev => ({ ...prev, [field.key]: e.target.value }))}
                    placeholder={field.placeholder}
                    className="w-full px-3 py-2 bg-dark-bg border border-dark-border rounded-lg text-dark-text text-sm focus:border-primary-500 focus:outline-none"
                />
                {field.help_text && <p className="text-xs text-dark-muted">{field.help_text}</p>}
            </div>
        )
    }

    // 加载中
    if (loading) {
        return (
            <div className="flex items-center justify-center h-full">
                <div className="w-8 h-8 border-2 border-primary-500 border-t-transparent rounded-full animate-spin"></div>
            </div>
        )
    }

    // 错误状态
    if (error) {
        return (
            <div className="flex flex-col items-center justify-center h-full gap-4">
                <p className="text-red-400">Error: {error}</p>
                <button onClick={loadChannels} className="px-4 py-2 bg-primary-600 hover:bg-primary-700 rounded-lg text-white text-sm transition-colors">
                    Retry
                </button>
            </div>
        )
    }

    return (
        <div className="flex flex-col h-full">
            {/* ====== 顶部操作栏 ====== */}
            <div className="flex items-center justify-between px-4 py-3 border-b border-dark-border shrink-0">
                <h2 className="text-base font-bold text-dark-text">📡 Channel Management</h2>
                <div className="flex items-center gap-2">
                    {/* 添加账户下拉菜单 */}
                    <div className="relative group">
                        <button className="px-3 py-1.5 bg-primary-600 hover:bg-primary-700 rounded-lg text-white text-sm transition-colors flex items-center gap-1">
                            + Add Account
                            <svg className="w-3 h-3" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={2}>
                                <path strokeLinecap="round" strokeLinejoin="round" d="M19 9l-7 7-7-7" />
                            </svg>
                        </button>
                        <div className="absolute right-0 top-full mt-1 w-48 bg-dark-surface border border-dark-border rounded-lg shadow-xl opacity-0 invisible group-hover:opacity-100 group-hover:visible transition-all z-10">
                            {data?.channels.map(ch => (
                                <button
                                    key={ch.id}
                                    onClick={() => handleCreate(ch.id)}
                                    className="block w-full px-4 py-2 text-left text-sm text-dark-text hover:bg-dark-border/50 transition-colors first:rounded-t-lg last:rounded-b-lg"
                                >
                                    {ch.icon} {ch.label}
                                </button>
                            ))}
                        </div>
                    </div>

                    {/* 刷新按钮 */}
                    <button onClick={loadChannels} className="p-1.5 rounded-lg hover:bg-dark-border/50 text-dark-muted hover:text-dark-text transition-colors">
                        <svg className="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={2}>
                            <path strokeLinecap="round" strokeLinejoin="round" d="M4 4v5h.582m15.356 2A8.001 8.001 0 004.582 9m0 0H9m11 11v-5h-.581m0 0a8.003 8.003 0 01-15.357-2m15.357 2H15" />
                        </svg>
                    </button>
                </div>
            </div>

            {/* ====== 主内容区 ====== */}
            <div className="flex-1 flex overflow-hidden">
                {/* 左侧：账户列表 */}
                <div className="w-64 border-r border-dark-border overflow-y-auto p-2 space-y-1">
                    {data?.accounts.length === 0 && (
                        <div className="text-center py-8 text-dark-muted text-sm">
                            <p>No accounts yet</p>
                            <p className="mt-1">Click "+ Add Account" to get started</p>
                        </div>
                    )}
                    {data?.accounts.map(account => (
                        <button
                            key={account.id}
                            onClick={() => setSelectedAccountId(account.id)}
                            className={`w-full text-left px-3 py-2 rounded-lg transition-colors ${
                                selectedAccountId === account.id
                                    ? 'bg-primary-600/20 border border-primary-500/50 text-primary-400'
                                    : 'hover:bg-dark-border/50 text-dark-text'
                            }`}
                        >
                            <div className="flex items-center justify-between">
                                <span className="font-medium text-sm truncate">{account.name}</span>
                                <StatusBadge status={account.status} />
                            </div>
                            <p className="text-xs text-dark-muted mt-0.5">{account.channel_id}</p>
                        </button>
                    ))}
                </div>

                {/* 右侧：配置表单 */}
                <div className="flex-1 overflow-y-auto p-4">
                    {selectedAccountId ? (
                        <div className="max-w-2xl mx-auto space-y-6">
                            {/* 基本信息 */}
                            <section className="space-y-3">
                                <h3 className="text-sm font-semibold text-dark-text uppercase tracking-wider">Basic Info</h3>
                                <div className="grid grid-cols-2 gap-3">
                                    <div className="space-y-1">
                                        <label className="block text-sm font-medium text-dark-text">Account Name</label>
                                        <input
                                            type="text"
                                            value={formValues.name || ''}
                                            onChange={e => setFormValues(prev => ({ ...prev, name: e.target.value }))}
                                            className="w-full px-3 py-2 bg-dark-bg border border-dark-border rounded-lg text-dark-text text-sm focus:border-primary-500 focus:outline-none"
                                        />
                                    </div>
                                    <div className="space-y-1">
                                        <label className="block text-sm font-medium text-dark-text">Status</label>
                                        <div className="flex items-center gap-2 h-[38px]">
                                            <StatusBadge status={data?.accounts.find(a => a.id === selectedAccountId)?.status || ''} />
                                            <button
                                                onClick={() => handleToggle(selectedAccountId!, !(data?.accounts.find(a => a.id === selectedAccountId)?.enabled ?? false))}
                                                className={`px-3 py-1 rounded text-xs font-medium transition-colors ${
                                                    data?.accounts.find(a => a.id === selectedAccountId)?.enabled
                                                        ? 'bg-red-500/20 text-red-400 hover:bg-red-500/30'
                                                        : 'bg-green-500/20 text-green-400 hover:bg-green-500/30'
                                                }`}
                                            >
                                                {data?.accounts.find(a => a.id === selectedAccountId)?.enabled ? 'Disable' : 'Enable'}
                                            </button>
                                        </div>
                                    </div>
                                </div>
                            </section>

                            {/* 动态配置字段（从 Schema 渲染）*/}
                            {schema?.fields && schema.fields.length > 0 && (
                                <section className="space-y-3">
                                    <h3 className="text-sm font-semibold text-dark-text uppercase tracking-wider">Configuration</h3>
                                    <div className="space-y-3">
                                        {schema.fields.map(field => renderField(field))}
                                    </div>
                                </section>
                            )}

                            {/* 安全策略 */}
                            <section className="space-y-3">
                                <h3 className="text-sm font-semibold text-dark-text uppercase tracking-wider">Security Policy</h3>
                                <div className="grid grid-cols-2 gap-3">
                                    <div className="p-3 bg-dark-bg/50 rounded-lg border border-dark-border">
                                        <p className="text-xs text-dark-muted mb-1">DM Policy</p>
                                        <select className="w-full px-2 py-1.5 bg-dark-bg border border-dark-border rounded text-sm text-dark-text">
                                            <option>Allow Everyone</option>
                                            <option>Allow List Only</option>
                                            <option>Owners Only</option>
                                        </select>
                                    </div>
                                    <div className="p-3 bg-dark-bg/50 rounded-lg border border-dark-border">
                                        <p className="text-xs text-dark-muted mb-1">Group Policy</p>
                                        <label className="flex items-center gap-2 text-sm text-dark-text">
                                            <input type="checkbox" defaultChecked />
                                            Require @mention in groups
                                        </label>
                                    </div>
                                </div>
                            </section>

                            {/* 流式传输设置 */}
                            <section className="space-y-3">
                                <h3 className="text-sm font-semibold text-dark-text uppercase tracking-wider">Streaming</h3>
                                <div className="grid grid-cols-3 gap-3">
                                    {(['off', 'partial', 'block'] as const).map(mode => (
                                        <label key={mode} className="flex items-center gap-2 p-2 bg-dark-bg/50 rounded-lg border border-dark-border cursor-pointer hover:border-primary-500 transition-colors">
                                            <input type="radio" name="streaming_mode" value={mode} defaultChecked={mode === 'partial'} />
                                            <span className="text-sm text-dark-text capitalize">{mode}</span>
                                        </label>
                                    ))}
                                </div>
                            </section>

                            {/* 操作按钮 */}
                            <section className="flex items-center justify-between pt-4 border-t border-dark-border">
                                <div className="flex items-center gap-2">
                                    <button
                                        onClick={() => selectedAccountId && handleTestConnection(selectedAccountId)}
                                        disabled={!!testing}
                                        className="px-4 py-2 bg-blue-600 hover:bg-blue-700 disabled:bg-blue-800 rounded-lg text-white text-sm transition-colors flex items-center gap-1"
                                    >
                                        {testing === selectedAccountId ? (
                                            <div className="w-4 h-4 border-2 border-white border-t-transparent rounded-full animate-spin" />
                                        ) : (
                                            '🔗 Test Connection'
                                        )}
                                    </button>
                                    <button
                                        onClick={handleDelete}
                                        className="px-4 py-2 bg-red-600/20 hover:bg-red-600/30 text-red-400 rounded-lg text-sm transition-colors"
                                    >
                                        Delete
                                    </button>
                                </div>
                                <button
                                    onClick={handleSave}
                                    disabled={saving}
                                    className="px-6 py-2 bg-primary-600 hover:bg-primary-700 disabled:bg-primary-800 rounded-lg text-white text-sm font-medium transition-colors flex items-center gap-1"
                                >
                                    {saving ? (
                                        <div className="w-4 h-4 border-2 border-white border-t-transparent rounded-full animate-spin" />
                                    ) : (
                                        '💾 Save Configuration'
                                    )}
                                </button>
                            </section>

                            {/* 测试消息发送 */}
                            <section className="pt-4 border-t border-dark-border space-y-3">
                                <h3 className="text-sm font-semibold text-dark-text uppercase tracking-wider">Send Test Message</h3>
                                <div className="flex gap-2">
                                    <input
                                        type="text"
                                        value={formValues.test_target || ''}
                                        onChange={e => setFormValues(prev => ({ ...prev, test_target: e.target.value }))}
                                        placeholder="Target Chat ID / User ID"
                                        className="flex-1 px-3 py-2 bg-dark-bg border border-dark-border rounded-lg text-dark-text text-sm focus:border-primary-500 focus:outline-none"
                                    />
                                </div>
                                <textarea
                                    value={formValues.test_message || ''}
                                    onChange={e => setFormValues(prev => ({ ...prev, test_message: e.target.value }))}
                                    placeholder="Type your message..."
                                    rows={3}
                                    className="w-full px-3 py-2 bg-dark-bg border border-dark-border rounded-lg text-dark-text text-sm focus:border-primary-500 focus:outline-none resize-none"
                                />
                                <button
                                    onClick={handleSendMessage}
                                    className="w-full px-4 py-2 bg-green-600 hover:bg-green-700 rounded-lg text-white text-sm font-medium transition-colors"
                                >
                                    📤 Send Test Message
                                </button>
                            </section>
                        </div>
                    ) : (
                        <div className="flex flex-col items-center justify-center h-full text-dark-muted">
                            <svg className="w-16 h-16 mb-4 opacity-50" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={1}>
                                <path strokeLinecap="round" strokeLinejoin="round" d="M8 12h.01M12 12h.01M16 12h.01M21 12c0 4.97-4.03 9-9 9s-9-4.03-9-9 4.03-9 9-9 9z" />
                            </svg>
                            <p>Select an account to configure</p>
                        </div>
                    )}
                </div>
            </div>
        </div>
    )
}
