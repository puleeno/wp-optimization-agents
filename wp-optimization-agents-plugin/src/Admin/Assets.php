<?php

namespace Puleeno\WPOptAgents\Admin;

class Assets
{
    public function register(): void
    {
        add_action('admin_enqueue_scripts', [$this, 'enqueue_admin_assets']);
    }

    public function enqueue_admin_assets($hook_suffix): void
    {
        $screen = get_current_screen();
        if (!$screen || strpos($screen->id, 'wp-opt-agents') === false) {
            return;
        }

        wp_enqueue_style(
            'wp-opt-agents-admin',
            WP_OPT_AGENTS_URL . 'assets/css/admin.css',
            [],
            WP_OPT_AGENTS_VERSION
        );

        wp_enqueue_script(
            'wp-opt-agents-admin',
            WP_OPT_AGENTS_URL . 'assets/js/admin.js',
            ['jquery', 'wp-api'],
            WP_OPT_AGENTS_VERSION,
            true
        );

        wp_localize_script('wp-opt-agents-admin', 'wpOptAgents', [
            'rest_url' => rest_url('wp-opt-agents/v1'),
            'nonce' => wp_create_nonce('wp_rest'),
            'api_token' => \Puleeno\WPOptAgents\Plugin::get_settings()['daemon_api_token'] ?? '',
        ]);
    }
}