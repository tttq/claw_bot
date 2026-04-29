// Claw Desktop - Hook面板 - 管理事件钩子的注册、配置和删除
import React, { useState, useEffect, useCallback } from 'react';
import { hookApi, HookDefinition } from '../../api/hooks';

const HOOK_EVENTS = [
  'pre_tool_call', 'post_tool_call',
  'pre_llm_call', 'post_llm_call',
  'on_session_start', 'on_session_end',
  'on_session_reset', 'on_message_received', 'on_message_sent',
];

export const HookPanel: React.FC<{ agentId?: string }> = ({ agentId }) => {
  const [hooks, setHooks] = useState<HookDefinition[]>([]);
  const [loading, setLoading] = useState(false);
  const [showCreate, setShowCreate] = useState(false);
  const [editHook, setEditHook] = useState<Partial<HookDefinition> | null>(null);

  const loadHooks = useCallback(async () => {
    setLoading(true);
    try {
      const data = await hookApi.list();
      setHooks(Array.isArray(data) ? data : []);
    } catch (e) {
      console.error('Failed to load hooks:', e);
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => { loadHooks(); }, [loadHooks]);

  const handleCreate = async () => {
    if (!editHook?.name || !editHook?.event) return;
    try {
      await hookApi.create(editHook);
      setShowCreate(false);
      setEditHook(null);
      loadHooks();
    } catch (e) {
      console.error('Failed to create hook:', e);
    }
  };

  const handleDelete = async (id: string) => {
    try {
      await hookApi.delete(id);
      loadHooks();
    } catch (e) {
      console.error('Failed to delete hook:', e);
    }
  };

  return (
    <div className="p-4 space-y-4">
      <div className="flex items-center justify-between">
        <h3 className="text-lg font-semibold text-gray-200">Lifecycle Hooks</h3>
        <button
          onClick={() => { setEditHook({ name: '', event: 'pre_tool_call', handler_type: 'log', priority: 0, enabled: true, handler_config: {} }); setShowCreate(true); }}
          className="px-3 py-1.5 bg-blue-600 hover:bg-blue-700 text-white text-sm rounded-lg transition-colors"
        >
          + New Hook
        </button>
      </div>

      {loading ? (
        <div className="text-gray-400 text-center py-8">Loading...</div>
      ) : hooks.length === 0 ? (
        <div className="text-gray-500 text-center py-8">No hooks configured</div>
      ) : (
        <div className="space-y-2">
          {hooks.map(hook => (
            <div key={hook.id} className="bg-gray-800/50 rounded-lg p-4 border border-gray-700/50">
              <div className="flex items-center justify-between mb-2">
                <div className="flex items-center gap-3">
                  <span className={`w-2 h-2 rounded-full ${hook.enabled ? 'bg-green-400' : 'bg-gray-500'}`} />
                  <span className="font-medium text-gray-200">{hook.name}</span>
                  <span className="text-xs text-blue-400 bg-blue-900/30 px-2 py-0.5 rounded">{hook.event}</span>
                  <span className="text-xs text-gray-500 bg-gray-700/50 px-2 py-0.5 rounded">{hook.handler_type}</span>
                </div>
                <button onClick={() => handleDelete(hook.id)} className="text-xs px-2 py-1 bg-red-700/50 hover:bg-red-600/50 text-red-300 rounded transition-colors">Delete</button>
              </div>
              {hook.pattern && <p className="text-sm text-gray-400">Pattern: {hook.pattern}</p>}
              <div className="text-xs text-gray-500 mt-1">Priority: {hook.priority}</div>
            </div>
          ))}
        </div>
      )}

      {showCreate && editHook && (
        <div className="fixed inset-0 bg-black/60 flex items-center justify-center z-50" onClick={() => setShowCreate(false)}>
          <div className="bg-gray-800 rounded-xl p-6 w-full max-w-lg border border-gray-700" onClick={e => e.stopPropagation()}>
            <h3 className="text-lg font-semibold text-gray-200 mb-4">Create Hook</h3>
            <div className="space-y-3">
              <div>
                <label className="block text-sm text-gray-400 mb-1">Name</label>
                <input value={editHook.name || ''} onChange={e => setEditHook({ ...editHook, name: e.target.value })} className="w-full px-3 py-2 bg-gray-700 text-gray-200 rounded-lg border border-gray-600 focus:border-blue-500 outline-none" />
              </div>
              <div>
                <label className="block text-sm text-gray-400 mb-1">Event</label>
                <select value={editHook.event || 'pre_tool_call'} onChange={e => setEditHook({ ...editHook, event: e.target.value })} className="w-full px-3 py-2 bg-gray-700 text-gray-200 rounded-lg border border-gray-600 focus:border-blue-500 outline-none">
                  {HOOK_EVENTS.map(ev => <option key={ev} value={ev}>{ev}</option>)}
                </select>
              </div>
              <div>
                <label className="block text-sm text-gray-400 mb-1">Handler Type</label>
                <select value={editHook.handler_type || 'log'} onChange={e => setEditHook({ ...editHook, handler_type: e.target.value })} className="w-full px-3 py-2 bg-gray-700 text-gray-200 rounded-lg border border-gray-600 focus:border-blue-500 outline-none">
                  <option value="log">Log</option>
                  <option value="filter">Filter</option>
                  <option value="modify">Modify</option>
                </select>
              </div>
              <div>
                <label className="block text-sm text-gray-400 mb-1">Pattern (optional)</label>
                <input value={editHook.pattern || ''} onChange={e => setEditHook({ ...editHook, pattern: e.target.value })} placeholder="Wildcard pattern" className="w-full px-3 py-2 bg-gray-700 text-gray-200 rounded-lg border border-gray-600 focus:border-blue-500 outline-none" />
              </div>
              <div>
                <label className="block text-sm text-gray-400 mb-1">Priority</label>
                <input type="number" value={editHook.priority || 0} onChange={e => setEditHook({ ...editHook, priority: parseInt(e.target.value) || 0 })} className="w-full px-3 py-2 bg-gray-700 text-gray-200 rounded-lg border border-gray-600 focus:border-blue-500 outline-none" />
              </div>
            </div>
            <div className="flex justify-end gap-2 mt-6">
              <button onClick={() => setShowCreate(false)} className="px-4 py-2 text-gray-400 hover:text-gray-200 transition-colors">Cancel</button>
              <button onClick={handleCreate} className="px-4 py-2 bg-blue-600 hover:bg-blue-700 text-white rounded-lg transition-colors">Create</button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
};
