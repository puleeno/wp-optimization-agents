<?php

namespace Puleeno\WPOptAgents;

use Puleeno\WPOptAgents\Admin\Assets;
use Puleeno\WPOptAgents\Admin\Menu;
use Puleeno\WPOptAgents\Admin\Pages\Renderer;
use Puleeno\WPOptAgents\Admin\Settings;
use Puleeno\WPOptAgents\Contracts\SettingsRepositoryInterface;
use Puleeno\WPOptAgents\Database\Schema;
use Puleeno\WPOptAgents\Repository\SettingsRepository;
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

        if (is_admin()) {
            $this->menu()->register();
            $this->assets()->register();
            $this->settingsController()->register();
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

    public function restApi(): RestApi
    {
        return $this->make('rest_api', function (Plugin $plugin) {
            return new RestApi($plugin->settingsService());
        });
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