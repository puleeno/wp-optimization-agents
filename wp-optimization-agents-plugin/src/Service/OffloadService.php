<?php

namespace Puleeno\WPOptAgents\Service;

use Puleeno\WPOptAgents\Contracts\ImageRepositoryInterface;
use Puleeno\WPOptAgents\Contracts\OffloadServiceInterface;
use Puleeno\WPOptAgents\Contracts\SettingsRepositoryInterface;
use Puleeno\WPOptAgents\Repository\ImageRepository;

class OffloadService implements OffloadServiceInterface
{
    /** @var ImageRepositoryInterface */
    private $imageRepository;
    /** @var SettingsRepositoryInterface */
    private $settingsRepository;

    public function __construct(
        ImageRepositoryInterface $imageRepository,
        SettingsRepositoryInterface $settingsRepository
    ) {
        $this->imageRepository = $imageRepository;
        $this->settingsRepository = $settingsRepository;
    }

    public function scan_post_images(int $post_id): array
    {
        $post = get_post($post_id);
        if (!$post) {
            return [];
        }

        $content = (string)$post->post_content;
        $images = [];
        $pattern = '/<img[^>]+src=["\']([^"\']+)["\'][^>]*>/i';

        preg_match_all($pattern, $content, $matches, PREG_SET_ORDER);

        foreach ($matches as $index => $match) {
            $src = $match[1];
            if (!$this->is_local_wp_image($src)) {
                continue;
            }
            $attachment_id = attachment_url_to_postid($src);
            $images[] = [
                'index' => $index,
                'origin_url' => $src,
                'attachment_id' => $attachment_id ?: null,
                'full_tag' => $match[0],
            ];
        }

        return $images;
    }

    public function register_images_for_offload(int $post_id, array $images): array
    {
        $registered = [];
        foreach ($images as $image) {
            $origin = (string)($image['origin_url'] ?? '');
            if ($origin === '') {
                continue;
            }

            $existing = $this->imageRepository->find_by_origin_url($origin);
            if ($existing) {
                // Ensure post/attachment linkage is up to date
                $patch = [];
                if (!empty($post_id) && (int)($existing['post_id'] ?? 0) !== $post_id) {
                    $patch['post_id'] = $post_id;
                }
                if (!empty($image['attachment_id']) && empty($existing['attachment_id'])) {
                    $patch['attachment_id'] = (int)$image['attachment_id'];
                }
                if (!empty($patch)) {
                    $this->imageRepository->update((int)$existing['id'], $patch);
                    $existing = $this->imageRepository->find_by_id((int)$existing['id']) ?: $existing;
                }
                $registered[] = $existing;
                continue;
            }

            $meta = $this->resolve_local_image_meta($origin, $image);
            $id = $this->imageRepository->create([
                'post_id' => $post_id ?: null,
                'attachment_id' => !empty($image['attachment_id']) ? (int)$image['attachment_id'] : ($meta['attachment_id'] ?? null),
                'origin_url' => $origin,
                'origin_url_hash' => ImageRepository::hash_origin_url($origin),
                'width' => $meta['width'] ?? null,
                'height' => $meta['height'] ?? null,
                'mime_type' => $meta['mime_type'] ?? null,
                'original_size_bytes' => $meta['original_size_bytes'] ?? null,
                'status' => 'pending',
            ]);

            if ($id) {
                $registered[] = $this->imageRepository->find_by_id($id);
            }
        }

        return array_values(array_filter($registered));
    }

    public function mark_image_uploaded(int $image_id, string $blogger_url, string $blogger_image_id, string $transformed_params, array $meta): bool
    {
        return $this->imageRepository->update($image_id, [
            'blogger_url' => $blogger_url,
            'blogger_image_id' => $blogger_image_id,
            'transformed_params' => $transformed_params,
            'width' => $meta['width'] ?? null,
            'height' => $meta['height'] ?? null,
            'mime_type' => $meta['mime_type'] ?? null,
            'original_size_bytes' => $meta['original_size_bytes'] ?? null,
            'offload_size_bytes' => $meta['offload_size_bytes'] ?? null,
            'status' => 'uploaded',
            'error_message' => null,
        ]);
    }

    public function mark_image_replaced(int $image_id): bool
    {
        return $this->imageRepository->mark_replaced($image_id, true);
    }

    public function mark_image_failed(int $image_id, string $error): bool
    {
        return $this->imageRepository->update_status($image_id, 'failed', $error);
    }

    public function get_post_offload_status(int $post_id): array
    {
        $images = $this->imageRepository->find_by_post_id($post_id);
        $total = count($images);
        $replaced = 0;
        $uploaded = 0;
        $pending = 0;
        $failed = 0;

        foreach ($images as $img) {
            switch ($img['status']) {
                case 'replaced':
                    $replaced++;
                    break;
                case 'uploaded':
                    $uploaded++;
                    break;
                case 'failed':
                    $failed++;
                    break;
                default:
                    $pending++;
            }
        }

        return [
            'post_id' => $post_id,
            'total_images' => $total,
            'replaced_images' => $replaced,
            'uploaded_images' => $uploaded,
            'pending_images' => $pending,
            'failed_images' => $failed,
            'images' => $images,
        ];
    }

    public function replace_post_content(int $post_id, array $replacements): array
    {
        $settings = $this->settingsRepository->get_all();
        $backup_enabled = !empty($settings['backup_before_rewrite']);

        $post = get_post($post_id);
        if (!$post) {
            return ['success' => false, 'error' => 'post_not_found'];
        }

        $content = (string)$post->post_content;
        $backup = null;
        if ($backup_enabled) {
            $backup = $this->create_post_backup($post_id, $content);
        }

        $applied = 0;
        foreach ($replacements as $replacement) {
            $origin = (string)($replacement['origin_url'] ?? '');
            $target = (string)($replacement['blogger_url'] ?? '');
            if ($origin === '' || $target === '') {
                continue;
            }
            if (strpos($content, $origin) === false) {
                continue;
            }
            $content = str_replace($origin, $target, $content);
            $applied++;
        }

        $result = wp_update_post([
            'ID' => $post_id,
            'post_content' => $content,
        ], true);

        if (is_wp_error($result)) {
            return ['success' => false, 'error' => $result->get_error_message(), 'backup' => $backup];
        }

        // Mark replaced + activate 301 redirects for mapped images in this post
        $images = $this->imageRepository->find_by_post_id($post_id);
        foreach ($images as $img) {
            if (($img['status'] ?? '') === 'uploaded' || (($img['status'] ?? '') === 'replaced')) {
                if (!empty($img['blogger_url'])) {
                    $this->imageRepository->mark_replaced((int)$img['id'], true);
                }
            }
        }

        return ['success' => true, 'applied' => $applied, 'backup' => $backup];
    }

    public function get_redirect_target(string $origin_url): ?string
    {
        $row = $this->imageRepository->find_active_redirect_by_origin_url($origin_url);
        if (!$row || empty($row['blogger_url'])) {
            return null;
        }
        return (string)$row['blogger_url'];
    }

    public function maybe_redirect_old_url(): void
    {
        $settings = $this->settingsRepository->get_all();
        if (isset($settings['enable_old_url_redirect_301']) && !$settings['enable_old_url_redirect_301']) {
            return;
        }
        if (is_admin() || (defined('REST_REQUEST') && REST_REQUEST) || wp_doing_ajax()) {
            return;
        }

        $request_uri = $_SERVER['REQUEST_URI'] ?? '';
        if (!is_string($request_uri) || $request_uri === '') {
            return;
        }

        $current = home_url($request_uri);
        $target = $this->get_redirect_target($current);
        if (!$target) {
            // try without query normalization differences
            $path_only = strtok($current, '?') ?: $current;
            $target = $this->get_redirect_target($path_only);
        }
        if (!$target) {
            return;
        }

        wp_redirect($target, 301);
        exit;
    }

    private function create_post_backup(int $post_id, string $content): array
    {
        $backup = [
            'post_id' => $post_id,
            'content' => $content,
            'timestamp' => current_time('mysql'),
        ];
        $key = 'wp_opt_agents_backup_' . $post_id;
        update_post_meta($post_id, $key, $backup);
        return ['meta_key' => $key, 'timestamp' => $backup['timestamp']];
    }

    private function is_local_wp_image(string $url): bool
    {
        $upload_dir = wp_upload_dir();
        $base_url = $upload_dir['baseurl'] ?? '';
        if ($base_url && strpos($url, $base_url) === 0) {
            return true;
        }
        $home = home_url('/');
        if ($home && strpos($url, $home) === 0 && strpos($url, '/wp-content/uploads/') !== false) {
            return true;
        }
        return false;
    }

    public function link_blogger_post(int $post_id, array $data): bool
    {
        if ($post_id <= 0) {
            return false;
        }
        $map = [
            'blogger_post_id' => '_wpoa_blogger_post_id',
            'blogger_post_url' => '_wpoa_blogger_post_url',
            'blogger_album_url' => '_wpoa_blogger_album_url',
            'blogger_blog_id' => '_wpoa_blogger_blog_id',
        ];
        $ok = true;
        foreach ($map as $key => $meta_key) {
            if (!array_key_exists($key, $data)) {
                continue;
            }
            $val = is_scalar($data[$key]) ? (string)$data[$key] : '';
            if ($val === '') {
                continue;
            }
            $ok = (bool)update_post_meta($post_id, $meta_key, sanitize_text_field($val)) || $ok;
        }
        update_post_meta($post_id, '_wpoa_blogger_linked_at', current_time('mysql'));
        return $ok;
    }

    public function get_blogger_post_link(int $post_id): array
    {
        return [
            'post_id' => $post_id,
            'blogger_post_id' => (string)get_post_meta($post_id, '_wpoa_blogger_post_id', true),
            'blogger_post_url' => (string)get_post_meta($post_id, '_wpoa_blogger_post_url', true),
            'blogger_album_url' => (string)get_post_meta($post_id, '_wpoa_blogger_album_url', true),
            'blogger_blog_id' => (string)get_post_meta($post_id, '_wpoa_blogger_blog_id', true),
            'linked_at' => (string)get_post_meta($post_id, '_wpoa_blogger_linked_at', true),
        ];
    }

    private function resolve_local_image_meta(string $origin_url, array $image): array
    {
        $attachment_id = !empty($image['attachment_id']) ? (int)$image['attachment_id'] : attachment_url_to_postid($origin_url);
        $meta = ['attachment_id' => $attachment_id ?: null];

        if ($attachment_id) {
            $file = get_attached_file($attachment_id);
            if ($file && file_exists($file)) {
                $meta['original_size_bytes'] = filesize($file) ?: null;
                $info = @getimagesize($file);
                if (is_array($info)) {
                    $meta['width'] = $info[0] ?? null;
                    $meta['height'] = $info[1] ?? null;
                    $meta['mime_type'] = $info['mime'] ?? null;
                }
            }
            if (empty($meta['mime_type'])) {
                $meta['mime_type'] = get_post_mime_type($attachment_id) ?: null;
            }
        }

        return $meta;
    }
}