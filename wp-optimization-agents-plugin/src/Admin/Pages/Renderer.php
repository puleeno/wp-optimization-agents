<?php

namespace Puleeno\WPOptAgents\Admin\Pages;

use Puleeno\WPOptAgents\Plugin;
use Puleeno\WPOptAgents\Repository\ImageRepository;

class Renderer
{
    public function render_dashboard(): void
    {
        $settings = Plugin::get_settings();
        $repo = new ImageRepository();
        $stats = $repo->get_stats();
        $replaced = (int)($stats['replaced'] ?? 0);
        $pending = (int)($stats['pending'] ?? 0) + (int)($stats['uploading'] ?? 0) + (int)($stats['uploaded'] ?? 0) + (int)($stats['replacing'] ?? 0);
        ?>
        <div class="wrap wp-opt-agents-dashboard">
            <h1>WP Optimization Agents</h1>
            <p class="description">Offload WordPress images to Blogger via Chrome Web UI (no Google API). 1 WP post = 1 Blogger post/album. Rust agent replaces URLs in DB and plugin serves 301 redirects for old URLs.</p>

            <div class="wp-opt-agents-cards">
                <div class="card">
                    <h2>Plugin API / Agent</h2>
                    <div id="daemon-status">Checking...</div>
                </div>
                <div class="card">
                    <h2>Replaced (Live)</h2>
                    <div class="stat-number"><?php echo esc_html((string)$replaced); ?></div>
                    <div class="stat-detail">
                        <span><?php echo esc_html((string)$pending); ?> in progress</span>
                        <span><?php echo esc_html((string)($stats['failed'] ?? 0)); ?> failed</span>
                    </div>
                </div>
                <div class="card">
                    <h2>Total Mappings</h2>
                    <div class="stat-number"><?php echo esc_html((string)($stats['total'] ?? 0)); ?></div>
                </div>
            </div>

            <div class="card">
                <h2>Quick Actions</h2>
                <p>
                    <button class="button button-primary" id="test-daemon-connection">Test Plugin API</button>
                    <button class="button" id="generate-api-token">Generate API Token</button>
                </p>
            </div>

            <div class="card">
                <h2>Configuration Summary</h2>
                <table class="wp-list-table widefat fixed striped">
                    <tbody>
                        <tr><th>Site URL</th><td><?php echo esc_html(home_url()); ?></td></tr>
                        <tr><th>Blogger Blog ID</th><td><?php echo esc_html($settings['blogger_blog_id'] ?: 'Not configured'); ?></td></tr>
                        <tr><th>Chrome Profile</th><td><?php echo esc_html($settings['chrome_profile'] ?: 'default'); ?></td></tr>
                        <tr><th>Replace URLs in DB</th><td><?php echo !empty($settings['replace_post_content']) ? 'Yes' : 'No'; ?></td></tr>
                        <tr><th>301 Redirect Old URLs</th><td><?php echo !empty($settings['enable_old_url_redirect_301']) ? 'Enabled' : 'Disabled'; ?></td></tr>
                        <tr><th>Daemon Port (UI/API)</th><td><?php echo esc_html((string)$settings['daemon_port']); ?></td></tr>
                        <tr><th>DB Tables</th><td><?php global $wpdb; echo esc_html($wpdb->prefix . 'wp_opt_agents_images, ' . $wpdb->prefix . 'wp_opt_agents_jobs'); ?></td></tr>
                    </tbody>
                </table>
            </div>
        </div>
        <?php
    }

    public function render_settings(): void
    {
        ?>
        <div class="wrap">
            <h1>WP Optimization Agents - Settings</h1>
            <form method="post" action="options.php">
                <?php
                settings_fields('wp_opt_agents');
                do_settings_sections('wp_opt_agents');
                submit_button('Save Settings');
                ?>
            </form>
        </div>
        <?php
    }

    public function render_offload_status(): void
    {
        $repo = new ImageRepository();
        $stats = $repo->get_stats();
        ?>
        <div class="wrap">
            <h1>Image Offload Status</h1>

            <div class="wp-opt-agents-stats">
                <div class="stat-box synced"><?php echo (int)($stats['replaced'] ?? 0); ?><span>Replaced</span></div>
                <div class="stat-box processing"><?php echo (int)($stats['uploaded'] ?? 0); ?><span>Uploaded</span></div>
                <div class="stat-box pending"><?php echo (int)($stats['pending'] ?? 0) + (int)($stats['uploading'] ?? 0) + (int)($stats['replacing'] ?? 0); ?><span>In Progress</span></div>
                <div class="stat-box failed"><?php echo (int)($stats['failed'] ?? 0); ?><span>Failed</span></div>
            </div>

            <div id="offload-images-table"><p>Loading...</p></div>
        </div>
        <?php
    }

    public function render_daily_audit(): void
    {
        ?>
        <div class="wrap">
            <h1>WP ↔ Blogger Links</h1>
            <div class="card">
                <p>Each WordPress post is mapped to one Blogger post/album. Links are stored in post meta (<code>_wpoa_blogger_post_id</code>, <code>_wpoa_blogger_post_url</code>, <code>_wpoa_blogger_album_url</code>) and image mappings in <code><?php global $wpdb; echo esc_html($wpdb->prefix . 'wp_opt_agents_images'); ?></code>.</p>
                <p>Rust agent performs upload via Chrome and replacement in DB. After all replacements complete, old image URLs return <strong>301</strong> to Blogger URLs when redirect is enabled.</p>
            </div>
        </div>
        <?php
    }
}
