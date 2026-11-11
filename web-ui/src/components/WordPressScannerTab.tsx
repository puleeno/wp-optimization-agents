import React, { useState } from 'react';
import {
  FileText,
  Search,
  Filter,
  RefreshCw,
  Zap,
  CheckCircle2,
  AlertTriangle,
  ArrowRight,
  ExternalLink,
  Layers,
  Sparkles,
  HardDrive,
  Globe,
} from 'lucide-react';
import { WordPressPost } from '../types/dashboard';

interface WordPressScannerTabProps {
  posts: WordPressPost[];
  onBatchOffloadPost: (postId: number) => void;
  onBatchOffloadAll: () => void;
}

export const WordPressScannerTab: React.FC<WordPressScannerTabProps> = ({
  posts,
  onBatchOffloadPost,
  onBatchOffloadAll,
}) => {
  const [searchTerm, setSearchTerm] = useState('');
  const [statusFilter, setStatusFilter] = useState<'all' | 'pending' | 'completed'>('all');
  const [selectedPost, setSelectedPost] = useState<WordPressPost | null>(null);
  const [isProcessingBatch, setIsProcessingBatch] = useState(false);
  const [batchProgress, setBatchProgress] = useState(0);

  const filteredPosts = posts.filter((post) => {
    if (searchTerm && !post.title.toLowerCase().includes(searchTerm.toLowerCase())) return false;
    const isCompleted = post.offloadedImages === post.totalImages;
    if (statusFilter === 'pending' && isCompleted) return false;
    if (statusFilter === 'completed' && !isCompleted) return false;
    return true;
  });

  const totalImagesInWp = posts.reduce((acc, p) => acc + p.totalImages, 0);
  const totalOffloadedInWp = posts.reduce((acc, p) => acc + p.offloadedImages, 0);
  const pendingOffloadCount = totalImagesInWp - totalOffloadedInWp;

  const handleStartFullBatch = () => {
    setIsProcessingBatch(true);
    setBatchProgress(10);

    const interval = setInterval(() => {
      setBatchProgress((prev) => {
        if (prev >= 100) {
          clearInterval(interval);
          setIsProcessingBatch(false);
          onBatchOffloadAll();
          return 100;
        }
        return prev + 25;
      });
    }, 400);
  };

  return (
    <div className="space-y-6">
      {/* Top Banner & Batch Action */}
      <div className="p-5 rounded-xl bg-white dark:bg-slate-900 border border-slate-200 dark:border-slate-800 flex flex-col md:flex-row md:items-center justify-between gap-4">
        <div>
          <div className="flex items-center gap-2 text-xs text-orange-600 dark:text-orange-400 font-mono font-medium">
            <span>WORDPRESS REST API DISPATCHER</span>
            <span>·</span>
            <span>BULK OFFLOAD AGENT</span>
          </div>
          <h2 className="text-lg font-bold text-slate-900 dark:text-white mt-0.5">
            Quét & Thay thế ảnh bài viết WordPress sang Blogger CDN
          </h2>
          <p className="text-xs text-slate-500 dark:text-slate-400 mt-1 max-w-2xl">
            Tự động tìm kiếm các liên kết hình ảnh cục bộ (`/wp-content/uploads/`) trong nội dung bài viết HTML,
            chuyển tiếp qua Rust daemon để upload Blogger và cập nhật lại bài viết mà không làm gãy layout.
          </p>
        </div>

        <div className="flex items-center gap-3">
          <div className="text-right hidden sm:block">
            <div className="text-xs font-mono font-bold text-slate-900 dark:text-white">
              {pendingOffloadCount} ảnh chưa offload
            </div>
            <div className="text-[11px] text-slate-400">
              Đã hoàn thành {totalOffloadedInWp}/{totalImagesInWp}
            </div>
          </div>

          <button
            onClick={handleStartFullBatch}
            disabled={isProcessingBatch || pendingOffloadCount === 0}
            className="px-4 py-2 text-xs font-semibold text-white bg-orange-600 hover:bg-orange-500 disabled:opacity-50 rounded-lg shadow-sm transition-all flex items-center gap-1.5 whitespace-nowrap"
          >
            <RefreshCw className={`w-3.5 h-3.5 ${isProcessingBatch ? 'animate-spin' : ''}`} />
            <span>{isProcessingBatch ? `Đang xử lý ${batchProgress}%...` : 'Bắt đầu Offload toàn bộ'}</span>
          </button>
        </div>
      </div>

      {/* Batch Progress Bar if active */}
      {isProcessingBatch && (
        <div className="p-4 rounded-xl bg-orange-50 dark:bg-orange-950/40 border border-orange-200 dark:border-orange-900 space-y-2">
          <div className="flex justify-between text-xs font-mono text-orange-900 dark:text-orange-200 font-semibold">
            <span>Tiến trình Rust Tokio Worker đang thực thi...</span>
            <span>{batchProgress}%</span>
          </div>
          <div className="w-full bg-orange-200 dark:bg-orange-900 h-2 rounded-full overflow-hidden">
            <div
              className="bg-orange-600 h-full transition-all duration-300"
              style={{ width: `${batchProgress}%` }}
            />
          </div>
          <div className="text-[11px] text-orange-700 dark:text-orange-400 flex items-center gap-2">
            <span>Tải lên Blogger Album</span>
            <span>→</span>
            <span>Sinh URL `=w800-h500-rw`</span>
            <span>→</span>
            <span>Ghi đè HTML post qua REST API</span>
          </div>
        </div>
      )}

      {/* Filter & Search Bar */}
      <div className="flex flex-col sm:flex-row sm:items-center justify-between gap-3">
        <div className="relative flex-1 max-w-md">
          <Search className="w-4 h-4 text-slate-400 absolute left-3 top-2.5" />
          <input
            type="text"
            placeholder="Tìm theo tiêu đề bài viết WordPress..."
            value={searchTerm}
            onChange={(e) => setSearchTerm(e.target.value)}
            className="w-full pl-9 pr-4 py-2 text-xs rounded-lg border border-slate-200 dark:border-slate-800 bg-white dark:bg-slate-900 text-slate-900 dark:text-white placeholder-slate-400 focus:outline-none focus:ring-1 focus:ring-orange-500"
          />
        </div>

        <div className="flex items-center gap-1 p-1 bg-slate-100 dark:bg-slate-800 rounded-lg text-xs">
          <button
            onClick={() => setStatusFilter('all')}
            className={`px-3 py-1.5 rounded-md font-medium transition-colors ${
              statusFilter === 'all'
                ? 'bg-white dark:bg-slate-900 text-slate-900 dark:text-white shadow-sm'
                : 'text-slate-600 dark:text-slate-400 hover:text-slate-900'
            }`}
          >
            Tất cả ({posts.length})
          </button>
          <button
            onClick={() => setStatusFilter('pending')}
            className={`px-3 py-1.5 rounded-md font-medium transition-colors ${
              statusFilter === 'pending'
                ? 'bg-white dark:bg-slate-900 text-slate-900 dark:text-white shadow-sm'
                : 'text-slate-600 dark:text-slate-400 hover:text-slate-900'
            }`}
          >
            Chưa hoàn thành
          </button>
          <button
            onClick={() => setStatusFilter('completed')}
            className={`px-3 py-1.5 rounded-md font-medium transition-colors ${
              statusFilter === 'completed'
                ? 'bg-white dark:bg-slate-900 text-slate-900 dark:text-white shadow-sm'
                : 'text-slate-600 dark:text-slate-400 hover:text-slate-900'
            }`}
          >
            Đã tối ưu 100%
          </button>
        </div>
      </div>

      {/* Post Table */}
      <div className="rounded-xl border border-slate-200 dark:border-slate-800 bg-white dark:bg-slate-900 overflow-hidden">
        <div className="overflow-x-auto">
          <table className="w-full text-left text-xs">
            <thead className="bg-slate-50 dark:bg-slate-800/60 border-b border-slate-200 dark:border-slate-800 text-slate-500 dark:text-slate-400 uppercase font-mono text-[11px]">
              <tr>
                <th className="py-3 px-4">Bài viết WordPress</th>
                <th className="py-3 px-4">Tác giả & Ngày</th>
                <th className="py-3 px-4">Trạng thái Offload</th>
                <th className="py-3 px-4">Ảnh đã thay thế</th>
                <th className="py-3 px-4 text-right">Thao tác</th>
              </tr>
            </thead>
            <tbody className="divide-y divide-slate-100 dark:divide-slate-800">
              {filteredPosts.map((post) => {
                const isFullyDone = post.offloadedImages === post.totalImages;
                const pct = Math.round((post.offloadedImages / post.totalImages) * 100);

                return (
                  <tr
                    key={post.id}
                    className="hover:bg-slate-50/70 dark:hover:bg-slate-800/40 transition-colors"
                  >
                    <td className="py-3 px-4">
                      <div className="font-semibold text-slate-900 dark:text-white">
                        {post.title}
                      </div>
                      <div className="text-[11px] text-slate-400 font-mono mt-0.5">
                        /{post.slug}/ · ID: {post.id}
                      </div>
                    </td>

                    <td className="py-3 px-4 text-slate-600 dark:text-slate-300">
                      <div>{post.author}</div>
                      <div className="text-[11px] text-slate-400 font-mono">{post.publishedDate}</div>
                    </td>

                    <td className="py-3 px-4">
                      {isFullyDone ? (
                        <div className="inline-flex items-center gap-1.5 text-emerald-600 dark:text-emerald-400 font-medium">
                          <CheckCircle2 className="w-4 h-4" />
                          <span>100% Hoàn tất</span>
                        </div>
                      ) : (
                        <div className="inline-flex items-center gap-1.5 text-amber-600 dark:text-amber-400 font-medium">
                          <AlertTriangle className="w-4 h-4" />
                          <span>Đang chờ ({post.totalImages - post.offloadedImages} ảnh)</span>
                        </div>
                      )}
                    </td>

                    <td className="py-3 px-4">
                      <div className="flex items-center gap-2">
                        <div className="w-24 bg-slate-200 dark:bg-slate-700 h-2 rounded-full overflow-hidden">
                          <div
                            className={`h-full rounded-full ${
                              isFullyDone ? 'bg-emerald-500' : 'bg-orange-500'
                            }`}
                            style={{ width: `${pct}%` }}
                          />
                        </div>
                        <span className="font-mono text-slate-700 dark:text-slate-300 tabular-nums">
                          {post.offloadedImages}/{post.totalImages}
                        </span>
                      </div>
                    </td>

                    <td className="py-3 px-4 text-right">
                      <div className="flex items-center justify-end gap-2">
                        <button
                          onClick={() => setSelectedPost(post)}
                          className="px-2.5 py-1 text-slate-600 dark:text-slate-400 hover:text-slate-900 dark:hover:text-white bg-slate-100 dark:bg-slate-800 rounded font-medium hover:bg-slate-200 transition-colors"
                        >
                          Chi tiết ảnh
                        </button>

                        {!isFullyDone && (
                          <button
                            onClick={() => onBatchOffloadPost(post.id)}
                            className="px-2.5 py-1 text-white bg-orange-600 hover:bg-orange-500 rounded font-medium transition-colors"
                          >
                            Offload ngay
                          </button>
                        )}
                      </div>
                    </td>
                  </tr>
                );
              })}
            </tbody>
          </table>
        </div>
      </div>

      {/* Post Images Detail Modal */}
      {selectedPost && (
        <div className="fixed inset-0 z-50 flex items-center justify-center p-4 bg-black/60 backdrop-blur-sm animate-fadeIn">
          <div className="w-full max-w-2xl bg-white dark:bg-slate-900 rounded-xl border border-slate-200 dark:border-slate-800 p-6 space-y-4 max-h-[85vh] overflow-y-auto">
            <div className="flex items-start justify-between">
              <div>
                <span className="text-[11px] font-mono text-orange-500 uppercase font-bold">
                  Chi tiết hình ảnh trong bài viết
                </span>
                <h3 className="text-base font-bold text-slate-900 dark:text-white mt-1">
                  {selectedPost.title}
                </h3>
              </div>
              <button
                onClick={() => setSelectedPost(null)}
                className="text-slate-400 hover:text-slate-600 dark:hover:text-white text-sm"
              >
                ✕
              </button>
            </div>

            <div className="space-y-3 pt-2">
              {selectedPost.images.map((img, idx) => (
                <div
                  key={idx}
                  className="p-3 rounded-lg border border-slate-200 dark:border-slate-800 bg-slate-50 dark:bg-slate-800/50 space-y-2 text-xs"
                >
                  <div className="flex justify-between items-center">
                    <span className="font-semibold text-slate-800 dark:text-slate-200 font-mono">
                      Ảnh #{idx + 1} ({img.sizeKb} KB)
                    </span>
                    <span
                      className={`font-semibold font-mono text-[11px] px-2 py-0.5 rounded ${
                        img.isOffloaded
                          ? 'bg-emerald-100 dark:bg-emerald-950/60 text-emerald-700 dark:text-emerald-400'
                          : 'bg-amber-100 dark:bg-amber-950/60 text-amber-700 dark:text-amber-400'
                      }`}
                    >
                      {img.isOffloaded ? 'Blogger CDN Active' : 'Chưa Offload (WordPress local)'}
                    </span>
                  </div>

                  <div className="space-y-1 font-mono text-[11px]">
                    <div className="text-slate-500 truncate">
                      <span className="text-slate-400">Origin: </span>
                      {img.originUrl}
                    </div>
                    {img.bloggerUrl ? (
                      <div className="text-emerald-600 dark:text-emerald-400 truncate font-semibold">
                        <span className="text-slate-400">Blogger: </span>
                        {img.bloggerUrl}
                      </div>
                    ) : (
                      <div className="text-amber-600 dark:text-amber-400">
                        Sẽ được upload lên Blogger album và sinh tham số WebP tự động.
                      </div>
                    )}
                  </div>
                </div>
              ))}
            </div>

            <div className="pt-3 border-t border-slate-100 dark:border-slate-800 flex justify-end gap-2">
              <button
                onClick={() => setSelectedPost(null)}
                className="px-4 py-2 text-xs font-medium rounded-lg text-slate-600 dark:text-slate-300 hover:bg-slate-100 dark:hover:bg-slate-800"
              >
                Đóng
              </button>
              {selectedPost.offloadedImages < selectedPost.totalImages && (
                <button
                  onClick={() => {
                    onBatchOffloadPost(selectedPost.id);
                    setSelectedPost(null);
                  }}
                  className="px-4 py-2 text-xs font-semibold text-white bg-orange-600 hover:bg-orange-500 rounded-lg shadow-sm"
                >
                  Tiến hành Offload bài này
                </button>
              )}
            </div>
          </div>
        </div>
      )}
    </div>
  );
};
