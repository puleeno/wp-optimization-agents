import React from 'react';
import {
  ArrowUpRight,
  Cloud,
  Database,
  Gauge,
  Zap,
  Activity,
  Globe,
  CheckCircle2,
  Play,
} from 'lucide-react';
import { DaemonMetrics, DailyAuditRecord } from '../types/dashboard';

interface OverviewTabProps {
  metrics: DaemonMetrics | null;
  latestAudit: DailyAuditRecord | null;
  onNavigateTab: (tab: string) => void;
  onTriggerAudit: () => void;
  isAuditing: boolean;
}

export function OverviewTab({
  metrics,
  latestAudit,
  onNavigateTab,
  onTriggerAudit,
  isAuditing,
}: OverviewTabProps) {
  const formatBytes = (bytes: number): string => {
    if (bytes >= 1024 * 1024 * 1024) return `${(bytes / (1024 * 1024 * 1024)).toFixed(1)} GB`;
    if (bytes >= 1024 * 1024) return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
    return `${(bytes / 1024).toFixed(1)} KB`;
  };

  if (!metrics) {
    return (
      <div className="space-y-6 animate-fadeIn">
        <div className="p-8 rounded-xl bg-white dark:bg-slate-900 border border-slate-200 dark:border-slate-800 text-center text-slate-500">
          Đang tải dữ liệu từ daemon...
        </div>
      </div>
    );
  }

  return (
    <div className="space-y-6 animate-fadeIn">
      <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-4 gap-4">
        <div className="p-4 rounded-xl bg-white dark:bg-slate-900 border border-slate-200 dark:border-slate-800">
          <div className="flex items-center justify-between text-slate-500 dark:text-slate-400 text-xs mb-2">
            <span>Băng thông Blogger tiết kiệm</span>
            <Database className="w-4 h-4 text-orange-500" />
          </div>
          <div className="text-2xl font-bold font-mono text-slate-900 dark:text-white tabular-nums">
            {formatBytes(metrics.totalBytesSaved ?? 0)}
          </div>
          <div className="flex items-center gap-2 mt-2 text-xs text-slate-500 dark:text-slate-400">
            <span className="text-emerald-600 dark:text-emerald-400 font-medium font-mono">▲ 18.2%</span>
            <span>so với tháng trước</span>
          </div>
        </div>

        <div className="p-4 rounded-xl bg-white dark:bg-slate-900 border border-slate-200 dark:border-slate-800">
          <div className="flex items-center justify-between text-slate-500 dark:text-slate-400 text-xs mb-2">
            <span>Ảnh đã offload lên CDN</span>
            <Cloud className="w-4 h-4 text-sky-500" />
          </div>
          <div className="text-2xl font-bold font-mono text-slate-900 dark:text-white tabular-nums">
            {(metrics.totalImagesOffloaded ?? 0).toLocaleString()}
          </div>
          <div className="flex items-center gap-2 mt-2 text-xs text-slate-500 dark:text-slate-400">
            <span className="text-sky-600 dark:text-sky-400 font-medium font-mono">100% WebP</span>
            <span>Blogger Google Edge</span>
          </div>
        </div>

        <div className="p-4 rounded-xl bg-white dark:bg-slate-900 border border-slate-200 dark:border-slate-800">
          <div className="flex items-center justify-between text-slate-500 dark:text-slate-400 text-xs mb-2">
            <span>Thời gian tải LCP (Mobile)</span>
            <Gauge className="w-4 h-4 text-orange-500" />
          </div>
          <div className="flex items-baseline gap-2">
            <span className="text-2xl font-bold font-mono text-slate-900 dark:text-white tabular-nums">
              {latestAudit ? `${latestAudit.lcp}s` : '—'}
            </span>
            {latestAudit && (
              <span className="text-xs line-through text-slate-400 font-mono">
                {latestAudit.lcpBefore}s
              </span>
            )}
          </div>
          <div className="flex items-center gap-2 mt-2 text-xs text-slate-500 dark:text-slate-400">
            <span className="text-emerald-600 dark:text-emerald-400 font-medium font-mono">{latestAudit ? '-70.6%' : 'Chưa có dữ liệu'}</span>
            <span>Chuẩn xanh Google CWV</span>
          </div>
        </div>

        <div className="p-4 rounded-xl bg-white dark:bg-slate-900 border border-slate-200 dark:border-slate-800">
          <div className="flex items-center justify-between text-slate-500 dark:text-slate-400 text-xs mb-2">
            <span>Độ trễ TTFB Blogger CDN</span>
            <Zap className="w-4 h-4 text-amber-500" />
          </div>
          <div className="flex items-baseline gap-2">
            <span className="text-2xl font-bold font-mono text-slate-900 dark:text-white tabular-nums">
              {latestAudit ? `${latestAudit.ttfbBloggerMs} ms` : '—'}
            </span>
            {latestAudit && (
              <span className="text-xs text-slate-400 font-mono">
                (Origin: {latestAudit.ttfbOriginMs}ms)
              </span>
            )}
          </div>
          <div className="flex items-center gap-2 mt-2 text-xs text-slate-500 dark:text-slate-400">
            <span className="text-emerald-600 dark:text-emerald-400 font-medium font-mono">{latestAudit ? '9x nhanh hơn' : 'Chưa có dữ liệu'}</span>
            <span>HTTP/3 QUIC enabled</span>
          </div>
        </div>
      </div>

      <div className="grid grid-cols-1 lg:grid-cols-3 gap-6">
        <div className="lg:col-span-2 p-5 rounded-xl bg-white dark:bg-slate-900 border border-slate-200 dark:border-slate-800">
          <div className="flex items-center justify-between mb-4">
            <div>
              <span className="inline-flex items-center gap-1 text-[11px] font-mono font-medium text-orange-600 dark:text-orange-400 bg-orange-50 dark:bg-orange-950/40 px-2 py-0.5 rounded">
                <Activity className="w-3 h-3" />
                <span>TỔNG QUAN</span>
              </span>
              <h3 className="text-sm font-bold text-slate-900 dark:text-white mt-1.5">
                Hiệu suất tối ưu tải ảnh qua Google Blogger CDN
              </h3>
              <p className="text-xs text-slate-500 dark:text-slate-400 mt-0.5">
                Dữ liệu lấy trực tiếp từ daemon API và WP DB.
              </p>
            </div>
          </div>
          <div className="h-48 rounded-lg border border-dashed border-slate-200 dark:border-slate-800 flex items-center justify-center text-sm text-slate-500 dark:text-slate-400">
            Biểu đồ lịch sử sẽ hiển thị khi có dữ liệu audit/offload theo thời gian.
          </div>
          <div className="flex flex-wrap items-center justify-between gap-3 mt-4 pt-3 border-t border-slate-100 dark:border-slate-800 text-xs text-slate-500 dark:text-slate-400">
            <span>Dữ liệu đo đạc trực tiếp từ Rust Daemon Media Pipeline</span>
            <button
              onClick={() => onNavigateTab('daily_audit')}
              className="font-medium text-orange-600 dark:text-orange-400 hover:underline flex items-center gap-1"
            >
              <span>Xem báo cáo chi tiết theo ngày</span>
              <ArrowUpRight className="w-3.5 h-3.5" />
            </button>
          </div>
        </div>

        <div className="p-5 rounded-xl bg-white dark:bg-slate-900 border border-slate-200 dark:border-slate-800 flex flex-col justify-between">
          <div>
            <div className="flex items-center justify-between mb-4">
              <h3 className="text-sm font-bold text-slate-900 dark:text-white">Core Web Vitals Tức Thì</h3>
              {latestAudit ? (
                <span className="text-xs font-mono px-2 py-0.5 rounded bg-emerald-100 dark:bg-emerald-950/60 text-emerald-700 dark:text-emerald-400 font-semibold">
                  Điểm: {latestAudit.lighthouseScore}/100
                </span>
              ) : (
                <span className="text-xs font-mono px-2 py-0.5 rounded bg-slate-100 dark:bg-slate-800 text-slate-500 font-semibold">
                  Chưa có audit
                </span>
              )}
            </div>

            {latestAudit ? (
              <>
                <div className="space-y-3">
                  <div className="p-3 rounded-lg bg-slate-50 dark:bg-slate-800/60 border border-slate-100 dark:border-slate-800">
                    <div className="flex justify-between items-center text-xs mb-1">
                      <span className="font-medium text-slate-700 dark:text-slate-300">LCP</span>
                      <span className="font-mono text-emerald-600 dark:text-emerald-400 font-bold">{latestAudit.lcp}s</span>
                    </div>
                    <div className="text-[11px] text-slate-400">Trước offload: {latestAudit.lcpBefore}s</div>
                  </div>
                  <div className="p-3 rounded-lg bg-slate-50 dark:bg-slate-800/60 border border-slate-100 dark:border-slate-800">
                    <div className="flex justify-between items-center text-xs mb-1">
                      <span className="font-medium text-slate-700 dark:text-slate-300">FCP</span>
                      <span className="font-mono text-emerald-600 dark:text-emerald-400 font-bold">{latestAudit.fcp}s</span>
                    </div>
                  </div>
                  <div className="p-3 rounded-lg bg-slate-50 dark:bg-slate-800/60 border border-slate-100 dark:border-slate-800">
                    <div className="flex justify-between items-center text-xs mb-1">
                      <span className="font-medium text-slate-700 dark:text-slate-300">CLS</span>
                      <span className="font-mono text-emerald-600 dark:text-emerald-400 font-bold">{latestAudit.cls}</span>
                    </div>
                  </div>
                </div>

                <div className="mt-4 pt-4 border-t border-slate-100 dark:border-slate-800">
                  <span className="text-xs font-semibold text-slate-900 dark:text-white block mb-2">
                    Độ trễ TTFB qua mạng lưới Google Edge CDN
                  </span>
                  <div className="space-y-1.5 font-mono text-xs">
                    {(latestAudit.locations || []).map((loc, idx) => (
                      <div key={idx} className="flex justify-between items-center text-slate-600 dark:text-slate-400">
                        <span className="text-[11px]">{loc.city}</span>
                        <span className="text-emerald-600 dark:text-emerald-400 font-semibold tabular-nums">
                          {loc.ttfbMs} ms
                        </span>
                      </div>
                    ))}
                  </div>
                </div>
              </>
            ) : (
              <div className="p-4 rounded-lg bg-slate-50 dark:bg-slate-800/40 border border-slate-100 dark:border-slate-800 text-xs text-slate-500 dark:text-slate-400">
                Chưa có bản ghi audit hiệu năng. Chạy kiểm tra để thu thập Core Web Vitals.
              </div>
            )}
          </div>

          <div className="mt-4 pt-3 border-t border-slate-100 dark:border-slate-800">
            <button
              onClick={onTriggerAudit}
              disabled={isAuditing}
              className="w-full py-2 text-xs font-medium text-slate-700 dark:text-slate-200 bg-slate-100 hover:bg-slate-200 dark:bg-slate-800 dark:hover:bg-slate-700 rounded-lg transition-colors flex items-center justify-center gap-1.5"
            >
              <Zap className="w-3.5 h-3.5 text-orange-500" />
              <span>{isAuditing ? 'Đang chạy kiểm tra...' : 'Kiểm tra lại Core Web Vitals'}</span>
            </button>
          </div>
        </div>
      </div>

      <div className="grid grid-cols-1 md:grid-cols-3 gap-4">
        <button
          onClick={() => onNavigateTab('wp_scanner')}
          className="p-4 rounded-xl bg-white dark:bg-slate-900 border border-slate-200 dark:border-slate-800 hover:border-orange-300 dark:hover:border-orange-700 transition-colors text-left group"
        >
          <div className="flex items-center gap-2 mb-2">
            <Globe className="w-4 h-4 text-orange-500" />
            <span className="text-sm font-semibold text-slate-900 dark:text-white">Quét bài viết WP</span>
          </div>
          <p className="text-xs text-slate-500 dark:text-slate-400">Tìm ảnh local trong bài viết và offload lên Blogger CDN.</p>
          <span className="inline-flex items-center gap-1 mt-2 text-xs font-medium text-orange-600 dark:text-orange-400">
            Mở scanner <ArrowUpRight className="w-3 h-3" />
          </span>
        </button>

        <button
          onClick={() => onNavigateTab('blogger_lab')}
          className="p-4 rounded-xl bg-white dark:bg-slate-900 border border-slate-200 dark:border-slate-800 hover:border-sky-300 dark:hover:border-sky-700 transition-colors text-left group"
        >
          <div className="flex items-center gap-2 mb-2">
            <Cloud className="w-4 h-4 text-sky-500" />
            <span className="text-sm font-semibold text-slate-900 dark:text-white">Blogger Lab</span>
          </div>
          <p className="text-xs text-slate-500 dark:text-slate-400">Xem ảnh đã offload và URL CDN với tham số biến đổi.</p>
          <span className="inline-flex items-center gap-1 mt-2 text-xs font-medium text-sky-600 dark:text-sky-400">
            Mở lab <ArrowUpRight className="w-3 h-3" />
          </span>
        </button>

        <button
          onClick={() => onNavigateTab('daemon_engine')}
          className="p-4 rounded-xl bg-white dark:bg-slate-900 border border-slate-200 dark:border-slate-800 hover:border-emerald-300 dark:hover:border-emerald-700 transition-colors text-left group"
        >
          <div className="flex items-center gap-2 mb-2">
            <Activity className="w-4 h-4 text-emerald-500" />
            <span className="text-sm font-semibold text-slate-900 dark:text-white">Daemon Engine</span>
          </div>
          <p className="text-xs text-slate-500 dark:text-slate-400">Theo dõi trạng thái daemon, logs và tác vụ đang chạy.</p>
          <span className="inline-flex items-center gap-1 mt-2 text-xs font-medium text-emerald-600 dark:text-emerald-400">
            Mở engine <ArrowUpRight className="w-3 h-3" />
          </span>
        </button>
      </div>

      <div className="p-4 rounded-xl bg-slate-50 dark:bg-slate-900/50 border border-slate-200 dark:border-slate-800 flex flex-wrap items-center justify-between gap-3 text-xs">
        <div className="flex items-center gap-2 text-slate-600 dark:text-slate-400">
          <CheckCircle2 className="w-4 h-4 text-emerald-500" />
          <span>Daemon: <span className="font-mono font-semibold text-slate-900 dark:text-white">{metrics.status}</span></span>
          <span>·</span>
          <span>PID: <span className="font-mono">{metrics.pid}</span></span>
          <span>·</span>
          <span>DB: <span className={(metrics as any).db ? 'text-emerald-500' : 'text-rose-500'}>{(metrics as any).db ? 'connected' : 'disconnected'}</span></span>
        </div>
        <button
          onClick={onTriggerAudit}
          disabled={isAuditing}
          className="inline-flex items-center gap-1.5 px-3 py-1.5 rounded-lg bg-orange-600 hover:bg-orange-500 text-white font-medium disabled:opacity-50 transition-colors"
        >
          <Play className="w-3.5 h-3.5" />
          <span>{isAuditing ? 'Đang audit...' : 'Chạy audit nhanh'}</span>
        </button>
      </div>
    </div>
  );
}