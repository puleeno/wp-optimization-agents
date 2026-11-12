(function($) {
    'use strict';

    var wpOptAgents = window.wpOptAgents || {};

    function apiRequest(endpoint, options) {
        return $.ajax($.extend({
            url: (wpOptAgents.rest_url || '') + endpoint,
            beforeSend: function(xhr) {
                if (wpOptAgents.api_token) {
                    xhr.setRequestHeader('Authorization', 'Bearer ' + wpOptAgents.api_token);
                }
                if (wpOptAgents.nonce) {
                    xhr.setRequestHeader('X-WP-Nonce', wpOptAgents.nonce);
                }
            }
        }, options || {}));
    }

    function testDaemonConnection() {
        var $btn = $('#test-daemon-connection');
        $btn.prop('disabled', true).text('Testing...');
        apiRequest('/config', { method: 'GET' })
            .done(function(response) {
                if (response && response.success) {
                    var port = response.data && response.data.daemon ? response.data.daemon.port : '';
                    $('#daemon-status').html('<span style="color:#46b450;">✓ Plugin API OK</span>' + (port ? (' Daemon: ' + port) : ''));
                } else {
                    $('#daemon-status').html('<span style="color:#dc3232;">✗ Failed</span>');
                }
            })
            .fail(function(xhr) {
                var msg = (xhr.responseJSON && (xhr.responseJSON.message || xhr.responseJSON.data)) || xhr.statusText || 'Error';
                $('#daemon-status').html('<span style="color:#dc3232;">✗ ' + $('<div/>').text(msg).html() + '</span>');
            })
            .always(function() { $btn.prop('disabled', false).text('Test Plugin API'); });
    }

    function generateApiToken() {
        var $btn = $('#generate-api-token');
        var token = 'wpopt_' + Math.random().toString(36).slice(2) + Math.random().toString(36).slice(2) + Date.now().toString(36);
        $('input[name="wp_opt_agents_settings[daemon_api_token]"]').val(token);
        $btn.text('Generated').prop('disabled', true);
        setTimeout(function() { $btn.text('Generate API Token').prop('disabled', false); }, 1500);
    }

    function loadOffloadImages() {
        var $table = $('#offload-images-table');
        $table.html('<p>Loading...</p>');
        apiRequest('/stats', { method: 'GET' })
            .done(function(response) {
                if (!response || !response.success) {
                    $table.html('<p class="notice notice-error">Failed to load stats</p>');
                    return;
                }
                var stats = response.data || {};
                var rows = [
                    ['replaced', 'Replaced (301 active)'],
                    ['uploaded', 'Uploaded (awaiting replace)'],
                    ['replacing', 'Replacing'],
                    ['uploading', 'Uploading'],
                    ['pending', 'Pending'],
                    ['failed', 'Failed']
                ];
                var html = '<table class="wp-list-table widefat fixed striped"><thead><tr><th>Status</th><th>Count</th></tr></thead><tbody>';
                rows.forEach(function(r) {
                    html += '<tr><td>' + r[1] + '</td><td>' + (stats[r[0]] || 0) + '</td></tr>';
                });
                html += '<tr><th>Total</th><th>' + (stats.total || 0) + '</th></tr></tbody></table>';
                $table.html(html);
            })
            .fail(function() { $table.html('<p class="notice notice-error">Failed to load stats</p>'); });
    }

    $(document).ready(function() {
        $('#test-daemon-connection').on('click', testDaemonConnection);
        $('#generate-api-token').on('click', generateApiToken);
        if ($('#offload-images-table').length) loadOffloadImages();
        if ($('#daemon-status').length) testDaemonConnection();
    });
})(jQuery);
