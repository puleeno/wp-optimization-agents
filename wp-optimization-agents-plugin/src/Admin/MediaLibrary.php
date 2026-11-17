<?php

namespace Puleeno\WPOptAgents\Admin;

use Puleeno\WPOptAgents\Repository\ImageRepository;

class MediaLibrary
{
    /** @var ImageRepository */
    private $imageRepository;

    public function __construct(?ImageRepository $imageRepository = null)
    {
        $this->imageRepository = $imageRepository ?? new ImageRepository();
    }

    public function register(): void
    {
        add_filter('manage_media_columns', [$this, 'add_columns']);
        add_filter('manage_upload_columns', [$this, 'add_columns']);
        add_action('manage_media_custom_column', [$this, 'render_column'], 10, 2);
    }

    /**
     * @param array<string,string> $columns
     * @return array<string,string>
     */
    public function add_columns(array $columns): array
    {
        $new = [];
        foreach ($columns as $key => $label) {
            $new[$key] = $label;
            if ($key === 'parent') {
                $new['wpoa_imgbb'] = __('ImgBB Upload', 'wp-optimization-agents');
                $new['wpoa_blogger'] = __('Blogger Upload', 'wp-optimization-agents');
            }
        }
        if (!isset($new['wpoa_imgbb'])) {
            $new['wpoa_imgbb'] = __('ImgBB Upload', 'wp-optimization-agents');
            $new['wpoa_blogger'] = __('Blogger Upload', 'wp-optimization-agents');
        }
        return $new;
    }

    public function render_column(string $column_name, int $post_id): void
    {
        if (!in_array($column_name, ['wpoa_imgbb', 'wpoa_blogger'], true)) {
            return;
        }
        if (!wp_attachment_is_image($post_id)) {
            echo '<span class="wpoa-upload-status wpoa-status-na">—</span>';
            return;
        }

        $rows = $this->imageRepository->find_by_attachment_id($post_id);
        if (empty($rows)) {
            echo $this->render_badge('none', $column_name === 'wpoa_imgbb' ? 'Chưa upload' : 'Chưa upload');
            return;
        }

        if ($column_name === 'wpoa_imgbb') {
            echo $this->render_imgbb_status($rows);
            return;
        }
        echo $this->render_blogger_status($rows);
    }

    /**
     * @param array<int,array<string,mixed>> $rows
     */
    private function render_imgbb_status(array $rows): string
    {
        $status = $this->aggregate_imgbb_status($rows);
        $url = '';
        foreach ($rows as $row) {
            if (!empty($row['imgbb_url'])) {
                $url = (string) $row['imgbb_url'];
                break;
            }
        }
        $meta = $this->build_meta($rows, 'imgbb');
        $badge = $this->render_badge($status, $this->label_for_imgbb($status));
        if ($url !== '') {
            $badge .= '<div class="wpoa-upload-meta"><a href="' . esc_url($url) . '" target="_blank" rel="noopener noreferrer">ImgBB</a>' . $meta . '</div>';
        } elseif ($meta !== '') {
            $badge .= '<div class="wpoa-upload-meta">' . ltrim($meta, ' · ') . '</div>';
        }
        return $badge;
    }

    /**
     * @param array<int,array<string,mixed>> $rows
     */
    private function render_blogger_status(array $rows): string
    {
        $status = $this->aggregate_blogger_status($rows);
        $url = '';
        foreach ($rows as $row) {
            if (!empty($row['blogger_url'])) {
                $url = (string) $row['blogger_url'];
                break;
            }
        }
        $meta = $this->build_meta($rows, 'blogger');
        $badge = $this->render_badge($status, $this->label_for_blogger($status));
        if ($url !== '') {
            $badge .= '<div class="wpoa-upload-meta"><a href="' . esc_url($url) . '" target="_blank" rel="noopener noreferrer">Blogger</a>' . $meta . '</div>';
        } elseif ($meta !== '') {
            $badge .= '<div class="wpoa-upload-meta">' . ltrim($meta, ' · ') . '</div>';
        }
        return $badge;
    }

    /**
     * @param array<int,array<string,mixed>> $rows
     */
    private function aggregate_imgbb_status(array $rows): string
    {
        $statuses = [];
        foreach ($rows as $row) {
            $s = (string) ($row['imgbb_status'] ?? 'none');
            if ($s === '') $s = 'none';
            $statuses[$s] = true;
            if (!empty($row['imgbb_url']) && ($s === 'none' || $s === 'pending')) {
                $statuses['uploaded'] = true;
            }
        }
        if (isset($statuses['failed'])) return 'failed';
        if (isset($statuses['uploading']) || isset($statuses['processing'])) return 'uploading';
        if (isset($statuses['uploaded'])) return 'uploaded';
        if (isset($statuses['pending'])) return 'pending';
        return 'none';
    }

    /**
     * @param array<int,array<string,mixed>> $rows
     */
    private function aggregate_blogger_status(array $rows): string
    {
        $hasReplaced = false; $hasUploaded = false; $hasProcessing = false; $hasFailed = false; $hasPending = false;
        foreach ($rows as $row) {
            $s = (string) ($row['status'] ?? 'pending');
            if ($s === 'replaced') $hasReplaced = true;
            elseif ($s === 'uploaded') $hasUploaded = true;
            elseif (in_array($s, ['uploading', 'replacing'], true)) $hasProcessing = true;
            elseif ($s === 'failed') $hasFailed = true;
            elseif ($s === 'pending') $hasPending = true;
        }
        if ($hasFailed) return 'failed';
        if ($hasReplaced) return 'replaced';
        if ($hasProcessing) return 'uploading';
        if ($hasUploaded) return 'uploaded';
        if ($hasPending) return 'pending';
        return 'none';
    }

    /**
     * @param array<int,array<string,mixed>> $rows
     */
    private function build_meta(array $rows, string $provider): string
    {
        $count = count($rows);
        $parts = [];
        if ($count > 1) {
            $parts[] = $count . ' bản ghi';
        }
        if ($provider === 'blogger') {
            $replaced = 0;
            foreach ($rows as $row) {
                if (($row['status'] ?? '') === 'replaced') $replaced++;
            }
            if ($count > 0 && $replaced > 0 && $replaced < $count) {
                $parts[] = $replaced . '/' . $count . ' replaced';
            }
        }
        return $parts ? ' · ' . implode(' · ', $parts) : '';
    }

    private function render_badge(string $status, string $label): string
    {
        $status = sanitize_html_class($status ?: 'none');
        return '<span class="wpoa-upload-status wpoa-status-' . esc_attr($status) . '">' . esc_html($label) . '</span>';
    }

    private function label_for_imgbb(string $status): string
    {
        $map = [
            'none' => 'Chưa upload',
            'pending' => 'Chờ ImgBB',
            'uploading' => 'Đang upload ImgBB',
            'processing' => 'Đang upload ImgBB',
            'uploaded' => 'Đã lên ImgBB',
            'failed' => 'Lỗi ImgBB',
        ];
        return $map[$status] ?? $status;
    }

    private function label_for_blogger(string $status): string
    {
        $map = [
            'none' => 'Chưa upload',
            'pending' => 'Chờ Blogger',
            'uploading' => 'Đang xử lý Blogger',
            'uploaded' => 'Đã lên Blogger',
            'replaced' => 'Đã thay URL/301',
            'failed' => 'Lỗi Blogger',
        ];
        return $map[$status] ?? $status;
    }
}
