# WordPress Media Offload Daemon - Complete Agent Specification

## Overview

**wp-offload-daemon** is a cross-platform Rust-based daemon service that automates media offloading from WordPress to Google Blogger. The service provides robust Chrome automation, API integration, and image transformation capabilities.

## Architecture

### Core Components

1. **Rust Daemon (agents/daemon)**
   - Primary implementation in Rust for performance and cross-platform support
   - Configurable daemon service with plugin architecture for extensibility
   - PyO3 bindings for Python integration

2. **Python Scripts (agents/python)**
   - High-level orchestration layer
   - Configuration management
   - Error handling and retry logic
   - Monitoring and logging integration

3. **Cross-Platform Support**
   - Windows (x86_64-pc-windows-msvc)
   - macOS (x86_64-apple-darwin)
   - Linux (x86_64-unknown-linux-gnu)

## Technical Specifications

### Rust Backend

#### Module Structure
```
agents/daemon/src/
├── config.rs          # Configuration management
├── error.rs           # Custom error types and handling
├── media_processor.rs # Image transformation and URL manipulation
├── main_lib.rs        # Core daemon logic and state management
├── wordpress_api.rs   # WordPress REST API integration
├── blogger_api.rs     # Blogger API integration
├── chrome_automation.rs # Chrome browser control
├── types.rs           # Shared data structures and enums
└── lib.rs             # PyO3 bindings and public interface
```

#### Key Features

**Configuration Management**
- JSON-based configuration with validation
- Environment variable support
- Platform-specific configuration options
- Dynamic reloading capability

**API Integration**
- WordPress REST API v2 (media, posts, users)
- Blogger API v3 (posts, media)
- OAuth2 authentication support
- Rate limiting and retry logic

**Chrome Automation**
- WebDriver-compatible browser control
- Multi-tab and multi-window support
- Session persistence
- Headless and GUI modes

**Media Processing**
- Image URL transformation with Blogger parameters
- Size, crop, and quality controls
- Format conversion (WebP, JPEG, PNG)
- Batch processing capabilities

#### Python Interface

**PyO3 Bindings**
- Direct Python module loading
- Type-safe function mappings
- Async/sync compatibility
- Error propagation

**Available Python Functions**
```python
import wp_offload_ffi

# Create offload manager with configuration
manager = wp_offload_ffi.create_offload_manager("config.json")

# Process media URLs with transformations
processed_url = wp_offload_ffi.process_media_url(
    input_url="https://example.com/image.jpg",
    width=800,
    height=600
)

# Daemon control
manager.run()
manager.stop()
```

### Python Layer

#### Module Structure
```
agents/python/
├── config.py        # Configuration management
├── api_clients.py   # WordPress and Blogger API clients
├── media_processor.py # Image processing utilities
├── offload.py       # Main orchestration
├── monitor.py       # Monitoring and logging
├── schema.py        # Data validation
└── __init__.py      # Package initialization
```

#### Key Features

**Configuration**
- Environment variable integration
- Configuration validation
- Schema-based configuration
- Hot reloading support

**API Clients**
- WordPress REST API wrapper
- Blogger API wrapper
- Authentication management
- Rate limiting

**Media Processing**
- Image resizing and cropping
- Format conversion
- Quality optimization
- Batch operations

**Orchestration**
- Workflow management
- Error handling and recovery
- Progress tracking
- Notification system

## Build and Deployment

### Building

#### Rust Daemon
```bash
cd agents/daemon
cargo build --release

# Cross-platform builds
# Windows
cargo build --target x86_64-pc-windows-msvc --release

# macOS
cargo build --target x86_64-apple-darwin --release

# Linux
cargo build --target x86_64-unknown-linux-gnu --release
```

#### Python Scripts
```bash
cd agents/python
pip install -r requirements.txt
python -m pip install -e .
```

### Installation

#### System Service
- **Windows**: Use NSSM or Windows Service Manager
- **macOS**: Use launchd with plist configuration
- **Linux**: Use systemd with service file

#### Configuration
```json
{
  "wordpress": {
    "api_url": "https://your-wordpress.com/wp-json",
    "username": "your-username",
    "app_password": "your-app-password",
    "site_url": "https://your-wordpress.com"
  },
  "blogger": {
    "blog_id": "your-blogger-blog-id",
    "token": "your-blogger-oauth-token",
    "base_url": "https://www.blogger.com/api"
  },
  "chrome": {
    "chrome_path": "/usr/bin/google-chrome",
    "user_data_dir": "./chrome-user-data",
    "headless": false
  },
  "media": {
    "max_width": 1200,
    "max_height": 800,
    "quality": 85,
    "upload_timeout": 30
  },
  "logging": {
    "level": "info",
    "file": "./daemon.log",
    "max_size_mb": 10,
    "max_files": 5
  }
}
```

### Dependencies

#### Rust Dependencies
```toml
[dependencies]
tokio = { version = "1.37", features = ["rt-multi-thread", "macros", "time"] }
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
thiserror = "1.0"
pyo3 = { version = "0.21", features = ["extension-module"] }
reqwest = { version = "0.12", features = ["json"] }
url = "2.5"
once_cell = "1.5"
dotenv = "0.15"
```

#### Python Dependencies
```requirements.txt
requests>=2.31.0
pydantic>=2.5.0
PyYAML>=6.0
requests-toolbelt>=1.0.0
python-json-logger>=2.0.7
click>=8.1.0
colorama>=0.4.6
```

## Configuration Schema

### Config Schema
```python
from pydantic import BaseModel, Field
from typing import Optional, Dict, Any

class WordPressConfig(BaseModel):
    api_url: str = Field(..., description="WordPress REST API URL")
    username: str = Field(..., description="WordPress username")
    app_password: str = Field(..., description="WordPress application password")
    site_url: str = Field(..., description="WordPress site URL")

class BloggerConfig(BaseModel):
    blog_id: str = Field(..., description="Blogger blog ID")
    token: str = Field(..., description="Blogger OAuth token")
    base_url: str = Field(default="https://www.blogger.com/api", description="Blogger API base URL")

class ChromeConfig(BaseModel):
    chrome_path: Optional[str] = Field(None, description="Path to Chrome executable")
    user_data_dir: Optional[str] = Field(None, description="Chrome user data directory")
    headless: bool = Field(default=False, description="Run Chrome in headless mode")

class MediaConfig(BaseModel):
    max_width: int = Field(default=1200, description="Maximum image width")
    max_height: int = Field(default=800, description="Maximum image height")
    quality: int = Field(default=85, description="Image quality (1-100)")
    upload_timeout: int = Field(default=30, description="Upload timeout in seconds")

class LoggingConfig(BaseModel):
    level: str = Field(default="info", description="Logging level")
    file: Optional[str] = Field(None, description="Log file path")
    max_size_mb: int = Field(default=10, description="Maximum log file size in MB")
    max_files: int = Field(default=5, description="Maximum number of log files to keep")

class DaemonConfig(BaseModel):
    wordpress: WordPressConfig
    blogger: BloggerConfig
    chrome: ChromeConfig
    media: MediaConfig
    logging: LoggingConfig
```

## Testing

### Rust Tests
```bash
cargo test
cargo test --release
```

### Python Tests
```bash
pytest agents/python/tests/
```

## Performance Characteristics

- **Memory Usage**: ~50MB baseline
- **CPU Usage**: <5% idle, <20% during operations
- **Network**: HTTP/1.1 with connection pooling
- **Response Time**: <100ms for URL processing
- **Scalability**: Multi-threaded for concurrent operations

## Security Considerations

- OAuth2 for Blogger authentication
- Application passwords for WordPress
- TLS/SSL enforcement for all network traffic
- Secure storage of credentials
- Input validation and sanitization
- Rate limiting to prevent abuse

## Monitoring and Logging

### Log Levels
- **ERROR**: Critical errors that require intervention
- **WARN**: Non-critical issues that may indicate problems
- **INFO**: Operational information
- **DEBUG**: Detailed debugging information

### Metrics
- Operation success/failure rates
- Processing times
- Network latency
- Resource utilization
- Error categorization

## Deployment Examples

### Docker
```dockerfile
FROM rust:1.75-slim as builder
WORKDIR /app
COPY agents/daemon/Cargo.toml Cargo.toml
COPY agents/daemon/Cargo.lock Cargo.lock
COPY agents/daemon/src ./src
RUN cargo build --release

FROM debian:bullseye-slim
RUN apt-get update && apt-get install -y ca-certificates
COPY --from=builder /app/target/release/wp-offload-daemon /usr/local/bin/
COPY agents/daemon/config.json /etc/daemon/
CMD ["wp-offload-daemon"]
```

### Kubernetes
```yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: wp-offload-daemon
spec:
  replicas: 2
  selector:
    matchLabels:
      app: wp-offload-daemon
  template:
    metadata:
      labels:
        app: wp-offload-daemon
    spec:
      containers:
      - name: daemon
        image: your-registry/wp-offload-daemon:latest
        env:
        - name: RUST_LOG
          value: "info"
        resources:
          limits:
            memory: "512Mi"
            cpu: "500m"
          requests:
            memory: "256Mi"
            cpu: "250m"
        volumeMounts:
        - name: config
          mountPath: /etc/daemon
        volumes:
        - name: config
          configMap:
            name: daemon-config
```

## Future Enhancements

1. **Plugin Architecture**: Support for third-party plugins
2. **Distributed Processing**: Horizontal scaling capabilities
3. **Advanced Image Processing**: ML-based image optimization
4. **WebSocket Notifications**: Real-time status updates
5. **Database Integration**: Persistent storage for operation history
6. **Monitoring Integration**: Prometheus metrics export

## Troubleshooting

### Common Issues

1. **Configuration Errors**
   - Validate JSON syntax
   - Check required fields
   - Verify API credentials

2. **Chrome Not Found**
   - Install Chrome/Chromium
   - Set correct chrome_path in config
   - Check permissions

3. **Network Errors**
   - Check internet connectivity
   - Verify firewall settings
   - Check API rate limits

4. **Python Import Errors**
   - Install dependencies
   - Check virtual environment
   - Verify build artifacts

### Debug Commands
```bash
# Rust daemon logs
cd agents/daemon
cargo run --release -- -v

# Python script logs
cd agents/python
python -m offload.monitor

# Test URL processing
python -c "from python.media_processor import MediaProcessor; mp = MediaProcessor(800, 600); print(mp.process_url('https://example.com/image.jpg'))"
```
