export type DaemonStatus = 'running' | 'idle' | 'stopped' | 'error';
export type RuntimeTarget = 'linux-systemd' | 'macos-launchd' | 'docker';

export interface DaemonMetrics {
  status: DaemonStatus;
  target: RuntimeTarget;
  pid: number;
  uptimeSeconds: number;
  memoryUsageMb: number;
  cpuPercent: number;
  tokioWorkers: number;
  pyo3GilLocked: boolean;
  activeTasks: number;
  totalImagesOffloaded: number;
  totalBytesSaved: number;
  lastHeartbeat: string;
}

export interface BloggerTransformParams {
  sizeMode: 'original' | 'max_dim' | 'exact' | 'preset';
  preset: 'thumb' | 'medium' | 'large' | 'banner' | 'avatar';
  maxDimension: number;
  width: number;
  height: number;
  smartCrop: boolean;
  webpFormat: boolean;
  lossless: boolean;
  quality: number;
  blurPlaceholder: boolean;
}

export interface OffloadImage {
  id: string;
  filename: string;
  mimeType: string;
  originalSizeBytes: number;
  offloadSizeBytes: number;
  width: number;
  height: number;
  originWpUrl: string;
  bloggerBaseUrl: string;
  uploadedAt: string;
  status: 'synced' | 'processing' | 'failed';
  transformedParamString: string;
  previewUrl: string;
}

export interface DailyAuditRecord {
  id: string;
  timestamp: string;
  dateStr: string;
  targetSiteUrl: string;
  device: 'mobile' | 'desktop';
  lighthouseScore: number;
  lcp: number; // in seconds
  lcpBefore: number;
  cls: number;
  fcp: number;
  ttfbBloggerMs: number;
  ttfbOriginMs: number;
  bandwidthSavedGb: number;
  imagesAudited: number;
  status: 'passed' | 'warning' | 'failed';
  notes: string;
  locations: {
    region: string;
    city: string;
    ttfbMs: number;
  }[];
}

export interface WordPressPost {
  id: number;
  title: string;
  slug: string;
  author: string;
  publishedDate: string;
  status: 'publish' | 'draft';
  totalImages: number;
  offloadedImages: number;
  images: {
    originUrl: string;
    bloggerUrl?: string;
    isOffloaded: boolean;
    sizeKb: number;
  }[];
}

export interface AgentLog {
  id: string;
  timestamp: string;
  level: 'INFO' | 'WARN' | 'ERROR' | 'DEBUG';
  subsystem: 'rust_daemon' | 'py_orchestrator' | 'media_processor' | 'blogger_client' | 'wp_client' | 'chrome_audit';
  message: string;
}

export interface AgentConfig {
  daemon: {
    port: number;
    logLevel: string;
    workerThreads: number;
    pollIntervalSec: number;
    autoRetryLimit: number;
  };
  wordpress: {
    endpointUrl: string;
    username: string;
    appPasswordMasked: string;
    mediaSyncChunkSize: number;
    replacePostContent: boolean;
    backupBeforeRewrite: boolean;
  };
  blogger: {
    blogId: string;
    albumName: string;
    authType: 'oauth2' | 'service_account';
    enableWebpAuto: boolean;
    defaultQuality: number;
    cdnDomainOverride: string;
  };
  dailyAudit: {
    enabled: boolean;
    cronSchedule: string;
    targetUrls: string[];
    slackWebhook: string;
    alertOnLcpAboveMs: number;
  };
}
