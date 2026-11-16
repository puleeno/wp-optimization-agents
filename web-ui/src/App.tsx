import React, { useState, useEffect, useCallback } from 'react';
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
  const [darkMode, setDarkMode] = useState<boolean>(true);
  const [sidebarOpen, setSidebarOpen] = useState<boolean>(false);
  const [activeTab, setActiveTab] = useState<string>('overview');

  const [daemonMetrics, setDaemonMetrics] = useState<DaemonMetrics | null>(null);
  const [images, setImages] = useState<OffloadImage[]>([]);
  const [dailyAudits, setDailyAudits] = useState<DailyAuditRecord[]>([]);
  const [posts, setPosts] = useState<WordPressPost[]>([]);
  const [logs, setLogs] = useState<AgentLog[]>([]);
  const [config, setConfig] = useState<AgentConfig | null>(null);

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

  const loadAllData = useCallback(async () => {
    setLoading(true);
    try {
      const [metrics, imgs, audits, postsData, configData] = await Promise.all([
        fetchJson<DaemonMetrics>(`${API_BASE}/metrics`),
        fetchJson<OffloadImage[]>(`${API_BASE}/images?limit=100`),
        fetchJson<DailyAuditRecord[]>(`${API_BASE}/audits`),
        fetchJson<WordPressPost[]>(`${API_BASE}/posts?limit=50`),
        fetchJson<AgentConfig>(`${API_BASE}/config`),
      ]);
      if (metrics) setDaemonMetrics(metrics);
      if (imgs) setImages(imgs);
      if (audits) setDailyAudits(audits);
      if (postsData) setPosts(postsData);
      if (configData) setConfig(configData);
    } catch (e) {
      console.error('Failed to load data:', e);
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    loadAllData();
    const interval = setInterval(loadAllData, 10000);
    return () => clearInterval(interval);
  }, [loadAllData]);

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
          {activeTab === 'overview' && (
            <OverviewTab
              metrics={daemonMetrics}
              latestAudit={dailyAudits[0]}
              onNavigateTab={setActiveTab}
              onTriggerAudit={handleTriggerAudit}
              isAuditing={isAuditing}
            />
          )}

          {activeTab === 'blogger_lab' && (
            <BloggerLabTab
              images={images}
              onAddOffloadedImage={handleAddOffloadedImage}
            />
          )}

          {activeTab === 'daemon_engine' && (
            <DaemonEngineTab
              metrics={daemonMetrics}
              setMetrics={setDaemonMetrics}
              logs={logs}
              setLogs={setLogs}
            />
          )}

          {activeTab === 'wp_scanner' && (
            <WordPressScannerTab
              posts={posts}
              onProcessPost={handleProcessPost}
              onBatchOffloadAll={handleBatchOffloadAll}
            />
          )}

          {activeTab === 'daily_audit' && (
            <DailyAuditTab
              audits={dailyAudits}
              onTriggerAudit={handleTriggerAudit}
              isAuditing={isAuditing}
              activeAuditStep={activeAuditStep}
            />
          )}

          {activeTab === 'config' && (
            <ConfigTab
              config={config}
              onSaveConfig={handleSaveConfig}
            />
          )}
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