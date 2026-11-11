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
        $current = $this->repository->get_all();
        $merged = array_merge($current, $settings);
        $this->repository->set_all($merged);
    }

    public function ensureDefaults(): void
    {
        $defaults = $this->get_defaults();
        $current = $this->repository->get_all();
        $merged = array_merge($defaults, $current);
        $this->repository->set_all($merged);
    }

    public function get_defaults(): array
    {
        return [
            'blogger_blog_id' => '',
            'blogger_album_name' => 'WP Offloaded Images',
            'blogger_auth_type' => 'oauth2',
            'blogger_cdn_domain_override' => '',
            'enable_webp_auto' => true,
            'default_quality' => 85,
            'media_sync_chunk_size' => 20,
            'replace_post_content' => true,
            'backup_before_rewrite' => true,
            'daily_audit_enabled' => false,
            'daily_audit_cron_schedule' => '0 3 * * *',
            'daily_audit_target_urls' => [],
            'daily_audit_slack_webhook' => '',
            'daily_audit_alert_lcp_ms' => 2500,
            'daemon_port' => 2095,
            'daemon_log_level' => 'info',
            'daemon_worker_threads' => 4,
            'daemon_poll_interval_sec' => 60,
            'daemon_auto_retry_limit' => 3,
        ];
    }

    public function get_rest_api_config(): array
    {
        $settings = $this->all();
        return [
            'api_url' => rest_url('wp-opt-agents/v1'),
            'site_url' => home_url(),
            'blogger_blog_id' => $settings['blogger_blog_id'],
            'blogger_album_name' => $settings['blogger_album_name'],
            'blogger_auth_type' => $settings['blogger_auth_type'],
            'blogger_cdn_domain_override' => $settings['blogger_cdn_domain_override'],
            'enable_webp_auto' => $settings['enable_webp_auto'],
            'default_quality' => $settings['default_quality'],
            'media_sync_chunk_size' => $settings['media_sync_chunk_size'],
            'replace_post_content' => $settings['replace_post_content'],
            'backup_before_rewrite' => $settings['backup_before_rewrite'],
        ];
    }
}