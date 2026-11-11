import React, { useState, useRef } from 'react';
import {
  Upload,
  Link,
  Copy,
  Check,
  Code2,
  Image as ImageIcon,
  Sliders,
  Crop,
  Sparkles,
  Zap,
  ExternalLink,
  RotateCcw,
  CheckCircle2,
  FileCheck,
  Layers,
  ArrowRight,
  Maximize2,
  Download,
} from 'lucide-react';
import { OffloadImage, BloggerTransformParams } from '../types/dashboard';
import {
  buildBloggerParamString,
  applyBloggerTransform,
  formatBytes,
  generateHtmlSnippet,
} from '../utils/bloggerUrl';

interface BloggerLabTabProps {
  images: OffloadImage[];
  onAddOffloadedImage: (image: OffloadImage) => void;
}

export const BloggerLabTab: React.FC<BloggerLabTabProps> = ({
  images,
  onAddOffloadedImage,
}) => {
  // Current selected or uploaded image
  const [selectedImage, setSelectedImage] = useState<OffloadImage>(images[0]);
  const [copiedType, setCopiedType] = useState<string | null>(null);

  // Transform parameters state
  const [params, setParams] = useState<BloggerTransformParams>({
    sizeMode: 'exact',
    preset: 'large',
    maxDimension: 1200,
    width: 1200,
    height: 675,
    smartCrop: true,
    webpFormat: true,
    lossless: false,
    quality: 85,
    blurPlaceholder: false,
  });

  // Upload simulation state
  const [isUploading, setIsUploading] = useState(false);
  const [uploadSuccessMsg, setUploadSuccessMsg] = useState<string | null>(null);
  const [customUrlInput, setCustomUrlInput] = useState('');
  const [activeCodeTab, setActiveCodeTab] = useState<'url' | 'html' | 'markdown' | 'php'>('url');
  const fileInputRef = useRef<HTMLInputElement>(null);

  // Calculate live transformed URL
  const currentSuffix = buildBloggerParamString(params);
  const liveTransformedUrl = applyBloggerTransform(selectedImage.bloggerBaseUrl, params);

  // Estimated size savings
  const estimatedOffloadBytes = params.webpFormat
    ? Math.round(selectedImage.originalSizeBytes * 0.055)
    : Math.round(selectedImage.originalSizeBytes * 0.15);

  const savingsPct = Math.round(
    ((selectedImage.originalSizeBytes - estimatedOffloadBytes) / selectedImage.originalSizeBytes) * 100
  );

  const handleCopy = (text: string, type: string) => {
    navigator.clipboard.writeText(text);
    setCopiedType(type);
    setTimeout(() => setCopiedType(null), 2000);
  };

  // Handle local file drop/picker
  const handleFileUpload = (e: React.ChangeEvent<HTMLInputElement>) => {
    const file = e.target.files?.[0];
    if (!file) return;

    setIsUploading(true);
    setUploadSuccessMsg(null);

    const reader = new FileReader();
    reader.onload = (event) => {
      const result = event.target?.result as string;

      // Create an image to get true dimensions
      const img = new Image();
      img.onload = () => {
        setTimeout(() => {
          const fakeBloggerId = `AVvXsE${Math.random().toString(36).substring(2, 10)}_${file.name.replace(/[^a-zA-Z0-9]/g, '_').substring(0, 20)}`;
          const newOffload: OffloadImage = {
            id: `img-${Date.now()}`,
            filename: file.name,
            mimeType: file.type || 'image/jpeg',
            originalSizeBytes: file.size,
            offloadSizeBytes: Math.round(file.size * 0.06),
            width: img.naturalWidth || 1920,
            height: img.naturalHeight || 1080,
            originWpUrl: `https://vietnamtechreview.com/wp-content/uploads/2026/11/${file.name}`,
            bloggerBaseUrl: `https://blogger.googleusercontent.com/img/a/${fakeBloggerId}`,
            uploadedAt: 'Vừa xong',
            status: 'synced',
            transformedParamString: buildBloggerParamString(params),
            previewUrl: result,
          };

          onAddOffloadedImage(newOffload);
          setSelectedImage(newOffload);
          setIsUploading(false);
          setUploadSuccessMsg(`Upload Blogger thành công! Tạo URL offload: ${newOffload.bloggerBaseUrl}`);
          setTimeout(() => setUploadSuccessMsg(null), 4000);
        }, 800);
      };
      img.src = result;
    };
    reader.readAsDataURL(file);
  };

  // Handle custom URL import
  const handleImportUrl = () => {
    if (!customUrlInput.trim()) return;
    setIsUploading(true);
    setTimeout(() => {
      const filename = customUrlInput.split('/').pop()?.split('?')[0] || 'custom-image.jpg';
      const fakeBloggerId = `AVvXsE_WebUrl_${Math.random().toString(36).substring(2, 8)}`;
      const newOffload: OffloadImage = {
        id: `img-${Date.now()}`,
        filename,
        mimeType: 'image/jpeg',
        originalSizeBytes: 2800000,
        offloadSizeBytes: 140000,
        width: 1920,
        height: 1080,
        originWpUrl: customUrlInput,
        bloggerBaseUrl: `https://blogger.googleusercontent.com/img/a/${fakeBloggerId}`,
        uploadedAt: 'Vừa xong',
        status: 'synced',
        transformedParamString: buildBloggerParamString(params),
        previewUrl: customUrlInput,
      };
      onAddOffloadedImage(newOffload);
      setSelectedImage(newOffload);
      setIsUploading(false);
      setCustomUrlInput('');
      setUploadSuccessMsg(`Đã nạp và tạo URL Blogger CDN thành công!`);
      setTimeout(() => setUploadSuccessMsg(null), 4000);
    }, 600);
  };

  return (
    <div className="space-y-6">
      {/* Header Description */}
      <div className="flex flex-col md:flex-row md:items-center justify-between gap-3 p-4 rounded-xl bg-white dark:bg-slate-900 border border-slate-200 dark:border-slate-800">
        <div>
          <div className="flex items-center gap-2 text-xs text-orange-600 dark:text-orange-400 font-mono font-medium">
            <span>BLOGGER MEDIA OFFLOAD ENGINE</span>
            <span>·</span>
            <span>GOOGLE USERCONTENT PARAMS</span>
          </div>
          <h2 className="text-lg font-bold text-slate-900 dark:text-white mt-0.5">
            Phòng thí nghiệm Upload ảnh Blogger & Tạo URL Offload
          </h2>
          <p className="text-xs text-slate-500 dark:text-slate-400 mt-1 max-w-3xl">
            Tải ảnh lên hạ tầng Blogger Album của Google, tự động sinh tham số tối ưu hóa (crop, resize, format WebP, chất lượng)
            mà không cần re-upload. Thay thế trực tiếp URL gốc trong bài viết WordPress để giảm tải 90%+ máy chủ.
          </p>
        </div>

        <div className="flex items-center gap-2">
          <input
            type="file"
            ref={fileInputRef}
            onChange={handleFileUpload}
            accept="image/*"
            className="hidden"
          />
          <button
            onClick={() => fileInputRef.current?.click()}
            disabled={isUploading}
            className="px-4 py-2 text-xs font-semibold text-white bg-orange-600 hover:bg-orange-500 rounded-lg shadow-sm transition-all flex items-center gap-1.5 whitespace-nowrap"
          >
            <Upload className="w-4 h-4" />
            <span>{isUploading ? 'Đang tải lên...' : 'Tải ảnh mới lên Blogger'}</span>
          </button>
        </div>
      </div>

      {uploadSuccessMsg && (
        <div className="p-3 rounded-lg bg-emerald-50 dark:bg-emerald-950/60 border border-emerald-200 dark:border-emerald-800 text-xs text-emerald-800 dark:text-emerald-300 flex items-center gap-2">
          <CheckCircle2 className="w-4 h-4 text-emerald-600 shrink-0" />
          <span>{uploadSuccessMsg}</span>
        </div>
      )}

      {/* Main 2-Column Workspace */}
      <div className="grid grid-cols-1 lg:grid-cols-12 gap-6">
        {/* Left Column (5 Cols): Parameter Controls & Transformation Settings */}
        <div className="lg:col-span-5 space-y-4">
          <div className="p-5 rounded-xl bg-white dark:bg-slate-900 border border-slate-200 dark:border-slate-800 space-y-5">
            <div className="flex items-center justify-between border-b border-slate-100 dark:border-slate-800 pb-3">
              <div className="flex items-center gap-2">
                <Sliders className="w-4 h-4 text-orange-500" />
                <h3 className="text-sm font-bold text-slate-900 dark:text-white">
                  Bộ điều khiển tham số Blogger CDN
                </h3>
              </div>
              <span className="text-[11px] font-mono px-2 py-0.5 rounded bg-slate-100 dark:bg-slate-800 text-slate-700 dark:text-slate-300">
                {currentSuffix}
              </span>
            </div>

            {/* Mode selection buttons */}
            <div>
              <label className="text-xs font-semibold text-slate-700 dark:text-slate-300 block mb-2">
                Chế độ kích thước (Size Mode)
              </label>
              <div className="grid grid-cols-4 gap-1 p-1 bg-slate-100 dark:bg-slate-800 rounded-lg text-xs">
                {(
                  [
                    { id: 'exact', label: 'Cố định' },
                    { id: 'max_dim', label: 'Kích cỡ max' },
                    { id: 'preset', label: 'Mẫu sẵn' },
                    { id: 'original', label: 'Ảnh gốc' },
                  ] as const
                ).map((mode) => (
                  <button
                    key={mode.id}
                    onClick={() => setParams({ ...params, sizeMode: mode.id })}
                    className={`py-1.5 px-2 text-[11px] font-medium rounded-md transition-colors whitespace-nowrap ${
                      params.sizeMode === mode.id
                        ? 'bg-white dark:bg-slate-900 text-slate-900 dark:text-white shadow-sm font-semibold'
                        : 'text-slate-600 dark:text-slate-400 hover:text-slate-900'
                    }`}
                  >
                    {mode.label}
                  </button>
                ))}
              </div>
            </div>

            {/* Presets if preset mode */}
            {params.sizeMode === 'preset' && (
              <div className="space-y-2">
                <label className="text-xs font-semibold text-slate-700 dark:text-slate-300 block">
                  Chọn kích thước khuôn mẫu
                </label>
                <div className="grid grid-cols-2 gap-2 text-xs">
                  {(
                    [
                      { id: 'thumb', label: 'Thumbnail (150x150)', suffix: '=w150-h150-c-rw' },
                      { id: 'medium', label: 'Nội dung vừa (600x400)', suffix: '=w600-h400-c-rw' },
                      { id: 'large', label: 'Banner lớn (1200x800)', suffix: '=w1200-h800-rw' },
                      { id: 'banner', label: 'Full Header (1920x600)', suffix: '=w1920-h600-c-rw' },
                    ] as const
                  ).map((preset) => (
                    <button
                      key={preset.id}
                      onClick={() => setParams({ ...params, preset: preset.id })}
                      className={`p-2.5 rounded-lg border text-left transition-all ${
                        params.preset === preset.id
                          ? 'border-orange-500 bg-orange-50/50 dark:bg-orange-950/20 text-orange-900 dark:text-orange-200'
                          : 'border-slate-200 dark:border-slate-800 text-slate-600 dark:text-slate-400 hover:border-slate-300'
                      }`}
                    >
                      <div className="font-semibold text-[11px]">{preset.label}</div>
                      <div className="font-mono text-[10px] text-slate-400 mt-0.5">{preset.suffix}</div>
                    </button>
                  ))}
                </div>
              </div>
            )}

            {/* Exact dimensions */}
            {params.sizeMode === 'exact' && (
              <div className="space-y-3">
                <div className="grid grid-cols-2 gap-3">
                  <div>
                    <div className="flex justify-between text-xs text-slate-600 dark:text-slate-400 mb-1">
                      <span>Chiều rộng (w)</span>
                      <span className="font-mono font-bold text-slate-900 dark:text-white">
                        {params.width}px
                      </span>
                    </div>
                    <input
                      type="range"
                      min={100}
                      max={2560}
                      step={20}
                      value={params.width}
                      onChange={(e) => setParams({ ...params, width: Number(e.target.value) })}
                      className="w-full accent-orange-600"
                    />
                  </div>

                  <div>
                    <div className="flex justify-between text-xs text-slate-600 dark:text-slate-400 mb-1">
                      <span>Chiều cao (h)</span>
                      <span className="font-mono font-bold text-slate-900 dark:text-white">
                        {params.height}px
                      </span>
                    </div>
                    <input
                      type="range"
                      min={100}
                      max={1800}
                      step={20}
                      value={params.height}
                      onChange={(e) => setParams({ ...params, height: Number(e.target.value) })}
                      className="w-full accent-orange-600"
                    />
                  </div>
                </div>

                {/* Aspect ratio quick buttons */}
                <div className="flex items-center gap-1.5 pt-1">
                  <span className="text-[11px] text-slate-500">Tỷ lệ:</span>
                  {[
                    { label: '16:9', w: 1200, h: 675 },
                    { label: '4:3', w: 1000, h: 750 },
                    { label: '1:1', w: 800, h: 800 },
                    { label: '21:9', w: 1260, h: 540 },
                  ].map((ratio) => (
                    <button
                      key={ratio.label}
                      onClick={() => setParams({ ...params, width: ratio.w, height: ratio.h })}
                      className="px-2 py-0.5 rounded text-[11px] bg-slate-100 dark:bg-slate-800 text-slate-600 dark:text-slate-300 hover:bg-slate-200"
                    >
                      {ratio.label}
                    </button>
                  ))}
                </div>
              </div>
            )}

            {/* Max dimension slider */}
            {params.sizeMode === 'max_dim' && (
              <div>
                <div className="flex justify-between text-xs text-slate-600 dark:text-slate-400 mb-1">
                  <span>Kích thước cạnh dài tối đa (s)</span>
                  <span className="font-mono font-bold text-slate-900 dark:text-white">
                    {params.maxDimension}px
                  </span>
                </div>
                <input
                  type="range"
                  min={200}
                  max={2560}
                  step={50}
                  value={params.maxDimension}
                  onChange={(e) => setParams({ ...params, maxDimension: Number(e.target.value) })}
                  className="w-full accent-orange-600"
                />
                <div className="flex justify-between text-[10px] text-slate-400 font-mono mt-1">
                  <span>s200</span>
                  <span>s800</span>
                  <span>s1200</span>
                  <span>s1600 (Chuẩn Blogger)</span>
                  <span>s2560</span>
                </div>
              </div>
            )}

            {/* Quality Slider */}
            <div>
              <div className="flex justify-between text-xs text-slate-600 dark:text-slate-400 mb-1">
                <span>Chất lượng nén ảnh (-q)</span>
                <span className="font-mono font-bold text-slate-900 dark:text-white">
                  q{params.quality}
                </span>
              </div>
              <input
                type="range"
                min={40}
                max={100}
                step={5}
                value={params.quality}
                onChange={(e) => setParams({ ...params, quality: Number(e.target.value) })}
                className="w-full accent-orange-600"
              />
              <span className="text-[10px] text-slate-400 block mt-0.5">
                Khuyên dùng: q85 (Tối ưu tuyệt hảo giữa mắt nhìn và dung lượng file)
              </span>
            </div>

            {/* Parameter Toggles */}
            <div className="space-y-2.5 pt-2 border-t border-slate-100 dark:border-slate-800">
              {/* WebP */}
              <label className="flex items-center justify-between text-xs cursor-pointer p-2 rounded-lg hover:bg-slate-50 dark:hover:bg-slate-800/50 transition-colors">
                <div>
                  <span className="font-semibold text-slate-800 dark:text-slate-200 block">
                    Tự động chuyển đổi định dạng WebP (-rw)
                  </span>
                  <span className="text-[11px] text-slate-500 dark:text-slate-400">
                    Bật tham số `-rw` để Google CDN phục vụ ảnh WebP siêu nhẹ cho trình duyệt hỗ trợ.
                  </span>
                </div>
                <input
                  type="checkbox"
                  checked={params.webpFormat}
                  onChange={(e) => setParams({ ...params, webpFormat: e.target.checked })}
                  className="w-4 h-4 accent-orange-600 rounded"
                />
              </label>

              {/* Smart Crop */}
              <label className="flex items-center justify-between text-xs cursor-pointer p-2 rounded-lg hover:bg-slate-50 dark:hover:bg-slate-800/50 transition-colors">
                <div>
                  <span className="font-semibold text-slate-800 dark:text-slate-200 block">
                    Cắt ảnh thông minh Smart Crop (-c)
                  </span>
                  <span className="text-[11px] text-slate-500 dark:text-slate-400">
                    Tự động nhận diện chủ thể và căn giữa theo tỷ lệ đã đặt.
                  </span>
                </div>
                <input
                  type="checkbox"
                  checked={params.smartCrop}
                  onChange={(e) => setParams({ ...params, smartCrop: e.target.checked })}
                  className="w-4 h-4 accent-orange-600 rounded"
                />
              </label>

              {/* Lossless */}
              <label className="flex items-center justify-between text-xs cursor-pointer p-2 rounded-lg hover:bg-slate-50 dark:hover:bg-slate-800/50 transition-colors">
                <div>
                  <span className="font-semibold text-slate-800 dark:text-slate-200 block">
                    Bảo toàn chi tiết Lossless (-l)
                  </span>
                  <span className="text-[11px] text-slate-500 dark:text-slate-400">
                    Giữ độ sắc nét pixel cao nhất cho biểu đồ, logo và văn bản.
                  </span>
                </div>
                <input
                  type="checkbox"
                  checked={params.lossless}
                  onChange={(e) => setParams({ ...params, lossless: e.target.checked })}
                  className="w-4 h-4 accent-orange-600 rounded"
                />
              </label>
            </div>
          </div>

          {/* Quick Import URL Box */}
          <div className="p-4 rounded-xl bg-white dark:bg-slate-900 border border-slate-200 dark:border-slate-800 space-y-2">
            <label className="text-xs font-semibold text-slate-700 dark:text-slate-300 flex items-center gap-1.5">
              <Link className="w-3.5 h-3.5 text-orange-500" />
              <span>Nhập URL ảnh từ WordPress để chuyển đổi sang Blogger</span>
            </label>
            <div className="flex gap-2">
              <input
                type="text"
                placeholder="https://mysite.com/wp-content/uploads/2026/..."
                value={customUrlInput}
                onChange={(e) => setCustomUrlInput(e.target.value)}
                className="flex-1 px-3 py-1.5 text-xs rounded-lg border border-slate-200 dark:border-slate-700 bg-slate-50 dark:bg-slate-800 text-slate-900 dark:text-white font-mono focus:outline-none focus:ring-1 focus:ring-orange-500"
              />
              <button
                onClick={handleImportUrl}
                disabled={!customUrlInput.trim()}
                className="px-3 py-1.5 text-xs font-medium text-white bg-slate-900 hover:bg-slate-800 dark:bg-slate-100 dark:text-slate-900 rounded-lg disabled:opacity-40 transition-colors shrink-0"
              >
                Chuyển đổi
              </button>
            </div>
          </div>
        </div>

        {/* Right Column (7 Cols): Live Visual Preview & Code Export */}
        <div className="lg:col-span-7 space-y-4">
          {/* Visual Image Preview & Comparison Card */}
          <div className="p-5 rounded-xl bg-white dark:bg-slate-900 border border-slate-200 dark:border-slate-800 space-y-4">
            <div className="flex items-center justify-between">
              <div>
                <h3 className="text-sm font-bold text-slate-900 dark:text-white flex items-center gap-2">
                  <ImageIcon className="w-4 h-4 text-orange-500" />
                  <span>Xem trước ảnh và Hiệu quả nén</span>
                </h3>
                <span className="text-xs text-slate-500 dark:text-slate-400 font-mono">
                  {selectedImage.filename} ({selectedImage.width}x{selectedImage.height})
                </span>
              </div>

              {/* Savings badge */}
              <div className="text-right">
                <div className="text-xs font-bold text-emerald-600 dark:text-emerald-400 font-mono">
                  Tiết kiệm: -{savingsPct}%
                </div>
                <div className="text-[11px] text-slate-400">
                  {formatBytes(selectedImage.originalSizeBytes)} → ~{formatBytes(estimatedOffloadBytes)}
                </div>
              </div>
            </div>

            {/* Visual Image Preview Frame */}
            <div className="relative w-full h-72 rounded-xl bg-slate-950 overflow-hidden border border-slate-800 flex items-center justify-center group">
              <img
                src={selectedImage.previewUrl}
                alt={selectedImage.filename}
                referrerPolicy="no-referrer"
                className="max-h-full max-w-full object-contain transition-all duration-300"
                style={{
                  objectFit: params.smartCrop ? 'cover' : 'contain',
                }}
              />

              {/* Overlay parameter pill */}
              <div className="absolute bottom-3 left-3 px-2.5 py-1 rounded bg-black/75 backdrop-blur-md text-[11px] font-mono text-white border border-white/10 flex items-center gap-2">
                <span className="w-2 h-2 rounded-full bg-emerald-400" />
                <span>Blogger CDN: {currentSuffix}</span>
              </div>

              {/* Compare indicator */}
              <div className="absolute top-3 right-3 px-2.5 py-1 rounded bg-black/75 backdrop-blur-md text-[11px] font-mono text-white border border-white/10">
                WebP Google Cache Hit
              </div>
            </div>

            {/* Side by side comparison stats */}
            <div className="grid grid-cols-2 gap-3 text-xs">
              <div className="p-3 rounded-lg bg-slate-50 dark:bg-slate-800/40 border border-slate-200 dark:border-slate-800">
                <span className="text-slate-500 dark:text-slate-400 block font-medium mb-1">
                  Máy chủ WordPress gốc (Origin Host)
                </span>
                <div className="space-y-1 font-mono text-[11px]">
                  <div className="flex justify-between">
                    <span className="text-slate-400">Dung lượng:</span>
                    <span className="text-red-500 font-bold">{formatBytes(selectedImage.originalSizeBytes)}</span>
                  </div>
                  <div className="flex justify-between">
                    <span className="text-slate-400">Độ trễ TTFB:</span>
                    <span className="text-slate-700 dark:text-slate-300">~340 ms</span>
                  </div>
                  <div className="flex justify-between">
                    <span className="text-slate-400">Định dạng:</span>
                    <span className="text-slate-700 dark:text-slate-300">{selectedImage.mimeType}</span>
                  </div>
                </div>
              </div>

              <div className="p-3 rounded-lg bg-orange-50/50 dark:bg-orange-950/20 border border-orange-200 dark:border-orange-900/50">
                <span className="text-orange-900 dark:text-orange-300 block font-semibold mb-1">
                  Blogger Offload CDN (Google Edge)
                </span>
                <div className="space-y-1 font-mono text-[11px]">
                  <div className="flex justify-between">
                    <span className="text-orange-700 dark:text-orange-400">Dung lượng:</span>
                    <span className="text-emerald-600 dark:text-emerald-400 font-bold">~{formatBytes(estimatedOffloadBytes)}</span>
                  </div>
                  <div className="flex justify-between">
                    <span className="text-orange-700 dark:text-orange-400">Độ trễ TTFB:</span>
                    <span className="text-emerald-600 dark:text-emerald-400 font-bold">~38 ms (HTTP/3)</span>
                  </div>
                  <div className="flex justify-between">
                    <span className="text-orange-700 dark:text-orange-400">Định dạng:</span>
                    <span className="text-emerald-600 dark:text-emerald-400">image/webp</span>
                  </div>
                </div>
              </div>
            </div>
          </div>

          {/* Generated Code & Integration Snippets */}
          <div className="p-5 rounded-xl bg-white dark:bg-slate-900 border border-slate-200 dark:border-slate-800 space-y-3">
            <div className="flex items-center justify-between">
              <div className="flex items-center gap-2">
                <Code2 className="w-4 h-4 text-orange-500" />
                <h3 className="text-sm font-bold text-slate-900 dark:text-white">
                  Mã nhúng và URL Offload đã sinh
                </h3>
              </div>

              {/* Code Tab Switcher */}
              <div className="flex items-center gap-1 p-0.5 bg-slate-100 dark:bg-slate-800 rounded-md text-xs">
                {(['url', 'html', 'markdown', 'php'] as const).map((tab) => (
                  <button
                    key={tab}
                    onClick={() => setActiveCodeTab(tab)}
                    className={`px-2.5 py-1 rounded text-[11px] font-mono uppercase font-semibold transition-colors ${
                      activeCodeTab === tab
                        ? 'bg-white dark:bg-slate-900 text-orange-600 dark:text-orange-400 shadow-sm'
                        : 'text-slate-600 dark:text-slate-400 hover:text-slate-900'
                    }`}
                  >
                    {tab}
                  </button>
                ))}
              </div>
            </div>

            {/* Code Box */}
            <div className="relative rounded-lg bg-slate-950 p-3.5 border border-slate-800 font-mono text-xs text-slate-200 overflow-x-auto">
              <button
                onClick={() => {
                  let copyText = liveTransformedUrl;
                  if (activeCodeTab === 'html') {
                    copyText = generateHtmlSnippet(liveTransformedUrl, selectedImage.filename, params.width, params.height);
                  } else if (activeCodeTab === 'markdown') {
                    copyText = `![${selectedImage.filename}](${liveTransformedUrl})`;
                  } else if (activeCodeTab === 'php') {
                    copyText = `add_filter('the_content', function($content) {\n  return preg_replace('/${selectedImage.originWpUrl.replace(/[.*+?^${}()|[\]\\]/g, '\\$&')}/i', '${liveTransformedUrl}', $content);\n});`;
                  }
                  handleCopy(copyText, activeCodeTab);
                }}
                className="absolute top-2.5 right-2.5 p-1.5 rounded bg-slate-800 hover:bg-slate-700 text-slate-300 transition-colors flex items-center gap-1 text-[11px]"
                title="Sao chép vào bộ nhớ tạm"
              >
                {copiedType === activeCodeTab ? (
                  <>
                    <Check className="w-3.5 h-3.5 text-emerald-400" />
                    <span className="text-emerald-400">Đã chép!</span>
                  </>
                ) : (
                  <>
                    <Copy className="w-3.5 h-3.5" />
                    <span>Sao chép</span>
                  </>
                )}
              </button>

              <pre className="pr-20 text-[11px] leading-relaxed whitespace-pre-wrap break-all">
                {activeCodeTab === 'url' && liveTransformedUrl}
                {activeCodeTab === 'html' &&
                  generateHtmlSnippet(liveTransformedUrl, selectedImage.filename, params.width, params.height)}
                {activeCodeTab === 'markdown' && `![${selectedImage.filename}](${liveTransformedUrl})`}
                {activeCodeTab === 'php' &&
                  `add_filter('the_content', function($content) {\n  return preg_replace('/${selectedImage.originWpUrl.replace(/[.*+?^${}()|[\]\\]/g, '\\$&')}/i', '${liveTransformedUrl}', $content);\n});`}
              </pre>
            </div>

            <div className="flex items-center justify-between text-xs text-slate-500 dark:text-slate-400">
              <span>Được xử lý bởi Rust `media_processor.rs` và Python Pydantic bridge</span>
              <a
                href={selectedImage.previewUrl}
                target="_blank"
                rel="noreferrer"
                className="text-orange-600 dark:text-orange-400 hover:underline flex items-center gap-1 font-mono text-[11px]"
              >
                <span>Mở ảnh gốc</span>
                <ExternalLink className="w-3 h-3" />
              </a>
            </div>
          </div>
        </div>
      </div>

      {/* Gallery of Preloaded / Offloaded Images Strip */}
      <div className="p-5 rounded-xl bg-white dark:bg-slate-900 border border-slate-200 dark:border-slate-800 space-y-3">
        <div className="flex items-center justify-between">
          <h3 className="text-sm font-bold text-slate-900 dark:text-white">
            Danh sách ảnh đã offload trong thư viện ({images.length} tệp)
          </h3>
          <span className="text-xs text-slate-500 font-mono">Nhấn vào ảnh để nạp vào bộ biến đổi</span>
        </div>

        <div className="grid grid-cols-2 sm:grid-cols-3 md:grid-cols-5 gap-3">
          {images.map((img) => {
            const isSelected = selectedImage.id === img.id;
            return (
              <div
                key={img.id}
                onClick={() => setSelectedImage(img)}
                className={`group cursor-pointer rounded-lg border p-2 text-left transition-all relative overflow-hidden ${
                  isSelected
                    ? 'border-orange-500 bg-orange-50/30 dark:bg-orange-950/20 ring-1 ring-orange-500'
                    : 'border-slate-200 dark:border-slate-800 hover:border-slate-300 dark:hover:border-slate-700'
                }`}
              >
                <div className="w-full h-24 rounded bg-slate-900 overflow-hidden mb-2">
                  <img
                    src={img.previewUrl}
                    alt={img.filename}
                    referrerPolicy="no-referrer"
                    className="w-full h-full object-cover group-hover:scale-105 transition-transform"
                  />
                </div>
                <div className="text-xs font-semibold text-slate-800 dark:text-slate-200 truncate">
                  {img.filename}
                </div>
                <div className="text-[10px] text-slate-400 font-mono flex justify-between mt-1">
                  <span>{formatBytes(img.originalSizeBytes)}</span>
                  <span className="text-emerald-500 font-semibold">WebP</span>
                </div>
              </div>
            );
          })}
        </div>
      </div>
    </div>
  );
};
