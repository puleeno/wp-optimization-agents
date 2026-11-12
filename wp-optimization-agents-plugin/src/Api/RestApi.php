<?php

namespace Puleeno\WPOptAgents\Api;

use Puleeno\WPOptAgents\Contracts\ApiResponse;
use Puleeno\WPOptAgents\Service\OffloadService;
use Puleeno\WPOptAgents\Service\SettingsService;
use WP_Error;
use WP_REST_Request;
use WP_REST_Response;

class RestApi
{
    /** @var SettingsService */
    private $settingsService;
    /** @var OffloadService */
    private $offloadService;

    public function __construct(SettingsService $settingsService, OffloadService $offloadService)
    {
        $this->settingsService = $settingsService;
        $this->offloadService = $offloadService;
    }

    public function register_routes(): void
    {
        $namespace = 'wp-opt-agents/v1';

        register_rest_route($namespace, '/config', [
            'methods' => 'GET',
            'callback' => [$this, 'get_config'],
            'permission_callback' => [$this, 'check_daemon_permission'],
        ]);

        register_rest_route($namespace, '/posts/(?P<id>\d+)/scan', [
            'methods' => 'POST',
            'callback' => [$this, 'scan_post'],
            'permission_callback' => [$this, 'check_daemon_permission'],
        ]);

        register_rest_route($namespace, '/posts/(?P<id>\d+)/status', [
            'methods' => 'GET',
            'callback' => [$this, 'get_post_status'],
            'permission_callback' => [$this, 'check_daemon_permission'],
        ]);

        register_rest_route($namespace, '/posts/(?P<id>\d+)/link-blogger', [
            'methods' => 'POST',
            'callback' => [$this, 'link_blogger_post'],
            'permission_callback' => [$this, 'check_daemon_permission'],
        ]);

        register_rest_route($namespace, '/images/register', [
            'methods' => 'POST',
            'callback' => [$this, 'register_images'],
            'permission_callback' => [$this, 'check_daemon_permission'],
        ]);

        register_rest_route($namespace, '/images/(?P<id>\d+)/uploaded', [
            'methods' => 'POST',
            'callback' => [$this, 'mark_image_uploaded'],
            'permission_callback' => [$this, 'check_daemon_permission'],
        ]);

        register_rest_route($namespace, '/images/(?P<id>\d+)/replaced', [
            'methods' => 'POST',
            'callback' => [$this, 'mark_image_replaced'],
            'permission_callback' => [$this, 'check_daemon_permission'],
        ]);

        register_rest_route($namespace, '/images/(?P<id>\d+)/fail', [
            'methods' => 'POST',
            'callback' => [$this, 'mark_image_failed'],
            'permission_callback' => [$this, 'check_daemon_permission'],
        ]);

        register_rest_route($namespace, '/posts/(?P<id>\d+)/replace-content', [
            'methods' => 'POST',
            'callback' => [$this, 'replace_post_content'],
            'permission_callback' => [$this, 'check_daemon_permission'],
        ]);

        register_rest_route($namespace, '/stats', [
            'methods' => 'GET',
            'callback' => [$this, 'get_stats'],
            'permission_callback' => [$this, 'check_daemon_permission'],
        ]);

        // Compatibility/legacy aliases
        register_rest_route($namespace, '/posts/(?P<id>\d+)/images', [
            'methods' => 'GET',
            'callback' => [$this, 'get_post_status'],
            'permission_callback' => [$this, 'check_daemon_permission'],
        ]);
        register_rest_route($namespace, '/images', [
            'methods' => 'POST',
            'callback' => [$this, 'register_images'],
            'permission_callback' => [$this, 'check_daemon_permission'],
        ]);
        register_rest_route($namespace, '/images/(?P<id>\d+)/sync', [
            'methods' => 'POST',
            'callback' => [$this, 'mark_image_uploaded'],
            'permission_callback' => [$this, 'check_daemon_permission'],
        ]);
    }

    /**
     * Daemon authenticates with plugin token (compatibility endpoint). Rust agent may also read WP DB directly.
     * @return bool|WP_Error
     */
    public function check_daemon_permission(WP_REST_Request $request)
    {
        $auth_header = $request->get_header('Authorization');
        if (!$auth_header) {
            return new WP_Error('missing_auth', 'Authorization header required', ['status' => 401]);
        }
        if (!preg_match('/^Bearer\s+(.+)$/i', $auth_header, $matches)) {
            return new WP_Error('invalid_auth', 'Invalid authorization format', ['status' => 401]);
        }
        $token = $matches[1];
        $expected_token = (string)$this->settingsService->get('daemon_api_token', '');
        if ($expected_token === '' || !hash_equals($expected_token, $token)) {
            return new WP_Error('invalid_token', 'Invalid API token', ['status' => 401]);
        }
        return true;
    }

    public function get_config(WP_REST_Request $request): WP_REST_Response
    {
        return $this->respond(ApiResponse::success($this->settingsService->get_rest_api_config()));
    }

    public function scan_post(WP_REST_Request $request): WP_REST_Response
    {
        $post_id = (int)$request->get_param('id');
        $images = $this->offloadService->scan_post_images($post_id);
        return $this->respond(ApiResponse::success(['post_id' => $post_id, 'images' => $images]));
    }

    public function get_post_status(WP_REST_Request $request): WP_REST_Response
    {
        $post_id = (int)$request->get_param('id');
        return $this->respond(ApiResponse::success($this->offloadService->get_post_offload_status($post_id)));
    }

    public function link_blogger_post(WP_REST_Request $request): WP_REST_Response
    {
        $post_id = (int)$request->get_param('id');
        $params = $request->get_json_params() ?: [];
        $blogger_post_id = isset($params['blogger_post_id']) ? (string)$params['blogger_post_id'] : '';
        $blogger_post_url = isset($params['blogger_post_url']) ? (string)$params['blogger_post_url'] : '';
        $blogger_album_url = isset($params['blogger_album_url']) ? (string)$params['blogger_album_url'] : '';
        $blogger_blog_id = isset($params['blogger_blog_id']) ? (string)$params['blogger_blog_id'] : '';

        if ($blogger_post_id === '' && $blogger_post_url === '') {
            return $this->respond(ApiResponse::error('blogger_post_id or blogger_post_url is required', 400));
        }

        $ok = $this->offloadService->link_blogger_post($post_id, [
            'blogger_post_id' => $blogger_post_id,
            'blogger_post_url' => $blogger_post_url,
            'blogger_album_url' => $blogger_album_url,
            'blogger_blog_id' => $blogger_blog_id,
        ]);

        return $this->respond($ok
            ? ApiResponse::success(['linked' => true, 'post_id' => $post_id])
            : ApiResponse::error('Failed to link blogger post', 500));
    }

    public function register_images(WP_REST_Request $request): WP_REST_Response
    {
        $params = $request->get_json_params() ?: [];
        $post_id = (int)($request->get_param('post_id') ?: ($params['post_id'] ?? 0));
        $images = $params['images'] ?? null;
        if (!is_array($images)) {
            return $this->respond(ApiResponse::error('Invalid images data', 400));
        }
        $registered = $this->offloadService->register_images_for_offload($post_id, $images);
        return $this->respond(ApiResponse::success(['post_id' => $post_id, 'registered' => $registered]));
    }

    public function mark_image_uploaded(WP_REST_Request $request): WP_REST_Response
    {
        $image_id = (int)$request->get_param('id');
        $params = $request->get_json_params() ?: [];
        $blogger_url = (string)($params['blogger_url'] ?? '');
        $blogger_image_id = (string)($params['blogger_image_id'] ?? '');
        $transformed_params = (string)($params['transformed_params'] ?? '');
        if ($blogger_url === '') {
            return $this->respond(ApiResponse::error('blogger_url is required', 400));
        }
        $meta = [
            'width' => $params['width'] ?? null,
            'height' => $params['height'] ?? null,
            'mime_type' => $params['mime_type'] ?? null,
            'original_size_bytes' => $params['original_size_bytes'] ?? null,
            'offload_size_bytes' => $params['offload_size_bytes'] ?? null,
        ];
        $result = $this->offloadService->mark_image_uploaded($image_id, $blogger_url, $blogger_image_id, $transformed_params, $meta);
        return $this->respond($result
            ? ApiResponse::success(['uploaded' => true, 'image_id' => $image_id])
            : ApiResponse::error('Failed to update image', 500));
    }

    public function mark_image_replaced(WP_REST_Request $request): WP_REST_Response
    {
        $image_id = (int)$request->get_param('id');
        $result = $this->offloadService->mark_image_replaced($image_id);
        return $this->respond($result
            ? ApiResponse::success(['replaced' => true, 'redirect_active' => true, 'image_id' => $image_id])
            : ApiResponse::error('Failed to mark image replaced', 500));
    }

    public function mark_image_failed(WP_REST_Request $request): WP_REST_Response
    {
        $image_id = (int)$request->get_param('id');
        $params = $request->get_json_params() ?: [];
        $error = (string)($params['error'] ?? $params['message'] ?? 'Unknown error');
        $result = $this->offloadService->mark_image_failed($image_id, $error);
        return $this->respond($result
            ? ApiResponse::success(['failed' => true, 'image_id' => $image_id])
            : ApiResponse::error('Failed to update image', 500));
    }

    public function replace_post_content(WP_REST_Request $request): WP_REST_Response
    {
        $post_id = (int)$request->get_param('id');
        $params = $request->get_json_params() ?: [];
        $replacements = $params['replacements'] ?? null;
        if (!is_array($replacements)) {
            return $this->respond(ApiResponse::error('Invalid replacements data', 400));
        }
        $result = $this->offloadService->replace_post_content($post_id, $replacements);
        if (!empty($result['success'])) {
            return $this->respond(ApiResponse::success($result));
        }
        return $this->respond(ApiResponse::error($result['error'] ?? 'Failed to replace content', 500));
    }

    public function get_stats(WP_REST_Request $request): WP_REST_Response
    {
        $repo = new \Puleeno\WPOptAgents\Repository\ImageRepository();
        return $this->respond(ApiResponse::success($repo->get_stats()));
    }

    private function respond(ApiResponse $response): WP_REST_Response
    {
        return new WP_REST_Response($response->to_array(), $response->get_status_code());
    }
}