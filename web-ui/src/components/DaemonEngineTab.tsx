import React, { useEffect, useState, useRef } from 'react';
import {
  Server,
  RotateCw,
  Terminal,
  Trash2,
  CheckCircle2,
  AlertCircle,
  Activity,
  Database,
  Globe,
  KeyRound,
} from 'lucide-react';
import { DaemonMetrics, AgentConfig } from '../types/dashboard';

interface LogEntry {
  timestamp: string;
  level: string;
  subsystem: string;
  message: string;
  stage?: string | null;
  provider?: string | null;
  target?: string | null;
  attachment_id?: number | null;
  post_id?: number | null;
  task_id?: string | null;
}

interface DaemonEngineTabProps {
  metrics: DaemonMetrics | null;
  config: AgentConfig | null;
  siteUrl?: string;
}

const API_BASE = '/api';

export const DaemonEngineTab: React.FC<DaemonEngineTabProps> = ({ metrics, config, siteUrl }) => {
  const [logs, setLogs] = useState<LogEntry[]>([]);
  const [logFilter, setLogFilter] = useState<'ALL' | 'INFO' | 'WARN' | 'ERROR' | 'DEBUG'>('ALL');
  const [subsystemFilter, setSubsystemFilter] = useState<string>('ALL');
  const [logsError, setLogsError] = useState<string | null>(null);
  const [search, setSearch] = useState('');
  const [attachmentFilter, setAttachmentFilter] = useState('');
  const [logPage, setLogPage] = useState(0);
  const [totalLogs, setTotalLogs] = useState(0);
  const [autoTail, setAutoTail] = useState(true);
  const LOG_PAGE_SIZE = 200;
  const esRef = useRef<EventSource | null>(null);

  const loadLogs = async (targetPage?: number) => {
    try {
      const page = targetPage ?? logPage;
      const params = new URLSearchParams({
        limit: String(LOG_PAGE_SIZE),
        offset: String(page * LOG_PAGE_SIZE),
      });
      if (logFilter !== 'ALL') params.set('level', logFilter);
      if (subsystemFilter !== 'ALL') params.set('subsystem', subsystemFilter);
      if (search.trim()) params.set('q', search.trim());
      const aid = Number(attachmentFilter.trim());
      if (Number.isFinite(aid) && aid > 0) params.set('attachment_id', String(aid));
      const res = await fetch(`${API_BASE}/logs?${params.toString()}`);
      if (!res.ok) throw new Error(`HTTP ${res.status}`);
      const data = await res.json();
      const entries: LogEntry[] = Array.isArray(data) ? data : Array.isArray(data?.logs) ? data.logs : [];
      const total = Number(res.headers.get('x-total-count') || data?.total || 0);
      setLogs(entries);
      setTotalLogs(Number.isFinite(total) ? total : entries.length);
      setLogPage(page);
      setLogsError(data?.degraded ? 'DB log unavailable — showing in-memory buffer' : null);
    } catch (e: any) {
      setLogsError(e?.message || 'Failed to load logs');
    }
  };

  useEffect(() => {
    loadLogs(0);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [logFilter, subsystemFilter, search, attachmentFilter]);

  useEffect(() => {
    const params = new URLSearchParams(window.location.search);
    const aid = params.get('attachmentId');
    const lvl = params.get('level');
    const sub = params.get('subsystem');
    if (aid) setAttachmentFilter(aid);
    if (lvl) setLogFilter(lvl as any);
    if (sub) setSubsystemFilter(sub);
  }, []);

  useEffect(() => {
    if (!autoTail) return;
    if (typeof EventSource === 'undefined') return;
    try {
      const es = new EventSource(`${API_BASE}/logs/stream`);
      esRef.current = es;
      es.addEventListener('logs', (ev: MessageEvent) => {
        try {
          const data = JSON.parse(ev.data);
          if (Array.isArray(data?.logs) && data.logs.length) {
            setLogs((prev) => {
              const next = [...prev, ...data.logs];
              return next.length > 500 ? next.slice(next.length - 500) : next;
            });
          }
        } catch {}
      });
      es.onerror = () => { /* browser will retry; ignore */ };
      return () => es.close();
    } catch {
      return;
    }
  }, [autoTail]);

  const filteredLogs = logs.filter((log) => {
    if (logFilter !== 'ALL' && log.level !== logFilter) return false;
    if (subsystemFilter !== 'ALL' && log.subsystem !== subsystemFilter) return false;
    return true;
  });

  const status: string = (metrics as any)?.status || 'unknown';
  const pid = (metrics as any)?.pid ?? '—';
  const uptime = (metrics as any)?.uptimeSeconds ?? 0;
  const dbOk = (metrics as any)?.db === true;
  const activeTasks = (metrics as any)?.activeTasks ?? 0;
  const totalOffloaded = (metrics as any)?.totalImagesOffloaded ?? 0;

  const cfg: any = config || {};
  const cdnHost = cfg?.cdn?.host || '—';
  const imgbbEnabled = cfg?.imgbb?.enabled !== false;
  const imgbbFormat = cfg?.imgbb?.upload_format || cfg?.imgbb?.uploadFormat || 'url';
  const pollInterval = cfg?.daemon?.pollIntervalSec ?? '—';
  const port = cfg?.daemon?.port ?? 2095;
  const bloggerId = cfg?.blogger?.blogId || cfg?.blogger?.blog_id || '';

  const fmtUptime = (s: number) => {
    if (!s) return '—';
    const h = Math.floor(s / 3600);
    const m = Math.floor((s % 3600) / 60);
    const sec = s % 60;
    if (h > 0) return `${h}h ${m}m`;
    if (m > 0) return `${m}m ${sec}s`;
    return `${sec}s`;
  };

  return (
    <div className="space-y-6">
      {/* Status */}
      <div className="p-5 rounded-xl bg-white dark:bg-slate-900 border border-slate-200 dark:border-slate-800">
        <div className="flex flex-col lg:flex-row lg:items-center justify-between gap-4">
          <div>
            <div className="flex items-center gap-2 text-xs text-orange-600 dark:text-orange-400 font-mono font-medium">
              <span>AGENTS/DAEMON/</span>
              <span>·</span>
              <span>RUST TOKIO</span>
              <span>·</span>
              <span>CHROME CDP</span>
            </div>
            <h2 className="text-lg font-bold text-slate-900 dark:text-white mt-0.5">
              Daemon Engine & Runtime Status
            </h2>
            <p className="text-xs text-slate-500 dark:text-slate-400 mt-1 max-w-2xl">
              Trạng thái và nhật ký thực tế từ daemon đang chạy. Điều khiển start/stop/restart thực hiện qua launchd/systemd ở máy chủ (không giả lập trong UI).
            </p>
          </div>
          <div className="flex items-center gap-2">
            <button
              onClick={() => loadLogs()}
              className="px-3.5 py-2 text-xs font-semibold rounded-lg bg-slate-100 dark:bg-slate-800 hover:bg-slate-200 dark:hover:bg-slate-700 text-slate-800 dark:text-slate-200 border border-slate-200 dark:border-slate-700 transition-all flex items-center gap-1.5"
            >
              <RotateCw className="w-3.5 h-3.5" />
              <span>Tải lại log</span>
            </button>
          </div>
        </div>

        <div className="grid grid-cols-2 sm:grid-cols-3 lg:grid-cols-6 gap-3 mt-5 pt-4 border-t border-slate-100 dark:border-slate-800 text-xs">
          <div>
            <span className="text-slate-400 text-[11px] block">Trạng thái</span>
            <div className="flex items-center gap-1.5 font-bold mt-0.5">
              <span className={`w-2 h-2 rounded-full ${status === 'ok' || status === 'running' ? 'bg-emerald-500 ring-2 ring-emerald-500/20' : 'bg-slate-400'}`} />
              <span className="font-mono uppercase text-slate-900 dark:text-white">{status}</span>
            </div>
          </div>
          <div>
            <span className="text-slate-400 text-[11px] block">PID</span>
            <span className="font-mono font-bold text-slate-900 dark:text-white mt-0.5 block">{pid}</span>
          </div>
          <div>
            <span className="text-slate-400 text-[11px] block">Uptime</span>
            <span className="font-mono font-bold text-slate-900 dark:text-white mt-0.5 block">{fmtUptime(uptime)}</span>
          </div>
          <div>
            <span className="text-slate-400 text-[11px] block">Database</span>
            <span className={`font-mono font-bold mt-0.5 block ${dbOk ? 'text-emerald-600 dark:text-emerald-400' : 'text-rose-500'}`}>
              {dbOk ? 'Connected' : 'Unavailable'}
            </span>
          </div>
          <div>
            <span className="text-slate-400 text-[11px] block">Active tasks</span>
            <span className="font-mono font-bold text-slate-900 dark:text-white mt-0.5 block">{activeTasks}</span>
          </div>
          <div>
            <span className="text-slate-400 text-[11px] block">Ảnh đã xử lý</span>
            <span className="font-mono font-bold text-slate-900 dark:text-white mt-0.5 block">{totalOffloaded}</span>
          </div>
        </div>
      </div>

      {/* Real config summary */}
      <div className="grid grid-cols-1 lg:grid-cols-2 gap-6">
        <div className="p-5 rounded-xl bg-white dark:bg-slate-900 border border-slate-200 dark:border-slate-800 space-y-3">
          <div className="flex items-center gap-2 border-b border-slate-100 dark:border-slate-800 pb-2">
            <Server className="w-4 h-4 text-orange-500" />
            <h3 className="text-sm font-bold text-slate-900 dark:text-white">Pipeline & Runtime</h3>
          </div>
          <div className="grid grid-cols-1 sm:grid-cols-2 gap-2 text-xs">
            <div className="p-2.5 rounded-lg bg-slate-50 dark:bg-slate-800/40 border border-slate-100 dark:border-slate-800">
              <div className="text-slate-400 text-[11px]">API Port</div>
              <div className="font-mono font-semibold text-slate-900 dark:text-white mt-0.5">{port}</div>
            </div>
            <div className="p-2.5 rounded-lg bg-slate-50 dark:bg-slate-800/40 border border-slate-100 dark:border-slate-800">
              <div className="text-slate-400 text-[11px]">Poll interval</div>
              <div className="font-mono font-semibold text-slate-900 dark:text-white mt-0.5">{pollInterval}s</div>
            </div>
            <div className="p-2.5 rounded-lg bg-slate-50 dark:bg-slate-800/40 border border-slate-100 dark:border-slate-800 flex items-start gap-2">
              <Globe className="w-3.5 h-3.5 text-slate-400 mt-0.5" />
              <div>
                <div className="text-slate-400 text-[11px]">CDN host</div>
                <div className="font-mono font-semibold text-slate-900 dark:text-white mt-0.5 break-all">{cdnHost}</div>
              </div>
            </div>
            <div className="p-2.5 rounded-lg bg-slate-50 dark:bg-slate-800/40 border border-slate-100 dark:border-slate-800 flex items-start gap-2">
              <Database className="w-3.5 h-3.5 text-slate-400 mt-0.5" />
              <div>
                <div className="text-slate-400 text-[11px]">Site</div>
                <div className="font-mono font-semibold text-slate-900 dark:text-white mt-0.5 break-all">{siteUrl || cfg?.site?.url || '—'}</div>
              </div>
            </div>
          </div>
          <div className="text-[11px] text-slate-500 dark:text-slate-400">
            Dòng chảy: WP webhook → imgbb backup → thay URL WP sang CDN proxy → xóa file local → queue upload Blogger (Chrome CDP) → CDN trỏ bản Blogger mới nhất theo image_id + image_size.
          </div>
        </div>

        <div className="p-5 rounded-xl bg-white dark:bg-slate-900 border border-slate-200 dark:border-slate-800 space-y-3">
          <div className="flex items-center gap-2 border-b border-slate-100 dark:border-slate-800 pb-2">
            <KeyRound className="w-4 h-4 text-orange-500" />
            <h3 className="text-sm font-bold text-slate-900 dark:text-white">Integrations</h3>
          </div>
          <div className="space-y-2 text-xs">
            <div className="flex items-center justify-between p-2.5 rounded-lg bg-slate-50 dark:bg-slate-800/40 border border-slate-100 dark:border-slate-800">
              <div>
                <div className="font-semibold text-slate-900 dark:text-white">imgbb backup</div>
                <div className="text-[11px] text-slate-500">format: <span className="font-mono">{imgbbFormat}</span></div>
              </div>
              <div className={`flex items-center gap-1.5 font-semibold ${imgbbEnabled ? 'text-emerald-600 dark:text-emerald-400' : 'text-slate-400'}`}>
                {imgbbEnabled ? <CheckCircle2 className="w-3.5 h-3.5" /> : <AlertCircle className="w-3.5 h-3.5" />}
                <span>{imgbbEnabled ? 'Enabled' : 'Disabled'}</span>
              </div>
            </div>
            <div className="flex items-center justify-between p-2.5 rounded-lg bg-slate-50 dark:bg-slate-800/40 border border-slate-100 dark:border-slate-800">
              <div>
                <div className="font-semibold text-slate-900 dark:text-white">Blogger (Chrome CDP)</div>
                <div className="text-[11px] text-slate-500 font-mono break-all">{bloggerId || 'blog_id not configured'}</div>
              </div>
              <div className={`flex items-center gap-1.5 font-semibold ${bloggerId ? 'text-emerald-600 dark:text-emerald-400' : 'text-amber-500'}`}>
                {bloggerId ? <CheckCircle2 className="w-3.5 h-3.5" /> : <AlertCircle className="w-3.5 h-3.5" />}
                <span>{bloggerId ? 'Configured' : 'Needs config'}</span>
              </div>
            </div>
            <div className="flex items-center justify-between p-2.5 rounded-lg bg-slate-50 dark:bg-slate-800/40 border border-slate-100 dark:border-slate-800">
              <div>
                <div className="font-semibold text-slate-900 dark:text-white">WordPress webhook</div>
                <div className="text-[11px] text-slate-500 font-mono">POST /api/webhooks/media-upload</div>
              </div>
              <div className="flex items-center gap-1.5 font-semibold text-slate-600 dark:text-slate-300">
                <Activity className="w-3.5 h-3.5" />
                <span>Bearer auth</span>
              </div>
            </div>
          </div>
        </div>
      </div>

      {/* Real logs */}
      <div className="p-5 rounded-xl bg-slate-950 border border-slate-800 text-slate-300 font-mono text-xs space-y-3">
        <div className="flex flex-col sm:flex-row sm:items-center justify-between gap-2 border-b border-slate-800 pb-3">
          <div className="flex items-center gap-2">
            <Terminal className="w-4 h-4 text-orange-400" />
            <span className="font-bold text-white text-sm">Daemon logs (real)</span>
            {logsError && <span className="text-rose-400 text-[11px]">({logsError})</span>}
          </div>
          <div className="flex flex-wrap items-center gap-2">
            <input
              value={attachmentFilter}
              onChange={(e) => setAttachmentFilter(e.target.value)}
              placeholder="attachment_id"
              inputMode="numeric"
              className="w-28 bg-slate-900 text-slate-200 text-[11px] rounded px-2 py-1 border border-slate-800"
            />
            <input
              value={search}
              onChange={(e) => setSearch(e.target.value)}
              placeholder="từ khoá log / URL / path"
              className="w-56 bg-slate-900 text-slate-200 text-[11px] rounded px-2 py-1 border border-slate-800"
            />
            <div className="flex items-center gap-1 bg-slate-900 p-0.5 rounded text-[11px]">
              {(['ALL', 'INFO', 'WARN', 'ERROR', 'DEBUG'] as const).map((lvl) => (
                <button
                  key={lvl}
                  onClick={() => setLogFilter(lvl)}
                  className={`px-2 py-0.5 rounded transition-colors ${logFilter === lvl ? 'bg-slate-700 text-white font-bold' : 'text-slate-400 hover:text-slate-200'}`}
                >
                  {lvl}
                </button>
              ))}
            </div>
            <select
              value={subsystemFilter}
              onChange={(e) => setSubsystemFilter(e.target.value)}
              className="bg-slate-900 text-slate-300 text-[11px] rounded px-2 py-1 border border-slate-800"
            >
              <option value="ALL">Tất cả subsystems</option>
              <option value="rust_daemon">rust_daemon</option>
              <option value="blogger_client">blogger_client</option>
              <option value="media_processor">media_processor</option>
              <option value="wp_client">wp_client</option>
              <option value="chrome_audit">chrome_audit</option>
            </select>
            <button onClick={() => setLogs([])} className="p-1 text-slate-500 hover:text-slate-300" title="Xóa hiển thị">
              <Trash2 className="w-3.5 h-3.5" />
            </button>
          </div>
        </div>

        <div className="h-72 overflow-y-auto space-y-1.5 pr-2 text-[11px] select-text">
          {filteredLogs.length === 0 ? (
            <div className="text-slate-500 text-center py-10">
              {logsError ? 'Không đọc được log từ daemon (cần deploy bản daemon mới có /api/logs).' : 'Chưa có log.'}
            </div>
          ) : (
            filteredLogs.map((log, i) => {
              let badgeColor = 'text-slate-400';
              if (log.level === 'INFO') badgeColor = 'text-sky-400';
              if (log.level === 'WARN') badgeColor = 'text-amber-400';
              if (log.level === 'ERROR') badgeColor = 'text-rose-400';
              if (log.level === 'DEBUG') badgeColor = 'text-slate-500';
              return (
                <div key={`${log.timestamp}-${i}`} className="flex items-start gap-2 leading-relaxed hover:bg-slate-900/60 p-0.5 rounded">
                  <span className="text-slate-500 shrink-0">{log.timestamp}</span>
                  <span className={`font-bold w-12 shrink-0 ${badgeColor}`}>[{log.level}]</span>
                  <span className="text-orange-400/90 shrink-0">[{log.subsystem}]</span>
                  {log.stage && <span className="text-violet-400/90 shrink-0">[{log.stage}]</span>}
                  {log.attachment_id != null && <span className="text-sky-400/90 shrink-0">[att:{log.attachment_id}]</span>}
                  {log.provider && <span className="text-emerald-400/80 shrink-0">[{log.provider}]</span>}
                  <span className="text-slate-300 break-all">{log.message}</span>
                  {log.target && <span className="text-slate-500 break-all hidden md:inline">· {log.target}</span>}
                </div>
              );
            })
          )}
        </div>
        <div className="flex flex-col sm:flex-row sm:items-center justify-between gap-2 text-[11px] text-slate-500 border-t border-slate-900 pt-2">
          <div className="flex items-center gap-3">
            <label className="flex items-center gap-1.5 cursor-pointer">
              <input
                type="checkbox"
                checked={autoTail}
                onChange={(e) => setAutoTail(e.target.checked)}
                className="w-3.5 h-3.5 accent-orange-500"
              />
              <span>Realtime tail (SSE 1s)</span>
            </label>
            <span>{logs.length} dòng · tổng {totalLogs.toLocaleString()} trong DB</span>
            {logPage > 0 && <span>trang {logPage + 1}</span>}
            <span className="hidden lg:inline">config: <span className="font-mono">{(config as any)?._meta?.path || '—'}</span></span>
          </div>
          <div className="flex items-center gap-1.5">
            <button
              onClick={() => loadLogs(Math.max(0, logPage - 1))}
              disabled={logPage === 0}
              className="px-2 py-1 rounded border border-slate-800 disabled:opacity-40 hover:bg-slate-900"
            >
              ‹ Trước
            </button>
            <button
              onClick={() => loadLogs(logPage + 1)}
              disabled={totalLogs > 0 && (logPage + 1) * LOG_PAGE_SIZE >= totalLogs}
              className="px-2 py-1 rounded border border-slate-800 disabled:opacity-40 hover:bg-slate-900"
            >
              Sau ›
            </button>
            <button
              onClick={() => { setLogs([]); }}
              className="px-2 py-1 rounded border border-slate-800 hover:bg-slate-900"
              title="Xoá khỏi khung hiển thị (không xoá DB)"
            >
              Xoá khung
            </button>
            <span>log lưu tại <span className="font-mono">wp_opt_agents_logs</span></span>
          </div>
        </div>
      </div>
    </div>
  );
};
