<?php

namespace Puleeno\WPOptAgents\Admin\Pages;

use Puleeno\WPOptAgents\Plugin;
use Puleeno\WPOptAgents\Repository\ImageRepository;
use Puleeno\WPOptAgents\Repository\RedirectLogRepository;

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
            <p class="description">Offload WordPress images to Blogger via Chrome Web UI (no Google API). 1 WP post = 1 Blogger post/album. Rust agent replaces URLs in DB and plugin serves 301 redirects for old URLs with hit logging for statistics.</p>

            <div class="wp-opt-agents-cards">
                <div class="card">
                    <h2>Plugin API / Agent</h2>
                    <div id="daemon-status">Checking...</div>
                </div>
                <div class="card">
                    <h2>Replaced (301 active)</h2>
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
                        <tr><th>Replace URLs in DB</th><td><?php echo !empty($settings['replace_post_content']) ? 'Yes' : 'No'; ?></td></tr>
                        <tr><th>301 Redirect Old URLs</th><td><?php echo !empty($settings['enable_old_url_redirect_301']) ? 'Enabled' : 'Disabled'; ?></td></tr>
                        <tr><th>Daemon Port</th><td><?php echo esc_html((string)$settings['daemon_port']); ?></td></tr>
                        <tr><th>DB Tables</th><td><?php global $wpdb; echo esc_html($wpdb->prefix . 'wp_opt_agents_images, ' . $wpdb->prefix . 'wp_opt_agents_redirect_logs, ' . $wpdb->prefix . 'wp_opt_agents_jobs'); ?></td></tr>
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
        $redirectRepo = new RedirectLogRepository();
        $days = isset($_GET['days']) ? max(1, min(365, (int)$_GET['days'])) : 30;
        $redirectStats = [
            'period_days' => $days,
            'total_hits' => $redirectRepo->get_total_count(0),
            'period_hits' => $redirectRepo->get_total_count($days),
            'daily' => $redirectRepo->get_daily_stats($days),
            'top' => $redirectRepo->get_top_redirects(10, $days),
        ];
        $maxHits = 0;
        foreach ($redirectStats['daily'] as $d) {
            if ($d['hits'] > $maxHits) $maxHits = $d['hits'];
        }
        ?>
        <div class="wrap">
            <h1>Image Offload Status & Redirect Analytics</h1>

            <div class="wp-opt-agents-stats">
                <div class="stat-box synced"><?php echo (int)($stats['replaced'] ?? 0); ?><span>Replaced</span></div>
                <div class="stat-box processing"><?php echo (int)($stats['uploaded'] ?? 0); ?><span>Uploaded</span></div>
                <div class="stat-box pending"><?php echo (int)($stats['pending'] ?? 0) + (int)($stats['uploading'] ?? 0) + (int)($stats['replacing'] ?? 0); ?><span>In Progress</span></div>
                <div class="stat-box failed"><?php echo (int)($stats['failed'] ?? 0); ?><span>Failed</span></div>
            </div>

            <div class="card" style="margin-top:20px;">
                <h2>301 Redirect Statistics</h2>
                <p>
                    <strong>Total hits:</strong> <?php echo esc_html((string)$redirectStats['total_hits']); ?>
                    &nbsp;|&nbsp;
                    <strong>Last <?php echo esc_html((string)$days); ?> days:</strong> <?php echo esc_html((string)$redirectStats['period_hits']); ?>
                </p>
                <p>
                    <?php foreach ([7, 30, 90] as $opt): ?>
                        <a class="button <?php echo $days === $opt ? 'button-primary' : ''; ?>" href="<?php echo esc_url(admin_url('admin.php?page=wp-opt-agents-offload&days=' . $opt)); ?>"><?php echo $opt; ?> days</a>
                    <?php endforeach; ?>
                </p>

                <h3>Redirect Hits Chart (daily)</h3>
                <div style="overflow-x:auto; padding:12px 0;">
                    <div style="display:flex; align-items:flex-end; gap:4px; height:180px; border-left:1px solid #ccd0d4; border-bottom:1px solid #ccd0d4; padding:8px;">
                        <?php if (empty($redirectStats['daily'])): ?>
                            <p>No data yet.</p>
                        <?php else: foreach ($redirectStats['daily'] as $point):
                            $hits = (int)$point['hits'];
                            $height = $maxHits > 0 ? max(2, (int)round(($hits / $maxHits) * 150)) : 2;
                            $label = substr($point['day'], 5); // MM-DD
                        ?>
                            <div style="flex:1 0 14px; display:flex; flex-direction:column; align-items:center; min-width:14px;" title="<?php echo esc_attr($point['day'] . ': ' . $hits . ' hits'); ?>">
                                <div style="font-size:10px; color:#646970; height:14px; line-height:14px;"><?php echo $hits > 0 ? esc_html((string)$hits) : ''; ?></div>
                                <div style="width:100%; max-width:20px; height:<?php echo $height; ?>px; background:#2271b1; border-radius:2px 2px 0 0;"></div>
                                <div style="font-size:9px; color:#646970; margin-top:4px; transform:rotate(-45deg); transform-origin:top left; white-space:nowrap; height:24px;"><?php echo esc_html($label); ?></div>
                            </div>
                        <?php endforeach; endif; ?>
                    </div>
                </div>

                <h3>Top Redirected URLs (last <?php echo esc_html((string)$days); ?> days)</h3>
                <table class="wp-list-table widefat fixed striped">
                    <thead>
                        <tr>
                            <th>Origin URL</th>
                            <th>Target (Blogger)</th>
                            <th style="width:80px;">Hits</th>
                            <th style="width:170px;">Last Hit</th>
                        </tr>
                    </thead>
                    <tbody>
                    <?php if (empty($redirectStats['top'])): ?>
                        <tr><td colspan="4">No redirect hits recorded yet.</td></tr>
                    <?php else: foreach ($redirectStats['top'] as $row): ?>
                        <tr>
                            <td style="word-break:break-all;"><?php echo esc_html((string)$row['origin_url']); ?></td>
                            <td style="word-break:break-all;"><?php echo esc_html((string)$row['target_url']); ?></td>
                            <td><?php echo esc_html((string)$row['hits']); ?></td>
                            <td><?php echo esc_html((string)$row['last_hit']); ?></td>
                        </tr>
                    <?php endforeach; endif; ?>
                    </tbody>
                </table>

                <p style="margin-top:12px;"><em>Data is logged when the plugin serves a 301 redirect for an old image URL. Use REST endpoint <code>/wp-json/wp-opt-agents/v1/redirect-stats?days=<?php echo esc_html((string)$days); ?></code> for JSON/chart integrations.</em></p>
            </div>

            <div class="card" style="margin-top:20px;">
                <h2>Image Mapping Stats</h2>
                <div id="offload-images-table"><p>Loading...</p></div>
            </div>
        </div>
        <?php
    }

    public function render_daily_audit(): void
    {
        global $wpdb;
        ?>
        <div class="wrap">
            <h1>WP ↔ Blogger Links & Redirects</h1>
            <div class="card">
                <p>Each WordPress post is mapped to one Blogger post/album. Links are stored in post meta (<code>_wpoa_blogger_post_id</code>, <code>_wpoa_blogger_post_url</code>, <code>_wpoa_blogger_album_url</code>) and image mappings in <code><?php echo esc_html($wpdb->prefix . 'wp_opt_agents_images'); ?></code>.</p>
                <p>301 redirect hits are stored in <code><?php echo esc_html($wpdb->prefix . 'wp_opt_agents_redirect_logs'); ?></code> for chart/statistics (daily counts, top URLs).</p>
                <ul>
                    <li><strong>Offload stats:</strong> <code>GET /wp-json/wp-opt-agents/v1/stats</code></li>
                    <li><strong>Redirect stats:</strong> <code>GET /wp-json/wp-opt-agents/v1/redirect-stats?days=30</code></li>
                </ul>
            </div>
        </div>
        <?php
    }
}
