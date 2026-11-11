import React from 'react';
import {
  LayoutDashboard,
  CloudUpload,
  FileText,
  Gauge,
  Cpu,
  Settings,
  Zap,
  Globe,
  CheckCircle2,
  AlertTriangle,
  Play,
  RotateCw,
  X,
  ChevronRight,
  ShieldCheck,
  Server,
  Layers,
} from 'lucide-react';
import { DaemonMetrics } from '../types/dashboard';

interface SidebarProps {
  activeTab: string;
  setActiveTab: (tab: string) => void;
  daemonMetrics: DaemonMetrics;
  pendingPostsCount: number;
  isOpen: boolean;
  onClose: () => void;
  onTriggerAudit: () => void;
  isAuditing: boolean;
}

export const Sidebar: React.FC<SidebarProps> = ({
  activeTab,
  setActiveTab,
  daemonMetrics,
  pendingPostsCount,
  isOpen,
  onClose,
  onTriggerAudit,
  isAuditing,
}) => {
  const mainNavItems = [
    {
      id: 'overview',
      label: 'Tổng quan & Hiệu năng',
      icon: LayoutDashboard,
      badge: null,
    },
    {
      id: 'blogger_lab',
      label: 'Blogger Offload Lab',
      icon: CloudUpload,
      badge: 'Chính',
      badgeColor: 'bg-orange-500/10 text-orange-600 dark:text-orange-400 font-semibold',
    },
    {
      id: 'wp_scanner',
      label: 'Quét bài viết WP',
      icon: FileText,
      badge: pendingPostsCount > 0 ? `${pendingPostsCount} chờ` : null,
      badgeColor: 'bg-amber-500/10 text-amber-600 dark:text-amber-400 font-mono',
    },
  ];

  const systemNavItems = [
    {
      id: 'daily_audit',
      label: 'Kiểm tra tự động',
      icon: Gauge,
      badge: '03:00 UTC',
      badgeColor: 'text-slate-400 font-mono text-[10px]',
    },
    {
      id: 'daemon_engine',
      label: 'Rust & Python Daemon',
      icon: Cpu,
      badge: 'Tokio',
      badgeColor: 'bg-emerald-500/10 text-emerald-600 dark:text-emerald-400 font-mono',
    },
    {
      id: 'config',
      label: 'Cấu hình tham số',
      icon: Settings,
      badge: null,
    },
  ];

  return (
    <>
      {/* Mobile Backdrop Overlay */}
      {isOpen && (
        <div
          onClick={onClose}
          className="fixed inset-0 z-40 bg-black/60 backdrop-blur-sm lg:hidden animate-fadeIn"
          aria-hidden="true"
        />
      )}

      {/* Sidebar Container */}
      <aside
        className={`fixed top-0 bottom-0 left-0 z-50 w-64 bg-white dark:bg-slate-950 border-r border-slate-200 dark:border-slate-800 flex flex-col justify-between transition-transform duration-300 ease-in-out lg:translate-x-0 ${
          isOpen ? 'translate-x-0 shadow-2xl' : '-translate-x-full'
        }`}
      >
        {/* Top: Brand Wordmark & Agent Badge */}
        <div>
          <div className="h-16 flex items-center justify-between px-5 border-b border-slate-100 dark:border-slate-800">
            <div className="flex items-center gap-2.5">
              <div className="w-8 h-8 rounded-lg bg-gradient-to-tr from-amber-500 via-orange-600 to-red-600 flex items-center justify-center text-white shadow-sm font-black">
                <Zap className="w-4 h-4 text-white" />
              </div>
              <div>
                <span className="text-sm font-bold tracking-tight text-slate-900 dark:text-white block leading-none">
                  WP OptiAgent
                </span>
                <span className="text-[10px] text-slate-500 dark:text-slate-400 font-mono block mt-1">
                  Rust Tokio · PyO3 v2.4
                </span>
              </div>
            </div>

            {/* Mobile Close Button */}
            <button
              onClick={onClose}
              className="p-1.5 rounded-md text-slate-400 hover:text-slate-700 dark:hover:text-white lg:hidden"
            >
              <X className="w-5 h-5" />
            </button>
          </div>

          {/* WordPress Connection Status Strip */}
          <div className="px-5 py-2.5 bg-slate-50/70 dark:bg-slate-900/60 border-b border-slate-100 dark:border-slate-800/80 flex items-center justify-between text-xs">
            <div className="flex items-center gap-1.5 truncate">
              <span className="w-1.5 h-1.5 rounded-full bg-emerald-500 shrink-0" />
              <span className="text-[11px] text-slate-600 dark:text-slate-300 font-mono truncate">
                vietnamtechreview.com
              </span>
            </div>
            <span className="text-[10px] font-mono text-emerald-600 dark:text-emerald-400 font-semibold shrink-0">
              WP 6.8
            </span>
          </div>

          {/* Navigation Links */}
          <div className="p-3 space-y-6 overflow-y-auto">
            {/* Section 1: Main */}
            <div>
              <div className="px-3 mb-1.5 text-[10px] font-bold uppercase tracking-wider text-slate-400 font-mono">
                Quy trình chính
              </div>
              <nav className="space-y-0.5">
                {mainNavItems.map((item) => {
                  const isActive = activeTab === item.id;
                  const Icon = item.icon;
                  return (
                    <button
                      key={item.id}
                      onClick={() => {
                        setActiveTab(item.id);
                        onClose();
                      }}
                      className={`w-full flex items-center justify-between px-3 py-2 text-xs font-medium rounded-lg transition-colors text-left ${
                        isActive
                          ? 'bg-slate-900 text-white dark:bg-slate-100 dark:text-slate-900 shadow-sm'
                          : 'text-slate-600 dark:text-slate-400 hover:text-slate-900 dark:hover:text-slate-100 hover:bg-slate-100 dark:hover:bg-slate-900/80'
                      }`}
                    >
                      <div className="flex items-center gap-2.5 truncate">
                        <Icon className={`w-4 h-4 shrink-0 ${isActive ? 'text-orange-500 dark:text-orange-600' : ''}`} />
                        <span className="truncate">{item.label}</span>
                      </div>
                      {item.badge && (
                        <span
                          className={`text-[10px] px-1.5 py-0.5 rounded ${
                            isActive
                              ? 'bg-slate-800 text-slate-200 dark:bg-slate-200 dark:text-slate-800'
                              : item.badgeColor
                          }`}
                        >
                          {item.badge}
                        </span>
                      )}
                    </button>
                  );
                })}
              </nav>
            </div>

            {/* Section 2: Automation & Architecture */}
            <div>
              <div className="px-3 mb-1.5 text-[10px] font-bold uppercase tracking-wider text-slate-400 font-mono">
                Hệ thống & Tự động
              </div>
              <nav className="space-y-0.5">
                {systemNavItems.map((item) => {
                  const isActive = activeTab === item.id;
                  const Icon = item.icon;
                  return (
                    <button
                      key={item.id}
                      onClick={() => {
                        setActiveTab(item.id);
                        onClose();
                      }}
                      className={`w-full flex items-center justify-between px-3 py-2 text-xs font-medium rounded-lg transition-colors text-left ${
                        isActive
                          ? 'bg-slate-900 text-white dark:bg-slate-100 dark:text-slate-900 shadow-sm'
                          : 'text-slate-600 dark:text-slate-400 hover:text-slate-900 dark:hover:text-slate-100 hover:bg-slate-100 dark:hover:bg-slate-900/80'
                      }`}
                    >
                      <div className="flex items-center gap-2.5 truncate">
                        <Icon className={`w-4 h-4 shrink-0 ${isActive ? 'text-orange-500 dark:text-orange-600' : ''}`} />
                        <span className="truncate">{item.label}</span>
                      </div>
                      {item.badge && (
                        <span
                          className={`text-[10px] px-1.5 py-0.5 rounded ${
                            isActive
                              ? 'bg-slate-800 text-slate-200 dark:bg-slate-200 dark:text-slate-800'
                              : item.badgeColor
                          }`}
                        >
                          {item.badge}
                        </span>
                      )}
                    </button>
                  );
                })}
              </nav>
            </div>
          </div>
        </div>

        {/* Bottom Section: Daemon Service Status Card & Quick Action */}
        <div className="p-3 border-t border-slate-100 dark:border-slate-800 space-y-2">
          {/* Daemon Status Card */}
          <div className="p-3 rounded-lg bg-slate-50 dark:bg-slate-900/80 border border-slate-200 dark:border-slate-800">
            <div className="flex items-center justify-between mb-1.5">
              <div className="flex items-center gap-1.5 text-xs font-semibold text-slate-800 dark:text-slate-200">
                <span
                  className={`w-2 h-2 rounded-full ${
                    daemonMetrics.status === 'running'
                      ? 'bg-emerald-500 ring-2 ring-emerald-500/20'
                      : 'bg-rose-500'
                  }`}
                />
                <span>Rust Tokio Service</span>
              </div>
              <span className="text-[10px] font-mono text-slate-400">
                PID:{daemonMetrics.pid}
              </span>
            </div>

            <div className="grid grid-cols-2 gap-1 text-[11px] font-mono text-slate-500 dark:text-slate-400 pt-1 border-t border-slate-200/60 dark:border-slate-800/60">
              <div>RAM: {daemonMetrics.memoryUsageMb}MB</div>
              <div>CPU: {daemonMetrics.cpuPercent}%</div>
              <div>Tokio: {daemonMetrics.tokioWorkers}T</div>
              <div>PyO3: OK</div>
            </div>
          </div>

          {/* Quick trigger audit button */}
          <button
            onClick={onTriggerAudit}
            disabled={isAuditing}
            className="w-full py-2 px-3 text-xs font-semibold text-white bg-gradient-to-r from-orange-600 to-amber-600 hover:from-orange-500 hover:to-amber-500 rounded-lg shadow-sm transition-all flex items-center justify-center gap-1.5 disabled:opacity-50"
          >
            {isAuditing ? (
              <>
                <RotateCw className="w-3.5 h-3.5 animate-spin" />
                <span>Đang chạy audit...</span>
              </>
            ) : (
              <>
                <Play className="w-3.5 h-3.5 fill-current" />
                <span>Kiểm tra tốc độ ngay</span>
              </>
            )}
          </button>
        </div>
      </aside>
    </>
  );
};
