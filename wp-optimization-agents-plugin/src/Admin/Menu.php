<?php

namespace Puleeno\WPOptAgents\Admin;

class Menu
{
    private Renderer $renderer;

    public function __construct(Renderer $renderer)
    {
        $this->renderer = $renderer;
    }

    public function register(): void
    {
        add_menu_page(
            'WP Optimization Agents',
            'WP Opt Agents',
            'manage_options',
            'wp-opt-agents',
            [$this->renderer, 'render_dashboard'],
            'dashicons-rocket',
            58
        );

        add_submenu_page(
            'wp-opt-agents',
            'Settings',
            'Settings',
            'manage_options',
            'wp-opt-agents-settings',
            [$this->renderer, 'render_settings']
        );

        add_submenu_page(
            'wp-opt-agents',
            'Image Offload Status',
            'Offload Status',
            'manage_options',
            'wp-opt-agents-offload',
            [$this->renderer, 'render_offload_status']
        );

        add_submenu_page(
            'wp-opt-agents',
            'Daily Audit',
            'Daily Audit',
            'manage_options',
            'wp-opt-agents-audit',
            [$this->renderer, 'render_daily_audit']
        );
    }
}