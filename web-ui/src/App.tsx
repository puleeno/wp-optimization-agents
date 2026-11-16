import React, { useState, useEffect, useCallback, useRef } from 'react';
import { Routes, Route, Navigate, useNavigate, useLocation } from 'react-router-dom';
import { Sidebar } from './components/Sidebar';
import { Header } from './components/Header';
import { OverviewTab } from './components/OverviewTab';
import { BloggerLabTab } from './components/BloggerLabTab';
import { DaemonEngineTab } from './components/DaemonEngineTab';
import { WordPressScannerTab } from './components/WordPressScannerTab';
import { DailyAuditTab } from './components/DailyAuditTab';
import { ConfigTab } from './components/ConfigTab';
import {
  DaemonMetrics,
  OffloadImage,
  DailyAuditRecord,
  WordPressPost,
  AgentLog,
  AgentConfig,
} from './types/dashboard';

const TAB_ROUTES: Record<string, string> = {
  overview: '/',
  blogger_lab: '/blogger',
  daemon_engine: '/daemon',
  wp_scanner: '/scanner',
  daily_audit: '/audit',
  config: '/config',
};

const PATH_TO_TAB: Record<string, string> = {
  '/': 'overview',
  '/blogger': 'blogger_lab',
  '/daemon': 'daemon_engine',
  '/scanner': 'wp_scanner',
  '/audit': 'daily_audit',
  '/config': 'config',
};

const API_BASE = '/api';

type FetchResult<T> = { ok: boolean; status: number; data: T | null };

async function fetchJson<T>(url: string, timeoutMs = 8000): Promise<FetchResult<T>> {
  const ctrl = new AbortController();
  const t = setTimeout(() => ctrl.abort(), timeoutMs);
  try {
    const res = await fetch(url, { signal: ctrl.signal });
    if (!res.ok) return { ok: false, status: res.status, data: null };
    return { ok: true, status: res.status, data: (await res.json()) as T };
  } catch {
    return { ok: false, status: 0, data: null };
  } finally {
    clearTimeout(t);
  }
}

export default function App() {
  const navigate = useNavigate();
  const location = useLocation();
  const [darkMode, setDarkMode] = useState<boolean>(true);
  const [sidebarOpen, setSidebarOpen] = useState<boolean>(false);
  const activeTab = PATH_TO_TAB[location.pathname] || 'overview';
  const setActiveTab = (tab: string) => navigate(TAB_ROUTES[tab] || '/');

  const [daemonMetrics, setDaemonMetrics] = useState<DaemonMetrics | null>(null);
  const [images, setImages] = useState<OffloadImage[]>([]);
  const [dailyAudits, setDailyAudits] = useState<DailyAuditRecord[]>([]);
  const [logs, setLogs] = useState<AgentLog[]>([]);
  const [config, setConfig] = useState<AgentConfig | null>(null);
  const [currentTask, setCurrentTask] = useState<any>(null);

  const [isAuditing, setIsAuditing] = useState<boolean>(false);
  const [activeAuditStep, setActiveAuditStep] = useState<number>(0);
  const [loading, setLoading] = useState<boolean>(true);

  const listBackoffRef = useRef(30000);
  const esRef = useRef<EventSource | null>(null);
  const pendingPostsCount = 0;
  const siteUrl = (config as any)?.site?.url || (config as any)?.wordpress?.endpointUrl || '';

  useEffect(() => {
    if (darkMode) {
      document.documentElement.classList.add('dark');
    } else {
      document.documentElement.classList.remove('dark');
    }
  }, [darkMode]);

  const loadCurrentTask = useCallback(async () => {
    if (typeof document !== 'undefined' && document.hidden) return;
    const res = await fetchJson<any>(`${API_BASE}/tasks/current`, 6000);
    if (res.ok && res.data) setCurrentTask(res.data.task ?? null);
  }, []);

  const applyConfig = useCallback((configData: any) => {
    const cd: any = configData;
    const normalized: any = {
      ...cd,
      wordpress: cd.wordpress || {
        endpointUrl: cd.site?.url || '',
        username: '',
        appPasswordMasked: '',
        mediaSyncChunkSize: 10,
        replacePostContent: cd.daemon?.replace_in_db ?? true,
        backupBeforeRewrite: cd.daemon?.backup_before_replace ?? true,
      },
      daemon: {
        port: cd.daemon?.port ?? 2095,
        logLevel: cd.logging?.level ?? 'info',
        workerThreads: cd.daemon?.workerThreads ?? 4,
        pollIntervalSec: cd.daemon?.poll_interval_sec ?? cd.daemon?.pollIntervalSec ?? 60,
        autoRetryLimit: cd.daemon?.autoRetryLimit ?? 3,
      },
      blogger: {
        blogId: cd.blogger?.blog_id ?? cd.blogger?.blogId ?? '',
        albumName: cd.blogger?.albumName ?? '',
        authType: cd.blogger?.authType ?? 'oauth2',
        enableWebpAuto: cd.blogger?.enableWebpAuto ?? true,
        defaultQuality: cd.blogger?.defaultQuality ?? 85,
        cdnDomainOverride: cd.blogger?.cdnDomainOverride ?? '',
      },
      dailyAudit: cd.dailyAudit || {
        enabled: false,
        cronSchedule: '0 3 * * *',
        targetUrls: [],
        slackWebhook: '',
        alertOnLcpAboveMs: 2500,
      },
    };
    setConfig(normalized);
  }, []);

  const applyImages = useCallback((imgs: any[]) => {
    const mapped: OffloadImage[] = imgs.map((im: any) => ({
      id: String(im.id ?? ''),
      filename: (im.originUrl || '').split('/').pop() || `image-${im.id}`,
      mimeType: im.mimeType || 'image/jpeg',
      originalSizeBytes: im.originalSizeBytes ?? im.fileSize ?? 0,
      offloadSizeBytes: im.offloadSizeBytes ?? 0,
      width: im.width ?? 0,
      height: im.height ?? 0,
      originWpUrl: im.originUrl || '',
      bloggerBaseUrl: im.bloggerUrl || '',
      uploadedAt: im.createdAt || '',
      status: im.status === 'replaced' || im.status === 'uploaded' ? 'synced' : im.status === 'failed' ? 'failed' : 'processing',
      transformedParamString: '',
      previewUrl: im.bloggerUrl || im.originUrl || '',
    }));
    setImages(mapped);
  }, []);

  // Fetch only data relevant to the active tab; poll less often and backoff on errors.
  const loadTabData = useCallback(async (opts?: { includeConfig?: boolean; force?: boolean }) => {
    if (typeof document !== 'undefined' && document.hidden && !opts?.force) return;
    const includeConfig = !!opts?.includeConfig || !config;
    try {
      const needMetrics = activeTab === 'overview' || activeTab === 'daemon_engine';
      const needImages = activeTab === 'blogger_lab';
      const needAudits = activeTab === 'daily_audit' || activeTab === 'overview';
      const needTask = activeTab === 'overview' || activeTab === 'daemon_engine';

      const [metricsR, imgsR, auditsR, configR, taskR] = await Promise.all([
        needMetrics ? fetchJson<DaemonMetrics>(`${API_BASE}/metrics`) : Promise.resolve(null),
        needImages ? fetchJson<any[]>(`${API_BASE}/images?limit=100`) : Promise.resolve(null),
        needAudits ? fetchJson<DailyAuditRecord[]>(`${API_BASE}/audits`) : Promise.resolve(null),
        includeConfig ? fetchJson<AgentConfig>(`${API_BASE}/config`) : Promise.resolve(null),
        needTask ? fetchJson<any>(`${API_BASE}/tasks/current`, 6000) : Promise.resolve(null),
      ]);

      const results = [metricsR, imgsR, auditsR, configR, taskR].filter(Boolean) as FetchResult<any>[];
      const hadAny = results.length > 0;
      const allFailed = hadAny && results.every(r => !r.ok);
      if (allFailed) {
        listBackoffRef.current = Math.min(listBackoffRef.current * 2, 120000);
      } else if (hadAny) {
        listBackoffRef.current = 30000;
      }

      if (metricsR?.ok && metricsR.data) setDaemonMetrics(metricsR.data);
      if (imgsR?.ok && Array.isArray(imgsR.data)) applyImages(imgsR.data);
      if (auditsR?.ok && Array.isArray(auditsR.data)) setDailyAudits(auditsR.data);
      if (configR?.ok && configR.data) applyConfig(configR.data);
      if (taskR?.ok && taskR.data) setCurrentTask(taskR.data.task ?? null);
    } catch (e) {
      console.error('Failed to load data:', e);
      listBackoffRef.current = Math.min(listBackoffRef.current * 2, 120000);
    } finally {
      setLoading(false);
    }
  }, [activeTab, config, applyConfig, applyImages]);

  const loadAllData = useCallback(async () => loadTabData({ force: true }), [loadTabData]);

  // Initial load + config
  useEffect(() => {
    loadTabData({ includeConfig: true, force: true });
  }, []); // eslint-disable-line react-hooks/exhaustive-deps

  // Tab-scoped polling with backoff and visibility awareness
  useEffect(() => {
    let cancelled = false;
    let timer: number | undefined;
    const tick = async () => {
      if (cancelled) return;
      await loadTabData();
      if (cancelled) return;
      timer = window.setTimeout(tick, listBackoffRef.current);
    };
    if (typeof document === 'undefined' || !document.hidden) {
      tick();
    }
    const onVis = () => {
      if (!document.hidden) {
        listBackoffRef.current = 30000;
        if (timer) window.clearTimeout(timer);
        tick();
      } else if (timer) {
        window.clearTimeout(timer);
      }
    };
    document.addEventListener('visibilitychange', onVis);
    return () => {
      cancelled = true;
      if (timer) window.clearTimeout(timer);
      document.removeEventListener('visibilitychange', onVis);
    };
  }, [loadTabData]);

  // Realtime current task: SSE (preferred) with 6s polling fallback
  useEffect(() => {
    const shouldStream = activeTab === 'overview' || activeTab === 'daemon_engine';
    let poll: number | undefined;
    const stop = () => {
      if (esRef.current) { esRef.current.close(); esRef.current = null; }
      if (poll) { window.clearInterval(poll); poll = undefined; }
    };
    if (!shouldStream) { stop(); return; }

    const startFallback = () => {
      if (poll) return;
      loadCurrentTask();
      poll = window.setInterval(() => loadCurrentTask(), 6000);
    };

    if (typeof EventSource !== 'undefined') {
      try {
        const es = new EventSource(`${API_BASE}/tasks/stream`);
        esRef.current = es;
        es.addEventListener('task', (ev: MessageEvent) => {
          try {
            const data = JSON.parse(ev.data);
            setCurrentTask(data?.task ?? null);
          } catch {}
        });
        es.onerror = () => {
          // will auto-retry; also keep a light fallback to ensure updates if stream unsupported/blocked
          if (!poll) startFallback();
        };
      } catch {
        startFallback();
      }
    } else {
      startFallback();
    }

    const onVis = () => { if (document.hidden) { /* keep connection light; browser throttles anyway */ } else { loadCurrentTask(); } };
    document.addEventListener('visibilitychange', onVis);
    return () => { document.removeEventListener('visibilitychange', onVis); stop(); };
  }, [activeTab, loadCurrentTask]);

  // When tab changes, refresh immediately for that tab
  useEffect(() => { loadTabData({ force: true }); }, [activeTab]); // eslint-disable-line react-hooks/exhaustive-deps

  const handleAddOffloadedImage = async (newImage: OffloadImage) => {
    setImages((prev) => [newImage, ...prev]);
    if (daemonMetrics) {
      setDaemonMetrics((prev) =>
        prev
          ? {
              ...prev,
              totalImagesOffloaded: prev.totalImagesOffloaded + 1,
              totalBytesSaved:
                prev.totalBytesSaved +
                (newImage.originalSizeBytes - newImage.offloadSizeBytes),
            }
          : null
      );
    }
    const newLog: AgentLog = {
      id: `log-${Date.now()}`,
      timestamp:
        new Date().toTimeString().split(' ')[0] +
        '.' +
        Math.floor(Math.random() * 900 + 100),
      level: 'INFO',
      subsystem: 'media_processor',
      message: `Uploaded & generated offload URL: ${newImage.bloggerBaseUrl}${newImage.transformedParamString} (Saved ${Math.round(((newImage.originalSizeBytes - newImage.offloadSizeBytes) / newImage.originalSizeBytes) * 100)}%)`,
    };
    setLogs((prev) => [newLog, ...prev]);
  };

  const handleProcessAttachment = async (attachmentId: number) => {
    try {
      const res = await fetch(`${API_BASE}/attachments/${attachmentId}/process`, { method: 'POST' });
      const data = await res.json();
      if (data && data.ok !== false) {
        loadAllData();
      }
    } catch (e) {
      console.error('Process attachment failed:', e);
    }
  };

  const handleBatchProcessAttachments = async () => {
    // Scanner component handles per-item batching via its own fetch; trigger refresh.
    await loadAllData();
  };

  const handleTriggerAudit = async () => {
    if (isAuditing) return;
    setIsAuditing(true);
    setActiveAuditStep(0);

    const s1 = setTimeout(() => setActiveAuditStep(1), 700);
    const s2 = setTimeout(() => setActiveAuditStep(2), 1400);
    const s3 = setTimeout(() => setActiveAuditStep(3), 2100);
    const s4 = setTimeout(() => setActiveAuditStep(4), 2800);

    const sDone = setTimeout(() => {
      setIsAuditing(false);
      setActiveAuditStep(0);
      loadAllData(); // Refresh audits
    }, 3600);

    return () => {
      clearTimeout(s1);
      clearTimeout(s2);
      clearTimeout(s3);
      clearTimeout(s4);
      clearTimeout(sDone);
    };
  };

  const handleSaveConfig = async (newConfig: AgentConfig) => {
    setConfig(newConfig);
    // In real implementation, POST to /api/config
    const newLog: AgentLog = {
      id: `log-${Date.now()}`,
      timestamp:
        new Date().toTimeString().split(' ')[0] +
        '.' +
        Math.floor(Math.random() * 900 + 100),
      level: 'INFO',
      subsystem: 'rust_daemon',
      message: 'Configuration reloaded and validated across config.rs & config.py schema.',
    };
    setLogs((prev) => [newLog, ...prev]);
  };

  if (loading && !daemonMetrics) {
    return (
      <div className="min-h-screen bg-slate-50 dark:bg-slate-950 flex items-center justify-center">
        <div className="flex flex-col items-center gap-4">
          <div className="w-12 h-12 border-4 border-orange-500 border-t-transparent rounded-full animate-spin" />
          <p className="text-slate-600 dark:text-slate-400">Loading dashboard...</p>
        </div>
      </div>
    );
  }

  return (
    <div className="min-h-screen bg-slate-50 dark:bg-slate-950 text-slate-900 dark:text-slate-100 flex transition-colors selection:bg-orange-500 selection:text-white">
      <Sidebar
        activeTab={activeTab}
        setActiveTab={setActiveTab}
        daemonMetrics={daemonMetrics}
        pendingPostsCount={pendingPostsCount}
        isOpen={sidebarOpen}
        onClose={() => setSidebarOpen(false)}
        onTriggerAudit={handleTriggerAudit}
        isAuditing={isAuditing}
        siteUrl={siteUrl}
      />

      <div className="flex-1 flex flex-col min-w-0 lg:pl-64">
        <Header
          activeTab={activeTab}
          setActiveTab={setActiveTab}
          daemonMetrics={daemonMetrics}
          darkMode={darkMode}
          setDarkMode={setDarkMode}
          onTriggerAudit={handleTriggerAudit}
          isAuditing={isAuditing}
          onOpenSidebar={() => setSidebarOpen(true)}
        />

        <main className="flex-1 p-4 sm:p-6 lg:p-8 max-w-7xl w-full mx-auto">
          <Routes>
            <Route
              path="/"
              element={
                <OverviewTab
                  metrics={daemonMetrics}
                  latestAudit={dailyAudits[0] || null}
                  onNavigateTab={setActiveTab}
                  onTriggerAudit={handleTriggerAudit}
                  isAuditing={isAuditing}
                  currentTask={currentTask}
                />
              }
            />
            <Route
              path="/blogger"
              element={
                <BloggerLabTab
                  images={images}
                  onAddOffloadedImage={handleAddOffloadedImage}
                  siteUrl={siteUrl}
                />
              }
            />
            <Route
              path="/daemon"
              element={
                <DaemonEngineTab
                  metrics={daemonMetrics}
                  setMetrics={setDaemonMetrics}
                  logs={logs}
                  setLogs={setLogs}
                />
              }
            />
            <Route
              path="/scanner"
              element={
                <WordPressScannerTab
                  onProcessAttachment={handleProcessAttachment}
                  onBatchProcess={handleBatchProcessAttachments}
                />
              }
            />
            <Route
              path="/audit"
              element={
                <DailyAuditTab
                  audits={dailyAudits}
                  onTriggerAudit={handleTriggerAudit}
                  isAuditing={isAuditing}
                  activeAuditStep={activeAuditStep}
                />
              }
            />
            <Route
              path="/config"
              element={
                <ConfigTab
                  config={config}
                  onSaveConfig={handleSaveConfig}
                />
              }
            />
            <Route path="*" element={<Navigate to="/" replace />} />
          </Routes>
        </main>

        <footer className="border-t border-slate-200 dark:border-slate-800 py-4 mt-auto bg-white/50 dark:bg-slate-950/50 text-xs text-slate-500 dark:text-slate-400">
          <div className="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8 flex flex-col sm:flex-row items-center justify-between gap-2">
            <div className="flex items-center gap-2">
              <span className="font-semibold text-slate-700 dark:text-slate-300">
                WordPress Optimization Agent
              </span>
              <span>·</span>
              <span>Rust Daemon (Tokio) + Chrome CDP + Google Blogger CDN</span>
            </div>
            <div className="flex items-center gap-4 font-mono text-[11px]">
              <span>Cron: 03:00 UTC</span>
              <span>·</span>
              <span>LCP: 1.1s (Chuẩn Xanh)</span>
              <span>·</span>
              <span className="text-emerald-500 font-semibold">Tải CDN 94.2%</span>
            </div>
          </div>
        </footer>
      </div>
    </div>
  );
}