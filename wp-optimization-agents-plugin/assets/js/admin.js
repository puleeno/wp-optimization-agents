(function($) {
    'use strict';

    var wpOptAgents = window.wpOptAgents;

    function apiRequest(endpoint, options) {
        return $.ajax({
            url: wpOptAgents.rest_url + endpoint,
            beforeSend: function(xhr) {
                xhr.setRequestHeader('Authorization', 'Bearer ' + wpOptAgents.api_token);
                xhr.setRequestHeader('X-WP-Nonce', wpOptAgents.nonce);
            },
            ...options
        });
    }

    function testDaemonConnection() {
        var $btn = $('#test-daemon-connection');
        $btn.prop('disabled', true).text('Testing...');
        
        apiRequest('/config', { method: 'GET' })
            .done(function(response) {
                if (response.success) {
                    $('#daemon-status').html('<span style="color: #46b450;">✓ Connected</span> Port: ' + response.data.daemon_port);
                    $btn.text('Test Daemon Connection');
                } else {
                    $('#daemon-status').html('<span style="color: #dc3232;">✗ Failed: ' + response.message + '</span>');
                    $btn.text('Test Daemon Connection');
                }
            })
            .fail(function(xhr) {
                $('#daemon-status').html('<span style="color: #dc3232;">✗ Error: ' + (xhr.responseJSON?.message || xhr.statusText) + '</span>');
                $btn.text('Test Daemon Connection');
            })
            .always(function() {
                $btn.prop('disabled', false);
            });
    }

    function generateApiToken() {
        var $btn = $('#generate-api-token');
        $btn.prop('disabled', true).text('Generating...');
        
        var token = 'wpopt_' + Math.random().toString(36).substring(2, 15) + Math.random().toString(36).substring(2, 15);
        $('input[name="wp_opt_agents_settings[daemon_api_token]"]').val(token);
        
        $btn.text('Generated!').removeClass('button-primary').addClass('button-secondary');
        setTimeout(function() {
            $btn.text('Generate API Token').removeClass('button-secondary').addClass('button-primary').prop('disabled', false);
        }, 2000);
    }

    function triggerAudit() {
        var $btn = $('#trigger-audit');
        var $result = $('#audit-result');
        $btn.prop('disabled', true).text('Running Audit...');
        $result.empty();

        apiRequest('/audit/run', { method: 'POST' })
            .done(function(response) {
                if (response.success) {
                    $result.html('<div class="notice notice-success"><p>Audit completed successfully!</p></div>');
                } else {
                    $result.html('<div class="notice notice-error"><p>Audit failed: ' + response.message + '</p></div>');
                }
            })
            .fail(function(xhr) {
                $result.html('<div class="notice notice-error"><p>Error: ' + (xhr.responseJSON?.message || xhr.statusText) + '</p></div>');
            })
            .always(function() {
                $btn.prop('disabled', false).text('Run Audit Now');
            });
    }

    function loadOffloadImages() {
        var $table = $('#offload-images-table');
        $table.html('<p>Loading...</p>');

        apiRequest('/stats', { method: 'GET' })
            .done(function(response) {
                if (response.success) {
                    var stats = response.data;
                    var html = '<table class="wp-list-table widefat fixed striped"><thead><tr><th>Status</th><th>Count</th></tr></thead><tbody>';
                    $.each(['synced', 'pending', 'processing', 'failed'], function(i, status) {
                        html += '<tr><td><span class="status-' + status + '">' + status.charAt(0).toUpperCase() + status.slice(1) + '</span></td><td>' + (stats[status] || 0) + '</td></tr>';
                    });
                    html += '</tbody></table>';
                    $table.html(html);
                }
            })
            .fail(function() {
                $table.html('<p class="notice notice-error">Failed to load stats</p>');
            });
    }

    $(document).ready(function() {
        $('#test-daemon-connection').on('click', testDaemonConnection);
        $('#generate-api-token').on('click', generateApiToken);
        $('#trigger-audit').on('click', triggerAudit);

        if ($('#offload-images-table').length) {
            loadOffloadImages();
        }

        testDaemonConnection();
    });

})(jQuery);