<?php

namespace Puleeno\WPOptAgents\Contracts;

interface ApiResponseInterface
{
    public function to_array(): array;
    public function get_status_code(): int;
}

class ApiResponse implements ApiResponseInterface
{
    private bool $success;
    private mixed $data;
    private ?string $message;
    private int $status_code;

    public function __construct(bool $success, mixed $data = null, ?string $message = null, int $status_code = 200)
    {
        $this->success = $success;
        $this->data = $data;
        $this->message = $message;
        $this->status_code = $success ? $status_code : ($status_code ?: 400);
    }

    public static function success(mixed $data = null, ?string $message = null, int $status_code = 200): self
    {
        return new self(true, $data, $message, $status_code);
    }

    public static function error(?string $message = null, int $status_code = 400, mixed $data = null): self
    {
        return new self(false, $data, $message, $status_code);
    }

    public function to_array(): array
    {
        $result = ['success' => $this->success];
        if ($this->data !== null) {
            $result['data'] = $this->data;
        }
        if ($this->message !== null) {
            $result['message'] = $this->message;
        }
        return $result;
    }

    public function get_status_code(): int
    {
        return $this->status_code;
    }
}