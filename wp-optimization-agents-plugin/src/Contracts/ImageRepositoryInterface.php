<?php

namespace Puleeno\WPOptAgents\Contracts;

interface ImageRepositoryInterface
{
    public function create(array $data): int|false;
    public function find_by_id(int $id): ?array;
    public function find_by_post_id(int $post_id): array;
    public function find_by_origin_url(string $origin_url): ?array;
    public function find_by_origin_url_hash(string $hash): ?array;
    public function find_active_redirect_by_origin_url(string $origin_url): ?array;
    public function update(int $id, array $data): bool;
    public function update_status(int $id, string $status, ?string $error = null): bool;
    public function mark_replaced(int $id, bool $redirect_active = true): bool;
    public function delete(int $id): bool;
    public function get_pending_count(int $post_id): int;
    public function get_stats(): array;
}