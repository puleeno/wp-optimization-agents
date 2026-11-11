<?php

namespace Puleeno\WPOptAgents\Admin;

use Puleeno\WPOptAgents\Service\SettingsService;

class Settings
{
    private SettingsService $settingsService;

    public function __construct(SettingsService $settingsService)
    {
        $this->settingsService = $settingsService;
    }

    public function register(): void
    {
        add_action('admin_init', [$this, 'register_settings']);
    }

    public function register_settings(): void
    {
        register_setting('wp_opt_agents', 'wp_opt_agents_settings', [
            'type' => 'array',
            'sanitize_callback' => [$this, 'sanitize_settings'],
            'default' => $this->settingsService->get_defaults(),
        ]);

        // Blogger Settings Section
        add_settings_section(
            'wp_opt_agents_blogger',
            'Blogger Configuration',
            null,
            'wp_opt_agents'
        );

        add_settings_field(
            'blogger_blog_id',
            'Blog ID',
            [$this, 'field_blogger_blog_id'],
            'wp_opt_agents',
            'wp_opt_agents_blogger'
        );

        add_settings_field(
            'blogger_album_name',
            'Album Name',
            [$this, 'field_blogger_album_name'],
            'wp_opt_agents',
            'wp_opt_agents_blogger'
        );

        add_settings_field(
            'blogger_auth_type',
            'Auth Type',
            [$this, 'field_blogger_auth_type'],
            'wp_opt_agents',
            'wp_opt_agents_blogger'
        );

        add_settings_field(
            'blogger_cdn_domain_override',
            'CDN Domain Override',
            [$this, 'field_blogger_cdn_domain_override'],
            'wp_opt_agents',
            'wp_opt_agents_blogger'
        );

        // Media Settings Section
        add_settings_section(
            'wp_opt_agents_media',
            'Media Offload Settings',
            null,
            'wp_opt_agents'
        );

        add_settings_field(
            'enable_webp_auto',
            'Auto WebP Conversion',
            [$this, 'field_enable_webp_auto'],
            'wp_opt_agents',
            'wp_opt_agents_media'
        );

        add_settings_field(
            'default_quality',
            'Default Quality (1-100)',
            [$this, 'field_default_quality'],
            'wp_opt_agents',
            'wp_opt_agents_media'
        );

        add_settings_field(
            'media_sync_chunk_size',
            'Sync Chunk Size',
            [$this, 'field_media_sync_chunk_size'],
            'wp_opt_agents',
            'wp_opt_agents_media'
        );

        add_settings_field(
            'replace_post_content',
            'Replace Post Content',
            [$this, 'field_replace_post_content'],
            'wp_opt_agents',
            'wp_opt_agents_media'
        );

        add_settings_field(
            'backup_before_rewrite',
            'Backup Before Rewrite',
            [$this, 'field_backup_before_rewrite'],
            'wp_opt_agents',
            'wp_opt_agents_media'
        );

        // Daemon Settings Section
        add_settings_section(
            'wp_opt_agents_daemon',
            'Daemon Settings',
            null,
            'wp_opt_agents'
        );

        add_settings_field(
            'daemon_port',
            'Daemon Port',
            [$this, 'field_daemon_port'],
            'wp_opt_agents',
            'wp_opt_agents_daemon'
        );

        add_settings_field(
            'daemon_log_level',
            'Log Level',
            [$this, 'field_daemon_log_level'],
            'wp_opt_agents',
            'wp_opt_agents_daemon'
        );

        add_settings_field(
            'daemon_worker_threads',
            'Worker Threads',
            [$this, 'field_daemon_worker_threads'],
            'wp_opt_agents',
            'wp_opt_agents_daemon'
        );

        add_settings_field(
            'daemon_poll_interval_sec',
            'Poll Interval (seconds)',
            [$this, 'field_daemon_poll_interval_sec'],
            'wp_opt_agents',
            'wp_opt_agents_daemon'
        );

        add_settings_field(
            'daemon_auto_retry_limit',
            'Auto Retry Limit',
            [$this, 'field_daemon_auto_retry_limit'],
            'wp_opt_agents',
            'wp_opt_agents_daemon'
        );

        add_settings_field(
            'daemon_api_token',
            'Daemon API Token',
            [$this, 'field_daemon_api_token'],
            'wp_opt_agents',
            'wp_opt_agents_daemon'
        );

        // Daily Audit Section
        add_settings_section(
            'wp_opt_agents_audit',
            'Daily Audit Settings',
            null,
            'wp_opt_agents'
        );

        add_settings_field(
            'daily_audit_enabled',
            'Enable Daily Audit',
            [$this, 'field_daily_audit_enabled'],
            'wp_opt_agents',
            'wp_opt_agents_audit'
        );

        add_settings_field(
            'daily_audit_cron_schedule',
            'Cron Schedule',
            [$this, 'field_daily_audit_cron_schedule'],
            'wp_opt_agents',
            'wp_opt_agents_audit'
        );

        add_settings_field(
            'daily_audit_target_urls',
            'Target URLs (one per line)',
            [$this, 'field_daily_audit_target_urls'],
            'wp_opt_agents',
            'wp_opt_agents_audit'
        );

        add_settings_field(
            'daily_audit_slack_webhook',
            'Slack Webhook URL',
            [$this, 'field_daily_audit_slack_webhook'],
            'wp_opt_agents',
            'wp_opt_agents_audit'
        );

        add_settings_field(
            'daily_audit_alert_lcp_ms',
            'Alert LCP Threshold (ms)',
            [$this, 'field_daily_audit_alert_lcp_ms'],
            'wp_opt_agents',
            'wp_opt_agents_audit'
        );
    }

    public function sanitize_settings(array $input): array
    {
        $defaults = $this->settingsService->get_defaults();
        $sanitized = [];

        foreach ($defaults as $key => $default) {
            if (isset($input[$key])) {
                $value = $input[$key];
                if (is_bool($default)) {
                    $sanitized[$key] = !empty($value);
                } elseif (is_int($default)) {
                    $sanitized[$key] = (int)$value;
                } elseif (is_array($default)) {
                    $sanitized[$key] = array_map('sanitize_text_field', (array)$value);
                } else {
                    $sanitized[$key] = sanitize_text_field($value);
                }
            } else {
                $sanitized[$key] = $default;
            }
        }

        return $sanitized;
    }

    public function field_blogger_blog_id(): void
    {
        $settings = $this->settingsService->all();
        echo '<input type="text" name="wp_opt_agents_settings[blogger_blog_id]" value="' . esc_attr($settings['blogger_blog_id']) . '" class="regular-text" />';
        echo '<p class="description">Blogger Blog ID (from blogger.com URL)</p>';
    }

    public function field_blogger_album_name(): void
    {
        $settings = $this->settingsService->all();
        echo '<input type="text" name="wp_opt_agents_settings[blogger_album_name]" value="' . esc_attr($settings['blogger_album_name']) . '" class="regular-text" />';
    }

    public function field_blogger_auth_type(): void
    {
        $settings = $this->settingsService->all();
        $options = ['oauth2' => 'OAuth 2.0', 'service_account' => 'Service Account'];
        echo '<select name="wp_opt_agents_settings[blogger_auth_type]">';
        foreach ($options as $value => $label) {
            $selected = selected($settings['blogger_auth_type'], $value, false);
            echo "<option value=\"{$value}\" {$selected}>{$label}</option>";
        }
        echo '</select>';
    }

    public function field_blogger_cdn_domain_override(): void
    {
        $settings = $this->settingsService->all();
        echo '<input type="text" name="wp_opt_agents_settings[blogger_cdn_domain_override]" value="' . esc_attr($settings['blogger_cdn_domain_override']) . '" class="regular-text" placeholder="e.g., cdn.mydomain.com" />';
        echo '<p class="description">Optional: Custom CDN domain for Blogger images</p>';
    }

    public function field_enable_webp_auto(): void
    {
        $settings = $this->settingsService->all();
        $checked = $settings['enable_webp_auto'] ? 'checked="checked"' : '';
        echo "<input type=\"checkbox\" name=\"wp_opt_agents_settings[enable_webp_auto]\" value=\"1\" {$checked} />";
    }

    public function field_default_quality(): void
    {
        $settings = $this->settingsService->all();
        echo '<input type="number" name="wp_opt_agents_settings[default_quality]" value="' . esc_attr($settings['default_quality']) . '" min="1" max="100" class="small-text" />';
    }

    public function field_media_sync_chunk_size(): void
    {
        $settings = $this->settingsService->all();
        echo '<input type="number" name="wp_opt_agents_settings[media_sync_chunk_size]" value="' . esc_attr($settings['media_sync_chunk_size']) . '" min="1" max="100" class="small-text" />';
        echo '<p class="description">Number of images to process per batch</p>';
    }

    public function field_replace_post_content(): void
    {
        $settings = $this->settingsService->all();
        $checked = $settings['replace_post_content'] ? 'checked="checked"' : '';
        echo "<input type=\"checkbox\" name=\"wp_opt_agents_settings[replace_post_content]\" value=\"1\" {$checked} />";
        echo '<p class="description">Automatically replace image URLs in post content after offload</p>';
    }

    public function field_backup_before_rewrite(): void
    {
        $settings = $this->settingsService->all();
        $checked = $settings['backup_before_rewrite'] ? 'checked="checked"' : '';
        echo "<input type=\"checkbox\" name=\"wp_opt_agents_settings[backup_before_rewrite]\" value=\"1\" {$checked} />";
        echo '<p class="description">Create backup of post content before rewriting</p>';
    }

    public function field_daemon_port(): void
    {
        $settings = $this->settingsService->all();
        echo '<input type="number" name="wp_opt_agents_settings[daemon_port]" value="' . esc_attr($settings['daemon_port']) . '" min="1024" max="65535" class="small-text" />';
    }

    public function field_daemon_log_level(): void
    {
        $settings = $this->settingsService->all();
        $options = ['debug' => 'Debug', 'info' => 'Info', 'warn' => 'Warn', 'error' => 'Error'];
        echo '<select name="wp_opt_agents_settings[daemon_log_level]">';
        foreach ($options as $value => $label) {
            $selected = selected($settings['daemon_log_level'], $value, false);
            echo "<option value=\"{$value}\" {$selected}>{$label}</option>";
        }
        echo '</select>';
    }

    public function field_daemon_worker_threads(): void
    {
        $settings = $this->settingsService->all();
        echo '<input type="number" name="wp_opt_agents_settings[daemon_worker_threads]" value="' . esc_attr($settings['daemon_worker_threads']) . '" min="1" max="32" class="small-text" />';
    }

    public function field_daemon_poll_interval_sec(): void
    {
        $settings = $this->settingsService->all();
        echo '<input type="number" name="wp_opt_agents_settings[daemon_poll_interval_sec]" value="' . esc_attr($settings['daemon_poll_interval_sec']) . '" min="10" max="3600" class="small-text" />';
    }

    public function field_daemon_auto_retry_limit(): void
    {
        $settings = $this->settingsService->all();
        echo '<input type="number" name="wp_opt_agents_settings[daemon_auto_retry_limit]" value="' . esc_attr($settings['daemon_auto_retry_limit']) . '" min="0" max="10" class="small-text" />';
    }

    public function field_daemon_api_token(): void
    {
        $settings = $this->settingsService->all();
        echo '<input type="text" name="wp_opt_agents_settings[daemon_api_token]" value="' . esc_attr($settings['daemon_api_token']) . '" class="regular-text code" />';
        echo '<p class="description">Bearer token for daemon authentication. Leave empty to generate.</p>';
    }

    public function field_daily_audit_enabled(): void
    {
        $settings = $this->settingsService->all();
        $checked = $settings['daily_audit_enabled'] ? 'checked="checked"' : '';
        echo "<input type=\"checkbox\" name=\"wp_opt_agents_settings[daily_audit_enabled]\" value=\"1\" {$checked} />";
    }

    public function field_daily_audit_cron_schedule(): void
    {
        $settings = $this->settingsService->all();
        echo '<input type="text" name="wp_opt_agents_settings[daily_audit_cron_schedule]" value="' . esc_attr($settings['daily_audit_cron_schedule']) . '" class="regular-text code" placeholder="0 3 * * *" />';
        echo '<p class="description">Cron expression for daily audit (default: 03:00 UTC)</p>';
    }

    public function field_daily_audit_target_urls(): void
    {
        $settings = $this->settingsService->all();
        $value = is_array($settings['daily_audit_target_urls']) ? implode("\n", $settings['daily_audit_target_urls']) : '';
        echo '<textarea name="wp_opt_agents_settings[daily_audit_target_urls]" rows="5" class="large-text code">' . esc_textarea($value) . '</textarea>';
        echo '<p class="description">One URL per line</p>';
    }

    public function field_daily_audit_slack_webhook(): void
    {
        $settings = $this->settingsService->all();
        echo '<input type="url" name="wp_opt_agents_settings[daily_audit_slack_webhook]" value="' . esc_attr($settings['daily_audit_slack_webhook']) . '" class="regular-text" />';
    }

    public function field_daily_audit_alert_lcp_ms(): void
    {
        $settings = $this->settingsService->all();
        echo '<input type="number" name="wp_opt_agents_settings[daily_audit_alert_lcp_ms]" value="' . esc_attr($settings['daily_audit_alert_lcp_ms']) . '" min="100" max="10000" class="small-text" />';
    }
}