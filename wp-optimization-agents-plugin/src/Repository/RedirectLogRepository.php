<?php

namespace Puleeno\WPOptAgents\Repository;

use Puleeno\WPOptAgents\Contracts\RedirectLogRepositoryInterface;
use Puleeno\WPOptAgents\Database\Schema;

class RedirectLogRepository implements RedirectLogRepositoryInterface
{
    /** @var \wpdb */
    private $wpdb;
    /** @var string */
    private $table_name;

    public function __construct($wpdb = null)
    {
        $this->wpdb = $wpdb ?? $GLOBALS['wpdb'];
        $this->table_name = Schema::get_redirect_logs_table_name();
    }

    public function log(array $data): int|false
    {
        $defaults = [
            'image_id' => null,
            'origin_url' => '',
            'origin_url_hash' => '',
            'target_url' => '',
            'request_uri' => null,
            'referer' => null,
            'user_agent' => null,
            'ip_hash' => null,
            'status_code' => 301,
        ];
        $row = array_merge($defaults, $data);

        if (empty($row['origin_url_hash']) && !empty($row['origin_url'])) {
            $row['origin_url_hash'] = hash('sha256', $row['origin_url']);
        }
        if (!empty($row['ip_hash']) && strlen((string)$row['ip_hash']) > 64) {
            $row['ip_hash'] = hash('sha256', (string)$row['ip_hash']);
        }

        $result = $this->wpdb->insert($this->table_name, $row);
        return $result ? (int)$this->wpdb->insert_id : false;
    }

    public function get_daily_stats(int $days = 30): array
    {
        $days = max(1, min(365, $days));
        $sql = $this->wpdb->prepare(
            "SELECT DATE(created_at) AS day, COUNT(*) AS hits
             FROM {$this->table_name}
             WHERE created_at >= DATE_SUB(NOW(), INTERVAL %d DAY)
             GROUP BY DATE(created_at)
             ORDER BY day ASC",
            $days
        );
        $rows = $this->wpdb->get_results($sql, ARRAY_A) ?: [];

        // Fill missing days with 0 for chart continuity
        $map = [];
        foreach ($rows as $r) {
            $map[$r['day']] = (int)$r['hits'];
        }
        $result = [];
        $start = new \DateTimeImmutable("-" . ($days - 1) . " days", wp_timezone());
        $end = new \DateTimeImmutable("now", wp_timezone());
        for ($d = $start; $d <= $end; $d = $d->modify('+1 day')) {
            $key = $d->format('Y-m-d');
            $result[] = [
                'day' => $key,
                'hits' => $map[$key] ?? 0,
            ];
        }
        return $result;
    }

    public function get_top_redirects(int $limit = 10, int $days = 30): array
    {
        $limit = max(1, min(100, $limit));
        $days = max(1, min(365, $days));
        $sql = $this->wpdb->prepare(
            "SELECT origin_url, target_url, COUNT(*) AS hits, MAX(created_at) AS last_hit
             FROM {$this->table_name}
             WHERE created_at >= DATE_SUB(NOW(), INTERVAL %d DAY)
             GROUP BY origin_url_hash, origin_url, target_url
             ORDER BY hits DESC
             LIMIT %d",
            $days,
            $limit
        );
        return $this->wpdb->get_results($sql, ARRAY_A) ?: [];
    }

    public function get_total_count(int $days = 0): int
    {
        if ($days > 0) {
            $sql = $this->wpdb->prepare(
                "SELECT COUNT(*) FROM {$this->table_name} WHERE created_at >= DATE_SUB(NOW(), INTERVAL %d DAY)",
                $days
            );
            return (int)$this->wpdb->get_var($sql);
        }
        $sql = "SELECT COUNT(*) FROM {$this->table_name}";
        return (int)$this->wpdb->get_var($sql);
    }
}
