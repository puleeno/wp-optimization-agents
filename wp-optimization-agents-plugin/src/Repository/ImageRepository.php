<?php

namespace Puleeno\WPOptAgents\Repository;

use Puleeno\WPOptAgents\Contracts\ImageRepositoryInterface;
use Puleeno\WPOptAgents\Database\Schema;

class ImageRepository implements ImageRepositoryInterface
{
    /** @var \wpdb */
    private $wpdb;
    /** @var string */
    private $table_name;

    public function __construct($wpdb = null)
    {
        $this->wpdb = $wpdb ?? $GLOBALS['wpdb'];
        $this->table_name = Schema::get_images_table_name();
    }

    public static function hash_origin_url(string $origin_url): string
    {
        $normalized = self::normalize_origin_url($origin_url);
        return hash('sha256', $normalized);
    }

    public static function normalize_origin_url(string $origin_url): string
    {
        $parts = wp_parse_url($origin_url);
        if (!$parts || empty($parts['host'])) {
            return $origin_url;
        }
        $scheme = strtolower($parts['scheme'] ?? 'https');
        $host = strtolower($parts['host']);
        $path = $parts['path'] ?? '/';
        $query = isset($parts['query']) && $parts['query'] !== '' ? '?' . $parts['query'] : '';
        return $scheme . '://' . $host . $path . $query;
    }

    public function create(array $data): int|false
    {
        $defaults = [
            'attachment_id' => null,
            'post_id' => null,
            'origin_url' => '',
            'origin_url_hash' => '',
            'blogger_url' => null,
            'blogger_image_id' => null,
            'transformed_params' => null,
            'width' => null,
            'height' => null,
            'mime_type' => null,
            'original_size_bytes' => null,
            'offload_size_bytes' => null,
            'status' => 'pending',
            'redirect_active' => 0,
            'replaced_at' => null,
            'error_message' => null,
            'metadata' => null,
        ];

        $data = array_merge($defaults, $data);
        if (empty($data['origin_url_hash']) && !empty($data['origin_url'])) {
            $data['origin_url_hash'] = self::hash_origin_url((string)$data['origin_url']);
        }
        if (is_array($data['metadata'] ?? null)) {
            $data['metadata'] = wp_json_encode($data['metadata']);
        }

        $result = $this->wpdb->insert($this->table_name, $data);
        return $result ? (int)$this->wpdb->insert_id : false;
    }

    public function find_by_id(int $id): ?array
    {
        $sql = $this->wpdb->prepare("SELECT * FROM {$this->table_name} WHERE id = %d", $id);
        $row = $this->wpdb->get_row($sql, ARRAY_A);
        return $row ? $this->decode_row($row) : null;
    }

    public function find_by_post_id(int $post_id): array
    {
        $sql = $this->wpdb->prepare(
            "SELECT * FROM {$this->table_name} WHERE post_id = %d ORDER BY created_at ASC",
            $post_id
        );
        $rows = $this->wpdb->get_results($sql, ARRAY_A) ?: [];
        return array_map([$this, 'decode_row'], $rows);
    }

    public function find_by_origin_url(string $origin_url): ?array
    {
        $hash = self::hash_origin_url($origin_url);
        return $this->find_by_origin_url_hash($hash);
    }

    public function find_by_origin_url_hash(string $hash): ?array
    {
        $sql = $this->wpdb->prepare("SELECT * FROM {$this->table_name} WHERE origin_url_hash = %s LIMIT 1", $hash);
        $row = $this->wpdb->get_row($sql, ARRAY_A);
        return $row ? $this->decode_row($row) : null;
    }

    public function find_active_redirect_by_origin_url(string $origin_url): ?array
    {
        $hash = self::hash_origin_url($origin_url);
        $sql = $this->wpdb->prepare(
            "SELECT * FROM {$this->table_name} WHERE origin_url_hash = %s AND redirect_active = 1 AND status = 'replaced' AND blogger_url IS NOT NULL AND blogger_url <> '' LIMIT 1",
            $hash
        );
        $row = $this->wpdb->get_row($sql, ARRAY_A);
        return $row ? $this->decode_row($row) : null;
    }

    public function update(int $id, array $data): bool
    {
        $data['updated_at'] = current_time('mysql');
        if (isset($data['metadata']) && is_array($data['metadata'])) {
            $data['metadata'] = wp_json_encode($data['metadata']);
        }
        $result = $this->wpdb->update($this->table_name, $data, ['id' => $id]);
        return $result !== false;
    }

    public function update_status(int $id, string $status, ?string $error = null): bool
    {
        $data = ['status' => $status];
        if ($error !== null) {
            $data['error_message'] = $error;
        }
        return $this->update($id, $data);
    }

    public function mark_replaced(int $id, bool $redirect_active = true): bool
    {
        return $this->update($id, [
            'status' => 'replaced',
            'redirect_active' => $redirect_active ? 1 : 0,
            'replaced_at' => current_time('mysql'),
            'error_message' => null,
        ]);
    }

    public function delete(int $id): bool
    {
        $result = $this->wpdb->delete($this->table_name, ['id' => $id]);
        return $result !== false;
    }

    public function get_pending_count(int $post_id): int
    {
        $sql = $this->wpdb->prepare(
            "SELECT COUNT(*) FROM {$this->table_name} WHERE post_id = %d AND status IN ('pending','uploading','uploaded','replacing')",
            $post_id
        );
        return (int)$this->wpdb->get_var($sql);
    }

    public function get_stats(): array
    {
        $sql = "SELECT status, COUNT(*) as count FROM {$this->table_name} GROUP BY status";
        $results = $this->wpdb->get_results($sql, ARRAY_A) ?: [];
        $stats = [
            'total' => 0,
            'pending' => 0,
            'uploading' => 0,
            'uploaded' => 0,
            'replacing' => 0,
            'replaced' => 0,
            'failed' => 0,
        ];
        foreach ($results as $row) {
            $status = $row['status'];
            $count = (int)$row['count'];
            $stats[$status] = ($stats[$status] ?? 0) + $count;
            $stats['total'] += $count;
        }
        return $stats;
    }

    private function decode_row(array $row): array
    {
        if (!empty($row['metadata']) && is_string($row['metadata'])) {
            $decoded = json_decode($row['metadata'], true);
            $row['metadata'] = is_array($decoded) ? $decoded : $row['metadata'];
        }
        $row['redirect_active'] = (int)($row['redirect_active'] ?? 0);
        return $row;
    }
}