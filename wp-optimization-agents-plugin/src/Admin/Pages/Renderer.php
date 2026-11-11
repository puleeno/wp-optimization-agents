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
        ?>
        <div class="wrap wp-opt-agents-dashboard">
            <h1>WP Optimization Agents</h1>
            
            <div class="wp-opt-agents-cards">
                <div class="card">
                    <h2>Daemon Status</h2>
                    <div id="daemon-status">Checking...</div>
                </div>
                <div class="card">
                    <h2>Images Offloaded</h2>
                    <div class="stat-number"><?php echo $stats['synced']; ?></div>
                    <div class="stat-detail">
                        <span><?php echo $stats['pending']; ?> pending</span>
                        <span><?php echo $stats['failed']; ?> failed</span>
                    </div>
                </div>
                <div class="card">
                    <h2>Total Processed</h2>
                    <div class="stat-number"><?php echo $stats['total']; ?></div>
                </div>
            </div>

            <div class="card">
                <h2>Quick Actions</h2>
                <p>
                    <button class="button button-primary" id="test-daemon-connection">Test Daemon Connection</button>
                    <button class="button" id="generate-api-token">Generate API Token</button>
                </p>
            </div>

            <div class="card">
                <h2>Configuration Summary</h2>
                <table class="wp-list-table widefat fixed striped">
                    <tbody>
                        <tr><th>Blogger Blog ID</th><td><?php echo esc_html($settings['blogger_blog_id'] ?: 'Not configured'); ?></td></tr>
                        <tr><th>Album Name</th><td><?php echo esc_html($settings['blogger_album_name']); ?></td></tr>
                        <tr><th>Auto WebP</th><td><?php echo $settings['enable_webp_auto'] ? 'Enabled' : 'Disabled'; ?></td></tr>
                        <tr><th>Default Quality</th><td><?php echo esc_html($settings['default_quality']); ?></td></tr>
                        <tr><th>Replace Content</th><td><?php echo $settings['replace_post_content'] ? 'Yes' : 'No'; ?></td></tr>
                        <tr><th>Daemon Port</th><td><?php echo esc_html($settings['daemon_port']); ?></td></tr>
                        <tr><th>Daily Audit</th><td><?php echo $settings['daily_audit_enabled'] ? 'Enabled' : 'Disabled'; ?></td></tr>
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
                <div class="stat-box synced"><?php echo $stats['synced']; ?><span>Synced</span></div>
                <div class="stat-box pending"><?php echo $stats['pending']; ?><span>Pending</span></div>
                <div class="stat-box processing"><?php echo $stats['processing']; ?><span>Processing</span></div>
                <div class="stat-box failed"><?php echo $stats['failed']; ?><span>Failed</span></div>
            </div>

            <div id="offload-images-table">
                <p>Loading...</p>
            </div>
        </div>
        <?php
    }

    public function render_daily_audit(): void
    {
        $settings = Plugin::get_settings();
        ?>
        <div class="wrap">
            <h1>Daily Audit</h1>
            
            <div class="card">
                <h2>Configuration</h2>
                <table class="wp-list-table widefat fixed striped">
                    <tbody>
                        <tr><th>Enabled</th><td><?php echo $settings['daily_audit_enabled'] ? 'Yes' : 'No'; ?></td></tr>
                        <tr><th>Schedule</th><td><?php echo esc_html($settings['daily_audit_cron_schedule']); ?></td></tr>
                        <tr><th>Target URLs</th><td><?php echo esc_html(implode(', ', (array)$settings['daily_audit_target_urls'])); ?></td></tr>
                        <tr><th>Alert LCP</th><td><?php echo esc_html($settings['daily_audit_alert_lcp_ms']); ?>ms</td></tr>
                    </tbody>
                </table>
            </div>

            <div class="card">
                <h2>Trigger Manual Audit</h2>
                <button class="button button-primary" id="trigger-audit">Run Audit Now</button>
                <div id="audit-result"></div>
            </div>
        </div>
        <?php
    }
}