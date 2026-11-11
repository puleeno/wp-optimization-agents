import React, { useState } from 'react';
import {
  Settings,
  Save,
  Check,
  RotateCcw,
  Key,
  Globe,
  Sliders,
  Bell,
  Code2,
  FileJson,
  Download,
  ShieldCheck,
} from 'lucide-react';
import { AgentConfig } from '../types/dashboard';

interface ConfigTabProps {
  config: AgentConfig;
  onSaveConfig: (newConfig: AgentConfig) => void;
}

export const ConfigTab: React.FC<ConfigTabProps> = ({ config, onSaveConfig }) => {
  const [formData, setFormData] = useState<AgentConfig>(config);
  const [isSaved, setIsSaved] = useState(false);
  const [activeSubTab, setActiveSubTab] = useState<'form' | 'json'>('form');
  const [jsonText, setJsonText] = useState(JSON.stringify(config, null, 2));
  const [jsonError, setJsonError] = useState<string | null>(null);

  const handleSave = () => {
    onSaveConfig(formData);
    setJsonText(JSON.stringify(formData, null, 2));
    setIsSaved(true);
    setTimeout(() => setIsSaved(false), 2500);
  };

  const handleApplyJson = () => {
    try {
      const parsed = JSON.parse(jsonText);
      setFormData(parsed);
      onSaveConfig(parsed);
      setJsonError(null);
      setIsSaved(true);
      setTimeout(() => setIsSaved(false), 2500);
    } catch (err) {
      setJsonError('Cú pháp JSON không hợp lệ. Vui lòng kiểm tra lại cấu trúc.');
    }
  };

  const handleExportJson = () => {
    const blob = new Blob([JSON.stringify(formData, null, 2)], {
      type: 'application/json',
    });
    const url = URL.createObjectURL(blob);
    const a = document.createElement('a');
    a.href = url;
    a.download = 'wp-optimization-agent-config.json';
    a.click();
    URL.revokeObjectURL(url);
  };

  return (
    <div className="space-y-6">
      {/* Top Banner */}
      <div className="p-5 rounded-xl bg-white dark:bg-slate-900 border border-slate-200 dark:border-slate-800 flex flex-col md:flex-row md:items-center justify-between gap-4">
        <div>
          <div className="flex items-center gap-2 text-xs text-orange-600 dark:text-orange-400 font-mono font-medium">
            <span>CONFIG.RS & CONFIG.PY SYNCHRONIZATION</span>
            <span>·</span>
            <span>PYDANTIC V2</span>
          </div>
          <h2 className="text-lg font-bold text-slate-900 dark:text-white mt-0.5">
            Cấu hình tham số WordPress, Blogger API & Lịch kiểm tra tự động
          </h2>
          <p className="text-xs text-slate-500 dark:text-slate-400 mt-1 max-w-2xl">
            Cấu hình được kiểm duyệt qua Rust Serde và Python Pydantic validator trước khi ghi nhận vào daemon.
          </p>
        </div>

        <div className="flex items-center gap-2">
          {/* Sub tab toggle */}
          <div className="flex items-center gap-1 p-1 bg-slate-100 dark:bg-slate-800 rounded-lg text-xs mr-2">
            <button
              onClick={() => setActiveSubTab('form')}
              className={`px-3 py-1.5 rounded-md font-medium transition-colors ${
                activeSubTab === 'form'
                  ? 'bg-white dark:bg-slate-900 text-slate-900 dark:text-white shadow-sm'
                  : 'text-slate-600 dark:text-slate-400'
              }`}
            >
              Giao diện biểu mẫu
            </button>
            <button
              onClick={() => {
                setJsonText(JSON.stringify(formData, null, 2));
                setActiveSubTab('json');
              }}
              className={`px-3 py-1.5 rounded-md font-medium transition-colors ${
                activeSubTab === 'json'
                  ? 'bg-white dark:bg-slate-900 text-slate-900 dark:text-white shadow-sm'
                  : 'text-slate-600 dark:text-slate-400'
              }`}
            >
              JSON Schema Trực tiếp
            </button>
          </div>

          <button
            onClick={handleExportJson}
            className="p-2 text-slate-600 dark:text-slate-400 hover:text-slate-900 dark:hover:text-white bg-slate-100 dark:bg-slate-800 rounded-lg"
            title="Tải tệp config.json"
          >
            <Download className="w-4 h-4" />
          </button>

          <button
            onClick={activeSubTab === 'form' ? handleSave : handleApplyJson}
            className="px-4 py-2 text-xs font-semibold text-white bg-orange-600 hover:bg-orange-500 rounded-lg shadow-sm transition-all flex items-center gap-1.5 whitespace-nowrap"
          >
            {isSaved ? (
              <>
                <Check className="w-4 h-4 text-white" />
                <span>Đã lưu thành công!</span>
              </>
            ) : (
              <>
                <Save className="w-4 h-4" />
                <span>Lưu cấu hình</span>
              </>
            )}
          </button>
        </div>
      </div>

      {activeSubTab === 'form' ? (
        <div className="grid grid-cols-1 lg:grid-cols-2 gap-6">
          {/* Section 1: WordPress API Settings */}
          <div className="p-5 rounded-xl bg-white dark:bg-slate-900 border border-slate-200 dark:border-slate-800 space-y-4">
            <div className="flex items-center gap-2 border-b border-slate-100 dark:border-slate-800 pb-3">
              <Globe className="w-4 h-4 text-orange-500" />
              <h3 className="text-sm font-bold text-slate-900 dark:text-white">
                WordPress REST API Endpoint
              </h3>
            </div>

            <div className="space-y-3 text-xs">
              <div>
                <label className="text-slate-700 dark:text-slate-300 font-semibold block mb-1">
                  Địa chỉ trang WordPress (Endpoint URL)
                </label>
                <input
                  type="text"
                  value={formData.wordpress.endpointUrl}
                  onChange={(e) =>
                    setFormData({
                      ...formData,
                      wordpress: { ...formData.wordpress, endpointUrl: e.target.value },
                    })
                  }
                  className="w-full px-3 py-2 rounded-lg border border-slate-200 dark:border-slate-700 bg-slate-50 dark:bg-slate-800 text-slate-900 dark:text-white font-mono focus:outline-none focus:ring-1 focus:ring-orange-500"
                />
              </div>

              <div className="grid grid-cols-2 gap-3">
                <div>
                  <label className="text-slate-700 dark:text-slate-300 font-semibold block mb-1">
                    Tên người dùng REST API
                  </label>
                  <input
                    type="text"
                    value={formData.wordpress.username}
                    onChange={(e) =>
                      setFormData({
                        ...formData,
                        wordpress: { ...formData.wordpress, username: e.target.value },
                      })
                    }
                    className="w-full px-3 py-2 rounded-lg border border-slate-200 dark:border-slate-700 bg-slate-50 dark:bg-slate-800 text-slate-900 dark:text-white font-mono focus:outline-none focus:ring-1 focus:ring-orange-500"
                  />
                </div>

                <div>
                  <label className="text-slate-700 dark:text-slate-300 font-semibold block mb-1">
                    Application Password
                  </label>
                  <input
                    type="password"
                    value={formData.wordpress.appPasswordMasked}
                    onChange={(e) =>
                      setFormData({
                        ...formData,
                        wordpress: { ...formData.wordpress, appPasswordMasked: e.target.value },
                      })
                    }
                    className="w-full px-3 py-2 rounded-lg border border-slate-200 dark:border-slate-700 bg-slate-50 dark:bg-slate-800 text-slate-900 dark:text-white font-mono focus:outline-none focus:ring-1 focus:ring-orange-500"
                  />
                </div>
              </div>

              <div className="pt-2 space-y-2 border-t border-slate-100 dark:border-slate-800">
                <label className="flex items-center justify-between cursor-pointer">
                  <span className="text-slate-700 dark:text-slate-300">
                    Tự động thay thế đường dẫn ảnh trong HTML bài viết
                  </span>
                  <input
                    type="checkbox"
                    checked={formData.wordpress.replacePostContent}
                    onChange={(e) =>
                      setFormData({
                        ...formData,
                        wordpress: {
                          ...formData.wordpress,
                          replacePostContent: e.target.checked,
                        },
                      })
                    }
                    className="w-4 h-4 accent-orange-600 rounded"
                  />
                </label>

                <label className="flex items-center justify-between cursor-pointer">
                  <span className="text-slate-700 dark:text-slate-300">
                    Lưu bản sao lưu phục hồi trước khi sửa nội dung post
                  </span>
                  <input
                    type="checkbox"
                    checked={formData.wordpress.backupBeforeRewrite}
                    onChange={(e) =>
                      setFormData({
                        ...formData,
                        wordpress: {
                          ...formData.wordpress,
                          backupBeforeRewrite: e.target.checked,
                        },
                      })
                    }
                    className="w-4 h-4 accent-orange-600 rounded"
                  />
                </label>
              </div>
            </div>
          </div>

          {/* Section 2: Blogger API & Google CDN */}
          <div className="p-5 rounded-xl bg-white dark:bg-slate-900 border border-slate-200 dark:border-slate-800 space-y-4">
            <div className="flex items-center gap-2 border-b border-slate-100 dark:border-slate-800 pb-3">
              <Sliders className="w-4 h-4 text-orange-500" />
              <h3 className="text-sm font-bold text-slate-900 dark:text-white">
                Blogger API & CDN Google Usercontent
              </h3>
            </div>

            <div className="space-y-3 text-xs">
              <div className="grid grid-cols-2 gap-3">
                <div>
                  <label className="text-slate-700 dark:text-slate-300 font-semibold block mb-1">
                    Blogger Blog ID
                  </label>
                  <input
                    type="text"
                    value={formData.blogger.blogId}
                    onChange={(e) =>
                      setFormData({
                        ...formData,
                        blogger: { ...formData.blogger, blogId: e.target.value },
                      })
                    }
                    className="w-full px-3 py-2 rounded-lg border border-slate-200 dark:border-slate-700 bg-slate-50 dark:bg-slate-800 text-slate-900 dark:text-white font-mono focus:outline-none focus:ring-1 focus:ring-orange-500"
                  />
                </div>

                <div>
                  <label className="text-slate-700 dark:text-slate-300 font-semibold block mb-1">
                    Tên Album lưu trữ
                  </label>
                  <input
                    type="text"
                    value={formData.blogger.albumName}
                    onChange={(e) =>
                      setFormData({
                        ...formData,
                        blogger: { ...formData.blogger, albumName: e.target.value },
                      })
                    }
                    className="w-full px-3 py-2 rounded-lg border border-slate-200 dark:border-slate-700 bg-slate-50 dark:bg-slate-800 text-slate-900 dark:text-white font-mono focus:outline-none focus:ring-1 focus:ring-orange-500"
                  />
                </div>
              </div>

              <div>
                <label className="text-slate-700 dark:text-slate-300 font-semibold block mb-1">
                  Domain CDN Google
                </label>
                <input
                  type="text"
                  value={formData.blogger.cdnDomainOverride}
                  onChange={(e) =>
                    setFormData({
                      ...formData,
                      blogger: { ...formData.blogger, cdnDomainOverride: e.target.value },
                    })
                  }
                  className="w-full px-3 py-2 rounded-lg border border-slate-200 dark:border-slate-700 bg-slate-50 dark:bg-slate-800 text-slate-900 dark:text-white font-mono focus:outline-none focus:ring-1 focus:ring-orange-500"
                />
              </div>

              <div className="pt-2 space-y-2 border-t border-slate-100 dark:border-slate-800">
                <label className="flex items-center justify-between cursor-pointer">
                  <span className="text-slate-700 dark:text-slate-300">
                    Tự động gán cờ `-rw` (WebP Next-Gen) cho mọi ảnh offload
                  </span>
                  <input
                    type="checkbox"
                    checked={formData.blogger.enableWebpAuto}
                    onChange={(e) =>
                      setFormData({
                        ...formData,
                        blogger: {
                          ...formData.blogger,
                          enableWebpAuto: e.target.checked,
                        },
                      })
                    }
                    className="w-4 h-4 accent-orange-600 rounded"
                  />
                </label>
              </div>
            </div>
          </div>

          {/* Section 3: Daily Automated Audit Scheduler */}
          <div className="p-5 rounded-xl bg-white dark:bg-slate-900 border border-slate-200 dark:border-slate-800 space-y-4">
            <div className="flex items-center gap-2 border-b border-slate-100 dark:border-slate-800 pb-3">
              <Bell className="w-4 h-4 text-orange-500" />
              <h3 className="text-sm font-bold text-slate-900 dark:text-white">
                Lịch trình kiểm tra tự động hàng ngày
              </h3>
            </div>

            <div className="space-y-3 text-xs">
              <label className="flex items-center justify-between cursor-pointer">
                <div>
                  <span className="font-semibold text-slate-800 dark:text-slate-200 block">
                    Kích hoạt kiểm tra tự động hàng ngày
                  </span>
                  <span className="text-[11px] text-slate-400">
                    Chạy Headless Chrome lúc nửa đêm để đo lường LCP & TTFB
                  </span>
                </div>
                <input
                  type="checkbox"
                  checked={formData.dailyAudit.enabled}
                  onChange={(e) =>
                    setFormData({
                      ...formData,
                      dailyAudit: { ...formData.dailyAudit, enabled: e.target.checked },
                    })
                  }
                  className="w-4 h-4 accent-orange-600 rounded"
                />
              </label>

              <div>
                <label className="text-slate-700 dark:text-slate-300 font-semibold block mb-1">
                  Lịch biểu Cron (Cron Expression)
                </label>
                <input
                  type="text"
                  value={formData.dailyAudit.cronSchedule}
                  onChange={(e) =>
                    setFormData({
                      ...formData,
                      dailyAudit: { ...formData.dailyAudit, cronSchedule: e.target.value },
                    })
                  }
                  className="w-full px-3 py-2 rounded-lg border border-slate-200 dark:border-slate-700 bg-slate-50 dark:bg-slate-800 text-slate-900 dark:text-white font-mono focus:outline-none focus:ring-1 focus:ring-orange-500"
                />
                <span className="text-[10px] text-slate-400 font-mono mt-0.5 block">
                  `0 3 * * *` = 03:00 AM UTC mỗi ngày
                </span>
              </div>

              <div>
                <label className="text-slate-700 dark:text-slate-300 font-semibold block mb-1">
                  Ngưỡng cảnh báo LCP (Alert threshold)
                </label>
                <div className="flex items-center gap-2">
                  <input
                    type="number"
                    value={formData.dailyAudit.alertOnLcpAboveMs}
                    onChange={(e) =>
                      setFormData({
                        ...formData,
                        dailyAudit: {
                          ...formData.dailyAudit,
                          alertOnLcpAboveMs: Number(e.target.value),
                        },
                      })
                    }
                    className="w-32 px-3 py-2 rounded-lg border border-slate-200 dark:border-slate-700 bg-slate-50 dark:bg-slate-800 text-slate-900 dark:text-white font-mono"
                  />
                  <span className="text-slate-500 font-mono">ms (Mặc định 2500ms - Chuẩn Core Web Vitals)</span>
                </div>
              </div>
            </div>
          </div>

          {/* Section 4: Rust Tokio Daemon Runtime */}
          <div className="p-5 rounded-xl bg-white dark:bg-slate-900 border border-slate-200 dark:border-slate-800 space-y-4">
            <div className="flex items-center gap-2 border-b border-slate-100 dark:border-slate-800 pb-3">
              <Key className="w-4 h-4 text-orange-500" />
              <h3 className="text-sm font-bold text-slate-900 dark:text-white">
                Rust Tokio Runtime & Hàng đợi song song
              </h3>
            </div>

            <div className="space-y-3 text-xs">
              <div className="grid grid-cols-2 gap-3">
                <div>
                  <label className="text-slate-700 dark:text-slate-300 font-semibold block mb-1">
                    Cổng daemon cục bộ (Port)
                  </label>
                  <input
                    type="number"
                    value={formData.daemon.port}
                    onChange={(e) =>
                      setFormData({
                        ...formData,
                        daemon: { ...formData.daemon, port: Number(e.target.value) },
                      })
                    }
                    className="w-full px-3 py-2 rounded-lg border border-slate-200 dark:border-slate-700 bg-slate-50 dark:bg-slate-800 text-slate-900 dark:text-white font-mono"
                  />
                </div>

                <div>
                  <label className="text-slate-700 dark:text-slate-300 font-semibold block mb-1">
                    Số luồng Tokio Workers
                  </label>
                  <input
                    type="number"
                    value={formData.daemon.workerThreads}
                    onChange={(e) =>
                      setFormData({
                        ...formData,
                        daemon: { ...formData.daemon, workerThreads: Number(e.target.value) },
                      })
                    }
                    className="w-full px-3 py-2 rounded-lg border border-slate-200 dark:border-slate-700 bg-slate-50 dark:bg-slate-800 text-slate-900 dark:text-white font-mono"
                  />
                </div>
              </div>

              <div>
                <label className="text-slate-700 dark:text-slate-300 font-semibold block mb-1">
                  Mức độ ghi log (Rust RUST_LOG level)
                </label>
                <select
                  value={formData.daemon.logLevel}
                  onChange={(e) =>
                    setFormData({
                      ...formData,
                      daemon: { ...formData.daemon, logLevel: e.target.value },
                    })
                  }
                  className="w-full px-3 py-2 rounded-lg border border-slate-200 dark:border-slate-700 bg-slate-50 dark:bg-slate-800 text-slate-900 dark:text-white font-mono"
                >
                  <option value="trace">trace (Chi tiết tối đa)</option>
                  <option value="debug">debug (Phát triển & gỡ lỗi)</option>
                  <option value="info">info (Mặc định vận hành)</option>
                  <option value="warn">warn (Chỉ ghi nhận cảnh báo)</option>
                  <option value="error">error (Chỉ ghi nhận sự cố)</option>
                </select>
              </div>
            </div>
          </div>
        </div>
      ) : (
        /* Direct JSON Schema Editor */
        <div className="p-5 rounded-xl bg-slate-950 border border-slate-800 text-slate-200 font-mono text-xs space-y-3">
          <div className="flex items-center justify-between border-b border-slate-800 pb-2">
            <span className="text-slate-400 font-bold">
              config.json (Tuân thủ Serde & Pydantic Schema)
            </span>
            {jsonError && <span className="text-rose-400 text-[11px]">{jsonError}</span>}
          </div>

          <textarea
            value={jsonText}
            onChange={(e) => setJsonText(e.target.value)}
            rows={18}
            className="w-full bg-transparent text-emerald-400 font-mono text-xs focus:outline-none resize-none leading-relaxed"
          />
        </div>
      )}
    </div>
  );
};
