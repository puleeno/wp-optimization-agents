import React, { useEffect, useState } from 'react';
import {
  Save,
  Check,
  RotateCcw,
  Download,
  Globe,
  Server,
  KeyRound,
  Database,
  Chrome,
  Image as ImageIcon,
  FileJson,
  AlertTriangle,
} from 'lucide-react';

const API_BASE = '/api';

interface ConfigTabProps {
  config: any;
  onSaveConfig?: (newConfig: any) => void;
}

type Msg = { type: 'ok' | 'err'; text: string } | null;

export const ConfigTab: React.FC<ConfigTabProps> = ({ config, onSaveConfig }) => {
  const [form, setForm] = useState<any>(config || {});
  const [tab, setTab] = useState<'form' | 'json'>('form');
  const [jsonText, setJsonText] = useState('');
  const [busy, setBusy] = useState(false);
  const [msg, setMsg] = useState<Msg>(null);
  const [showSecret, setShowSecret] = useState(false);

  useEffect(() => {
    if (config) {
      setForm(config);
      setJsonText(JSON.stringify(config, null, 2));
    }
  }, [config]);

  const set = (fn: (draft: any) => void) =>
    setForm((prev: any) => {
      const draft = JSON.parse(JSON.stringify(prev || {}));
      fn(draft);
      return draft;
    });

  const reload = async () => {
    setBusy(true);
    try {
      const res = await fetch(`${API_BASE}/config`);
      const data = await res.json();
      setForm(data);
      setJsonText(JSON.stringify(data, null, 2));
      setMsg({ type: 'ok', text: 'Đã tải lại cấu hình hiện hành từ daemon.' });
    } catch (e: any) {
      setMsg({ type: 'err', text: e?.message || 'Không tải được cấu hình' });
    } finally {
      setBusy(false);
    }
  };

  // Only send sections the daemon config schema owns; never send DB creds / legacy aliases.
  const payloadOf = (src: any) => ({
    site: {
      url: src?.site?.url ?? '',
      wp_path: src?.site?.wp_path ?? '',
      uploads_dir: src?.site?.uploads_dir ?? '',
    },
    agent: {
      host: src?.agent?.host ?? '0.0.0.0',
      port: Number(src?.agent?.port ?? 2095),
      poll_interval_secs: Number(src?.agent?.poll_interval_secs ?? 60),
      replace_in_db: src?.agent?.replace_in_db !== false,
      backup_before_replace: src?.agent?.backup_before_replace !== false,
      enable_old_url_redirect_301: src?.agent?.enable_old_url_redirect_301 !== false,
      daemon_api_token: src?.agent?.daemon_api_token ?? '',
      scan_enabled: src?.agent?.scan_enabled !== false,
      scan_interval_secs: Number(src?.agent?.scan_interval_secs ?? 900),
      scan_batch_size: Number(src?.agent?.scan_batch_size ?? 20),
      delete_local_after_offload: src?.agent?.delete_local_after_offload !== false,
      finish_workers: Number(src?.agent?.finish_workers ?? 4),
    },
    cdn: { host: src?.cdn?.host ?? '', scheme: src?.cdn?.scheme || 'https' },
    imgbb: {
      enabled: src?.imgbb?.enabled !== false,
      api_key: src?.imgbb?.api_key ?? '',
      upload_format: src?.imgbb?.upload_format || 'url',
    },
    chrome: {
      executable: src?.chrome?.executable || null,
      user_data_dir: src?.chrome?.user_data_dir || null,
      profile_directory: src?.chrome?.profile_directory || 'Default',
      headless: src?.chrome?.headless === true,
      remote_debugging_port: Number(src?.chrome?.remote_debugging_port ?? 9222),
      launch_timeout_secs: Number(src?.chrome?.launch_timeout_secs ?? 30),
    },
    blogger: {
      blog_id: src?.blogger?.blog_id ?? '',
      upload_via: src?.blogger?.upload_via || 'chrome_web_ui',
      one_wp_post_per_blogger_post: src?.blogger?.one_wp_post_per_blogger_post !== false,
    },
    logging: { level: src?.logging?.level || 'info' },
  });

  const save = async (payload: any) => {
    setBusy(true);
    setMsg(null);
    try {
      const res = await fetch(`${API_BASE}/config`, {
        method: 'PUT',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify(payload),
      });
      const data = await res.json().catch(() => ({}));
      if (!res.ok || data?.ok === false) throw new Error(data?.error || `HTTP ${res.status}`);
      setMsg({
        type: 'ok',
        text: `Đã ghi ${data.path || 'config.json'}. Khởi động lại daemon để áp dụng host/port/chrome/database.`,
      });
      onSaveConfig?.(payload);
      await reload();
    } catch (e: any) {
      setMsg({ type: 'err', text: e?.message || 'Lưu cấu hình thất bại' });
    } finally {
      setBusy(false);
    }
  };

  const saveJson = () => {
    try {
      const parsed = JSON.parse(jsonText);
      save(payloadOf(parsed));
    } catch {
      setMsg({ type: 'err', text: 'JSON không hợp lệ.' });
    }
  };

  const exportJson = () => {
    const blob = new Blob([JSON.stringify(form, null, 2)], { type: 'application/json' });
    const url = URL.createObjectURL(blob);
    const a = document.createElement('a');
    a.href = url;
    a.download = 'config.json';
    a.click();
    URL.revokeObjectURL(url);
  };

  const [preflight, setPreflight] = useState<any>(null);

  const runPreflight = async () => {
    try {
      const res = await fetch('/api/chrome/preflight');
      setPreflight(await res.json());
    } catch {
      setPreflight({ ok: false, advice: ['Không gọi được /api/chrome/preflight'] });
    }
  };

  const copyPath = async () => {
    const path = form?._meta?.path || '';
    if (!path) return;
    try {
      await navigator.clipboard.writeText(path);
      setMsg({ type: 'ok', text: `Đã copy đường dẫn: ${path}` });
    } catch {
      setMsg({ type: 'err', text: `Không copy được. Đường dẫn: ${path}` });
    }
  };

  const inputCls =
    'w-full px-3 py-2 rounded-lg border border-slate-200 dark:border-slate-700 bg-slate-50 dark:bg-slate-800 text-slate-900 dark:text-white font-mono text-xs focus:outline-none focus:ring-1 focus:ring-orange-500';
  const labelCls = 'text-slate-700 dark:text-slate-300 font-semibold block mb-1 text-xs';

  return (
    <div className="space-y-6">
      <div className="p-5 rounded-xl bg-white dark:bg-slate-900 border border-slate-200 dark:border-slate-800 flex flex-col md:flex-row md:items-center justify-between gap-4">
        <div>
          <div className="flex items-center gap-2 text-xs text-orange-600 dark:text-orange-400 font-mono font-medium">
            <span>AGENTS/CONFIG.JSON</span>
            <span>·</span>
            <span>RUST SERDE VALIDATED</span>
          </div>
          <h2 className="text-lg font-bold text-slate-900 dark:text-white mt-0.5">
            Cấu hình daemon (ghi trực tiếp vào file config)
          </h2>
          <p className="text-xs text-slate-500 dark:text-slate-400 mt-1 max-w-2xl">
            Các thay đổi được kiểm tra bằng schema Rust rồi ghi vào file <code className="font-mono">agents/config.json</code> (hoặc <code className="font-mono">DAEMON_CONFIG</code>). Host/port/chrome/database cần restart daemon để áp dụng.
          </p>
        </div>
        <div className="flex items-center gap-2">
          <div className="flex items-center gap-1 p-1 bg-slate-100 dark:bg-slate-800 rounded-lg text-xs">
            <button
              onClick={() => setTab('form')}
              className={`px-3 py-1.5 rounded-md font-medium ${tab === 'form' ? 'bg-white dark:bg-slate-900 shadow-sm' : 'text-slate-500'}`}
            >
              Biểu mẫu
            </button>
            <button
              onClick={() => {
                setJsonText(JSON.stringify(form, null, 2));
                setTab('json');
              }}
              className={`px-3 py-1.5 rounded-md font-medium ${tab === 'json' ? 'bg-white dark:bg-slate-900 shadow-sm' : 'text-slate-500'}`}
            >
              JSON
            </button>
          </div>
          <button
            onClick={exportJson}
            className="p-2 rounded-lg bg-slate-100 dark:bg-slate-800 text-slate-600 dark:text-slate-300 hover:bg-slate-200"
            title="Xuất config.json"
          >
            <Download className="w-4 h-4" />
          </button>
          <button
            onClick={reload}
            disabled={busy}
            className="p-2 rounded-lg bg-slate-100 dark:bg-slate-800 text-slate-600 dark:text-slate-300 hover:bg-slate-200 disabled:opacity-50"
            title="Tải lại từ daemon"
          >
            <RotateCcw className={`w-4 h-4 ${busy ? 'animate-spin' : ''}`} />
          </button>
          <button
            onClick={() => (tab === 'form' ? save(payloadOf(form)) : saveJson())}
            disabled={busy}
            className="px-4 py-2 text-xs font-semibold text-white bg-orange-600 hover:bg-orange-500 disabled:opacity-50 rounded-lg flex items-center gap-1.5 whitespace-nowrap"
          >
            <Save className="w-4 h-4" />
            <span>{busy ? 'Đang lưu...' : 'Lưu cấu hình'}</span>
          </button>
        </div>
      </div>

      <div className="p-3 rounded-lg bg-slate-50 dark:bg-slate-800/40 border border-slate-200 dark:border-slate-700 flex flex-col sm:flex-row sm:items-center justify-between gap-2 text-xs">
        <div className="min-w-0">
          <div className="text-slate-500 dark:text-slate-400">File cấu hình daemon đang dùng:</div>
          <div className="font-mono text-[11px] text-slate-900 dark:text-slate-100 break-all">
            {form?._meta?.path || '(không xác định — đặt biến môi trường DAEMON_CONFIG)'}
          </div>
        </div>
        <div className="flex items-center gap-2 shrink-0">
          <span className="text-[10px] text-slate-400">PID {form?._meta?.pid ?? '—'}</span>
          <button
            onClick={copyPath}
            className="px-2.5 py-1 rounded border border-slate-200 dark:border-slate-700 hover:bg-white dark:hover:bg-slate-900 text-slate-600 dark:text-slate-300"
          >
            Copy path
          </button>
        </div>
      </div>

      {msg && (
        <div
          className={`p-3 rounded-lg text-xs border ${
            msg.type === 'ok'
              ? 'bg-emerald-50 dark:bg-emerald-950/30 border-emerald-200 dark:border-emerald-900 text-emerald-700 dark:text-emerald-300'
              : 'bg-rose-50 dark:bg-rose-950/30 border-rose-200 dark:border-rose-900 text-rose-700 dark:text-rose-300'
          }`}
        >
          {msg.text}
        </div>
      )}

      {tab === 'json' ? (
        <div className="p-5 rounded-xl bg-slate-950 border border-slate-800 text-slate-200 font-mono text-xs space-y-3">
          <div className="flex items-center justify-between border-b border-slate-800 pb-2">
            <span className="text-slate-400 font-bold">config.json</span>
            <span className="text-[11px] text-slate-500">PUT /api/config</span>
          </div>
          <textarea
            value={jsonText}
            onChange={(e) => setJsonText(e.target.value)}
            rows={20}
            className="w-full bg-transparent text-emerald-400 font-mono text-xs focus:outline-none resize-none leading-relaxed"
          />
          <div className="flex justify-end">
            <button
              onClick={saveJson}
              disabled={busy}
              className="px-3 py-1.5 rounded bg-orange-600 hover:bg-orange-500 text-white font-semibold disabled:opacity-50"
            >
              Lưu JSON này
            </button>
          </div>
        </div>
      ) : (
        <div className="grid grid-cols-1 lg:grid-cols-2 gap-6">
          {/* Site & WordPress */}
          <div className="p-5 rounded-xl bg-white dark:bg-slate-900 border border-slate-200 dark:border-slate-800 space-y-4">
            <div className="flex items-center gap-2 border-b border-slate-100 dark:border-slate-800 pb-3">
              <Globe className="w-4 h-4 text-orange-500" />
              <h3 className="text-sm font-bold text-slate-900 dark:text-white">Site &amp; WordPress</h3>
            </div>
            <div className="space-y-3 text-xs">
              <div>
                <label className={labelCls}>Site URL (nguồn ảnh gốc, dùng tạo URL uploads)</label>
                <input
                  className={inputCls}
                  value={form.site?.url || ''}
                  onChange={(e) => set((d) => { d.site = { ...(d.site || {}), url: e.target.value }; })}
                  placeholder="https://nghesachxua.localhost"
                />
              </div>
              <div>
                <label className={labelCls}>Thư mục uploads trên máy</label>
                <input
                  className={inputCls}
                  value={form.site?.uploads_dir || ''}
                  onChange={(e) => set((d) => { d.site = { ...(d.site || {}), uploads_dir: e.target.value }; })}
                  placeholder="/Users/puleeno/Projects/nghesachxua.com/wp-content/uploads"
                />
                <span className="text-[10px] text-slate-400 mt-0.5 block">
                  Hỗ trợ <code className="font-mono">${'{'}VAR{'}'}</code> / <code className="font-mono">${'{'}VAR:-default{'}'}</code> và đường dẫn tương đối (quy chiếu thư mục chứa config). Dùng để tìm file vật lý khi upload imgbb dạng blob.
                </span>
              </div>
              <div className="p-2.5 rounded-lg bg-slate-50 dark:bg-slate-800/40 border border-slate-100 dark:border-slate-800">
                <div className="flex items-center gap-1.5 text-slate-500 mb-1">
                  <Database className="w-3.5 h-3.5" />
                  <span className="font-semibold">Database (chỉ đọc)</span>
                </div>
                <div className="font-mono text-[11px] text-slate-600 dark:text-slate-300 break-all">
                  {form.database
                    ? `${form.database.user}@${form.database.host}:${form.database.port}/${form.database.name} · prefix ${form.database.table_prefix}`
                    : '—'}
                </div>
              </div>
            </div>
          </div>

          {/* Daemon */}
          <div className="p-5 rounded-xl bg-white dark:bg-slate-900 border border-slate-200 dark:border-slate-800 space-y-4">
            <div className="flex items-center gap-2 border-b border-slate-100 dark:border-slate-800 pb-3">
              <Server className="w-4 h-4 text-orange-500" />
              <h3 className="text-sm font-bold text-slate-900 dark:text-white">Daemon runtime</h3>
            </div>
            <div className="space-y-3 text-xs">
              <div className="grid grid-cols-2 gap-3">
                <div>
                  <label className={labelCls}>Bind host</label>
                  <input className={inputCls} value={form.agent?.host ?? ''} onChange={(e) => set((d) => { d.agent = { ...(d.agent || {}), host: e.target.value }; })} />
                </div>
                <div>
                  <label className={labelCls}>Port (restart)</label>
                  <input type="number" className={inputCls} value={form.agent?.port ?? 2095} onChange={(e) => set((d) => { d.agent = { ...(d.agent || {}), port: Number(e.target.value) }; })} />
                </div>
              </div>
              <div className="pt-2 border-t border-slate-100 dark:border-slate-800 space-y-2">
                <label className="flex items-center justify-between cursor-pointer gap-3">
                  <span className="text-slate-700 dark:text-slate-300">
                    Xoá file gốc trong wp-content/uploads sau khi offload thành công
                  </span>
                  <input
                    type="checkbox"
                    className="w-4 h-4 accent-orange-600 rounded"
                    checked={form.agent?.delete_local_after_offload !== false}
                    onChange={(e) => set((d) => { d.agent = { ...(d.agent || {}), delete_local_after_offload: e.target.checked }; })}
                  />
                </label>
                <label className="flex items-center justify-between cursor-pointer gap-3">
                  <span className="text-slate-700 dark:text-slate-300">
                    Tự động quét &amp; offload media mới/chưa xử lý
                  </span>
                  <input
                    type="checkbox"
                    className="w-4 h-4 accent-orange-600 rounded"
                    checked={form.agent?.scan_enabled !== false}
                    onChange={(e) => set((d) => { d.agent = { ...(d.agent || {}), scan_enabled: e.target.checked }; })}
                  />
                </label>
                <div className="grid grid-cols-3 gap-3">
                  <div>
                    <label className={labelCls}>Chu kỳ quét (giây)</label>
                    <input type="number" className={inputCls} value={form.agent?.scan_interval_secs ?? 900} onChange={(e) => set((d) => { d.agent = { ...(d.agent || {}), scan_interval_secs: Number(e.target.value) }; })} />
                  </div>
                  <div>
                    <label className={labelCls}>Số media mỗi lượt</label>
                    <input type="number" className={inputCls} value={form.agent?.scan_batch_size ?? 20} onChange={(e) => set((d) => { d.agent = { ...(d.agent || {}), scan_batch_size: Number(e.target.value) }; })} />
                  </div>
                  <div>
                    <label className={labelCls}>Luồng finish song song</label>
                    <input type="number" className={inputCls} value={form.agent?.finish_workers ?? 4} onChange={(e) => set((d) => { d.agent = { ...(d.agent || {}), finish_workers: Number(e.target.value) }; })} />
                  </div>
                </div>
                <span className="text-[10px] text-slate-400 block">
                  Luồng <b>finish</b> chạy độc lập với luồng upload imgbb và luồng upload Blogger: lấy URL Blogger đã upload, crop/resize theo từng image size rồi cập nhật CDN sang phiên bản mới nhất của <code className="font-mono">attachment_id + image_size</code>.
                </span>
                <span className="text-[10px] text-slate-400 block">
                  Worker chạy nền: quét <code className="font-mono">wp_posts.post_type=attachment</code> chưa có mapping CDN/Blogger, sau đó upload imgbb → thay URL content → queue Blogger.
                </span>
              </div>

              <div>
                <label className={labelCls}>Chu kỳ poll queue Blogger (giây)</label>
                <input type="number" className={inputCls} value={form.agent?.poll_interval_secs ?? 60} onChange={(e) => set((d) => { d.agent = { ...(d.agent || {}), poll_interval_secs: Number(e.target.value) }; })} />
              </div>
              <div className="pt-2 space-y-2 border-t border-slate-100 dark:border-slate-800">
                {[
                  { key: 'replace_in_db', label: 'Thay URL ảnh trong DB WordPress sang CDN proxy' },
                  { key: 'backup_before_replace', label: 'Backup nội dung post trước khi thay URL' },
                  { key: 'enable_old_url_redirect_301', label: 'Bật redirect 301 cho URL cũ' },
                ].map((item) => (
                  <label key={item.key} className="flex items-center justify-between cursor-pointer gap-3">
                    <span className="text-slate-700 dark:text-slate-300">{item.label}</span>
                    <input
                      type="checkbox"
                      className="w-4 h-4 accent-orange-600 rounded"
                      checked={form.agent?.[item.key] !== false}
                      onChange={(e) => set((d) => { d.agent = { ...(d.agent || {}), [item.key]: e.target.checked }; })}
                    />
                  </label>
                ))}
              </div>
              <div>
                <label className={labelCls}>Daemon API token (plugin WP gửi webhook)</label>
                <div className="flex items-center gap-2">
                  <input
                    type={showSecret ? 'text' : 'password'}
                    className={inputCls}
                    value={form.agent?.daemon_api_token || ''}
                    onChange={(e) => set((d) => { d.agent = { ...(d.agent || {}), daemon_api_token: e.target.value }; })}
                  />
                  <button
                    onClick={() => setShowSecret((v) => !v)}
                    className="px-2 py-2 rounded bg-slate-100 dark:bg-slate-800 text-slate-500 text-[11px] shrink-0"
                  >
                    {showSecret ? 'Ẩn' : 'Hiện'}
                  </button>
                </div>
                <span className="text-[10px] text-slate-400 mt-0.5 block">
                  Phải trùng với <code className="font-mono">daemon_api_token</code> trong plugin WP.
                </span>
              </div>
            </div>
          </div>

          {/* CDN */}
          <div className="p-5 rounded-xl bg-white dark:bg-slate-900 border border-slate-200 dark:border-slate-800 space-y-4">
            <div className="flex items-center gap-2 border-b border-slate-100 dark:border-slate-800 pb-3">
              <KeyRound className="w-4 h-4 text-orange-500" />
              <h3 className="text-sm font-bold text-slate-900 dark:text-white">CDN proxy</h3>
            </div>
            <div className="space-y-3 text-xs">
              <div>
                <label className={labelCls}>CDN host (domain phục vụ URL pretty)</label>
                <input
                  className={inputCls}
                  value={form.cdn?.host || ''}
                  onChange={(e) => set((d) => { d.cdn = { ...(d.cdn || {}), host: e.target.value }; })}
                  placeholder="cdn.nghesachxua.localhost"
                />
              </div>
              <div>
                <label className={labelCls}>Scheme</label>
                <select
                  className={inputCls}
                  value={form.cdn?.scheme || 'https'}
                  onChange={(e) => set((d) => { d.cdn = { ...(d.cdn || {}), scheme: e.target.value }; })}
                >
                  <option value="https">https</option>
                  <option value="http">http</option>
                </select>
              </div>
              <div className="text-[11px] text-slate-500 dark:text-slate-400 bg-slate-50 dark:bg-slate-800/40 border border-slate-100 dark:border-slate-800 rounded-lg p-2.5">
                URL public có dạng <code className="font-mono">{`${form.cdn?.scheme || 'https'}://${form.cdn?.host || 'cdn.example.com'}/wp-content/uploads/...`}</code>.
                Proxy: imgbb → Blogger (bản mới nhất theo image_id + image_size).
              </div>
            </div>
          </div>

          {/* imgbb */}
          <div className="p-5 rounded-xl bg-white dark:bg-slate-900 border border-slate-200 dark:border-slate-800 space-y-4">
            <div className="flex items-center gap-2 border-b border-slate-100 dark:border-slate-800 pb-3">
              <ImageIcon className="w-4 h-4 text-orange-500" />
              <h3 className="text-sm font-bold text-slate-900 dark:text-white">imgbb backup</h3>
            </div>
            <div className="space-y-3 text-xs">
              <label className="flex items-center justify-between cursor-pointer">
                <span className="text-slate-700 dark:text-slate-300">Bật imgbb backup (nguồn trước Blogger)</span>
                <input
                  type="checkbox"
                  className="w-4 h-4 accent-orange-600 rounded"
                  checked={form.imgbb?.enabled !== false}
                  onChange={(e) => set((d) => { d.imgbb = { ...(d.imgbb || {}), enabled: e.target.checked }; })}
                />
              </label>
              <div>
                <label className={labelCls}>imgbb API key</label>
                <input
                  type={showSecret ? 'text' : 'password'}
                  className={inputCls}
                  value={form.imgbb?.api_key || ''}
                  onChange={(e) => set((d) => { d.imgbb = { ...(d.imgbb || {}), api_key: e.target.value }; })}
                  placeholder={form.imgbb?.api_key_set ? '•••••• (đã lưu — để trống nếu không đổi)' : '(bắt buộc để chạy pipeline)'}
                />
                <span className={`text-[10px] mt-0.5 block ${form.imgbb?.api_key || form.imgbb?.api_key_set ? 'text-emerald-600' : 'text-amber-600'}`}>
                  {form.imgbb?.api_key || form.imgbb?.api_key_set ? 'Đã cấu hình' : 'Chưa có key — webhook/scan sẽ lỗi khi xử lý media'}
                </span>
              </div>
              <div>
                <label className={labelCls}>Upload format</label>
                <select
                  className={inputCls}
                  value={form.imgbb?.upload_format || 'url'}
                  onChange={(e) => set((d) => { d.imgbb = { ...(d.imgbb || {}), upload_format: e.target.value }; })}
                >
                  <option value="url">url — imgbb tự tải từ URL WP (cần public)</option>
                  <option value="blob">blob — daemon upload file local</option>
                </select>
              </div>
            </div>
          </div>

          {/* Chrome */}
          <div className="p-5 rounded-xl bg-white dark:bg-slate-900 border border-slate-200 dark:border-slate-800 space-y-4">
            <div className="flex items-center gap-2 border-b border-slate-100 dark:border-slate-800 pb-3">
              <Chrome className="w-4 h-4 text-orange-500" />
              <h3 className="text-sm font-bold text-slate-900 dark:text-white">Chrome (Blogger upload &amp; audit)</h3>
            </div>
            <div className="space-y-3 text-xs">
              <div className="grid grid-cols-2 gap-3">
                <div>
                  <label className={labelCls}>Remote debug port</label>
                  <input type="number" className={inputCls} value={form.chrome?.remote_debugging_port ?? 9222} onChange={(e) => set((d) => { d.chrome = { ...(d.chrome || {}), remote_debugging_port: Number(e.target.value) }; })} />
                </div>
                <div>
                  <label className={labelCls}>Launch timeout (giây)</label>
                  <input type="number" className={inputCls} value={form.chrome?.launch_timeout_secs ?? 30} onChange={(e) => set((d) => { d.chrome = { ...(d.chrome || {}), launch_timeout_secs: Number(e.target.value) }; })} />
                </div>
              </div>
              <label className="flex items-center justify-between cursor-pointer">
                <span className="text-slate-700 dark:text-slate-300">Chạy Chrome headless khi audit tốc độ</span>
                <input
                  type="checkbox"
                  className="w-4 h-4 accent-orange-600 rounded"
                  checked={form.chrome?.headless === true}
                  onChange={(e) => set((d) => { d.chrome = { ...(d.chrome || {}), headless: e.target.checked }; })}
                />
              </label>
              <div>
                <label className={labelCls}>user_data_dir (profile Chrome đã đăng nhập Blogger)</label>
                <input className={inputCls} value={form.chrome?.user_data_dir || ''} onChange={(e) => set((d) => { d.chrome = { ...(d.chrome || {}), user_data_dir: e.target.value }; })} />
              </div>
              <button
                onClick={runPreflight}
                className="w-full px-3 py-2 rounded-lg text-xs font-semibold text-white bg-slate-800 hover:bg-slate-700 dark:bg-slate-700 dark:hover:bg-slate-600"
              >
                Kiểm tra Chrome / profile / port debug
              </button>
              {preflight && (
                <div className="p-2.5 rounded-lg bg-slate-50 dark:bg-slate-800/40 border border-slate-100 dark:border-slate-800 space-y-0.5 text-[11px] font-mono break-all">
                  <div>chrome: {preflight.chrome_path} {preflight.chrome_exists ? '✅' : '❌'}</div>
                  <div>profile: {preflight.profile_dir} {preflight.profile_exists ? '✅' : '⚠️'}</div>
                  <div>port {preflight.debug_port}: {preflight.port_open ? '✅ open' : '❌ closed'} · headless: {String(preflight.headless)}</div>
                  {(preflight.advice || []).map((a: string, i: number) => (
                    <div key={i} className="text-amber-600 dark:text-amber-400">{a}</div>
                  ))}
                </div>
              )}
              <div>
                <label className={labelCls}>Đường dẫn Chrome executable (để trống = tự dò theo OS)</label>
                <input className={inputCls} value={form.chrome?.executable || ''} onChange={(e) => set((d) => { d.chrome = { ...(d.chrome || {}), executable: e.target.value }; })} placeholder="/Applications/Google Chrome.app/Contents/MacOS/Google Chrome" />
              </div>
            </div>
          </div>

          {/* Blogger */}
          <div className="p-5 rounded-xl bg-white dark:bg-slate-900 border border-slate-200 dark:border-slate-800 space-y-4">
            <div className="flex items-center gap-2 border-b border-slate-100 dark:border-slate-800 pb-3">
              <FileJson className="w-4 h-4 text-orange-500" />
              <h3 className="text-sm font-bold text-slate-900 dark:text-white">Blogger</h3>
            </div>
            <div className="space-y-3 text-xs">
              <div>
                <label className={labelCls}>Blog ID</label>
                <input
                  className={inputCls}
                  value={form.blogger?.blog_id || ''}
                  onChange={(e) => set((d) => { d.blogger = { ...(d.blogger || {}), blog_id: e.target.value }; })}
                  placeholder="VD: 1234567890123456789"
                />
                <span className={`text-[10px] mt-0.5 block ${form.blogger?.blog_id && !String(form.blogger?.blog_id).includes('YOUR_') ? 'text-emerald-600' : 'text-amber-600'}`}>
                  {form.blogger?.blog_id && !String(form.blogger?.blog_id).includes('YOUR_') ? 'Đã cấu hình' : 'Chưa cấu hình — queue Blogger sẽ lỗi'}
                </span>
              </div>
              <div>
                <label className={labelCls}>Upload method</label>
                <input className={inputCls} value={form.blogger?.upload_via || 'chrome_web_ui'} readOnly />
                <span className="text-[10px] text-slate-400 mt-0.5 block">
                  Chỉ hỗ trợ upload qua Chrome UI (không dùng Google API).
                </span>
              </div>
              <label className="flex items-center justify-between cursor-pointer">
                <span className="text-slate-700 dark:text-slate-300">1 post WP = 1 bài Blogger</span>
                <input
                  type="checkbox"
                  className="w-4 h-4 accent-orange-600 rounded"
                  checked={form.blogger?.one_wp_post_per_blogger_post !== false}
                  onChange={(e) => set((d) => { d.blogger = { ...(d.blogger || {}), one_wp_post_per_blogger_post: e.target.checked }; })}
                />
              </label>
              <div className="flex items-start gap-1.5 text-[11px] text-amber-700 dark:text-amber-400 bg-amber-50 dark:bg-amber-950/30 border border-amber-200 dark:border-amber-900/50 rounded-lg p-2.5">
                <AlertTriangle className="w-3.5 h-3.5 mt-0.5 shrink-0" />
                <span>
                  Audit tốc độ chạy bằng nút <b>Kiểm tra ngay</b> hoặc <code className="font-mono">POST /api/audits/run</code>; không có scheduler cron trong daemon.
                </span>
              </div>
            </div>
          </div>
        </div>
      )}
    </div>
  );
};
