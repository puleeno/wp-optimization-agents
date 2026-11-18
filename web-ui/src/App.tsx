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
import { CrawlSourcesTab } from './components/CrawlSourcesTab';
import {
  DaemonMetrics,
  OffloadImage,
  DailyAuditRecord,
  AgentConfig,
} from './types/dashboard';

const TAB_ROUTES: Record<string, string> = {
  overview: '/overview',
  blogger_lab: '/blogger',
  daemon_engine: '/daemon',
  crawl_sources: '/crawl',
  crawl_wp_media: '/crawl/wp-media',
  crawl_audit: '/crawl/audit',
  config: '/config',
};

const PATH_TO_TAB: Record<string, string> = {
  '/': 'overview',
  '/overview': 'overview',
  '/blogger': 'blogger_lab',
  '/daemon': 'daemon_engine',
  '/crawl': 'crawl_sources',
  '/crawl/wp-media': 'crawl_wp_media',
  '/crawl/audit': 'crawl_audit',
  '/config': 'config',
  // legacy redirects mapping
  '/scanner': 'crawl_wp_media',
  '/audit': 'crawl_audit',
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
  const [config, setConfig] = useState<AgentConfig | null>(null);
  const [currentTask, setCurrentTask] = useState<any>(null);

  const [isAuditing, setIsAuditing] = useState<boolean>(false);
  const [auditStage, setAuditStage] = useState<string>('');
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
      site: {
        ...(cd.site || {}),
        url: cd.site?.url || '',
      },
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
        pollIntervalSec: cd.daemon?.poll_interval_secs ?? cd.daemon?.poll_interval_sec ?? cd.daemon?.pollIntervalSec ?? 60,
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
      cdn: {
        host: cd.cdn?.host ?? 'cdn.nghesachxua.localhost',
        scheme: cd.cdn?.scheme ?? 'https',
      },
      imgbb: {
        ...(cd.imgbb || {}),
        enabled: cd.imgbb?.enabled !== false,
        apiKey: cd.imgbb?.api_key ?? cd.imgbb?.apiKey ?? '',
        api_key: cd.imgbb?.api_key ?? cd.imgbb?.apiKey ?? '',
        uploadFormat: cd.imgbb?.upload_format ?? cd.imgbb?.uploadFormat ?? 'url',
        upload_format: cd.imgbb?.upload_format ?? cd.imgbb?.uploadFormat ?? 'url',
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
      const needAudits = activeTab === 'crawl_audit' || activeTab === 'crawl_sources' || activeTab === 'overview';
      const needTask = activeTab === 'overview' || activeTab === 'daemon_engine' || activeTab === 'crawl_wp_media' || activeTab === 'crawl_audit';
      const needScanStatus = activeTab === 'crawl_sources' || activeTab === 'crawl_wp_media';

      const [metricsR, imgsR, auditsR, configR, taskR, scanR] = await Promise.all([
        needMetrics ? fetchJson<DaemonMetrics>(`${API_BASE}/metrics`) : Promise.resolve(null),
        needImages ? fetchJson<any[]>(`${API_BASE}/images?limit=100`) : Promise.resolve(null),
        needAudits ? fetchJson<DailyAuditRecord[]>(`${API_BASE}/audits`) : Promise.resolve(null),
        includeConfig ? fetchJson<AgentConfig>(`${API_BASE}/config`) : Promise.resolve(null),
        needTask ? fetchJson<any>(`${API_BASE}/tasks/current`, 6000) : Promise.resolve(null),
        needScanStatus ? fetchJson<any>(`${API_BASE}/scan/status`, 6000) : Promise.resolve(null),
      ]);

      const results = [metricsR, imgsR, auditsR, configR, taskR, scanR].filter(Boolean) as FetchResult<any>[];
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
      if (scanR?.ok && scanR.data) (window as any).__wpoaScanInfo = scanR.data;
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
    const shouldStream = activeTab === 'overview' || activeTab === 'daemon_engine' || activeTab === 'crawl_wp_media' || activeTab === 'crawl_audit';
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
    // log captured by daemon /api/logs
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
    setAuditStage('Đang khởi tạo audit...');

    try {
      const res = await fetch(`${API_BASE}/audits/run`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ urls: [] }),
      });
      if (!res.ok) {
        console.error('audit run failed', res.status, await res.text().catch(() => ''));
      }

      for (let i = 0; i < 180; i++) {
        await new Promise((r) => setTimeout(r, 1000));
        const t = await fetchJson<any>(`${API_BASE}/tasks/current`, 5000);
        const task = t.data?.task ?? null;
        if (!task || !task.active) break;
        setAuditStage(task.message || task.stage || 'Đang kiểm tra...');
        setActiveAuditStep(typeof task.progress_current === 'number' ? task.progress_current : 0);
      }
      await loadAllData();
      setAuditStage('');
    } catch (e) {
      console.error('audit trigger failed', e);
      setAuditStage('');
    } finally {
      setIsAuditing(false);
      setActiveAuditStep(0);
    }
  };

  const handleSaveConfig = async (newConfig: any) => {
    // ConfigTab already persisted via PUT /api/config; just sync local state.
    setConfig((prev: any) => ({ ...(prev || {}), ...(newConfig || {}) }));
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
          auditStage={auditStage}
          onOpenSidebar={() => setSidebarOpen(true)}
        />

        <main className="flex-1 p-4 sm:p-6 lg:p-8 max-w-7xl w-full mx-auto">
          <Routes>
            <Route path="/" element={<Navigate to="/overview" replace />} />
            <Route
              path="/overview"
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
                  config={config}
                  siteUrl={siteUrl}
                />
              }
            />
            <Route
              path="/crawl"
              element={
                <CrawlSourcesTab
                  onNavigate={(path) => navigate(path)}
                  scanInfo={(window as any).__wpoaScanInfo}
                  auditCount={dailyAudits.length}
                />
              }
            />
            <Route
              path="/crawl/wp-media"
              element={
                <WordPressScannerTab
                  onProcessAttachment={handleProcessAttachment}
                  onBatchProcess={handleBatchProcessAttachments}
                />
              }
            />
            <Route
              path="/crawl/audit"
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
            {/* Legacy route compatibility */}
            <Route path="/scanner" element={<Navigate to="/crawl/wp-media" replace />} />
            <Route path="/audit" element={<Navigate to="/crawl/audit" replace />} />
            <Route path="*" element={<Navigate to="/overview" replace />} />
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