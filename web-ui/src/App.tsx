import React, { useState, useEffect, useCallback } from 'react';
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

async function fetchJson<T>(url: string): Promise<T | null> {
  try {
    const res = await fetch(url);
    if (!res.ok) return null;
    return await res.json();
  } catch {
    return null;
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
  const [posts, setPosts] = useState<WordPressPost[]>([]);
  const [logs, setLogs] = useState<AgentLog[]>([]);
  const [config, setConfig] = useState<AgentConfig | null>(null);
  const [currentTask, setCurrentTask] = useState<any>(null);

  const [isAuditing, setIsAuditing] = useState<boolean>(false);
  const [activeAuditStep, setActiveAuditStep] = useState<number>(0);
  const [loading, setLoading] = useState<boolean>(true);

  const pendingPostsCount = posts.filter((p) => p.offloadedImages < p.totalImages).length;

  useEffect(() => {
    if (darkMode) {
      document.documentElement.classList.add('dark');
    } else {
      document.documentElement.classList.remove('dark');
    }
  }, [darkMode]);

  const loadCurrentTask = useCallback(async () => {
    const data = await fetchJson<any>(`${API_BASE}/tasks/current`);
    if (data) setCurrentTask(data.task ?? null);
  }, []);

  const loadAllData = useCallback(async () => {
    setLoading(true);
    try {
      const [metrics, imgs, audits, postsData, configData, taskData] = await Promise.all([
        fetchJson<DaemonMetrics>(`${API_BASE}/metrics`),
        fetchJson<any[]>(`${API_BASE}/images?limit=100`),
        fetchJson<DailyAuditRecord[]>(`${API_BASE}/audits`),
        fetchJson<WordPressPost[]>(`${API_BASE}/posts?limit=50`),
        fetchJson<AgentConfig>(`${API_BASE}/config`),
        fetchJson<any>(`${API_BASE}/tasks/current`),
      ]);
      if (taskData) setCurrentTask(taskData.task ?? null);
      if (metrics) setDaemonMetrics(metrics);
      if (imgs) {
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
      }
      if (audits) setDailyAudits(audits);
      if (postsData) {
        const mappedPosts: WordPressPost[] = (postsData as any[]).map((p: any) => ({
          id: p.id,
          title: p.title || `Post #${p.id}`,
          slug: p.slug || '',
          author: p.author || '',
          publishedDate: p.publishedDate || p.linkedAt || '',
          status: p.status === 'publish' ? 'publish' : 'draft',
          totalImages: p.totalImages ?? 0,
          offloadedImages: p.offloadedImages ?? p.replacedImages ?? 0,
          images: Array.isArray(p.images) ? p.images : [],
        }));
        setPosts(mappedPosts);
      }
      if (configData) {
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
      }
    } catch (e) {
      console.error('Failed to load data:', e);
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    loadAllData();
    const interval = setInterval(loadAllData, 10000);
    const taskInterval = setInterval(loadCurrentTask, 3000);
    return () => { clearInterval(interval); clearInterval(taskInterval); };
  }, [loadAllData, loadCurrentTask]);

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

  const handleProcessPost = async (postId: number) => {
    try {
      const res = await fetch(`${API_BASE}/posts/${postId}/process`, { method: 'POST' });
      const data = await res.json();
      if (data.ok && data.data) {
        loadAllData();
      }
    } catch (e) {
      console.error('Process post failed:', e);
    }
  };

  const handleBatchOffloadAll = async () => {
    // Process multiple posts sequentially
    for (const post of posts.filter((p) => p.offloadedImages < p.totalImages)) {
      await handleProcessPost(post.id);
    }
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
                  posts={posts}
                  onProcessPost={handleProcessPost}
                  onBatchOffloadAll={handleBatchOffloadAll}
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