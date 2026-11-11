<?php

namespace Puleeno\WPOptAgents\Api;

use Puleeno\WPOptAgents\Contracts\ApiResponse;
use Puleeno\WPOptAgents\Service\SettingsService;
use Puleeno\WPOptAgents\Service\OffloadService;
use WP_REST_Request;
use WP_REST_Response;
use WP_Error;

class RestApi
{
    private SettingsService $settingsService;
    private OffloadService $offloadService;

    public function __construct(SettingsService $settingsService)
    {
        $this->settingsService = $settingsService;
        $this->offloadService = new OffloadService(
            new \Puleeno\WPOptAgents\Repository\ImageRepository(),
            new \Puleeno\WPOptAgents\Repository\SettingsRepository()
        );
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

        register_rest_route($namespace, '/posts/(?P<id>\d+)/images', [
            'methods' => 'GET',
            'callback' => [$this, 'get_post_images'],
            'permission_callback' => [$this, 'check_daemon_permission'],
        ]);

        register_rest_route($namespace, '/images', [
            'methods' => 'POST',
            'callback' => [$this, 'register_images'],
            'permission_callback' => [$this, 'check_daemon_permission'],
        ]);

        register_rest_route($namespace, '/images/(?P<id>\d+)/sync', [
            'methods' => 'POST',
            'callback' => [$this, 'mark_image_synced'],
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

        register_rest_route($namespace, '/posts/(?P<id>\d+)/status', [
            'methods' => 'GET',
            'callback' => [$this, 'get_post_status'],
            'permission_callback' => [$this, 'check_daemon_permission'],
        ]);

        register_rest_route($namespace, '/stats', [
            'methods' => 'GET',
            'callback' => [$this, 'get_stats'],
            'permission_callback' => [$this, 'check_daemon_permission'],
        ]);
    }

    public function check_daemon_permission(WP_REST_Request $request): bool|WP_Error
    {
        $auth_header = $request->get_header('Authorization');
        if (!$auth_header) {
            return new WP_Error('missing_auth', 'Authorization header required', ['status' => 401]);
        }

        if (!preg_match('/^Bearer\s+(.+)$/i', $auth_header, $matches)) {
            return new WP_Error('invalid_auth', 'Invalid authorization format', ['status' => 401]);
        }

        $token = $matches[1];
        $expected_token = $this->settingsService->get('daemon_api_token', '');

        if (!$expected_token || !hash_equals($expected_token, $token)) {
            return new WP_Error('invalid_token', 'Invalid API token', ['status' => 401]);
        }

        return true;
    }

    public function get_config(WP_REST_Request $request): WP_REST_Response
    {
        $config = $this->settingsService->get_rest_api_config();
        return $this->respond(ApiResponse::success($config));
    }

    public function scan_post(WP_REST_Request $request): WP_REST_Response
    {
        $post_id = (int)$request->get_param('id');
        $images = $this->offloadService->scan_post_images($post_id);
        return $this->respond(ApiResponse::success(['images' => $images]));
    }

    public function get_post_images(WP_REST_Request $request): WP_REST_Response
    {
        $post_id = (int)$request->get_param('id');
        $status = $this->offloadService->get_post_offload_status($post_id);
        return $this->respond(ApiResponse::success($status));
    }

    public function register_images(WP_REST_Request $request): WP_REST_Response
    {
        $post_id = (int)$request->get_param('post_id');
        $images = $request->get_json_params();

        if (!isset($images['images']) || !is_array($images['images'])) {
            return $this->respond(ApiResponse::error('Invalid images data', 400));
        }

        $registered = $this->offloadService->register_images_for_offload($post_id, $images['images']);
        return $this->respond(ApiResponse::success(['registered' => $registered]));
    }

    public function mark_image_synced(WP_REST_Request $request): WP_REST_Response
    {
        $image_id = (int)$request->get_param('id');
        $params = $request->get_json_params();

        $required = ['blogger_url', 'blogger_image_id', 'transformed_params'];
        foreach ($required as $field) {
            if (!isset($params[$field])) {
                return $this->respond(ApiResponse::error("Missing required field: {$field}", 400));
            }
        }

        $meta = [
            'width' => $params['width'] ?? null,
            'height' => $params['height'] ?? null,
            'mime_type' => $params['mime_type'] ?? null,
            'original_size_bytes' => $params['original_size_bytes'] ?? null,
            'offload_size_bytes' => $params['offload_size_bytes'] ?? null,
        ];

        $result = $this->offloadService->mark_image_synced(
            $image_id,
            $params['blogger_url'],
            $params['blogger_image_id'],
            $params['transformed_params'],
            $meta
        );

        return $this->respond($result
            ? ApiResponse::success(['synced' => true])
            : ApiResponse::error('Failed to update image', 500)
        );
    }

    public function mark_image_failed(WP_REST_Request $request): WP_REST_Response
    {
        $image_id = (int)$request->get_param('id');
        $params = $request->get_json_params();

        if (!isset($params['error'])) {
            return $this->respond(ApiResponse::error('Missing error message', 400));
        }

        $result = $this->offloadService->mark_image_failed($image_id, $params['error']);
        return $this->respond($result
            ? ApiResponse::success(['failed' => true])
            : ApiResponse::error('Failed to update image', 500)
        );
    }

    public function replace_post_content(WP_REST_Request $request): WP_REST_Response
    {
        $post_id = (int)$request->get_param('id');
        $params = $request->get_json_params();

        if (!isset($params['replacements']) || !is_array($params['replacements'])) {
            return $this->respond(ApiResponse::error('Invalid replacements data', 400));
        }

        $result = $this->offloadService->replace_post_content($post_id, $params['replacements']);
        return $this->respond($result
            ? ApiResponse::success(['replaced' => true])
            : ApiResponse::error('Failed to replace content', 500)
        );
    }

    public function get_post_status(WP_REST_Request $request): WP_REST_Response
    {
        $post_id = (int)$request->get_param('id');
        $status = $this->offloadService->get_post_offload_status($post_id);
        return $this->respond(ApiResponse::success($status));
    }

    public function get_stats(WP_REST_Request $request): WP_REST_Response
    {
        $repo = new \Puleeno\WPOptAgents\Repository\ImageRepository();
        $stats = $repo->get_stats();
        return $this->respond(ApiResponse::success($stats));
    }

    private function respond(ApiResponse $response): WP_REST_Response
    {
        return new WP_REST_Response($response->to_array(), $response->get_status_code());
    }
}