import React, { useEffect, useMemo, useState } from 'react';
import {
  CheckCircle2,
  AlertTriangle,
  RefreshCw,
  Search,
  Image as ImageIcon,
  Package,
  FileText,
  Link2,
} from 'lucide-react';

type AttachmentItem = {
  id: number;
  attachmentId?: number;
  parentPostId?: number | null;
  title: string;
  mimeType?: string | null;
  originUrl: string;
  uploadedAt?: string | null;
  sizesCount?: number;
  totalMappings?: number;
  offloadedMappings?: number;
  replacedMappings?: number;
  failedMappings?: number;
  latestStatus?: string | null;
  affectedPostIds?: number[];
  affectedPostTypes?: string[];
};

interface WordPressScannerTabProps {
  onProcessAttachment?: (attachmentId: number) => void;
  onBatchProcess?: () => void;
}

export const WordPressScannerTab: React.FC<WordPressScannerTabProps> = ({
  onProcessAttachment,
  onBatchProcess,
}) => {
  const [items, setItems] = useState<AttachmentItem[]>([]);
  const [loading, setLoading] = useState(true);
  const [page, setPage] = useState(0);
  const [total, setTotal] = useState(0);
  const [pageSize, setPageSize] = useState(200);
  const [scanInfo, setScanInfo] = useState<any>(null);
  const [error, setError] = useState<string | null>(null);
  const [searchTerm, setSearchTerm] = useState('');
  const [statusFilter, setStatusFilter] = useState<'all' | 'pending' | 'processing' | 'done' | 'failed'>('all');
  const [processingId, setProcessingId] = useState<number | null>(null);
  const [batchRunning, setBatchRunning] = useState(false);

  const load = async (targetPage?: number) => {
    try {
      const p = targetPage ?? page;
      setLoading(true);
      setError(null);
      const offset = p * pageSize;
      const res = await fetch(`/api/attachments?limit=${pageSize}&offset=${offset}`);
      if (!res.ok) throw new Error(`HTTP ${res.status}`);
      const data = await res.json();
      const arr = Array.isArray(data) ? data : (Array.isArray(data?.data) ? data.data : []);
      const totalCount = Number(res.headers.get('x-total-count') || 0);
      setItems(arr);
      if (totalCount > 0) setTotal(totalCount);
      setPage(p);
      const s = await fetch('/api/scan/status').then((r) => r.json()).catch(() => null);
      if (s) setScanInfo(s);
    } catch (e: any) {
      setError(e?.message || 'Không tải được danh sách media');
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => { load(0); }, [pageSize]); // eslint-disable-line react-hooks/exhaustive-deps

  const deriveState = (it: AttachmentItem) => {
    const off = Number(it.offloadedMappings || 0);
    const rep = Number(it.replacedMappings || 0);
    const fail = Number(it.failedMappings || 0);
    const total = Number(it.totalMappings || 0);
    if (fail > 0) return 'failed';
    if (total > 0 && rep >= total) return 'done';
    if (total > 0 && off > 0) return 'processing';
    return 'pending';
  };

  const filtered = useMemo(() => {
    return items.filter((it) => {
      const q = searchTerm.trim().toLowerCase();
      if (q) {
        const hay = `${it.title || ''} ${it.originUrl || ''} ${(it.affectedPostTypes || []).join(' ')}`.toLowerCase();
        if (!hay.includes(q)) return false;
      }
      const st = deriveState(it);
      if (statusFilter !== 'all' && st !== statusFilter) return false;
      return true;
    });
  }, [items, searchTerm, statusFilter]);

  const counts = useMemo(() => {
    let pending = 0, processing = 0, done = 0, failed = 0;
    for (const it of items) {
      const st = deriveState(it);
      if (st === 'pending') pending++;
      else if (st === 'processing') processing++;
      else if (st === 'done') done++;
      else failed++;
    }
    return { total: Math.max(total, items.length), pending, processing, done, failed };
  }, [items]);

  const processOne = async (id: number) => {
    if (!onProcessAttachment) return;
    setProcessingId(id);
    try {
      await onProcessAttachment(id);
      await load();
    } finally {
      setProcessingId(null);
    }
  };

  const processBatch = async () => {
    if (batchRunning) return;
    setBatchRunning(true);
    try {
      if (onBatchProcess) {
        await onBatchProcess();
      } else {
        const queue = items.filter((it) => deriveState(it) !== 'done').slice(0, 50);
        for (const it of queue) {
          await processOne(it.id);
        }
      }
      await load(page);
    } finally {
      setBatchRunning(false);
    }
  };

  return (
    <div className="space-y-6">
      <div className="p-5 rounded-xl bg-white dark:bg-slate-900 border border-slate-200 dark:border-slate-800 flex flex-col md:flex-row md:items-center justify-between gap-4">
        <div>
          <div className="flex items-center gap-2 text-xs text-orange-600 dark:text-orange-400 font-mono font-medium">
            <span>MEDIA LIBRARY SCANNER</span>
            <span>·</span>
            <span>ATTACHMENTS / ALL POST TYPES</span>
          </div>
          <h2 className="text-lg font-bold text-slate-900 dark:text-white mt-0.5">
            Quét thư viện ảnh WordPress (bao gồm sản phẩm WooCommerce, page, custom post type)
          </h2>
          <p className="text-xs text-slate-500 dark:text-slate-400 mt-1 max-w-3xl">
            Quét trực tiếp từ <code>wp_posts.post_type=attachment</code> + metadata/sizes. Ảnh được xử lý theo luồng webhook: upload imgbb (backup) → thay URL trong WP sang CDN agent → xóa file vật lý → queue upload Blogger → chuyển CDN sang bản Blogger mới nhất theo <code>image_id + image_size</code>.
          </p>
        </div>
        <div className="flex items-center gap-2">
          <select
            value={pageSize}
            onChange={(e) => setPageSize(Number(e.target.value))}
            className="px-2 py-2 text-xs rounded-lg border border-slate-200 dark:border-slate-700 bg-white dark:bg-slate-900"
            title="Số media mỗi trang"
          >
            <option value={100}>100/trang</option>
            <option value={200}>200/trang</option>
            <option value={500}>500/trang</option>
            <option value={1000}>1000/trang</option>
          </select>
          <button
            onClick={async () => {
              try {
                await fetch('/api/scan/run', { method: 'POST', headers: { 'Content-Type': 'application/json' }, body: JSON.stringify({}) });
              } catch {}
              await load(page);
            }}
            disabled={batchRunning}
            title="Yêu cầu daemon quét & offload ngay batch media chưa xử lý"
            className="px-3 py-2 text-xs font-semibold text-amber-700 dark:text-amber-300 border border-amber-300 dark:border-amber-800 bg-amber-50 dark:bg-amber-950/30 hover:bg-amber-100 rounded-lg disabled:opacity-50"
          >
            Quét ngay
          </button>
          <button onClick={() => load(page)} className="px-3 py-2 text-xs font-medium rounded-lg border border-slate-200 dark:border-slate-700 hover:bg-slate-50 dark:hover:bg-slate-800">Tải lại</button>
          <button
            onClick={processBatch}
            disabled={batchRunning || counts.pending + counts.processing + counts.failed === 0}
            className="px-4 py-2 text-xs font-semibold text-white bg-orange-600 hover:bg-orange-500 disabled:opacity-50 rounded-lg shadow-sm flex items-center gap-1.5"
          >
            <RefreshCw className={`w-3.5 h-3.5 ${batchRunning ? 'animate-spin' : ''}`} />
            <span>{batchRunning ? 'Đang xử lý hàng loạt...' : 'Xử lý media chưa xong'}</span>
          </button>
        </div>
      </div>

      <div className="grid grid-cols-2 md:grid-cols-5 gap-3 text-xs">
        <div className="p-3 rounded-lg bg-white dark:bg-slate-900 border border-slate-200 dark:border-slate-800">
          <div className="text-slate-400">Tổng media</div>
          <div className="text-lg font-bold">{counts.total}</div>
          <div className="text-[10px] text-slate-400 mt-0.5">
            {scanInfo
              ? `chờ xử lý: ${Number(scanInfo.pending ?? 0).toLocaleString()} · đã xử lý: ${Number(scanInfo.processed ?? 0).toLocaleString()}`
              : 'chưa có dữ liệu hàng đợi'}
          </div>
        </div>
        <div className="p-3 rounded-lg bg-white dark:bg-slate-900 border border-slate-200 dark:border-slate-800"><div className="text-slate-400">Chờ xử lý</div><div className="text-lg font-bold text-amber-600">{counts.pending}</div></div>
        <div className="p-3 rounded-lg bg-white dark:bg-slate-900 border border-slate-200 dark:border-slate-800"><div className="text-slate-400">Đang/đã backup</div><div className="text-lg font-bold text-sky-600">{counts.processing}</div></div>
        <div className="p-3 rounded-lg bg-white dark:bg-slate-900 border border-slate-200 dark:border-slate-800"><div className="text-slate-400">Hoàn tất</div><div className="text-lg font-bold text-emerald-600">{counts.done}</div></div>
        <div className="p-3 rounded-lg bg-white dark:bg-slate-900 border border-slate-200 dark:border-slate-800"><div className="text-slate-400">Lỗi</div><div className="text-lg font-bold text-rose-600">{counts.failed}</div></div>
      </div>

      <div className="flex flex-col sm:flex-row sm:items-center justify-between gap-3">
        <div className="relative flex-1 max-w-md">
          <Search className="w-4 h-4 text-slate-400 absolute left-3 top-2.5" />
          <input
            type="text"
            placeholder="Tìm theo tên, URL, post type (product, page, post...)"
            value={searchTerm}
            onChange={(e) => setSearchTerm(e.target.value)}
            className="w-full pl-9 pr-4 py-2 text-xs rounded-lg border border-slate-200 dark:border-slate-800 bg-white dark:bg-slate-900"
          />
        </div>
        <div className="flex items-center gap-1 p-1 bg-slate-100 dark:bg-slate-800 rounded-lg text-xs">
          {(['all','pending','processing','done','failed'] as const).map((s) => (
            <button key={s} onClick={() => setStatusFilter(s)} className={`px-3 py-1.5 rounded-md font-medium ${statusFilter===s?'bg-white dark:bg-slate-900 shadow-sm':''}`}>{s}</button>
          ))}
        </div>
      </div>

      {scanInfo && (!scanInfo.imgbb_ready || !scanInfo.blogger_ready || !scanInfo.scan_enabled) && (
        <div className="p-3 rounded-lg text-xs border bg-amber-50 dark:bg-amber-950/30 border-amber-200 dark:border-amber-900 text-amber-800 dark:text-amber-300 space-y-1">
          <div className="font-semibold">Worker chưa thể chạy — thiếu cấu hình:</div>
          <ul className="list-disc list-inside space-y-0.5">
            {!scanInfo.imgbb_ready && <li>imgbb API key chưa cấu hình (vào <b>/config</b> → imgbb backup → nhập API key).</li>}
            {!scanInfo.blogger_ready && <li>Blogger Blog ID chưa cấu hình (vào <b>/config</b> → Blogger → Blog ID).</li>}
            {!scanInfo.scan_enabled && <li>Tự động quét đang tắt (<b>/config</b> → Daemon runtime → bật “Tự động quét”).</li>}
          </ul>
          <div className="text-[11px] opacity-80">
            {scanInfo.delete_local_after_offload !== false && (
              <li className="text-rose-600 dark:text-rose-400">
                Đang bật <b>Xoá file gốc sau khi offload</b> — có thể tắt ở <b>/config</b> → Daemon runtime nếu muốn giữ file gốc.
              </li>
            )}
            Hàng đợi Blogger: {scanInfo.queue?.queued ?? 0} chờ · {scanInfo.queue?.uploaded ?? 0} chờ crop/resize · {scanInfo.queue?.finished ?? 0} xong · {scanInfo.queue?.failed ?? 0} lỗi · chu kỳ {scanInfo.scan_interval_secs}s · batch {scanInfo.scan_batch_size}/lượt · {scanInfo.finish_workers} luồng finish.
          </div>
        </div>
      )}

      <div className="rounded-xl border border-slate-200 dark:border-slate-800 bg-white dark:bg-slate-900 overflow-hidden">
        {error && <div className="p-4 text-sm text-rose-600 border-b border-slate-200 dark:border-slate-800">{error}</div>}
        <div className="overflow-x-auto">
          <table className="w-full text-left text-xs">
            <thead className="bg-slate-50 dark:bg-slate-800/60 border-b border-slate-200 dark:border-slate-800 text-slate-500 dark:text-slate-400 uppercase font-mono text-[11px]">
              <tr>
                <th className="py-3 px-4">Media / Attachment</th>
                <th className="py-3 px-4">Sizes</th>
                <th className="py-3 px-4">Ảnh hưởng (post types)</th>
                <th className="py-3 px-4">Trạng thái offload/CDN</th>
                <th className="py-3 px-4 text-right">Thao tác</th>
              </tr>
            </thead>
            <tbody className="divide-y divide-slate-100 dark:divide-slate-800">
              {!loading && filtered.map((it) => {
                const st = deriveState(it);
                const types = it.affectedPostTypes || [];
                return (
                  <tr key={it.id} className="hover:bg-slate-50/70 dark:hover:bg-slate-800/40">
                    <td className="py-3 px-4">
                      <div className="flex items-start gap-2">
                        <ImageIcon className="w-4 h-4 mt-0.5 text-slate-400" />
                        <div className="min-w-0">
                          <div className="font-semibold text-slate-900 dark:text-white truncate max-w-[420px]">{it.title || `Attachment #${it.id}`}</div>
                          <div className="text-[11px] text-slate-400 font-mono truncate max-w-[420px]">#{it.id} · {it.mimeType || 'image'}</div>
                          <div className="text-[11px] text-slate-500 truncate max-w-[420px]">{it.originUrl}</div>
                        </div>
                      </div>
                    </td>
                    <td className="py-3 px-4 font-mono">
                      <div>{it.sizesCount ?? 1} size(s)</div>
                      <div className="text-[11px] text-slate-400">mappings: {it.replacedMappings || 0}/{it.totalMappings || 0}</div>
                    </td>
                    <td className="py-3 px-4">
                      <div className="flex flex-wrap gap-1">
                        {types.length ? types.map((t) => (
                          <span key={t} className={`px-2 py-0.5 rounded text-[11px] font-mono ${t==='product'?'bg-violet-100 text-violet-700 dark:bg-violet-950/50 dark:text-violet-300':'bg-slate-100 text-slate-700 dark:bg-slate-800 dark:text-slate-300'}`}>{t}</span>
                        )) : <span className="text-slate-400 text-[11px]">chưa phát hiện / attachment đơn</span>}
                      </div>
                      <div className="text-[11px] text-slate-400 mt-1">posts ảnh hưởng: {(it.affectedPostIds || []).length}</div>
                    </td>
                    <td className="py-3 px-4">
                      {st === 'done' ? (
                        <div className="inline-flex items-center gap-1.5 text-emerald-600 font-medium"><CheckCircle2 className="w-4 h-4" /><span>Đã CDN/Blogger theo phiên bản</span></div>
                      ) : st === 'failed' ? (
                        <div className="inline-flex items-center gap-1.5 text-rose-600 font-medium"><AlertTriangle className="w-4 h-4" /><span>Lỗi ({it.failedMappings || 0})</span></div>
                      ) : st === 'processing' ? (
                        <div className="inline-flex items-center gap-1.5 text-sky-600 font-medium"><RefreshCw className="w-4 h-4" /><span>Đã backup/đang chờ Blogger</span></div>
                      ) : (
                        <div className="inline-flex items-center gap-1.5 text-amber-600 font-medium"><AlertTriangle className="w-4 h-4" /><span>Chưa xử lý</span></div>
                      )}
                      {it.latestStatus && <div className="text-[11px] text-slate-400 font-mono mt-0.5">{it.latestStatus}</div>}
                    </td>
                    <td className="py-3 px-4 text-right">
                      <div className="inline-flex items-center gap-1.5">
                      <button
                        onClick={() => processOne(it.id)}
                        disabled={processingId === it.id}
                        className="px-2.5 py-1 text-white bg-orange-600 hover:bg-orange-500 disabled:opacity-50 rounded font-medium"
                      >
                        {processingId === it.id ? 'Đang xử lý...' : 'Upload imgbb + queue'}
                      </button>
                      <a
                        href={`/daemon?attachmentId=${it.id}`}
                        className="px-2.5 py-1 text-slate-700 dark:text-slate-200 border border-slate-200 dark:border-slate-700 hover:bg-slate-50 dark:hover:bg-slate-800 rounded font-medium"
                        title="Xem log xử lý của media này"
                      >
                        Log
                      </a>
                      </div>
                    </td>
                  </tr>
                );
              })}
              {loading && (
                <tr><td colSpan={5} className="py-8 px-4 text-center text-slate-500">Đang quét thư viện ảnh...</td></tr>
              )}
              {!loading && !filtered.length && !error && (
                <tr><td colSpan={5} className="py-8 px-4 text-center text-slate-500">Không có attachment phù hợp.</td></tr>
              )}
            </tbody>
          </table>
        </div>

        <div className="flex flex-col sm:flex-row sm:items-center justify-between gap-2 px-4 py-3 border-t border-slate-200 dark:border-slate-800 text-xs bg-slate-50/60 dark:bg-slate-800/30">
          <div className="text-slate-500 dark:text-slate-400">
            Hiển thị <span className="font-mono font-semibold text-slate-700 dark:text-slate-200">{items.length}</span> / <span className="font-mono font-semibold text-slate-700 dark:text-slate-200">{total.toLocaleString()}</span> media
            {total > 0 && <> · Trang <span className="font-mono font-semibold text-slate-700 dark:text-slate-200">{page + 1}</span>/<span className="font-mono">{Math.max(1, Math.ceil(total / pageSize))}</span></>}
          </div>
          <div className="flex items-center gap-1.5">
            <button
              onClick={() => load(0)}
              disabled={loading || page === 0}
              className="px-2.5 py-1.5 rounded border border-slate-200 dark:border-slate-700 disabled:opacity-40 hover:bg-white dark:hover:bg-slate-900"
            >
              « Đầu
            </button>
            <button
              onClick={() => load(Math.max(0, page - 1))}
              disabled={loading || page === 0}
              className="px-2.5 py-1.5 rounded border border-slate-200 dark:border-slate-700 disabled:opacity-40 hover:bg-white dark:hover:bg-slate-900"
            >
              ‹ Trước
            </button>
            <button
              onClick={() => load(page + 1)}
              disabled={loading || (total > 0 && (page + 1) * pageSize >= total)}
              className="px-2.5 py-1.5 rounded border border-slate-200 dark:border-slate-700 disabled:opacity-40 hover:bg-white dark:hover:bg-slate-900"
            >
              Sau ›
            </button>
            <button
              onClick={() => load(Math.max(0, Math.ceil(total / pageSize) - 1))}
              disabled={loading || total === 0 || (page + 1) * pageSize >= total}
              className="px-2.5 py-1.5 rounded border border-slate-200 dark:border-slate-700 disabled:opacity-40 hover:bg-white dark:hover:bg-slate-900"
            >
              Cuối »
            </button>
          </div>
        </div>
      </div>
    </div>
  );
};
