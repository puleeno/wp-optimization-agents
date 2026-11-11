import { BloggerTransformParams } from '../types/dashboard';

/**
 * Builds standard Google/Blogger CDN parameter suffix
 * E.g. =w800-h600-c-rw
 */
export function buildBloggerParamString(params: BloggerTransformParams): string {
  if (params.sizeMode === 'original') {
    return params.webpFormat ? '=s0-rw' : '=s0';
  }

  if (params.sizeMode === 'max_dim') {
    let suffix = `=s${params.maxDimension}`;
    if (params.smartCrop) suffix += '-c';
    if (params.webpFormat) suffix += '-rw';
    if (params.lossless) suffix += '-l';
    if (params.quality && params.quality !== 85) suffix += `-q${params.quality}`;
    return suffix;
  }

  if (params.sizeMode === 'preset') {
    switch (params.preset) {
      case 'thumb':
        return '=w150-h150-c-rw';
      case 'medium':
        return '=w600-h400-c-rw';
      case 'large':
        return '=w1200-h800-rw';
      case 'banner':
        return '=w1920-h600-c-rw';
      case 'avatar':
        return '=w96-h96-c-rw';
    }
  }

  // Exact mode
  const parts: string[] = [];
  if (params.width > 0) parts.push(`w${params.width}`);
  if (params.height > 0) parts.push(`h${params.height}`);
  if (params.smartCrop) parts.push('c');
  if (params.webpFormat) parts.push('rw');
  if (params.lossless) parts.push('l');
  if (params.quality && params.quality !== 85) parts.push(`q${params.quality}`);

  return parts.length > 0 ? `=${parts.join('-')}` : '=s0';
}

/**
 * Attaches the transform string to a Blogger base URL
 */
export function applyBloggerTransform(baseUrl: string, params: BloggerTransformParams): string {
  // Strip existing parameter if present (e.g. '=w...' or '=s...')
  const cleanBase = baseUrl.split('=')[0];
  const paramStr = buildBloggerParamString(params);
  return `${cleanBase}${paramStr}`;
}

export function formatBytes(bytes: number, decimals = 1): string {
  if (bytes === 0) return '0 B';
  const k = 1024;
  const dm = decimals < 0 ? 0 : decimals;
  const sizes = ['B', 'KB', 'MB', 'GB', 'TB'];
  const i = Math.floor(Math.log(bytes) / Math.log(k));
  return `${parseFloat((bytes / Math.pow(k, i)).toFixed(dm))} ${sizes[i]}`;
}

export function generateHtmlSnippet(transformedUrl: string, altText: string, width = 800, height = 500): string {
  const cleanBase = transformedUrl.split('=')[0];
  return `<picture>
  <source 
    type="image/webp" 
    srcset="${cleanBase}=w${width}-rw 1x, ${cleanBase}=w${width * 2}-rw 2x" 
  />
  <img 
    src="${transformedUrl}" 
    alt="${altText}" 
    loading="lazy" 
    decoding="async" 
    width="${width}" 
    height="${height}" 
    class="wp-offload-image"
  />
</picture>`;
}
