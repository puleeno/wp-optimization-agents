<?php

namespace Puleeno\WPOptAgents\Repository;

use Puleeno\WPOptAgents\Contracts\ImageRepositoryInterface;
use Puleeno\WPOptAgents\Database\Schema;
use wpdb;

class ImageRepository implements ImageRepositoryInterface
{
    private wpdb $wpdb;
    private string $table_name;

    public function __construct(?wpdb $wpdb = null)
    {
        $this->wpdb = $wpdb ?? $GLOBALS['wpdb'];
        $this->table_name = Schema::get_table_name();
    }

    public function create(array $data): int|false
    {
        $defaults = [
            'post_id' => 0,
            'origin_url' => '',
            'blogger_url' => null,
            'blogger_image_id' => null,
            'transformed_params' => null,
            'width' => null,
            'height' => null,
            'mime_type' => null,
            'original_size_bytes' => null,
            'offload_size_bytes' => null,
            'status' => 'pending',
            'error_message' => null,
        ];

        $data = array_merge($defaults, $data);
        $result = $this->wpdb->insert($this->table_name, $data);
        return $result ? (int)$this->wpdb->insert_id : false;
    }

    public function find_by_id(int $id): ?array
    {
        $sql = $this->wpdb->prepare("SELECT * FROM {$this->table_name} WHERE id = %d", $id);
        $result = $this->wpdb->get_row($sql, ARRAY_A);
        return $result ?: null;
    }

    public function find_by_post_id(int $post_id): array
    {
        $sql = $this->wpdb->prepare(
            "SELECT * FROM {$this->table_name} WHERE post_id = %d ORDER BY created_at ASC",
            $post_id
        );
        $results = $this->wpdb->get_results($sql, ARRAY_A);
        return $results ?: [];
    }

    public function find_by_origin_url(string $origin_url): ?array
    {
        $sql = $this->wpdb->prepare("SELECT * FROM {$this->table_name} WHERE origin_url = %s LIMIT 1", $origin_url);
        $result = $this->wpdb->get_row($sql, ARRAY_A);
        return $result ?: null;
    }

    public function update(int $id, array $data): bool
    {
        $data['updated_at'] = current_time('mysql');
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

    public function delete(int $id): bool
    {
        $result = $this->wpdb->delete($this->table_name, ['id' => $id]);
        return $result !== false;
    }

    public function get_pending_count(int $post_id): int
    {
        $sql = $this->wpdb->prepare(
            "SELECT COUNT(*) FROM {$this->table_name} WHERE post_id = %d AND status IN ('pending', 'processing')",
            $post_id
        );
        return (int)$this->wpdb->get_var($sql);
    }

    public function get_stats(): array
    {
        $sql = "SELECT status, COUNT(*) as count FROM {$this->table_name} GROUP BY status";
        $results = $this->wpdb->get_results($sql, ARRAY_A);
        $stats = [
            'total' => 0,
            'pending' => 0,
            'processing' => 0,
            'synced' => 0,
            'failed' => 0,
        ];
        foreach ($results as $row) {
            $stats[$row['status']] = (int)$row['count'];
            $stats['total'] += (int)$row['count'];
        }
        return $stats;
    }
}