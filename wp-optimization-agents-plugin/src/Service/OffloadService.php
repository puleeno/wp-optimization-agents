<?php

namespace Puleeno\WPOptAgents\Service;

use Puleeno\WPOptAgents\Contracts\ImageRepositoryInterface;
use Puleeno\WPOptAgents\Contracts\OffloadServiceInterface;
use Puleeno\WPOptAgents\Contracts\SettingsRepositoryInterface;

class OffloadService implements OffloadServiceInterface
{
    private ImageRepositoryInterface $imageRepository;
    private SettingsRepositoryInterface $settingsRepository;

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

        $content = $post->post_content;
        $images = [];
        $pattern = '/<img[^>]+src=["\']([^"\']+)["\'][^>]*>/i';

        preg_match_all($pattern, $content, $matches, PREG_SET_ORDER);

        foreach ($matches as $index => $match) {
            $src = $match[1];
            if ($this->is_local_wp_image($src)) {
                $images[] = [
                    'index' => $index,
                    'origin_url' => $src,
                    'full_tag' => $match[0],
                ];
            }
        }

        return $images;
    }

    public function register_images_for_offload(int $post_id, array $images): array
    {
        $registered = [];
        foreach ($images as $image) {
            $existing = $this->imageRepository->find_by_origin_url($image['origin_url']);
            if ($existing) {
                $registered[] = $existing;
                continue;
            }

            $id = $this->imageRepository->create([
                'post_id' => $post_id,
                'origin_url' => $image['origin_url'],
                'status' => 'pending',
            ]);

            if ($id) {
                $registered[] = $this->imageRepository->find_by_id($id);
            }
        }

        return $registered;
    }

    public function mark_image_synced(int $image_id, string $blogger_url, string $blogger_image_id, string $transformed_params, array $meta): bool
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
            'status' => 'synced',
            'error_message' => null,
        ]);
    }

    public function mark_image_failed(int $image_id, string $error): bool
    {
        return $this->imageRepository->update_status($image_id, 'failed', $error);
    }

    public function get_post_offload_status(int $post_id): array
    {
        $images = $this->imageRepository->find_by_post_id($post_id);
        $total = count($images);
        $synced = 0;
        $pending = 0;
        $failed = 0;

        foreach ($images as $img) {
            switch ($img['status']) {
                case 'synced':
                    $synced++;
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
            'synced_images' => $synced,
            'pending_images' => $pending,
            'failed_images' => $failed,
            'images' => $images,
        ];
    }

    public function replace_post_content(int $post_id, array $replacements): bool
    {
        $settings = $this->settingsRepository->get_all();
        $backup_enabled = $settings['backup_before_rewrite'] ?? true;

        if ($backup_enabled) {
            $this->backup_post_content($post_id);
        }

        $post = get_post($post_id);
        if (!$post) {
            return false;
        }

        $content = $post->post_content;
        foreach ($replacements as $replacement) {
            $content = str_replace($replacement['origin_url'], $replacement['blogger_url'], $content);
        }

        $result = wp_update_post([
            'ID' => $post_id,
            'post_content' => $content,
        ]);

        return (bool)$result;
    }

    public function backup_post_content(int $post_id): string
    {
        $post = get_post($post_id);
        if (!$post) {
            return '';
        }

        $backup_key = "wp_opt_agents_backup_{$post_id}";
        update_post_meta($post_id, $backup_key, [
            'content' => $post->post_content,
            'timestamp' => current_time('mysql'),
        ]);

        return $post->post_content;
    }

    private function is_local_wp_image(string $url): bool
    {
        $upload_dir = wp_upload_dir();
        $base_url = $upload_dir['baseurl'];
        return str_starts_with($url, $base_url);
    }
}