<?php

namespace Puleeno\WPOptAgents\Database;

class Schema
{
    public static function install(): void
    {
        global $wpdb;
        $charset_collate = $wpdb->get_charset_collate();
        $images_table = self::get_images_table_name();
        $jobs_table = self::get_jobs_table_name();
        $redirect_logs_table = self::get_redirect_logs_table_name();

        // Mapping & sync state for offloaded images (source of truth for Rust agent DB access)
        $sql_images = "CREATE TABLE $images_table (
            id bigint(20) unsigned NOT NULL AUTO_INCREMENT,
            attachment_id bigint(20) unsigned DEFAULT NULL,
            post_id bigint(20) unsigned DEFAULT NULL,
            origin_url text NOT NULL,
            origin_url_hash char(64) NOT NULL,
            blogger_url text DEFAULT NULL,
            blogger_image_id varchar(255) DEFAULT NULL,
            transformed_params text DEFAULT NULL,
            width int(11) DEFAULT NULL,
            height int(11) DEFAULT NULL,
            mime_type varchar(100) DEFAULT NULL,
            original_size_bytes bigint(20) unsigned DEFAULT NULL,
            offload_size_bytes bigint(20) unsigned DEFAULT NULL,
            status enum('pending','uploading','uploaded','replacing','replaced','failed') NOT NULL DEFAULT 'pending',
            imgbb_url text DEFAULT NULL,
            imgbb_status varchar(20) NOT NULL DEFAULT 'none',
            redirect_active tinyint(1) NOT NULL DEFAULT 0,
            replaced_at datetime DEFAULT NULL,
            error_message text DEFAULT NULL,
            metadata longtext DEFAULT NULL,
            created_at datetime NOT NULL DEFAULT CURRENT_TIMESTAMP,
            updated_at datetime NOT NULL DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP,
            PRIMARY KEY (id),
            UNIQUE KEY origin_url_hash (origin_url_hash),
            KEY attachment_id (attachment_id),
            KEY post_id (post_id),
            KEY status (status),
            KEY imgbb_status (imgbb_status),
            KEY redirect_active (redirect_active),
            KEY blogger_image_id (blogger_image_id(191))
        ) $charset_collate;";

        // Optional job/audit records (managed/queried via plugin compatibility endpoints)
        $sql_jobs = "CREATE TABLE $jobs_table (
            id bigint(20) unsigned NOT NULL AUTO_INCREMENT,
            job_type varchar(50) NOT NULL,
            target varchar(255) DEFAULT NULL,
            status varchar(30) NOT NULL DEFAULT 'queued',
            payload longtext DEFAULT NULL,
            result longtext DEFAULT NULL,
            started_at datetime DEFAULT NULL,
            finished_at datetime DEFAULT NULL,
            created_at datetime NOT NULL DEFAULT CURRENT_TIMESTAMP,
            updated_at datetime NOT NULL DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP,
            PRIMARY KEY (id),
            KEY job_type (job_type),
            KEY status (status),
            KEY created_at (created_at)
        ) $charset_collate;";

        // Redirect hit logs for 301 analytics/charts
        $sql_redirect_logs = "CREATE TABLE $redirect_logs_table (
            id bigint(20) unsigned NOT NULL AUTO_INCREMENT,
            image_id bigint(20) unsigned DEFAULT NULL,
            origin_url text NOT NULL,
            origin_url_hash char(64) NOT NULL,
            target_url text NOT NULL,
            request_uri text DEFAULT NULL,
            referer text DEFAULT NULL,
            user_agent text DEFAULT NULL,
            ip_hash char(64) DEFAULT NULL,
            status_code smallint(5) unsigned NOT NULL DEFAULT 301,
            created_at datetime NOT NULL DEFAULT CURRENT_TIMESTAMP,
            PRIMARY KEY (id),
            KEY image_id (image_id),
            KEY origin_url_hash (origin_url_hash),
            KEY created_at (created_at),
            KEY status_code (status_code)
        ) $charset_collate;";

        require_once ABSPATH . 'wp-admin/includes/upgrade.php';
        dbDelta($sql_images);
        dbDelta($sql_jobs);
        dbDelta($sql_redirect_logs);
    }

    public static function maybe_upgrade(): void
    {
        $installed_version = get_option('wp_opt_agents_db_version', '0.0.0');
        if (version_compare($installed_version, WP_OPT_AGENTS_DB_VERSION, '<')) {
            self::install();
            update_option('wp_opt_agents_db_version', WP_OPT_AGENTS_DB_VERSION);
        }
    }

    public static function get_images_table_name(): string
    {
        global $wpdb;
        return $wpdb->prefix . 'wp_opt_agents_images';
    }

    public static function get_jobs_table_name(): string
    {
        global $wpdb;
        return $wpdb->prefix . 'wp_opt_agents_jobs';
    }

    public static function get_redirect_logs_table_name(): string
    {
        global $wpdb;
        return $wpdb->prefix . 'wp_opt_agents_redirect_logs';
    }

    // Backward-compatible alias
    public static function get_table_name(): string
    {
        return self::get_images_table_name();
    }
}