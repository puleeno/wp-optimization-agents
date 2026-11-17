<?php

namespace Puleeno\WPOptAgents;

use Puleeno\WPOptAgents\Admin\Assets;
use Puleeno\WPOptAgents\Admin\Menu;
use Puleeno\WPOptAgents\Admin\MediaLibrary;
use Puleeno\WPOptAgents\Admin\Pages\Renderer;
use Puleeno\WPOptAgents\Admin\Settings;
use Puleeno\WPOptAgents\Contracts\SettingsRepositoryInterface;
use Puleeno\WPOptAgents\Database\Schema;
use Puleeno\WPOptAgents\Repository\ImageRepository;
use Puleeno\WPOptAgents\Repository\RedirectLogRepository;
use Puleeno\WPOptAgents\Repository\SettingsRepository;
use Puleeno\WPOptAgents\Service\OffloadService;
use Puleeno\WPOptAgents\Service\SettingsService;
use Puleeno\WPOptAgents\Api\RestApi;

class Plugin
{
    /** @var Plugin|null */
    private static $instance = null;

    /** @var array<string,mixed> */
    private $services = [];

    /** @var bool */
    private $booted = false;

    public static function instance(): Plugin
    {
        if (self::$instance === null) {
            self::$instance = new self();
        }
        return self::$instance;
    }

    public function boot(): void
    {
        if ($this->booted) {
            return;
        }
        $this->booted = true;

        add_action('admin_init', [Schema::class, 'maybe_upgrade']);
        add_action('template_redirect', [$this->offloadService(), 'maybe_redirect_old_url'], 1);
        add_filter('wp_generate_attachment_metadata', [$this, 'on_attachment_metadata_generated'], 20, 2);

        if (is_admin()) {
            $this->menu()->register();
            $this->assets()->register();
            $this->settingsController()->register();
            $this->mediaLibrary()->register();
        }

        add_action('rest_api_init', [$this->restApi(), 'register_routes']);
    }

    public function make(string $id, callable $factory)
    {
        if (!array_key_exists($id, $this->services)) {
            $this->services[$id] = $factory($this);
        }
        return $this->services[$id];
    }

    public function settingsRepository(): SettingsRepositoryInterface
    {
        return $this->make('settings_repository', function () {
            return new SettingsRepository();
        });
    }

    public function settingsService(): SettingsService
    {
        return $this->make('settings_service', function (Plugin $plugin) {
            return new SettingsService($plugin->settingsRepository());
        });
    }

    public function imageRepository(): ImageRepository
    {
        return $this->make('image_repository', function () {
            return new ImageRepository();
        });
    }

    public function redirectLogRepository(): RedirectLogRepository
    {
        return $this->make('redirect_log_repository', function () {
            return new RedirectLogRepository();
        });
    }

    public function offloadService(): OffloadService
    {
        return $this->make('offload_service', function (Plugin $plugin) {
            return new OffloadService(
                $plugin->imageRepository(),
                $plugin->settingsRepository(),
                $plugin->redirectLogRepository()
            );
        });
    }

    public function restApi(): RestApi
    {
        return $this->make('rest_api', function (Plugin $plugin) {
            return new RestApi($plugin->settingsService(), $plugin->offloadService());
        });
    }

    /**
     * Notify daemon after WP generates attachment metadata (file + sizes ready).
     * @param array<string,mixed> $metadata
     * @return array<string,mixed>
     */
    public function on_attachment_metadata_generated($metadata, $attachment_id)
    {
        try {
            $attachment_id = (int)$attachment_id;
            if ($attachment_id <= 0 || !wp_attachment_is_image($attachment_id)) {
                return $metadata;
            }
            $settings = $this->settingsService()->all();
            if (empty($settings['notify_daemon_on_upload'])) {
                return $metadata;
            }
            $daemon_url = isset($settings['daemon_url']) ? rtrim((string)$settings['daemon_url'], '/') : '';
            if ($daemon_url === '') {
                return $metadata;
            }
            $file = get_attached_file($attachment_id);
            $url = wp_get_attachment_url($attachment_id);
            $mime = get_post_mime_type($attachment_id);
            $post_parent = (int)wp_get_post_parent_id($attachment_id);
            $width = isset($metadata['width']) ? (int)$metadata['width'] : null;
            $height = isset($metadata['height']) ? (int)$metadata['height'] : null;
            $filesize = ($file && file_exists($file)) ? (filesize($file) ?: null) : null;
            $upload_dir = wp_upload_dir();
            $base_dir = isset($upload_dir['basedir']) ? (string)$upload_dir['basedir'] : '';
            $base_url = isset($upload_dir['baseurl']) ? (string)$upload_dir['baseurl'] : '';
            $sizes = [];
            if (is_array($metadata) && !empty($metadata['sizes']) && is_array($metadata['sizes'])) {
                $rel_dir = '';
                if (!empty($metadata['file']) && is_string($metadata['file'])) {
                    $rel_dir = trim(dirname($metadata['file']), '/\\.');
                }
                foreach ($metadata['sizes'] as $name => $info) {
                    if (!is_array($info) || empty($info['file'])) {
                        continue;
                    }
                    $sz_file = (string)$info['file'];
                    $rel_path = $rel_dir !== '' ? ($rel_dir . '/' . $sz_file) : $sz_file;
                    $sizes[] = [
                        'size' => (string)$name,
                        'url' => $base_url ? rtrim($base_url, '/') . '/' . ltrim($rel_path, '/') : '',
                        'path' => $base_dir ? rtrim($base_dir, DIRECTORY_SEPARATOR) . DIRECTORY_SEPARATOR . ltrim(str_replace('/', DIRECTORY_SEPARATOR, $rel_path), DIRECTORY_SEPARATOR) : null,
                        'width' => isset($info['width']) ? (int)$info['width'] : null,
                        'height' => isset($info['height']) ? (int)$info['height'] : null,
                        'mime_type' => isset($info['mime-type']) ? (string)$info['mime-type'] : null,
                        'file_size' => (isset($info['filesize']) ? (int)$info['filesize'] : null),
                    ];
                }
            }
            $payload = [
                'attachment_id' => $attachment_id,
                'post_id' => $post_parent > 0 ? $post_parent : null,
                'file' => $file ?: null,
                'url' => $url ?: null,
                'mime_type' => $mime ?: null,
                'width' => $width,
                'height' => $height,
                'file_size' => $filesize,
                'sizes' => $sizes,
            ];
            $headers = ['Content-Type' => 'application/json'];
            $token = isset($settings['daemon_api_token']) ? (string)$settings['daemon_api_token'] : '';
            if ($token !== '') {
                $headers['Authorization'] = 'Bearer ' . $token;
            }
            wp_remote_post($daemon_url . '/api/webhooks/media-upload', [
                'headers' => $headers,
                'timeout' => 5,
                'blocking' => false,
                'body' => wp_json_encode($payload),
            ]);
        } catch (\Throwable $e) {
            // never break uploads
        }
        return $metadata;
    }

    public function renderer(): Renderer
    {
        return $this->make('renderer', function () {
            return new Renderer();
        });
    }

    public function menu(): Menu
    {
        return $this->make('menu', function (Plugin $plugin) {
            return new Menu($plugin->renderer());
        });
    }

    public function assets(): Assets
    {
        return $this->make('assets', function () {
            return new Assets();
        });
    }

    public function mediaLibrary(): MediaLibrary
    {
        return $this->make('media_library', function (Plugin $plugin) {
            return new MediaLibrary($plugin->imageRepository());
        });
    }

    public function settingsController(): Settings
    {
        return $this->make('settings_controller', function (Plugin $plugin) {
            return new Settings($plugin->settingsService());
        });
    }

    public static function activate(): void
    {
        $plugin = self::instance();
        $plugin->settingsService()->ensureDefaults();
        Schema::install();
    }

    public static function deactivate(): void
    {
    }

    public static function get_settings(): array
    {
        return self::instance()->settingsService()->all();
    }

    public static function update_settings(array $settings): void
    {
        self::instance()->settingsService()->save($settings);
    }
}