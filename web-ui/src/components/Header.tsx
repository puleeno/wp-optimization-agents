import React from 'react';
import { Menu, Sun, Moon, RotateCw, ChevronRight, Upload } from 'lucide-react';
import { DaemonMetrics } from '../types/dashboard';

interface HeaderProps {
  activeTab: string;
  setActiveTab: (tab: string) => void;
  daemonMetrics: DaemonMetrics | null;
  darkMode: boolean;
  setDarkMode: (val: boolean) => void;
  onTriggerAudit: () => void;
  isAuditing: boolean;
  auditStage?: string;
  onOpenSidebar: () => void;
}

export const Header: React.FC<HeaderProps> = ({
  activeTab,
  setActiveTab,
  daemonMetrics,
  darkMode,
  setDarkMode,
  onTriggerAudit,
  isAuditing,
  auditStage,
  onOpenSidebar,
}) => {
  const tabTitles: Record<string, string> = {
    overview: 'Tổng quan & Hiệu năng',
    blogger_lab: 'Blogger Offload Lab & URL Generator',
    daemon_engine: 'Rust Tokio Daemon & Runtime',
    crawl_sources: 'Nguồn crawl',
    crawl_wp_media: 'Nguồn crawl: WordPress Media Library',
    crawl_audit: 'Nguồn crawl: Audit hiệu năng (Headless Chrome)',
    config: 'Cấu hình tham số hệ thống',
  };

  return (
    <header className="sticky top-0 z-30 w-full border-b border-slate-200 dark:border-slate-800 bg-white/85 dark:bg-slate-950/85 backdrop-blur-md transition-colors">
      <div className="px-4 sm:px-6 lg:px-8">
        <div className="flex items-center justify-between gap-4 h-16">
          <div className="flex items-center gap-3">
            <button onClick={onOpenSidebar} className="p-2 -ml-2 rounded-lg text-slate-500 hover:text-slate-900 dark:hover:text-white hover:bg-slate-100 dark:hover:bg-slate-900 lg:hidden" aria-label="Mở menu điều hướng">
              <Menu className="w-5 h-5" />
            </button>
            <div className="flex items-center gap-1.5 text-xs text-slate-500 dark:text-slate-400 font-mono">
              <span className="hidden sm:inline hover:text-slate-900 dark:hover:text-white cursor-pointer" onClick={() => setActiveTab('overview')}>Bảng điều khiển</span>
              <ChevronRight className="w-3.5 h-3.5 text-slate-400 hidden sm:inline" />
              <span className="font-semibold text-slate-900 dark:text-white truncate max-w-xs sm:max-w-md">{tabTitles[activeTab] || activeTab}</span>
            </div>
          </div>

          <div className="hidden md:flex items-center gap-3 text-xs">
            <div className="flex items-center gap-2 px-3 py-1 rounded-full bg-slate-100 dark:bg-slate-900 border border-slate-200 dark:border-slate-800 text-slate-600 dark:text-slate-300">
              <span className={`w-2 h-2 rounded-full ${(daemonMetrics as any)?.status === 'running' || (daemonMetrics as any)?.status === 'ok' ? 'bg-emerald-500 ring-2 ring-emerald-500/20' : 'bg-rose-500'}`} />
              <span className="font-mono text-[11px]">Rust Daemon: {String((daemonMetrics as any)?.status || 'unknown').toUpperCase()}</span>
            </div>
            <span className="text-slate-400 font-mono text-[11px]">Lưu lượng: {((daemonMetrics as any)?.totalImagesOffloaded ?? 0).toLocaleString()} ảnh</span>
          </div>

          <div className="flex items-center gap-2 shrink-0">
            {activeTab !== 'blogger_lab' && (
              <button onClick={() => setActiveTab('blogger_lab')} className="hidden sm:flex items-center gap-1.5 px-3 py-1.5 text-xs font-semibold text-orange-600 dark:text-orange-400 bg-orange-50 dark:bg-orange-950/40 hover:bg-orange-100 dark:hover:bg-orange-900/60 border border-orange-200 dark:border-orange-900/50 rounded-lg transition-all">
                <Upload className="w-3.5 h-3.5" />
                <span>Upload ảnh</span>
              </button>
            )}
            <button onClick={onTriggerAudit} disabled={isAuditing} className="flex items-center gap-1.5 px-3.5 py-1.5 text-xs font-semibold text-white bg-gradient-to-r from-orange-600 to-amber-600 hover:from-orange-500 hover:to-amber-500 rounded-lg shadow-sm transition-all disabled:opacity-50 whitespace-nowrap" title="Kích hoạt audit hiệu năng">
              {isAuditing ? (<><RotateCw className="w-3.5 h-3.5 animate-spin" /><span>{auditStage ? auditStage.slice(0, 24) : 'Đang test...'}</span></>) : (<span>Chạy audit</span>)}
            </button>
            <button onClick={() => setDarkMode(!darkMode)} className="p-2 rounded-lg text-slate-500 hover:text-slate-900 dark:hover:text-white hover:bg-slate-100 dark:hover:bg-slate-900" title={darkMode ? 'Sáng' : 'Tối'}>
              {darkMode ? <Sun className="w-4 h-4" /> : <Moon className="w-4 h-4" />}
            </button>
          </div>
        </div>
      </div>
    </header>
  );
};
