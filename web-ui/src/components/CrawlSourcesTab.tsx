import React from 'react';
import { Database, Gauge, ChevronRight, Globe, Image as ImageIcon, ScanSearch } from 'lucide-react';

interface CrawlSourcesTabProps {
  onNavigate: (path: string) => void;
  scanInfo?: any;
  auditCount?: number;
}

export const CrawlSourcesTab: React.FC<CrawlSourcesTabProps> = ({ onNavigate, scanInfo, auditCount = 0 }) => {
  const sources = [
    {
      id: 'wp-media',
      title: 'WordPress Media Library',
      desc: 'Quét attachments (post, page, product, custom post type), offload ảnh sang imgbb/Blogger và thay URL CDN.',
      path: '/crawl/wp-media',
      icon: ImageIcon,
      accent: 'text-orange-600 dark:text-orange-400',
      bg: 'bg-orange-50 dark:bg-orange-950/30 border-orange-200 dark:border-orange-900/40',
      stats: scanInfo ? `${Number(scanInfo.pending ?? 0).toLocaleString()} chờ · ${Number(scanInfo.processed ?? 0).toLocaleString()} đã xử lý` : 'Nguồn crawl media WP',
    },
    {
      id: 'audit',
      title: 'Audit hiệu năng (Chrome Headless)',
      desc: 'Thu thập Lighthouse/LCP/TTFB theo lịch hoặc thủ công cho các URL mục tiêu.',
      path: '/crawl/audit',
      icon: Gauge,
      accent: 'text-sky-600 dark:text-sky-400',
      bg: 'bg-sky-50 dark:bg-sky-950/30 border-sky-200 dark:border-sky-900/40',
      stats: `${auditCount} bản ghi audit`,
    },
  ];

  return (
    <div className="space-y-6">
      <div className="p-5 rounded-xl bg-white dark:bg-slate-900 border border-slate-200 dark:border-slate-800">
        <div className="flex items-center gap-2 text-xs text-orange-600 dark:text-orange-400 font-mono font-medium">
          <ScanSearch className="w-4 h-4" />
          <span>NGUỒN CRAWL</span>
        </div>
        <h2 className="text-lg font-bold text-slate-900 dark:text-white mt-1">Nguồn crawl & thu thập dữ liệu</h2>
        <p className="text-xs text-slate-500 dark:text-slate-400 mt-1 max-w-3xl">
          Mỗi nguồn crawl được tách thành màn hình riêng với route rõ ràng để theo dõi, cấu hình và debug độc lập.
        </p>
      </div>

      <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
        {sources.map((s) => {
          const Icon = s.icon;
          return (
            <button
              key={s.id}
              onClick={() => onNavigate(s.path)}
              className={`text-left p-5 rounded-xl border ${s.bg} hover:shadow-sm transition-all`}
            >
              <div className="flex items-start justify-between gap-3">
                <div className="flex items-center gap-3">
                  <div className="w-10 h-10 rounded-lg bg-white/70 dark:bg-slate-900/60 border border-slate-200/60 dark:border-slate-700 flex items-center justify-center">
                    <Icon className={`w-5 h-5 ${s.accent}`} />
                  </div>
                  <div>
                    <div className="text-sm font-bold text-slate-900 dark:text-white">{s.title}</div>
                    <div className="text-[11px] font-mono text-slate-500 dark:text-slate-400 mt-0.5">{s.stats}</div>
                  </div>
                </div>
                <ChevronRight className="w-4 h-4 text-slate-400 mt-1" />
              </div>
              <p className="text-xs text-slate-600 dark:text-slate-300 mt-3 leading-relaxed">{s.desc}</p>
              <div className="mt-3 text-[11px] font-mono text-slate-500 dark:text-slate-400 flex items-center gap-1.5">
                <Globe className="w-3.5 h-3.5" /> Route: <span className="font-semibold">{s.path}</span>
              </div>
            </button>
          );
        })}
      </div>

      <div className="p-4 rounded-xl bg-slate-50 dark:bg-slate-900/60 border border-dashed border-slate-300 dark:border-slate-700 text-xs text-slate-600 dark:text-slate-400 flex items-start gap-2">
        <Database className="w-4 h-4 mt-0.5 shrink-0" />
        <div>
          <div className="font-semibold text-slate-800 dark:text-slate-200">Thêm nguồn crawl mới</div>
          <div className="mt-0.5">Nguồn crawl mới (ví dụ PPDVN hoặc site khác) nên được thêm như một mục riêng trong nhóm này với route <code>/crawl/&lt;source&gt;</code>, log và trạng thái tách biệt.</div>
        </div>
      </div>
    </div>
  );
};
