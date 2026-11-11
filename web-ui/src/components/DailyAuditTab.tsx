import React, { useState } from 'react';
import {
  Calendar,
  CheckCircle2,
  AlertTriangle,
  Play,
  RotateCw,
  Globe,
  Gauge,
  Smartphone,
  Monitor,
  Sparkles,
  ArrowUpRight,
  ShieldAlert,
  Clock,
  Send,
  Zap,
} from 'lucide-react';
import { DailyAuditRecord } from '../types/dashboard';

interface DailyAuditTabProps {
  audits: DailyAuditRecord[];
  onTriggerAudit: () => void;
  isAuditing: boolean;
  activeAuditStep?: number;
}

export const DailyAuditTab: React.FC<DailyAuditTabProps> = ({
  audits,
  onTriggerAudit,
  isAuditing,
  activeAuditStep = 0,
}) => {
  const [selectedAudit, setSelectedAudit] = useState<DailyAuditRecord>(audits[0]);
  const [deviceFilter, setDeviceFilter] = useState<'all' | 'mobile' | 'desktop'>('all');

  const filteredAudits = audits.filter((audit) => {
    if (deviceFilter !== 'all' && audit.device !== deviceFilter) return false;
    return true;
  });

  const auditSteps = [
    'Khởi động Chrome Automation Headless (DevTools Protocol)',
    'Giả lập thiết bị di động & Băng thông mạng 4G',
    'Tải trang & Đo đạc chỉ số Core Web Vitals (LCP, CLS, FCP)',
    'Kiểm tra tính hợp lệ của thẻ ảnh Blogger CDN và WebP Cache',
    'Hoàn tất báo cáo và đồng bộ dữ liệu vào Dashboard',
  ];

  return (
    <div className="space-y-6">
      {/* Top Banner & Run Test CTA */}
      <div className="p-5 rounded-xl bg-white dark:bg-slate-900 border border-slate-200 dark:border-slate-800 flex flex-col md:flex-row md:items-center justify-between gap-4">
        <div>
          <div className="flex items-center gap-2 text-xs text-orange-600 dark:text-orange-400 font-mono font-medium">
            <span>AUTOMATED DAILY AUDIT SUITE</span>
            <span>·</span>
            <span>HEADLESS CHROME ENGINE</span>
          </div>
          <h2 className="text-lg font-bold text-slate-900 dark:text-white mt-0.5">
            Báo cáo hiệu năng định kỳ & Công cụ kiểm tra tự động hàng ngày
          </h2>
          <p className="text-xs text-slate-500 dark:text-slate-400 mt-1 max-w-2xl">
            Lập lịch chạy tự động lúc 03:00 AM UTC mỗi ngày bằng Headless Chrome để giám sát các chỉ số
            Core Web Vitals quan trọng, thời gian tải ảnh thực tế và cảnh báo suy giảm tốc độ.
          </p>
        </div>

        <div className="flex items-center gap-2">
          <button
            onClick={onTriggerAudit}
            disabled={isAuditing}
            className="px-4 py-2 text-xs font-semibold text-white bg-orange-600 hover:bg-orange-500 disabled:opacity-50 rounded-lg shadow-sm transition-all flex items-center gap-1.5 whitespace-nowrap"
          >
            {isAuditing ? (
              <>
                <RotateCw className="w-3.5 h-3.5 animate-spin" />
                <span>Đang thực hiện kiểm tra...</span>
              </>
            ) : (
              <>
                <Play className="w-3.5 h-3.5 fill-current" />
                <span>Chạy kiểm tra tự động ngay</span>
              </>
            )}
          </button>
        </div>
      </div>

      {/* Live Chrome Headless Runner Step Card if active */}
      {isAuditing && (
        <div className="p-5 rounded-xl bg-slate-900 text-white border border-orange-500/50 shadow-xl space-y-4 animate-fadeIn">
          <div className="flex items-center justify-between">
            <div className="flex items-center gap-2">
              <span className="w-2.5 h-2.5 rounded-full bg-orange-500 animate-ping" />
              <h3 className="text-sm font-bold text-white font-mono">
                Headless Chrome Automation Worker Đang Chạy
              </h3>
            </div>
            <span className="text-xs font-mono text-amber-400">
              Mục tiêu: https://vietnamtechreview.com/danh-gia-sony-a7-iv/
            </span>
          </div>

          <div className="space-y-2.5">
            {auditSteps.map((step, idx) => {
              const isCurrent = activeAuditStep === idx;
              const isDone = activeAuditStep > idx;

              return (
                <div
                  key={idx}
                  className={`flex items-center gap-3 text-xs p-2.5 rounded-lg font-mono transition-all ${
                    isCurrent
                      ? 'bg-orange-950/60 border border-orange-500/40 text-orange-200'
                      : isDone
                      ? 'bg-slate-800/60 text-emerald-400'
                      : 'text-slate-500'
                  }`}
                >
                  <div className="w-5 h-5 rounded-full flex items-center justify-center shrink-0 text-[10px] font-bold">
                    {isDone ? (
                      <CheckCircle2 className="w-4 h-4 text-emerald-400" />
                    ) : isCurrent ? (
                      <RotateCw className="w-3.5 h-3.5 animate-spin text-orange-400" />
                    ) : (
                      <span>{idx + 1}</span>
                    )}
                  </div>
                  <span>{step}</span>
                </div>
              );
            })}
          </div>
        </div>
      )}

      {/* Selected Audit Deep-Dive Spotlight Card */}
      <div className="p-5 rounded-xl bg-white dark:bg-slate-900 border border-slate-200 dark:border-slate-800 space-y-4">
        <div className="flex flex-col sm:flex-row sm:items-center justify-between gap-3 border-b border-slate-100 dark:border-slate-800 pb-3">
          <div>
            <div className="flex items-center gap-2">
              <span className="text-xs font-bold font-mono text-orange-600 dark:text-orange-400">
                CHI TIẾT ĐỢT KIỂM TRA
              </span>
              <span>·</span>
              <span className="text-xs font-mono text-slate-500">{selectedAudit.timestamp}</span>
            </div>
            <h3 className="text-sm font-bold text-slate-900 dark:text-white mt-0.5 truncate max-w-xl">
              {selectedAudit.targetSiteUrl}
            </h3>
          </div>

          <div className="flex items-center gap-2">
            <span
              className={`px-2.5 py-1 rounded text-xs font-mono font-bold ${
                selectedAudit.status === 'passed'
                  ? 'bg-emerald-100 dark:bg-emerald-950/60 text-emerald-700 dark:text-emerald-400'
                  : 'bg-amber-100 dark:bg-amber-950/60 text-amber-700 dark:text-amber-400'
              }`}
            >
              Lighthouse: {selectedAudit.lighthouseScore}/100 ({selectedAudit.status.toUpperCase()})
            </span>
          </div>
        </div>

        {/* 4 Detail Metrics */}
        <div className="grid grid-cols-2 md:grid-cols-4 gap-3 text-xs">
          <div className="p-3 rounded-lg bg-slate-50 dark:bg-slate-800/40 border border-slate-100 dark:border-slate-800">
            <span className="text-slate-400 text-[11px] block">LCP (Largest Paint)</span>
            <div className="text-lg font-bold font-mono text-emerald-600 dark:text-emerald-400 mt-0.5">
              {selectedAudit.lcp}s
            </div>
            <span className="text-[10px] text-slate-400 line-through">Gốc: {selectedAudit.lcpBefore}s</span>
          </div>

          <div className="p-3 rounded-lg bg-slate-50 dark:bg-slate-800/40 border border-slate-100 dark:border-slate-800">
            <span className="text-slate-400 text-[11px] block">CLS Layout Shift</span>
            <div className="text-lg font-bold font-mono text-emerald-600 dark:text-emerald-400 mt-0.5">
              {selectedAudit.cls}
            </div>
            <span className="text-[10px] text-emerald-500 font-semibold">Tuyệt đối ổn định</span>
          </div>

          <div className="p-3 rounded-lg bg-slate-50 dark:bg-slate-800/40 border border-slate-100 dark:border-slate-800">
            <span className="text-slate-400 text-[11px] block">Độ trễ TTFB Blogger CDN</span>
            <div className="text-lg font-bold font-mono text-emerald-600 dark:text-emerald-400 mt-0.5">
              {selectedAudit.ttfbBloggerMs} ms
            </div>
            <span className="text-[10px] text-slate-400">Máy chủ gốc: {selectedAudit.ttfbOriginMs}ms</span>
          </div>

          <div className="p-3 rounded-lg bg-slate-50 dark:bg-slate-800/40 border border-slate-100 dark:border-slate-800">
            <span className="text-slate-400 text-[11px] block">Băng thông tiết kiệm</span>
            <div className="text-lg font-bold font-mono text-sky-600 dark:text-sky-400 mt-0.5">
              {selectedAudit.bandwidthSavedGb} GB
            </div>
            <span className="text-[10px] text-slate-400">{selectedAudit.imagesAudited} ảnh kiểm tra</span>
          </div>
        </div>

        {/* Global CDN Latency Grid */}
        <div className="pt-2">
          <span className="text-xs font-semibold text-slate-700 dark:text-slate-300 block mb-2">
            Độ trễ truyền tải ảnh Blogger CDN theo khu vực địa lý:
          </span>
          <div className="grid grid-cols-2 sm:grid-cols-3 md:grid-cols-5 gap-2 font-mono text-xs">
            {selectedAudit.locations.map((loc, idx) => (
              <div
                key={idx}
                className="p-2.5 rounded-lg border border-slate-200 dark:border-slate-800 bg-slate-50 dark:bg-slate-800/30 flex justify-between items-center"
              >
                <div>
                  <div className="font-semibold text-slate-800 dark:text-slate-200 text-[11px]">
                    {loc.city}
                  </div>
                  <div className="text-[10px] text-slate-400">{loc.region}</div>
                </div>
                <div className="text-emerald-600 dark:text-emerald-400 font-bold text-xs">
                  {loc.ttfbMs}ms
                </div>
              </div>
            ))}
          </div>
        </div>

        {/* Audit Notes */}
        <div className="p-3 rounded-lg bg-slate-50 dark:bg-slate-800/60 text-xs text-slate-600 dark:text-slate-300">
          <span className="font-semibold text-slate-900 dark:text-white mr-1.5">Ghi chú tự động:</span>
          {selectedAudit.notes}
        </div>
      </div>

      {/* Daily Audit History Table */}
      <div className="rounded-xl border border-slate-200 dark:border-slate-800 bg-white dark:bg-slate-900 overflow-hidden">
        <div className="p-4 border-b border-slate-100 dark:border-slate-800 flex items-center justify-between">
          <h3 className="text-sm font-bold text-slate-900 dark:text-white">
            Lịch sử kiểm tra tự động hàng ngày
          </h3>

          <div className="flex items-center gap-1 p-1 bg-slate-100 dark:bg-slate-800 rounded-lg text-xs">
            <button
              onClick={() => setDeviceFilter('all')}
              className={`px-2.5 py-1 rounded font-medium ${
                deviceFilter === 'all'
                  ? 'bg-white dark:bg-slate-900 text-slate-900 dark:text-white shadow-sm'
                  : 'text-slate-600 dark:text-slate-400'
              }`}
            >
              Tất cả
            </button>
            <button
              onClick={() => setDeviceFilter('mobile')}
              className={`px-2.5 py-1 rounded font-medium ${
                deviceFilter === 'mobile'
                  ? 'bg-white dark:bg-slate-900 text-slate-900 dark:text-white shadow-sm'
                  : 'text-slate-600 dark:text-slate-400'
              }`}
            >
              Mobile (4G)
            </button>
            <button
              onClick={() => setDeviceFilter('desktop')}
              className={`px-2.5 py-1 rounded font-medium ${
                deviceFilter === 'desktop'
                  ? 'bg-white dark:bg-slate-900 text-slate-900 dark:text-white shadow-sm'
                  : 'text-slate-600 dark:text-slate-400'
              }`}
            >
              Desktop
            </button>
          </div>
        </div>

        <div className="overflow-x-auto">
          <table className="w-full text-left text-xs">
            <thead className="bg-slate-50 dark:bg-slate-800/60 border-b border-slate-200 dark:border-slate-800 text-slate-500 dark:text-slate-400 uppercase font-mono text-[11px]">
              <tr>
                <th className="py-3 px-4">Ngày chạy</th>
                <th className="py-3 px-4">URL bài viết / Trang</th>
                <th className="py-3 px-4">Thiết bị</th>
                <th className="py-3 px-4">Lighthouse</th>
                <th className="py-3 px-4">LCP Tải ảnh</th>
                <th className="py-3 px-4">TTFB Blogger</th>
                <th className="py-3 px-4">Tiết kiệm</th>
                <th className="py-3 px-4 text-right">Xem lại</th>
              </tr>
            </thead>
            <tbody className="divide-y divide-slate-100 dark:divide-slate-800">
              {filteredAudits.map((audit) => {
                const isSelected = selectedAudit.id === audit.id;
                return (
                  <tr
                    key={audit.id}
                    onClick={() => setSelectedAudit(audit)}
                    className={`cursor-pointer transition-colors ${
                      isSelected
                        ? 'bg-orange-50/50 dark:bg-orange-950/20'
                        : 'hover:bg-slate-50/70 dark:hover:bg-slate-800/40'
                    }`}
                  >
                    <td className="py-3 px-4 font-mono font-semibold text-slate-800 dark:text-slate-200">
                      {audit.dateStr}
                    </td>

                    <td className="py-3 px-4">
                      <div className="text-slate-900 dark:text-white font-medium truncate max-w-xs">
                        {audit.targetSiteUrl}
                      </div>
                    </td>

                    <td className="py-3 px-4">
                      <span className="flex items-center gap-1 text-slate-600 dark:text-slate-300 capitalize">
                        {audit.device === 'mobile' ? (
                          <Smartphone className="w-3.5 h-3.5 text-orange-500" />
                        ) : (
                          <Monitor className="w-3.5 h-3.5 text-sky-500" />
                        )}
                        <span>{audit.device}</span>
                      </span>
                    </td>

                    <td className="py-3 px-4">
                      <span className="font-mono font-bold text-emerald-600 dark:text-emerald-400">
                        {audit.lighthouseScore}/100
                      </span>
                    </td>

                    <td className="py-3 px-4 font-mono font-bold text-slate-900 dark:text-white">
                      {audit.lcp}s
                    </td>

                    <td className="py-3 px-4 font-mono text-emerald-600 dark:text-emerald-400">
                      {audit.ttfbBloggerMs} ms
                    </td>

                    <td className="py-3 px-4 font-mono text-sky-600 dark:text-sky-400">
                      {audit.bandwidthSavedGb} GB
                    </td>

                    <td className="py-3 px-4 text-right">
                      <span className="text-orange-600 dark:text-orange-400 font-semibold text-[11px] hover:underline">
                        Chi tiết
                      </span>
                    </td>
                  </tr>
                );
              })}
            </tbody>
          </table>
        </div>
      </div>
    </div>
  );
};
