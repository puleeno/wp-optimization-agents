<?php

namespace Puleeno\WPOptAgents\Contracts;

interface SettingsRepositoryInterface
{
    public function get(string $key, $default = null);
    public function set(string $key, $value): bool;
    public function get_all(): array;
    public function set_all(array $settings): bool;
    public function delete(string $key): bool;
}