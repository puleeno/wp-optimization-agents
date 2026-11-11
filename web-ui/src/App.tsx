import React, { useState, useEffect } from 'react';
import { Sidebar } from './components/Sidebar';
import { Header } from './components/Header';
import { OverviewTab } from './components/OverviewTab';
import { BloggerLabTab } from './components/BloggerLabTab';
import { DaemonEngineTab } from './components/DaemonEngineTab';
import { WordPressScannerTab } from './components/WordPressScannerTab';
import { DailyAuditTab } from './components/DailyAuditTab';
import { ConfigTab } from './components/ConfigTab';
import {
  initialDaemonMetrics,
  sampleBloggerImages,
  initialDailyAudits,
  sampleWordPressPosts,
  initialLogs,
  initialConfig,
} from './data/mockData';
import {
  DaemonMetrics,
  OffloadImage,
  DailyAuditRecord,
  WordPressPost,
  AgentLog,
  AgentConfig,
} from './types/dashboard';

export default function App() {
  // Theme state
  const [darkMode, setDarkMode] = useState<boolean>(true);

  // Sidebar mobile drawer state
  const [sidebarOpen, setSidebarOpen] = useState<boolean>(false);

  // Tab state
  const [activeTab, setActiveTab] = useState<string>('overview');

  // Application data states
  const [daemonMetrics, setDaemonMetrics] = useState<DaemonMetrics>(initialDaemonMetrics);
  const [images, setImages] = useState<OffloadImage[]>(sampleBloggerImages);
  const [dailyAudits, setDailyAudits] = useState<DailyAuditRecord[]>(initialDailyAudits);
  const [posts, setPosts] = useState<WordPressPost[]>(sampleWordPressPosts);
  const [logs, setLogs] = useState<AgentLog[]>(initialLogs);
  const [config, setConfig] = useState<AgentConfig>(initialConfig);

  // Daily Chrome Headless Audit state
  const [isAuditing, setIsAuditing] = useState<boolean>(false);
  const [activeAuditStep, setActiveAuditStep] = useState<number>(0);

  // Count pending posts
  const pendingPostsCount = posts.filter(
    (p) => p.offloadedImages < p.totalImages
  ).length;

  // Sync dark class with document element
  useEffect(() => {
    if (darkMode) {
      document.documentElement.classList.add('dark');
    } else {
      document.documentElement.classList.remove('dark');
    }
  }, [darkMode]);

  // Periodic heartbeat timer simulation for Rust daemon
  useEffect(() => {
    const timer = setInterval(() => {
      setDaemonMetrics((prev) => {
        if (prev.status !== 'running') return prev;
        return {
          ...prev,
          uptimeSeconds: prev.uptimeSeconds + 5,
          memoryUsageMb: +(42.5 + Math.random() * 0.8).toFixed(1),
          cpuPercent: +(1.2 + Math.random() * 0.6).toFixed(1),
          lastHeartbeat: new Date().toISOString(),
        };
      });
    }, 5000);
    return () => clearInterval(timer);
  }, []);

  // Handler: Add new offloaded image
  const handleAddOffloadedImage = (newImage: OffloadImage) => {
    setImages((prev) => [newImage, ...prev]);
    setDaemonMetrics((prev) => ({
      ...prev,
      totalImagesOffloaded: prev.totalImagesOffloaded + 1,
      totalBytesSaved:
        prev.totalBytesSaved +
        (newImage.originalSizeBytes - newImage.offloadSizeBytes),
    }));

    // Append log
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

  // Handler: Batch offload single WP post
  const handleBatchOffloadPost = (postId: number) => {
    setPosts((prev) =>
      prev.map((post) => {
        if (post.id !== postId) return post;
        const updatedImages = post.images.map((img) => ({
          ...img,
          isOffloaded: true,
          bloggerUrl:
            img.bloggerUrl ||
            `https://blogger.googleusercontent.com/img/a/AVvXsE_${Math.random().toString(36).substring(2, 8)}=w1200-h800-rw`,
        }));
        return {
          ...post,
          offloadedImages: post.totalImages,
          images: updatedImages,
        };
      })
    );

    setDaemonMetrics((prev) => ({
      ...prev,
      totalImagesOffloaded: prev.totalImagesOffloaded + 4,
      totalBytesSaved: prev.totalBytesSaved + 14200000,
    }));

    const newLog: AgentLog = {
      id: `log-${Date.now()}`,
      timestamp:
        new Date().toTimeString().split(' ')[0] +
        '.' +
        Math.floor(Math.random() * 900 + 100),
      level: 'INFO',
      subsystem: 'py_orchestrator',
      message: `Post #${postId} media offload complete. Rewrote HTML content with Blogger WebP CDN sources.`,
    };
    setLogs((prev) => [newLog, ...prev]);
  };

  // Handler: Batch offload ALL pending WP posts
  const handleBatchOffloadAll = () => {
    setPosts((prev) =>
      prev.map((post) => ({
        ...post,
        offloadedImages: post.totalImages,
        images: post.images.map((img) => ({
          ...img,
          isOffloaded: true,
          bloggerUrl:
            img.bloggerUrl ||
            `https://blogger.googleusercontent.com/img/a/AVvXsE_${Math.random().toString(36).substring(2, 8)}=w1200-h800-rw`,
        })),
      }))
    );

    setDaemonMetrics((prev) => ({
      ...prev,
      totalImagesOffloaded: prev.totalImagesOffloaded + 12,
      totalBytesSaved: prev.totalBytesSaved + 42800000,
    }));

    const newLog: AgentLog = {
      id: `log-${Date.now()}`,
      timestamp:
        new Date().toTimeString().split(' ')[0] +
        '.' +
        Math.floor(Math.random() * 900 + 100),
      level: 'INFO',
      subsystem: 'rust_daemon',
      message: `Batch job finished: 100% of WordPress media successfully mapped to Blogger CDN endpoints.`,
    };
    setLogs((prev) => [newLog, ...prev]);
  };

  // Handler: Trigger Headless Chrome Daily Audit
  const handleTriggerAudit = () => {
    if (isAuditing) return;
    setIsAuditing(true);
    setActiveAuditStep(0);

    // Step-by-step progress
    const s1 = setTimeout(() => setActiveAuditStep(1), 700);
    const s2 = setTimeout(() => setActiveAuditStep(2), 1400);
    const s3 = setTimeout(() => setActiveAuditStep(3), 2100);
    const s4 = setTimeout(() => setActiveAuditStep(4), 2800);

    const sDone = setTimeout(() => {
      setIsAuditing(false);
      setActiveAuditStep(0);

      const newAudit: DailyAuditRecord = {
        id: `audit-${Date.now()}`,
        timestamp:
          new Date().toISOString().replace('T', ' ').substring(0, 19) + ' UTC',
        dateStr: 'Vừa xong (Manual Run)',
        targetSiteUrl: 'https://vietnamtechreview.com/danh-gia-sony-a7-iv/',
        device: 'mobile',
        lighthouseScore: 99,
        lcp: 1.08,
        lcpBefore: 3.92,
        cls: 0.002,
        fcp: 0.69,
        ttfbBloggerMs: 34,
        ttfbOriginMs: 340,
        bandwidthSavedGb: 28.2,
        imagesAudited: 42,
        status: 'passed',
        notes:
          'Kiểm tra thủ công bằng Headless Chrome thành công. Mọi hình ảnh đều phục vụ WebP qua Google Edge.',
        locations: [
          { region: 'APAC', city: 'TP. Hồ Chí Minh', ttfbMs: 22 },
          { region: 'APAC', city: 'Hà Nội', ttfbMs: 25 },
          { region: 'APAC', city: 'Singapore', ttfbMs: 18 },
          { region: 'US', city: 'San Jose, CA', ttfbMs: 52 },
          { region: 'EU', city: 'Frankfurt', ttfbMs: 58 },
        ],
      };

      setDailyAudits((prev) => [newAudit, ...prev]);

      const log: AgentLog = {
        id: `log-${Date.now()}`,
        timestamp:
          new Date().toTimeString().split(' ')[0] +
          '.' +
          Math.floor(Math.random() * 900 + 100),
        level: 'INFO',
        subsystem: 'chrome_audit',
        message:
          'Headless Chrome automated audit finished. Score: 99/100, LCP: 1.08s (Passes Core Web Vitals).',
      };
      setLogs((prev) => [log, ...prev]);
    }, 3600);

    return () => {
      clearTimeout(s1);
      clearTimeout(s2);
      clearTimeout(s3);
      clearTimeout(s4);
      clearTimeout(sDone);
    };
  };

  return (
    <div className="min-h-screen bg-slate-50 dark:bg-slate-950 text-slate-900 dark:text-slate-100 flex transition-colors selection:bg-orange-500 selection:text-white">
      {/* Sidebar Navigation Menu */}
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

      {/* Main Content Area (offset by sidebar width on lg screens) */}
      <div className="flex-1 flex flex-col min-w-0 lg:pl-64">
        {/* Top Header */}
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

        {/* Viewport Content */}
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
              onBatchOffloadPost={handleBatchOffloadPost}
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
              onSaveConfig={(newConfig) => {
                setConfig(newConfig);
                const log: AgentLog = {
                  id: `log-${Date.now()}`,
                  timestamp:
                    new Date().toTimeString().split(' ')[0] +
                    '.' +
                    Math.floor(Math.random() * 900 + 100),
                  level: 'INFO',
                  subsystem: 'rust_daemon',
                  message:
                    'Configuration reloaded and validated across config.rs & config.py schema.',
                };
                setLogs((prev) => [log, ...prev]);
              }}
            />
          )}
        </main>

        {/* Footer */}
        <footer className="border-t border-slate-200 dark:border-slate-800 py-4 mt-auto bg-white/50 dark:bg-slate-950/50 text-xs text-slate-500 dark:text-slate-400">
          <div className="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8 flex flex-col sm:flex-row items-center justify-between gap-2">
            <div className="flex items-center gap-2">
              <span className="font-semibold text-slate-700 dark:text-slate-300">
                WordPress Optimization Agent
              </span>
              <span>·</span>
              <span>Rust Daemon (Tokio) + Python Orchestrator + Google Blogger CDN</span>
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
