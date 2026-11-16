<?php

namespace Puleeno\WPOptAgents\Contracts;

interface RedirectLogRepositoryInterface
{
    public function log(array $data): int|false;
    public function get_daily_stats(int $days = 30): array;
    public function get_top_redirects(int $limit = 10, int $days = 30): array;
    public function get_total_count(int $days = 0): int;
}
