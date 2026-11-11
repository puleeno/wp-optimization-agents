<?php

namespace Puleeno\WPOptAgents\Contracts;

interface OffloadServiceInterface
{
    public function scan_post_images(int $post_id): array;
    public function register_images_for_offload(int $post_id, array $images): array;
    public function mark_image_synced(int $image_id, string $blogger_url, string $blogger_image_id, string $transformed_params, array $meta): bool;
    public function mark_image_failed(int $image_id, string $error): bool;
    public function get_post_offload_status(int $post_id): array;
    public function replace_post_content(int $post_id, array $replacements): bool;
    public function backup_post_content(int $post_id): string;
}