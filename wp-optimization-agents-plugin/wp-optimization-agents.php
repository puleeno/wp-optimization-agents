<?php
/**
 * Plugin Name: WP Optimization Agents
 * Plugin URI: https://nghesachxua.com/
 * Description: REST API endpoints for WP Optimization Agents daemon to offload images to Blogger CDN and optimize WordPress performance.
 * Version: 0.1.0
 * Author: Puleeno Nguyen
 * Author URI: mailto:puleeno@gmail.com
 * Text Domain: wp-optimization-agents
 * Domain Path: /languages
 * Requires at least: 6.0
 * Requires PHP: 7.4
 */

if (!defined('ABSPATH')) {
    exit;
}

define('WP_OPT_AGENTS_VERSION', '0.1.0');
define('WP_OPT_AGENTS_DB_VERSION', '1.0.0');
define('WP_OPT_AGENTS_FILE', __FILE__);
define('WP_OPT_AGENTS_PATH', plugin_dir_path(__FILE__));
define('WP_OPT_AGENTS_URL', plugin_dir_url(__FILE__));
define('WP_OPT_AGENTS_BASENAME', plugin_basename(__FILE__));

$autoload = WP_OPT_AGENTS_PATH . 'vendor/autoload.php';
if (file_exists($autoload)) {
    require_once $autoload;
}

spl_autoload_register(static function ($class) {
    $prefix = 'Puleeno\\WPOptAgents\\';
    $base_dir = WP_OPT_AGENTS_PATH . 'src/';
    $len = strlen($prefix);
    if (strncmp($prefix, $class, $len) !== 0) {
        return;
    }
    $relative_class = substr($class, $len);
    $file = $base_dir . str_replace('\\', '/', $relative_class) . '.php';
    if (file_exists($file)) {
        require $file;
    }
});

add_action('plugins_loaded', static function () {
    if (class_exists(\Puleeno\WPOptAgents\Plugin::class)) {
        \Puleeno\WPOptAgents\Plugin::instance()->boot();
    }
});

register_activation_hook(__FILE__, static function () {
    if (class_exists(\Puleeno\WPOptAgents\Plugin::class)) {
        \Puleeno\WPOptAgents\Plugin::activate();
    }
});

register_deactivation_hook(__FILE__, static function () {
    if (class_exists(\Puleeno\WPOptAgents\Plugin::class)) {
        \Puleeno\WPOptAgents\Plugin::deactivate();
    }
});