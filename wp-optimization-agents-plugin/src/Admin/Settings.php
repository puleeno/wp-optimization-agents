<?php

namespace Puleeno\WPOptAgents\Admin;

use Puleeno\WPOptAgents\Service\SettingsService;

class Settings
{
    /** @var SettingsService */
    private $settingsService;

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

        add_settings_section('wp_opt_agents_blogger', 'Blogger / Chrome Upload (Web UI)', function () {
            echo '<p>Agent uploads images to Blogger via Chrome browser (no Google API). One WordPress post = one Blogger album/post.</p>';
        }, 'wp_opt_agents');

        add_settings_field('blogger_blog_id', 'Blogger Blog ID', [$this, 'field_blogger_blog_id'], 'wp_opt_agents', 'wp_opt_agents_blogger');
        add_settings_field('chrome_profile', 'Chrome Profile', [$this, 'field_chrome_profile'], 'wp_opt_agents', 'wp_opt_agents_blogger');
        add_settings_field('chrome_executable', 'Chrome Executable Path', [$this, 'field_chrome_executable'], 'wp_opt_agents', 'wp_opt_agents_blogger');
        add_settings_field('chrome_user_data_dir', 'Chrome User Data Dir', [$this, 'field_chrome_user_data_dir'], 'wp_opt_agents', 'wp_opt_agents_blogger');
        add_settings_field('chrome_headless', 'Headless Chrome', [$this, 'field_chrome_headless'], 'wp_opt_agents', 'wp_opt_agents_blogger');

        add_settings_section('wp_opt_agents_media', 'Media Offload & Replace', null, 'wp_opt_agents');
        add_settings_field('enable_webp_auto', 'Prefer WebP/CDN params', [$this, 'field_enable_webp_auto'], 'wp_opt_agents', 'wp_opt_agents_media');
        add_settings_field('default_quality', 'Default Quality (1-100)', [$this, 'field_default_quality'], 'wp_opt_agents', 'wp_opt_agents_media');
        add_settings_field('media_sync_chunk_size', 'Sync Chunk Size', [$this, 'field_media_sync_chunk_size'], 'wp_opt_agents', 'wp_opt_agents_media');
        add_settings_field('replace_post_content', 'Replace URLs in WP DB/content', [$this, 'field_replace_post_content'], 'wp_opt_agents', 'wp_opt_agents_media');
        add_settings_field('backup_before_rewrite', 'Backup Before Rewrite', [$this, 'field_backup_before_rewrite'], 'wp_opt_agents', 'wp_opt_agents_media');
        add_settings_field('enable_old_url_redirect_301', '301 Redirect old URL -> new URL', [$this, 'field_enable_old_url_redirect_301'], 'wp_opt_agents', 'wp_opt_agents_media');

        add_settings_section('wp_opt_agents_daemon', 'Agent Daemon', null, 'wp_opt_agents');
        add_settings_field('daemon_port', 'Daemon Port', [$this, 'field_daemon_port'], 'wp_opt_agents', 'wp_opt_agents_daemon');
        add_settings_field('daemon_log_level', 'Log Level', [$this, 'field_daemon_log_level'], 'wp_opt_agents', 'wp_opt_agents_daemon');
        add_settings_field('daemon_poll_interval_sec', 'Poll Interval (seconds)', [$this, 'field_daemon_poll_interval_sec'], 'wp_opt_agents', 'wp_opt_agents_daemon');
        add_settings_field('daemon_auto_retry_limit', 'Auto Retry Limit', [$this, 'field_daemon_auto_retry_limit'], 'wp_opt_agents', 'wp_opt_agents_daemon');
        add_settings_field('daemon_api_token', 'Plugin Compatibility API Token', [$this, 'field_daemon_api_token'], 'wp_opt_agents', 'wp_opt_agents_daemon');
    }

    public function sanitize_settings(array $input): array
    {
        $defaults = $this->settingsService->get_defaults();
        $sanitized = [];
        foreach ($defaults as $key => $default) {
            if (!array_key_exists($key, $input)) {
                $sanitized[$key] = $default;
                continue;
            }
            $value = $input[$key];
            if (is_bool($default)) {
                $sanitized[$key] = !empty($value);
            } elseif (is_int($default)) {
                $sanitized[$key] = (int)$value;
            } elseif (is_array($default)) {
                if (is_string($value)) {
                    $value = array_filter(array_map('trim', preg_split('/\r\n|\r|\n/', $value)));
                }
                $sanitized[$key] = array_map('sanitize_text_field', (array)$value);
            } else {
                $sanitized[$key] = sanitize_text_field((string)$value);
            }
        }
        return $sanitized;
    }

    public function field_blogger_blog_id(): void
    {
        $settings = $this->settingsService->all();
        echo '<input type="text" name="wp_opt_agents_settings[blogger_blog_id]" value="' . esc_attr($settings['blogger_blog_id']) . '" class="regular-text" />';
        echo '<p class="description">Blogger Blog ID used by Chrome upload flow.</p>';
    }

    public function field_chrome_profile(): void
    {
        $settings = $this->settingsService->all();
        echo '<input type="text" name="wp_opt_agents_settings[chrome_profile]" value="' . esc_attr($settings['chrome_profile']) . '" class="regular-text" />';
        echo '<p class="description">Chrome profile name (must be signed in to Blogger/Google).</p>';
    }

    public function field_chrome_executable(): void
    {
        $settings = $this->settingsService->all();
        echo '<input type="text" name="wp_opt_agents_settings[chrome_executable]" value="' . esc_attr($settings['chrome_executable']) . '" class="regular-text code" placeholder="/Applications/Google Chrome.app/Contents/MacOS/Google Chrome" />';
    }

    public function field_chrome_user_data_dir(): void
    {
        $settings = $this->settingsService->all();
        echo '<input type="text" name="wp_opt_agents_settings[chrome_user_data_dir]" value="' . esc_attr($settings['chrome_user_data_dir']) . '" class="regular-text code" />';
    }

    public function field_chrome_headless(): void
    {
        $settings = $this->settingsService->all();
        $checked = !empty($settings['chrome_headless']) ? 'checked="checked"' : '';
        echo "<input type=\"checkbox\" name=\"wp_opt_agents_settings[chrome_headless]\" value=\"1\" {$checked} />";
        echo '<p class="description">Usually keep OFF for Google sign-in/Picker stability.</p>';
    }

    public function field_enable_webp_auto(): void
    {
        $settings = $this->settingsService->all();
        $checked = !empty($settings['enable_webp_auto']) ? 'checked="checked"' : '';
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
        echo '<p class="description">Images to process per batch/post run.</p>';
    }

    public function field_replace_post_content(): void
    {
        $settings = $this->settingsService->all();
        $checked = !empty($settings['replace_post_content']) ? 'checked="checked"' : '';
        echo "<input type=\"checkbox\" name=\"wp_opt_agents_settings[replace_post_content]\" value=\"1\" {$checked} />";
        echo '<p class="description">Rust agent replaces old image URLs in WordPress DB/content after Blogger upload.</p>';
    }

    public function field_backup_before_rewrite(): void
    {
        $settings = $this->settingsService->all();
        $checked = !empty($settings['backup_before_rewrite']) ? 'checked="checked"' : '';
        echo "<input type=\"checkbox\" name=\"wp_opt_agents_settings[backup_before_rewrite]\" value=\"1\" {$checked} />";
    }

    public function field_enable_old_url_redirect_301(): void
    {
        $settings = $this->settingsService->all();
        $checked = !empty($settings['enable_old_url_redirect_301']) ? 'checked="checked"' : '';
        echo "<input type=\"checkbox\" name=\"wp_opt_agents_settings[enable_old_url_redirect_301]\" value=\"1\" {$checked} />";
        echo '<p class="description">After replacement is complete, old WP URL returns HTTP 301 to new Blogger URL.</p>';
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
        echo '<input type="text" name="wp_opt_agents_settings[daemon_api_token]" value="' . esc_attr((string)($settings['daemon_api_token'] ?? '')) . '" class="regular-text code" />';
        echo '<p class="description">Optional token for plugin compatibility REST endpoints. Primary agent flow reads/writes WordPress DB directly.</p>';
    }
}
