import React, { useState } from 'react';
import {
  TrendingUp,
  HardDrive,
  Gauge,
  Zap,
  Globe,
  ArrowUpRight,
  ShieldCheck,
  CheckCircle2,
  Clock,
  Sparkles,
  Server,
  Layers,
} from 'lucide-react';
import { DaemonMetrics, DailyAuditRecord } from '../types/dashboard';
import { chartSavingsHistory } from '../data/mockData';
import { formatBytes } from '../utils/bloggerUrl';

interface OverviewTabProps {
  metrics: DaemonMetrics;
  latestAudit: DailyAuditRecord;
  onNavigateTab: (tab: string) => void;
  onTriggerAudit: () => void;
  isAuditing: boolean;
}

export const OverviewTab: React.FC<OverviewTabProps> = ({
  metrics,
  latestAudit,
  onNavigateTab,
  onTriggerAudit,
  isAuditing,
}) => {
  const [chartMetric, setChartMetric] = useState<'savedGb' | 'requestsOffloaded' | 'lcpAvg'>('savedGb');
  const [hoveredDataPoint, setHoveredDataPoint] = useState<typeof chartSavingsHistory[0] | null>(null);

  // Compute chart SVG paths
  const chartHeight = 180;
  const chartWidth = 700;
  const paddingX = 30;
  const paddingY = 25;

  const values = chartSavingsHistory.map((d) => d[chartMetric]);
  const minVal = Math.min(...values) * 0.9;
  const maxVal = Math.max(...values) * 1.05;

  const points = chartSavingsHistory.map((d, index) => {
    const x = paddingX + (index / (chartSavingsHistory.length - 1)) * (chartWidth - paddingX * 2);
    const y =
      chartHeight -
      paddingY -
      ((d[chartMetric] - minVal) / (maxVal - minVal || 1)) * (chartHeight - paddingY * 2);
    return { x, y, data: d };
  });

  const pathD = points.reduce((acc, curr, idx) => {
    return idx === 0 ? `M ${curr.x} ${curr.y}` : `${acc} L ${curr.x} ${curr.y}`;
  }, '');

  const areaD = `${pathD} L ${points[points.length - 1].x} ${chartHeight - paddingY} L ${points[0].x} ${chartHeight - paddingY} Z`;

  return (
    <div className="space-y-6">
      {/* Top Welcome & Agent Status Banner */}
      <div className="p-5 rounded-xl bg-gradient-to-r from-slate-900 via-slate-800 to-slate-900 dark:from-slate-900 dark:via-slate-900 dark:to-slate-950 text-white border border-slate-700/60 shadow-lg relative overflow-hidden">
        <div className="absolute right-0 top-0 bottom-0 w-96 bg-gradient-to-l from-orange-500/10 via-amber-500/5 to-transparent pointer-events-none" />
        <div className="relative z-10 flex flex-col md:flex-row items-start md:items-center justify-between gap-4">
          <div>
            <div className="flex items-center gap-2 text-xs text-amber-400 font-mono mb-1">
              <span>RUST TOKIO ENGINE</span>
              <span>·</span>
              <span>PYO3 BRIDGE ACTIVE</span>
              <span>·</span>
              <span>GOOGLE CDN OFF-LOADED</span>
            </div>
            <h1 className="text-xl md:text-2xl font-bold tracking-tight text-white">
              Hệ thống tối ưu hóa hình ảnh WordPress qua Blogger CDN
            </h1>
            <p className="text-sm text-slate-300 max-w-2xl mt-1">
              Toàn bộ media bài viết được chuyển tiếp lên hệ thống CDN toàn cầu của Google thông qua Blogger API.
              Giảm 94% tải băng thông máy chủ WordPress, tối ưu chỉ số Core Web Vitals và đạt chuẩn LCP xanh.
            </p>
          </div>
          <div className="flex items-center gap-2.5 shrink-0">
            <button
              onClick={() => onNavigateTab('blogger_lab')}
              className="px-3.5 py-2 text-xs font-semibold rounded-lg bg-orange-600 hover:bg-orange-500 text-white shadow transition-all flex items-center gap-1.5 whitespace-nowrap"
            >
              <Zap className="w-4 h-4" />
              <span>Thử nghiệm Offload URL</span>
            </button>
            <button
              onClick={() => onNavigateTab('daemon_engine')}
              className="px-3.5 py-2 text-xs font-semibold rounded-lg bg-slate-800 hover:bg-slate-700 border border-slate-600 text-slate-200 transition-all flex items-center gap-1.5 whitespace-nowrap"
            >
              <Server className="w-4 h-4" />
              <span>Xem tiến trình Rust</span>
            </button>
          </div>
        </div>
      </div>

      {/* 4 Key KPI Metrics Strip */}
      <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-4 gap-4">
        {/* Metric 1 */}
        <div className="p-4 rounded-xl bg-white dark:bg-slate-900 border border-slate-200 dark:border-slate-800">
          <div className="flex items-center justify-between text-slate-500 dark:text-slate-400 text-xs mb-2">
            <span>Băng thông đã tiết kiệm</span>
            <HardDrive className="w-4 h-4 text-emerald-500" />
          </div>
          <div className="text-2xl font-bold font-mono text-slate-900 dark:text-white tabular-nums">
            {formatBytes(metrics.totalBytesSaved)}
          </div>
          <div className="flex items-center gap-2 mt-2 text-xs text-slate-500 dark:text-slate-400">
            <span className="text-emerald-600 dark:text-emerald-400 font-medium font-mono flex items-center">
              <TrendingUp className="w-3 h-3 mr-0.5 inline" /> -94.2%
            </span>
            <span>tải origin host</span>
          </div>
        </div>

        {/* Metric 2 */}
        <div className="p-4 rounded-xl bg-white dark:bg-slate-900 border border-slate-200 dark:border-slate-800">
          <div className="flex items-center justify-between text-slate-500 dark:text-slate-400 text-xs mb-2">
            <span>Ảnh đã offload CDN</span>
            <Globe className="w-4 h-4 text-sky-500" />
          </div>
          <div className="text-2xl font-bold font-mono text-slate-900 dark:text-white tabular-nums">
            {metrics.totalImagesOffloaded.toLocaleString()}
          </div>
          <div className="flex items-center gap-2 mt-2 text-xs text-slate-500 dark:text-slate-400">
            <span className="text-sky-600 dark:text-sky-400 font-medium font-mono">100% WebP</span>
            <span>Blogger Google Edge</span>
          </div>
        </div>

        {/* Metric 3 */}
        <div className="p-4 rounded-xl bg-white dark:bg-slate-900 border border-slate-200 dark:border-slate-800">
          <div className="flex items-center justify-between text-slate-500 dark:text-slate-400 text-xs mb-2">
            <span>Thời gian tải LCP (Mobile)</span>
            <Gauge className="w-4 h-4 text-orange-500" />
          </div>
          <div className="flex items-baseline gap-2">
            <span className="text-2xl font-bold font-mono text-slate-900 dark:text-white tabular-nums">
              {latestAudit.lcp}s
            </span>
            <span className="text-xs line-through text-slate-400 font-mono">
              {latestAudit.lcpBefore}s
            </span>
          </div>
          <div className="flex items-center gap-2 mt-2 text-xs text-slate-500 dark:text-slate-400">
            <span className="text-emerald-600 dark:text-emerald-400 font-medium font-mono">-70.6%</span>
            <span>Chuẩn xanh Google CWV</span>
          </div>
        </div>

        {/* Metric 4 */}
        <div className="p-4 rounded-xl bg-white dark:bg-slate-900 border border-slate-200 dark:border-slate-800">
          <div className="flex items-center justify-between text-slate-500 dark:text-slate-400 text-xs mb-2">
            <span>Độ trễ TTFB Blogger CDN</span>
            <Zap className="w-4 h-4 text-amber-500" />
          </div>
          <div className="flex items-baseline gap-2">
            <span className="text-2xl font-bold font-mono text-slate-900 dark:text-white tabular-nums">
              {latestAudit.ttfbBloggerMs} ms
            </span>
            <span className="text-xs text-slate-400 font-mono">
              (Origin: {latestAudit.ttfbOriginMs}ms)
            </span>
          </div>
          <div className="flex items-center gap-2 mt-2 text-xs text-slate-500 dark:text-slate-400">
            <span className="text-emerald-600 dark:text-emerald-400 font-medium font-mono">9x nhanh hơn</span>
            <span>HTTP/3 QUIC enabled</span>
          </div>
        </div>
      </div>

      {/* Main Interactive Chart & Core Web Vitals Panel */}
      <div className="grid grid-cols-1 lg:grid-cols-3 gap-6">
        {/* Left 2 Cols: 14-day interactive chart */}
        <div className="lg:col-span-2 p-5 rounded-xl bg-white dark:bg-slate-900 border border-slate-200 dark:border-slate-800 flex flex-col justify-between">
          <div>
            <div className="flex flex-col sm:flex-row sm:items-center justify-between gap-3 mb-4">
              <div>
                <h3 className="text-sm font-bold text-slate-900 dark:text-white">
                  Biểu đồ hiệu năng tối ưu hóa (14 ngày qua)
                </h3>
                <p className="text-xs text-slate-500 dark:text-slate-400 mt-0.5">
                  Thống kê lưu lượng băng thông tiết kiệm và khối lượng ảnh offload thành công
                </p>
              </div>

              {/* Metric filter buttons */}
              <div className="flex items-center gap-1 p-1 bg-slate-100 dark:bg-slate-800 rounded-lg shrink-0">
                <button
                  onClick={() => setChartMetric('savedGb')}
                  className={`px-2.5 py-1 text-xs font-medium rounded-md transition-colors whitespace-nowrap ${
                    chartMetric === 'savedGb'
                      ? 'bg-white dark:bg-slate-900 text-slate-900 dark:text-white shadow-sm'
                      : 'text-slate-600 dark:text-slate-400 hover:text-slate-900'
                  }`}
                >
                  Băng thông (GB)
                </button>
                <button
                  onClick={() => setChartMetric('requestsOffloaded')}
                  className={`px-2.5 py-1 text-xs font-medium rounded-md transition-colors whitespace-nowrap ${
                    chartMetric === 'requestsOffloaded'
                      ? 'bg-white dark:bg-slate-900 text-slate-900 dark:text-white shadow-sm'
                      : 'text-slate-600 dark:text-slate-400 hover:text-slate-900'
                  }`}
                >
                  Lượt tải CDN
                </button>
                <button
                  onClick={() => setChartMetric('lcpAvg')}
                  className={`px-2.5 py-1 text-xs font-medium rounded-md transition-colors whitespace-nowrap ${
                    chartMetric === 'lcpAvg'
                      ? 'bg-white dark:bg-slate-900 text-slate-900 dark:text-white shadow-sm'
                      : 'text-slate-600 dark:text-slate-400 hover:text-slate-900'
                  }`}
                >
                  Tốc độ LCP (s)
                </button>
              </div>
            </div>

            {/* SVG Interactive Area Chart */}
            <div className="relative w-full overflow-hidden">
              <svg
                viewBox={`0 0 ${chartWidth} ${chartHeight}`}
                className="w-full h-48 overflow-visible"
              >
                <defs>
                  <linearGradient id="chartGradient" x1="0" y1="0" x2="0" y2="1">
                    <stop offset="0%" stopColor="#f97316" stopOpacity="0.4" />
                    <stop offset="100%" stopColor="#f97316" stopOpacity="0.0" />
                  </linearGradient>
                </defs>

                {/* Horizontal Guide Lines */}
                {[0.25, 0.5, 0.75].map((pct, idx) => {
                  const y = paddingY + pct * (chartHeight - paddingY * 2);
                  return (
                    <line
                      key={idx}
                      x1={paddingX}
                      y1={y}
                      x2={chartWidth - paddingX}
                      y2={y}
                      stroke="currentColor"
                      strokeDasharray="4 4"
                      className="text-slate-200 dark:text-slate-800"
                    />
                  );
                })}

                {/* Shaded Area */}
                <path d={areaD} fill="url(#chartGradient)" />

                {/* Line Path */}
                <path
                  d={pathD}
                  fill="none"
                  stroke="#ea580c"
                  strokeWidth="2.5"
                  strokeLinecap="round"
                  strokeLinejoin="round"
                />

                {/* Data Points */}
                {points.map((pt, idx) => {
                  const isHovered = hoveredDataPoint?.day === pt.data.day;
                  return (
                    <g key={idx}>
                      <circle
                        cx={pt.x}
                        cy={pt.y}
                        r={isHovered ? 6 : 3.5}
                        fill="#fff"
                        stroke="#ea580c"
                        strokeWidth={isHovered ? 3 : 2}
                        className="transition-all cursor-pointer"
                        onMouseEnter={() => setHoveredDataPoint(pt.data)}
                      />
                    </g>
                  );
                })}
              </svg>

              {/* Tooltip on Hover */}
              {hoveredDataPoint && (
                <div className="absolute top-2 right-4 p-2.5 rounded-lg bg-slate-900 text-white text-xs shadow-xl border border-slate-700 pointer-events-none animate-fadeIn">
                  <div className="font-semibold text-slate-300 font-mono">
                    Ngày {hoveredDataPoint.day}
                  </div>
                  <div className="text-amber-400 font-bold font-mono text-sm mt-0.5">
                    {chartMetric === 'savedGb' && `${hoveredDataPoint.savedGb} GB tiết kiệm`}
                    {chartMetric === 'requestsOffloaded' &&
                      `${hoveredDataPoint.requestsOffloaded.toLocaleString()} lượt CDN`}
                    {chartMetric === 'lcpAvg' && `${hoveredDataPoint.lcpAvg}s LCP`}
                  </div>
                  <div className="text-[11px] text-slate-400 mt-1">
                    LCP: {hoveredDataPoint.lcpAvg}s · Tiết kiệm: {hoveredDataPoint.savedGb} GB
                  </div>
                </div>
              )}
            </div>

            {/* X-axis labels */}
            <div className="flex justify-between items-center px-4 mt-2 text-[11px] text-slate-400 font-mono">
              <span>{chartSavingsHistory[0].day}</span>
              <span>{chartSavingsHistory[Math.floor(chartSavingsHistory.length / 2)].day}</span>
              <span>{chartSavingsHistory[chartSavingsHistory.length - 1].day} (Hiện tại)</span>
            </div>
          </div>

          <div className="pt-4 mt-4 border-t border-slate-100 dark:border-slate-800 flex items-center justify-between text-xs text-slate-500 dark:text-slate-400">
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

        {/* Right Col: Core Web Vitals & Global Latency Breakdown */}
        <div className="p-5 rounded-xl bg-white dark:bg-slate-900 border border-slate-200 dark:border-slate-800 flex flex-col justify-between">
          <div>
            <div className="flex items-center justify-between mb-4">
              <h3 className="text-sm font-bold text-slate-900 dark:text-white">
                Core Web Vitals Tức Thì
              </h3>
              <span className="text-xs font-mono px-2 py-0.5 rounded bg-emerald-100 dark:bg-emerald-950/60 text-emerald-700 dark:text-emerald-400 font-semibold">
                Điểm: {latestAudit.lighthouseScore}/100
              </span>
            </div>

            {/* Core Metrics List */}
            <div className="space-y-3">
              {/* LCP */}
              <div className="p-3 rounded-lg bg-slate-50 dark:bg-slate-800/60 border border-slate-100 dark:border-slate-800">
                <div className="flex justify-between items-center text-xs mb-1">
                  <span className="font-medium text-slate-700 dark:text-slate-300">
                    LCP (Largest Contentful Paint)
                  </span>
                  <span className="font-mono text-emerald-600 dark:text-emerald-400 font-bold">
                    {latestAudit.lcp}s (Tốt)
                  </span>
                </div>
                <div className="w-full bg-slate-200 dark:bg-slate-700 h-1.5 rounded-full overflow-hidden">
                  <div className="bg-emerald-500 h-full rounded-full" style={{ width: '85%' }} />
                </div>
                <div className="text-[11px] text-slate-400 mt-1 flex justify-between">
                  <span>Trước offload: {latestAudit.lcpBefore}s</span>
                  <span className="text-emerald-500 font-mono">Giảm 70.6%</span>
                </div>
              </div>

              {/* FCP */}
              <div className="p-3 rounded-lg bg-slate-50 dark:bg-slate-800/60 border border-slate-100 dark:border-slate-800">
                <div className="flex justify-between items-center text-xs mb-1">
                  <span className="font-medium text-slate-700 dark:text-slate-300">
                    FCP (First Contentful Paint)
                  </span>
                  <span className="font-mono text-emerald-600 dark:text-emerald-400 font-bold">
                    {latestAudit.fcp}s
                  </span>
                </div>
                <div className="w-full bg-slate-200 dark:bg-slate-700 h-1.5 rounded-full overflow-hidden">
                  <div className="bg-emerald-500 h-full rounded-full" style={{ width: '92%' }} />
                </div>
              </div>

              {/* CLS */}
              <div className="p-3 rounded-lg bg-slate-50 dark:bg-slate-800/60 border border-slate-100 dark:border-slate-800">
                <div className="flex justify-between items-center text-xs mb-1">
                  <span className="font-medium text-slate-700 dark:text-slate-300">
                    CLS (Cumulative Layout Shift)
                  </span>
                  <span className="font-mono text-emerald-600 dark:text-emerald-400 font-bold">
                    {latestAudit.cls}
                  </span>
                </div>
                <div className="w-full bg-slate-200 dark:bg-slate-700 h-1.5 rounded-full overflow-hidden">
                  <div className="bg-emerald-500 h-full rounded-full" style={{ width: '98%' }} />
                </div>
              </div>
            </div>

            {/* Global CDN Latency Map */}
            <div className="mt-4 pt-4 border-t border-slate-100 dark:border-slate-800">
              <span className="text-xs font-semibold text-slate-900 dark:text-white block mb-2">
                Độ trễ TTFB qua mạng lưới Google Edge CDN
              </span>
              <div className="space-y-1.5 font-mono text-xs">
                {latestAudit.locations.map((loc, idx) => (
                  <div key={idx} className="flex justify-between items-center text-slate-600 dark:text-slate-400">
                    <span className="text-[11px]">{loc.city}</span>
                    <span className="text-emerald-600 dark:text-emerald-400 font-semibold tabular-nums">
                      {loc.ttfbMs} ms
                    </span>
                  </div>
                ))}
              </div>
            </div>
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

      {/* Architecture Workflow Bar */}
      <div className="p-4 rounded-xl bg-white dark:bg-slate-900 border border-slate-200 dark:border-slate-800">
        <h3 className="text-xs font-bold text-slate-900 dark:text-white mb-3">
          Chu trình hoạt động của WordPress Optimization Agents
        </h3>
        <div className="grid grid-cols-1 md:grid-cols-4 gap-3 text-xs">
          <div className="p-3 rounded-lg bg-slate-50 dark:bg-slate-800/50 border border-slate-100 dark:border-slate-800">
            <div className="font-semibold text-slate-900 dark:text-slate-200 flex items-center gap-1.5 mb-1">
              <span className="w-5 h-5 rounded-full bg-slate-200 dark:bg-slate-700 flex items-center justify-center text-[10px] font-bold font-mono">1</span>
              WordPress REST API
            </div>
            <p className="text-slate-500 dark:text-slate-400 text-[11px]">
              Quét bài viết và thư viện media local (`/wp-content/uploads/`) để phát hiện ảnh chưa offload.
            </p>
          </div>

          <div className="p-3 rounded-lg bg-slate-50 dark:bg-slate-800/50 border border-slate-100 dark:border-slate-800">
            <div className="font-semibold text-slate-900 dark:text-slate-200 flex items-center gap-1.5 mb-1">
              <span className="w-5 h-5 rounded-full bg-orange-100 dark:bg-orange-900/50 text-orange-600 dark:text-orange-300 flex items-center justify-center text-[10px] font-bold font-mono">2</span>
              Rust Tokio Daemon
            </div>
            <p className="text-slate-500 dark:text-slate-400 text-[11px]">
              Xử lý bất đồng bộ đa luồng: tải ảnh gốc, chuyển đổi tham số kích thước và chuẩn bị payload upload.
            </p>
          </div>

          <div className="p-3 rounded-lg bg-slate-50 dark:bg-slate-800/50 border border-slate-100 dark:border-slate-800">
            <div className="font-semibold text-slate-900 dark:text-slate-200 flex items-center gap-1.5 mb-1">
              <span className="w-5 h-5 rounded-full bg-amber-100 dark:bg-amber-900/50 text-amber-600 dark:text-amber-300 flex items-center justify-center text-[10px] font-bold font-mono">3</span>
              Blogger API Upload
            </div>
            <p className="text-slate-500 dark:text-slate-400 text-[11px]">
              Upload ảnh vào Blogger Album qua Google API, sinh định danh `blogger.googleusercontent.com`.
            </p>
          </div>

          <div className="p-3 rounded-lg bg-slate-50 dark:bg-slate-800/50 border border-slate-100 dark:border-slate-800">
            <div className="font-semibold text-slate-900 dark:text-slate-200 flex items-center gap-1.5 mb-1">
              <span className="w-5 h-5 rounded-full bg-emerald-100 dark:bg-emerald-900/50 text-emerald-600 dark:text-emerald-300 flex items-center justify-center text-[10px] font-bold font-mono">4</span>
              Google Global Edge CDN
            </div>
            <p className="text-slate-500 dark:text-slate-400 text-[11px]">
              Tự động crop, resize, nén WebP theo tham số URL mà không tốn dung lượng máy chủ gốc.
            </p>
          </div>
        </div>
      </div>
    </div>
  );
};
