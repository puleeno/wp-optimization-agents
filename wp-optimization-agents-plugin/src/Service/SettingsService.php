<?php

namespace Puleeno\WPOptAgents\Service;

use Puleeno\WPOptAgents\Contracts\SettingsRepositoryInterface;

class SettingsService
{
    /** @var SettingsRepositoryInterface */
    private $repository;

    public function __construct(SettingsRepositoryInterface $repository)
    {
        $this->repository = $repository;
    }

    public function all(): array
    {
        return array_merge($this->get_defaults(), $this->repository->get_all());
    }

    public function get(string $key, $default = null)
    {
        return $this->repository->get($key, $default);
    }

    public function set(string $key, $value): bool
    {
        return $this->repository->set($key, $value);
    }

    public function save(array $settings): void
    {
        $defaults = $this->get_defaults();
        $current = $this->repository->get_all();
        $merged = array_merge($current, $settings);
        $clean = [];
        foreach ($defaults as $key => $default) {
            $clean[$key] = array_key_exists($key, $merged) ? $merged[$key] : $default;
        }
        $this->repository->set_all($clean);
    }

    public function ensureDefaults(): void
    {
        $defaults = $this->get_defaults();
        $current = $this->repository->get_all();
        $merged = array_merge($defaults, $current);
        $clean = [];
        foreach ($defaults as $key => $default) {
            $clean[$key] = array_key_exists($key, $merged) ? $merged[$key] : $default;
        }
        $this->repository->set_all($clean);
    }

    public function get_defaults(): array
    {
        return [
            'blogger_blog_id' => '',
            'blogger_album_name' => 'WP Offloaded Images',
            'chrome_profile' => 'default',
            'chrome_executable' => '',
            'chrome_user_data_dir' => '',
            'chrome_headless' => false,
            'enable_webp_auto' => true,
            'default_quality' => 85,
            'media_sync_chunk_size' => 20,
            'replace_post_content' => true,
            'backup_before_rewrite' => true,
            'enable_old_url_redirect_301' => true,
            'daemon_port' => 2095,
            'daemon_log_level' => 'info',
            'daemon_worker_threads' => 4,
            'daemon_poll_interval_sec' => 60,
            'daemon_auto_retry_limit' => 3,
            'daemon_api_token' => '',
        ];
    }

    /**
     * Config exposed to Rust agent via compatibility endpoint.
     * Agent reads WP DB directly for core operations; this is for runtime hints/auth.
     */
    public function get_rest_api_config(): array
    {
        $settings = $this->all();
        global $wpdb;
        return [
            'api_url' => rest_url('wp-opt-agents/v1'),
            'site_url' => home_url(),
            'db' => [
                'table_prefix' => $wpdb->prefix,
                'images_table' => $wpdb->prefix . 'wp_opt_agents_images',
                'jobs_table' => $wpdb->prefix . 'wp_opt_agents_jobs',
            ],
            'blogger' => [
                'blog_id' => $settings['blogger_blog_id'],
                'album_name' => $settings['blogger_album_name'],
                'upload_via' => 'chrome_web_ui',
            ],
            'chrome' => [
                'profile' => $settings['chrome_profile'],
                'executable' => $settings['chrome_executable'],
                'user_data_dir' => $settings['chrome_user_data_dir'],
                'headless' => (bool)$settings['chrome_headless'],
            ],
            'media' => [
                'enable_webp_auto' => (bool)$settings['enable_webp_auto'],
                'default_quality' => (int)$settings['default_quality'],
                'sync_chunk_size' => (int)$settings['media_sync_chunk_size'],
                'replace_post_content' => (bool)$settings['replace_post_content'],
                'backup_before_rewrite' => (bool)$settings['backup_before_rewrite'],
                'enable_old_url_redirect_301' => (bool)$settings['enable_old_url_redirect_301'],
            ],
            'daemon' => [
                'port' => (int)$settings['daemon_port'],
                'poll_interval_sec' => (int)$settings['daemon_poll_interval_sec'],
                'auto_retry_limit' => (int)$settings['daemon_auto_retry_limit'],
            ],
        ];
    }
}
