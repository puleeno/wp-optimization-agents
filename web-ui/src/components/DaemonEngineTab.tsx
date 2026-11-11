import React, { useState } from 'react';
import {
  Server,
  Play,
  Square,
  RotateCw,
  Cpu,
  Activity,
  Terminal,
  Shield,
  Layers,
  FileCode2,
  CheckCircle,
  AlertCircle,
  Download,
  Trash2,
  Code,
  Box,
} from 'lucide-react';
import { DaemonMetrics, AgentLog, RuntimeTarget, DaemonStatus } from '../types/dashboard';

interface DaemonEngineTabProps {
  metrics: DaemonMetrics;
  setMetrics: React.Dispatch<React.SetStateAction<DaemonMetrics>>;
  logs: AgentLog[];
  setLogs: React.Dispatch<React.SetStateAction<AgentLog[]>>;
}

export const DaemonEngineTab: React.FC<DaemonEngineTabProps> = ({
  metrics,
  setMetrics,
  logs,
  setLogs,
}) => {
  const [logFilter, setLogFilter] = useState<'ALL' | 'INFO' | 'WARN' | 'ERROR' | 'DEBUG'>('ALL');
  const [subsystemFilter, setSubsystemFilter] = useState<string>('ALL');
  const [selectedModule, setSelectedModule] = useState<'rust' | 'python'>('rust');
  const [isRestarting, setIsRestarting] = useState(false);

  // Toggle Daemon
  const handleToggleDaemon = () => {
    if (metrics.status === 'running') {
      setMetrics((prev) => ({
        ...prev,
        status: 'stopped',
        cpuPercent: 0,
      }));
      appendLog('WARN', 'rust_daemon', 'Signal SIGTERM received. Rust Tokio daemon stopped gracefully.');
    } else {
      setMetrics((prev) => ({
        ...prev,
        status: 'running',
        pid: Math.floor(10000 + Math.random() * 89999),
        uptimeSeconds: 1,
        cpuPercent: 1.2,
      }));
      appendLog('INFO', 'rust_daemon', 'Spawned Rust daemon process. Tokio runtime initialized with 4 workers.');
      appendLog('INFO', 'py_orchestrator', 'PyO3 bindings initialized. Loaded python module `agents.python.offload`.');
    }
  };

  const handleRestart = () => {
    setIsRestarting(true);
    appendLog('INFO', 'rust_daemon', 'Daemon restart requested via management IPC.');
    setTimeout(() => {
      setMetrics((prev) => ({
        ...prev,
        status: 'running',
        pid: Math.floor(10000 + Math.random() * 89999),
        uptimeSeconds: 0,
        cpuPercent: 1.6,
      }));
      setIsRestarting(false);
      appendLog('INFO', 'rust_daemon', 'Rust daemon restarted successfully. All tokio worker tasks healthy.');
    }, 1200);
  };

  const appendLog = (level: AgentLog['level'], subsystem: AgentLog['subsystem'], message: string) => {
    const newLog: AgentLog = {
      id: `log-${Date.now()}`,
      timestamp: new Date().toTimeString().split(' ')[0] + '.' + Math.floor(Math.random() * 900 + 100),
      level,
      subsystem,
      message,
    };
    setLogs((prev) => [newLog, ...prev]);
  };

  const filteredLogs = logs.filter((log) => {
    if (logFilter !== 'ALL' && log.level !== logFilter) return false;
    if (subsystemFilter !== 'ALL' && log.subsystem !== subsystemFilter) return false;
    return true;
  });

  return (
    <div className="space-y-6">
      {/* Top Daemon Status Strip & Controls */}
      <div className="p-5 rounded-xl bg-white dark:bg-slate-900 border border-slate-200 dark:border-slate-800">
        <div className="flex flex-col lg:flex-row lg:items-center justify-between gap-4">
          <div>
            <div className="flex items-center gap-2 text-xs text-orange-600 dark:text-orange-400 font-mono font-medium">
              <span>AGENTS/DAEMON/</span>
              <span>·</span>
              <span>CARGO TOKIO ENGINE</span>
              <span>·</span>
              <span>PYO3 FFI</span>
            </div>
            <h2 className="text-lg font-bold text-slate-900 dark:text-white mt-0.5">
              Rust Daemon Service & Python Orchestration Architecture
            </h2>
            <p className="text-xs text-slate-500 dark:text-slate-400 mt-1 max-w-2xl">
              Tiến trình nền Rust chịu trách nhiệm xử lý luồng song song không khóa (non-blocking async),
              tính toán biến đổi URL Blogger và điều phối kết nối PyO3 tới tầng kịch bản Python.
            </p>
          </div>

          {/* Action buttons */}
          <div className="flex items-center gap-2">
            <button
              onClick={handleToggleDaemon}
              className={`px-3.5 py-2 text-xs font-semibold rounded-lg flex items-center gap-1.5 transition-all shadow-sm ${
                metrics.status === 'running'
                  ? 'bg-rose-600 hover:bg-rose-500 text-white'
                  : 'bg-emerald-600 hover:bg-emerald-500 text-white'
              }`}
            >
              {metrics.status === 'running' ? (
                <>
                  <Square className="w-3.5 h-3.5 fill-current" />
                  <span>Dừng Daemon</span>
                </>
              ) : (
                <>
                  <Play className="w-3.5 h-3.5 fill-current" />
                  <span>Khởi động Daemon</span>
                </>
              )}
            </button>

            <button
              onClick={handleRestart}
              disabled={isRestarting}
              className="px-3.5 py-2 text-xs font-semibold rounded-lg bg-slate-100 dark:bg-slate-800 hover:bg-slate-200 dark:hover:bg-slate-700 text-slate-800 dark:text-slate-200 border border-slate-200 dark:border-slate-700 transition-all flex items-center gap-1.5"
            >
              <RotateCw className={`w-3.5 h-3.5 ${isRestarting ? 'animate-spin' : ''}`} />
              <span>{isRestarting ? 'Đang khởi động lại...' : 'Khởi động lại'}</span>
            </button>
          </div>
        </div>

        {/* Runtime & Telemetry Badges */}
        <div className="grid grid-cols-2 sm:grid-cols-3 lg:grid-cols-6 gap-3 mt-5 pt-4 border-t border-slate-100 dark:border-slate-800 text-xs">
          <div>
            <span className="text-slate-400 text-[11px] block">Trạng thái Service</span>
            <div className="flex items-center gap-1.5 font-bold mt-0.5">
              <span
                className={`w-2 h-2 rounded-full ${
                  metrics.status === 'running' ? 'bg-emerald-500 ring-2 ring-emerald-500/20' : 'bg-rose-500'
                }`}
              />
              <span className="font-mono capitalize text-slate-900 dark:text-white">
                {metrics.status}
              </span>
            </div>
          </div>

          <div>
            <span className="text-slate-400 text-[11px] block">Tiến trình (PID)</span>
            <span className="font-mono font-bold text-slate-900 dark:text-white mt-0.5 block">
              {metrics.status === 'running' ? metrics.pid : '—'}
            </span>
          </div>

          <div>
            <span className="text-slate-400 text-[11px] block">Tokio Workers</span>
            <span className="font-mono font-bold text-slate-900 dark:text-white mt-0.5 block">
              {metrics.tokioWorkers} Threads
            </span>
          </div>

          <div>
            <span className="text-slate-400 text-[11px] block">Bộ nhớ RAM (RSS)</span>
            <span className="font-mono font-bold text-slate-900 dark:text-white mt-0.5 block">
              {metrics.memoryUsageMb} MB
            </span>
          </div>

          <div>
            <span className="text-slate-400 text-[11px] block">PyO3 C-API State</span>
            <span className="font-mono font-bold text-emerald-600 dark:text-emerald-400 mt-0.5 block">
              Bound & Ready
            </span>
          </div>

          <div>
            <span className="text-slate-400 text-[11px] block">Runtime Target</span>
            <select
              value={metrics.target}
              onChange={(e) =>
                setMetrics({ ...metrics, target: e.target.value as RuntimeTarget })
              }
              className="mt-0.5 text-xs font-mono bg-slate-50 dark:bg-slate-800 border border-slate-200 dark:border-slate-700 rounded px-1.5 py-0.5 text-slate-800 dark:text-slate-200"
            >
              <option value="linux-systemd">Linux (systemd)</option>
              <option value="macos-launchd">macOS (launchd)</option>
              <option value="docker">Docker Container</option>
            </select>
          </div>
        </div>
      </div>

      {/* Architecture Modules Map (Rust vs Python) */}
      <div className="grid grid-cols-1 lg:grid-cols-2 gap-6">
        {/* Left: Rust Daemon Structure */}
        <div className="p-5 rounded-xl bg-white dark:bg-slate-900 border border-slate-200 dark:border-slate-800 space-y-3">
          <div className="flex items-center justify-between border-b border-slate-100 dark:border-slate-800 pb-2">
            <div className="flex items-center gap-2">
              <span className="px-2 py-0.5 rounded bg-orange-100 dark:bg-orange-950/60 text-orange-700 dark:text-orange-400 text-[11px] font-mono font-bold">
                RUST CORE
              </span>
              <h3 className="text-sm font-bold text-slate-900 dark:text-white">
                agents/daemon/ (Rust Tokio)
              </h3>
            </div>
            <span className="text-[11px] text-slate-400 font-mono">Cargo Build</span>
          </div>

          <div className="space-y-2 text-xs">
            <div className="p-2.5 rounded-lg bg-slate-50 dark:bg-slate-800/40 border border-slate-100 dark:border-slate-800">
              <div className="flex justify-between items-center font-mono font-semibold text-slate-900 dark:text-white">
                <span>config.rs</span>
                <span className="text-[10px] text-emerald-500 font-bold">Serde JSON</span>
              </div>
              <p className="text-[11px] text-slate-500 dark:text-slate-400 mt-0.5">
                Đọc và xác thực cấu hình JSON thời gian thực, quản lý các quy tắc kích thước ảnh tối đa.
              </p>
            </div>

            <div className="p-2.5 rounded-lg bg-slate-50 dark:bg-slate-800/40 border border-slate-100 dark:border-slate-800">
              <div className="flex justify-between items-center font-mono font-semibold text-slate-900 dark:text-white">
                <span>media_processor.rs</span>
                <span className="text-[10px] text-orange-500 font-bold">URL Transform</span>
              </div>
              <p className="text-[11px] text-slate-500 dark:text-slate-400 mt-0.5">
                Phân tích cú pháp URL Blogger Googleusercontent và sinh chuỗi tham số `=w...-h...-c-rw` siêu tốc.
              </p>
            </div>

            <div className="p-2.5 rounded-lg bg-slate-50 dark:bg-slate-800/40 border border-slate-100 dark:border-slate-800">
              <div className="flex justify-between items-center font-mono font-semibold text-slate-900 dark:text-white">
                <span>main_lib.rs</span>
                <span className="text-[10px] text-sky-500 font-bold">OffloadManager</span>
              </div>
              <p className="text-[11px] text-slate-500 dark:text-slate-400 mt-0.5">
                Điều phối hàng đợi tác vụ Tokio, chống xung đột ghi đè đồng thời và ghi log cảnh báo.
              </p>
            </div>

            <div className="p-2.5 rounded-lg bg-slate-50 dark:bg-slate-800/40 border border-slate-100 dark:border-slate-800">
              <div className="flex justify-between items-center font-mono font-semibold text-slate-900 dark:text-white">
                <span>lib.rs</span>
                <span className="text-[10px] text-purple-500 font-bold">PyO3 Public API</span>
              </div>
              <p className="text-[11px] text-slate-500 dark:text-slate-400 mt-0.5">
                Xuất các hàm native FFI `#[pyfunction]` cho phép Python gọi trực tiếp không mất overhead.
              </p>
            </div>
          </div>
        </div>

        {/* Right: Python Scripts Structure */}
        <div className="p-5 rounded-xl bg-white dark:bg-slate-900 border border-slate-200 dark:border-slate-800 space-y-3">
          <div className="flex items-center justify-between border-b border-slate-100 dark:border-slate-800 pb-2">
            <div className="flex items-center gap-2">
              <span className="px-2 py-0.5 rounded bg-blue-100 dark:bg-blue-950/60 text-blue-700 dark:text-blue-400 text-[11px] font-mono font-bold">
                PYTHON ORCHESTRATION
              </span>
              <h3 className="text-sm font-bold text-slate-900 dark:text-white">
                agents/python/ (Python 3.12+)
              </h3>
            </div>
            <span className="text-[11px] text-slate-400 font-mono">Pydantic & Aiohttp</span>
          </div>

          <div className="space-y-2 text-xs">
            <div className="p-2.5 rounded-lg bg-slate-50 dark:bg-slate-800/40 border border-slate-100 dark:border-slate-800">
              <div className="flex justify-between items-center font-mono font-semibold text-slate-900 dark:text-white">
                <span>config.py</span>
                <span className="text-[10px] text-emerald-500 font-bold">Pydantic V2</span>
              </div>
              <p className="text-[11px] text-slate-500 dark:text-slate-400 mt-0.5">
                Schema mô hình hóa cấu hình WordPress REST API, Blogger OAuth2 tokens và lịch audit hàng ngày.
              </p>
            </div>

            <div className="p-2.5 rounded-lg bg-slate-50 dark:bg-slate-800/40 border border-slate-100 dark:border-slate-800">
              <div className="flex justify-between items-center font-mono font-semibold text-slate-900 dark:text-white">
                <span>api_clients.py</span>
                <span className="text-[10px] text-sky-500 font-bold">WP & Blogger SDK</span>
              </div>
              <p className="text-[11px] text-slate-500 dark:text-slate-400 mt-0.5">
                Wrapper giao tiếp với WordPress REST endpoint và Google Blogger API v3 (tải ảnh vào album).
              </p>
            </div>

            <div className="p-2.5 rounded-lg bg-slate-50 dark:bg-slate-800/40 border border-slate-100 dark:border-slate-800">
              <div className="flex justify-between items-center font-mono font-semibold text-slate-900 dark:text-white">
                <span>offload.py</span>
                <span className="text-[10px] text-amber-500 font-bold">Workflow Orchestrator</span>
              </div>
              <p className="text-[11px] text-slate-500 dark:text-slate-400 mt-0.5">
                Kịch bản chính: tải ảnh WP, gọi Rust media_processor, upload Blogger và ghi đè nội dung post HTML.
              </p>
            </div>

            <div className="p-2.5 rounded-lg bg-slate-50 dark:bg-slate-800/40 border border-slate-100 dark:border-slate-800">
              <div className="flex justify-between items-center font-mono font-semibold text-slate-900 dark:text-white">
                <span>monitor.py</span>
                <span className="text-[10px] text-purple-500 font-bold">Metrics & Slack</span>
              </div>
              <p className="text-[11px] text-slate-500 dark:text-slate-400 mt-0.5">
                Thu thập thống kê tài nguyên, gửi webhook cảnh báo khi phát hiện LCP &gt; 2.5s hoặc lỗi upload.
              </p>
            </div>
          </div>
        </div>
      </div>

      {/* Interactive Terminal Log Viewer */}
      <div className="p-5 rounded-xl bg-slate-950 border border-slate-800 text-slate-300 font-mono text-xs space-y-3">
        <div className="flex flex-col sm:flex-row sm:items-center justify-between gap-2 border-b border-slate-800 pb-3">
          <div className="flex items-center gap-2">
            <Terminal className="w-4 h-4 text-orange-400" />
            <span className="font-bold text-white text-sm">
              Nhật ký thời gian thực (Agents Daemon Stream)
            </span>
          </div>

          {/* Filters */}
          <div className="flex items-center gap-2">
            {/* Level selector */}
            <div className="flex items-center gap-1 bg-slate-900 p-0.5 rounded text-[11px]">
              {(['ALL', 'INFO', 'WARN', 'ERROR', 'DEBUG'] as const).map((lvl) => (
                <button
                  key={lvl}
                  onClick={() => setLogFilter(lvl)}
                  className={`px-2 py-0.5 rounded transition-colors ${
                    logFilter === lvl
                      ? 'bg-slate-700 text-white font-bold'
                      : 'text-slate-400 hover:text-slate-200'
                  }`}
                >
                  {lvl}
                </button>
              ))}
            </div>

            {/* Subsystem selector */}
            <select
              value={subsystemFilter}
              onChange={(e) => setSubsystemFilter(e.target.value)}
              className="bg-slate-900 text-slate-300 text-[11px] rounded px-2 py-1 border border-slate-800"
            >
              <option value="ALL">Tất cả Subsystems</option>
              <option value="rust_daemon">rust_daemon</option>
              <option value="py_orchestrator">py_orchestrator</option>
              <option value="media_processor">media_processor</option>
              <option value="blogger_client">blogger_client</option>
              <option value="wp_client">wp_client</option>
              <option value="chrome_audit">chrome_audit</option>
            </select>

            <button
              onClick={() => setLogs([])}
              className="p-1 text-slate-500 hover:text-slate-300"
              title="Xóa nhật ký"
            >
              <Trash2 className="w-3.5 h-3.5" />
            </button>
          </div>
        </div>

        {/* Log Lines Container */}
        <div className="h-64 overflow-y-auto space-y-1.5 pr-2 font-mono text-[11px] select-text">
          {filteredLogs.length === 0 ? (
            <div className="text-slate-500 text-center py-10">Không có dòng log nào khớp với bộ lọc.</div>
          ) : (
            filteredLogs.map((log) => {
              let badgeColor = 'text-slate-400';
              if (log.level === 'INFO') badgeColor = 'text-sky-400';
              if (log.level === 'WARN') badgeColor = 'text-amber-400';
              if (log.level === 'ERROR') badgeColor = 'text-rose-400';
              if (log.level === 'DEBUG') badgeColor = 'text-slate-500';

              return (
                <div key={log.id} className="flex items-start gap-2 leading-relaxed hover:bg-slate-900/60 p-0.5 rounded">
                  <span className="text-slate-500 shrink-0">{log.timestamp}</span>
                  <span className={`font-bold w-12 shrink-0 ${badgeColor}`}>
                    [{log.level}]
                  </span>
                  <span className="text-orange-400/90 shrink-0">
                    [{log.subsystem}]
                  </span>
                  <span className="text-slate-300 break-all">{log.message}</span>
                </div>
              );
            })
          )}
        </div>

        <div className="flex items-center justify-between text-[11px] text-slate-500 border-t border-slate-900 pt-2">
          <span>RUST_LOG=debug · PyO3 thread pooling enabled</span>
          <button
            onClick={() => {
              appendLog(
                'INFO',
                'media_processor',
                `Manual URL transformation verified: https://blogger.googleusercontent.com/img/a/Test_${Date.now()}=w800-h500-c-rw`
              );
            }}
            className="text-orange-400 hover:underline flex items-center gap-1"
          >
            <Play className="w-3 h-3" />
            <span>Gửi lệnh Ping thử nghiệm Rust</span>
          </button>
        </div>
      </div>
    </div>
  );
};
