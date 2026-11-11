<?php

namespace Puleeno\WPOptAgents\Database;

use wpdb;

class Schema
{
    public static function install(): void
    {
        global $wpdb;
        $charset_collate = $wpdb->get_charset_collate();
        $table_name = self::get_table_name();

        $sql = "CREATE TABLE $table_name (
            id bigint(20) unsigned NOT NULL AUTO_INCREMENT,
            post_id bigint(20) unsigned NOT NULL,
            origin_url text NOT NULL,
            blogger_url text DEFAULT NULL,
            blogger_image_id varchar(255) DEFAULT NULL,
            transformed_params text DEFAULT NULL,
            width int(11) DEFAULT NULL,
            height int(11) DEFAULT NULL,
            mime_type varchar(100) DEFAULT NULL,
            original_size_bytes bigint(20) unsigned DEFAULT NULL,
            offload_size_bytes bigint(20) unsigned DEFAULT NULL,
            status enum('pending','processing','synced','failed') NOT NULL DEFAULT 'pending',
            error_message text DEFAULT NULL,
            created_at datetime NOT NULL DEFAULT CURRENT_TIMESTAMP,
            updated_at datetime NOT NULL DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP,
            PRIMARY KEY (id),
            KEY post_id (post_id),
            KEY status (status),
            KEY origin_url (origin_url(191))
        ) $charset_collate;";

        require_once ABSPATH . 'wp-admin/includes/upgrade.php';
        dbDelta($sql);
    }

    public static function maybe_upgrade(): void
    {
        $installed_version = get_option('wp_opt_agents_db_version', '0.0.0');
        if (version_compare($installed_version, WP_OPT_AGENTS_DB_VERSION, '<')) {
            self::install();
            update_option('wp_opt_agents_db_version', WP_OPT_AGENTS_DB_VERSION);
        }
    }

    public static function get_table_name(): string
    {
        global $wpdb;
        return $wpdb->prefix . 'wp_opt_agents_images';
    }
}