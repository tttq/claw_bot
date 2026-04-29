// Claw Desktop - 定时任务面板 - 管理Cron定时任务的创建、编辑、启用/禁用
import React, { useState, useEffect, useCallback } from 'react';
import { cronApi, CronJob } from '../../api/cron';

export const CronPanel: React.FC<{ agentId?: string }> = ({ agentId }) => {
  const [jobs, setJobs] = useState<CronJob[]>([]);
  const [loading, setLoading] = useState(false);
  const [showCreate, setShowCreate] = useState(false);
  const [editJob, setEditJob] = useState<Partial<CronJob> | null>(null);

  const loadJobs = useCallback(async () => {
    setLoading(true);
    try {
      const data = await cronApi.list();
      setJobs(Array.isArray(data) ? data : []);
    } catch (e) {
      console.error('Failed to load cron jobs:', e);
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => { loadJobs(); }, [loadJobs]);

  const handleCreate = async () => {
    if (!editJob?.name || !editJob?.schedule || !editJob?.prompt) return;
    try {
      await cronApi.create(editJob);
      setShowCreate(false);
      setEditJob(null);
      loadJobs();
    } catch (e) {
      console.error('Failed to create cron job:', e);
    }
  };

  const handleToggle = async (job: CronJob) => {
    try {
      await cronApi.update({ ...job, enabled: !job.enabled });
      loadJobs();
    } catch (e) {
      console.error('Failed to toggle cron job:', e);
    }
  };

  const handleDelete = async (id: string) => {
    try {
      await cronApi.delete(id);
      loadJobs();
    } catch (e) {
      console.error('Failed to delete cron job:', e);
    }
  };

  const handleTrigger = async (id: string) => {
    try {
      await cronApi.trigger(id);
      loadJobs();
    } catch (e) {
      console.error('Failed to trigger cron job:', e);
    }
  };

  const formatTime = (ts?: number) => {
    if (!ts) return '-';
    return new Date(ts * 1000).toLocaleString();
  };

  return (
    <div className="p-4 space-y-4">
      <div className="flex items-center justify-between">
        <h3 className="text-lg font-semibold text-gray-200">Scheduled Tasks</h3>
        <button
          onClick={() => { setEditJob({ name: '', schedule: '', prompt: '', enabled: true, silent_on_empty: false }); setShowCreate(true); }}
          className="px-3 py-1.5 bg-blue-600 hover:bg-blue-700 text-white text-sm rounded-lg transition-colors"
        >
          + New Task
        </button>
      </div>

      {loading ? (
        <div className="text-gray-400 text-center py-8">Loading...</div>
      ) : jobs.length === 0 ? (
        <div className="text-gray-500 text-center py-8">No scheduled tasks yet</div>
      ) : (
        <div className="space-y-2">
          {jobs.map(job => (
            <div key={job.id} className="bg-gray-800/50 rounded-lg p-4 border border-gray-700/50">
              <div className="flex items-center justify-between mb-2">
                <div className="flex items-center gap-3">
                  <span className={`w-2 h-2 rounded-full ${job.enabled ? 'bg-green-400' : 'bg-gray-500'}`} />
                  <span className="font-medium text-gray-200">{job.name}</span>
                  <span className="text-xs text-gray-500 bg-gray-700/50 px-2 py-0.5 rounded">{job.schedule}</span>
                </div>
                <div className="flex items-center gap-2">
                  <button onClick={() => handleTrigger(job.id)} className="text-xs px-2 py-1 bg-gray-700 hover:bg-gray-600 text-gray-300 rounded transition-colors">Run Now</button>
                  <button onClick={() => handleToggle(job)} className={`text-xs px-2 py-1 rounded transition-colors ${job.enabled ? 'bg-yellow-700/50 hover:bg-yellow-600/50 text-yellow-300' : 'bg-green-700/50 hover:bg-green-600/50 text-green-300'}`}>
                    {job.enabled ? 'Pause' : 'Enable'}
                  </button>
                  <button onClick={() => handleDelete(job.id)} className="text-xs px-2 py-1 bg-red-700/50 hover:bg-red-600/50 text-red-300 rounded transition-colors">Delete</button>
                </div>
              </div>
              <p className="text-sm text-gray-400 line-clamp-2 mb-2">{job.prompt}</p>
              <div className="flex items-center gap-4 text-xs text-gray-500">
                <span>Runs: {job.run_count}</span>
                <span>Last: {formatTime(job.last_run_at)}</span>
                <span>Next: {formatTime(job.next_run_at)}</span>
                {job.last_result && <span className="text-gray-400">Result: {job.last_result}</span>}
              </div>
            </div>
          ))}
        </div>
      )}

      {showCreate && editJob && (
        <div className="fixed inset-0 bg-black/60 flex items-center justify-center z-50" onClick={() => setShowCreate(false)}>
          <div className="bg-gray-800 rounded-xl p-6 w-full max-w-lg border border-gray-700" onClick={e => e.stopPropagation()}>
            <h3 className="text-lg font-semibold text-gray-200 mb-4">Create Scheduled Task</h3>
            <div className="space-y-3">
              <div>
                <label className="block text-sm text-gray-400 mb-1">Name</label>
                <input value={editJob.name || ''} onChange={e => setEditJob({ ...editJob, name: e.target.value })} className="w-full px-3 py-2 bg-gray-700 text-gray-200 rounded-lg border border-gray-600 focus:border-blue-500 outline-none" />
              </div>
              <div>
                <label className="block text-sm text-gray-400 mb-1">Schedule (Cron Expression)</label>
                <input value={editJob.schedule || ''} onChange={e => setEditJob({ ...editJob, schedule: e.target.value })} placeholder="0 9 * * *" className="w-full px-3 py-2 bg-gray-700 text-gray-200 rounded-lg border border-gray-600 focus:border-blue-500 outline-none" />
              </div>
              <div>
                <label className="block text-sm text-gray-400 mb-1">Prompt</label>
                <textarea value={editJob.prompt || ''} onChange={e => setEditJob({ ...editJob, prompt: e.target.value })} rows={4} className="w-full px-3 py-2 bg-gray-700 text-gray-200 rounded-lg border border-gray-600 focus:border-blue-500 outline-none resize-none" />
              </div>
              <div>
                <label className="block text-sm text-gray-400 mb-1">Agent ID (optional)</label>
                <input value={editJob.agent_id || ''} onChange={e => setEditJob({ ...editJob, agent_id: e.target.value })} className="w-full px-3 py-2 bg-gray-700 text-gray-200 rounded-lg border border-gray-600 focus:border-blue-500 outline-none" />
              </div>
              <div className="flex items-center gap-2">
                <input type="checkbox" checked={editJob.silent_on_empty || false} onChange={e => setEditJob({ ...editJob, silent_on_empty: e.target.checked })} className="rounded" />
                <label className="text-sm text-gray-400">Silent on empty response</label>
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
