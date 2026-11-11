<?php

namespace Puleeno\WPOptAgents\Repository;

use Puleeno\WPOptAgents\Contracts\SettingsRepositoryInterface;

class SettingsRepository implements SettingsRepositoryInterface
{
    private const OPTION_NAME = 'wp_opt_agents_settings';

    public function get(string $key, $default = null)
    {
        $settings = get_option(self::OPTION_NAME, []);
        return $settings[$key] ?? $default;
    }

    public function set(string $key, $value): bool
    {
        $settings = get_option(self::OPTION_NAME, []);
        $settings[$key] = $value;
        return update_option(self::OPTION_NAME, $settings);
    }

    public function get_all(): array
    {
        return get_option(self::OPTION_NAME, []);
    }

    public function set_all(array $settings): bool
    {
        return update_option(self::OPTION_NAME, $settings);
    }

    public function delete(string $key): bool
    {
        $settings = get_option(self::OPTION_NAME, []);
        if (!array_key_exists($key, $settings)) {
            return false;
        }
        unset($settings[$key]);
        return update_option(self::OPTION_NAME, $settings);
    }
}