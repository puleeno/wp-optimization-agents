# WordPress Offload Python Scripts - Complete Specification

## Overview

Collection of Python scripts that interface with the Rust daemon (`wp-offload-daemon`) for media offloading operations from WordPress to Google Blogger.

## Architecture

### Core Python Layer

**Directory Structure**
```
agents/python/
├── config.py        # Configuration management
├── api_clients.py   # WordPress and Blogger API clients
├── media_processor.py # Image processing utilities
├── offload.py       # Main orchestration
├── monitor.py       # Monitoring and logging
├── schema.py        # Data validation
├── __init__.py      # Package initialization
└── requirements.txt # Python dependencies
```

### Key Design Principles

1. **Layered Architecture**: Clear separation of concerns
2. **Type Safety**: Pydantic models for validation
3. **Error Handling**: Comprehensive error handling with recovery
4. **Logging**: Structured logging with multiple outputs
5. **Configuration**: Environment-aware configuration management
6. **Extensibility**: Plugin-friendly design

## Technical Specifications

### Configuration Management

**Core Configuration**
```python
class WordPressConfig(BaseModel):
    api_url: str = Field(..., description="WordPress REST API URL")
    username: str = Field(..., description="WordPress username")
    app_password: str = Field(..., description="WordPress application password")
    site_url: str = Field(..., description="WordPress site URL")

class BloggerConfig(BaseModel):
    blog_id: str = Field(..., description="Blogger blog ID")
    token: str = Field(..., description="Blogger OAuth token")
    base_url: str = Field(default="https://www.blogger.com/api", description="Blogger API base URL")
```

**Environment Variables**
- `WP_OFFLOAD_CONFIG_PATH`: Path to configuration file
- `WP_OFFLOAD_LOG_LEVEL`: Logging level (DEBUG, INFO, WARN, ERROR)
- `WP_OFFLOAD_DAEMON_HOST`: Daemon host address
- `WP_OFFLOAD_DAEMON_PORT`: Daemon port number

### API Clients

**WordPress Client**
```python
class WordPressClient:
    def __init__(self, config: WordPressConfig):
        self.base_url = config.api_url
        self.auth = (config.username, config.app_password)
        self.session = requests.Session()
        
    async def get_media(self, media_id: str) -> dict:
        """Get media details from WordPress"""
        
    async def download_media(self, media_url: str) -> bytes:
        """Download media content"""
        
    async def create_post(self, post_data: dict) -> dict:
        """Create a new post with media"""
```

**Blogger Client**
```python
class BloggerClient:
    def __init__(self, config: BloggerConfig):
        self.base_url = f"{config.base_url}/v3/blogs/{config.blog_id}"
        self.token = config.token
        self.session = requests.Session()
        
    async def upload_photo(self, media_data: bytes, title: str, content: str) -> dict:
        """Upload photo to Blogger"""
        
    async def update_post(self, post_id: str, post_data: dict) -> dict:
        """Update existing post"""
```

### Media Processing

**Image Processor**
```python
class MediaProcessor:
    def __init__(self, width: int = 1200, height: int = 800, quality: int = 85):
        self.width = width
        self.height = height
        self.quality = quality
        
    async def process_url(self, url: str) -> str:
        """Process URL with Blogger transformation parameters"""
        
    async def resize_image(self, image_data: bytes) -> bytes:
        """Resize image data"""
        
    async def convert_format(self, image_data: bytes, target_format: str) -> bytes:
        """Convert image format"""
```

### Offload Orchestration

**Offload Manager**
```python
class OffloadManager:
    def __init__(self, config_path: str):
        self.config = self._load_config(config_path)
        self.wp_client = WordPressClient(self.config.wordpress)
        self.blogger_client = BloggerClient(self.config.blogger)
        self.processor = MediaProcessor(
            self.config.media.max_width,
            self.config.media.max_height,
            self.config.media.quality
        )
        self.monitor = Monitor()
        
    async def run(self):
        """Main offload workflow"""
        async with aiohttp.ClientSession() as session:
            # Fetch WordPress media
            media_items = await self.wp_client.get_recent_media()
            
            # Process each media item
            for media in media_items:
                await self._process_media_item(media)
                
    async def _process_media_item(self, media: dict):
        """Process a single media item"""
        try:
            # Download media
            media_data = await self.wp_client.download_media(media['source_url'])
            
            # Process URL
            processed_url = await self.processor.process_url(media['source_url'])
            
            # Upload to Blogger
            blogger_result = await self.blogger_client.upload_photo(
                media_data, media['title'], processed_url
            )
            
            # Update WordPress post
            await self.wp_client.update_post(
                media['post_id'],
                {'blogger_url': blogger_result['url']}
            )
            
            self.monitor.log_success(f"Processed {media['title']}")
            
        except Exception as e:
            self.monitor.log_error(f"Failed to process {media['title']}: {e}")
            raise
```

### Monitoring and Logging

**Logger Configuration**
```python
class Logger:
    def __init__(self, level: str = "info", file: Optional[str] = None):
        self.level = level
        self.handlers = []
        
    def setup_handlers(self):
        """Setup logging handlers"""
        # Console handler
        console_handler = logging.StreamHandler()
        console_handler.setLevel(self.level)
        
        # File handler
        if self.file:
            file_handler = logging.FileHandler(self.file)
            file_handler.setLevel(self.level)
            
        # Structured JSON handler
        json_handler = logging.StreamHandler()
        json_handler.setLevel(self.level)
        
    def log_operation(self, operation: str, status: str, duration: float, metadata: dict):
        """Log operation with structured data"""
        
    def log_error(self, error: Exception, context: dict):
        """Log error with context"""
```

**Metrics Collection**
```python
class MetricsCollector:
    def __init__(self):
        self.counters = {}
        self.timers = {}
        self.gauges = {}
        
    def increment(self, name: str, tags: dict = None):
        """Increment counter"""
        
    def record(self, name: str, value: float, tags: dict = None):
        """Record timing metric"""
        
    def set(self, name: str, value: float, tags: dict = None):
        """Set gauge value"""
```

## Python API Interface

### Public Functions
```python
from wp_offload_ffi import create_offload_manager, process_media_url, OffloadManager

# Configuration-based offload manager
manager = create_offload_manager("config.json")

# URL processing
result_url = process_media_url("input.jpg", 800, 600)

# Daemon control
manager.run()
manager.stop()
```

### Command Line Interface
```python
import click

@click.command()
@click.option('--config', '-c', required=True, help='Path to configuration file')
@click.option('--daemon', '-d', is_flag=True, help='Run as daemon')
@click.option('--daemon-mode', type=click.Choice(['foreground', 'background']), 
              default='foreground', help='Daemon mode')
def main(config, daemon, daemon_mode):
    """WordPress Media Offload Tool"""
    
    if daemon:
        daemon = OffloadManager(config)
        if daemon_mode == 'background':
            daemon.daemonize()
        daemon.run()
    else:
        # Run single offload operation
        offload = OffloadManager(config)
        offload.run_sync()

if __name__ == '__main__':
    main()
```

## Python Dependencies

**Core Dependencies**
```txt
requests>=2.31.0
pydantic>=2.5.0
PyYAML>=6.0
requests-toolbelt>=1.0.0
python-json-logger>=2.0.7
click>=8.1.0
colorama>=0.4.6
aiohttp>=3.9.0
async-timeout>=4.0.3
python-dotenv>=1.0.0
pillow>=10.1.0
numpy>=1.24.0
opencv-python>=4.8.0
```

**Development Dependencies**
```txt
pytest>=7.4.0
pytest-asyncio>=0.21.0
pytest-cov>=4.1.0
black>=23.11.0
flake8>=6.1.0
mypy>=1.7.0
ruff>=0.1.6
pre-commit>=3.6.0
```

## Configuration Schema

**Pydantic Models**
```python
class MediaConfig(BaseModel):
    max_width: int = Field(default=1200, description="Maximum image width")
    max_height: int = Field(default=800, description="Maximum image height")
    quality: int = Field(default=85, description="Image quality (1-100)")
    upload_timeout: int = Field(default=30, description="Upload timeout in seconds")

class ChromeConfig(BaseModel):
    chrome_path: Optional[str] = Field(None, description="Path to Chrome executable")
    user_data_dir: Optional[str] = Field(None, description="Chrome user data directory")
    headless: bool = Field(default=False, description="Run Chrome in headless mode")

class LoggingConfig(BaseModel):
    level: str = Field(default="info", description="Logging level")
    file: Optional[str] = Field(None, description="Log file path")
    max_size_mb: int = Field(default=10, description="Maximum log file size in MB")
    max_files: int = Field(default=5, description="Maximum number of log files to keep")
```

## Testing

### Test Structure
```
agents/python/tests/
├── test_config.py          # Configuration tests
├── test_api_clients.py     # API client tests
├── test_media_processor.py # Media processing tests
├── test_offload.py         # Offload workflow tests
├── test_monitor.py         # Monitoring tests
└── conftest.py             # Test configuration
```

### Test Commands
```bash
# Run all tests
pytest agents/python/tests/

# Run with coverage
pytest --cov=agents.python --cov-report=html

# Run specific test module
pytest agents/python/tests/test_config.py

# Run with asyncio
pytest --asyncio-mode=auto
```

## Development Workflow

### Local Development
```bash
# Setup virtual environment
cd agents/python
python -m venv .venv
source .venv/bin/activate  # Linux/macOS
# or
.venv\Scripts\activate     # Windows

# Install dependencies
pip install -r requirements.txt
pip install -e .

# Format code
black agents/python/

# Lint code
ruff check agents/python/

# Type checking
mypy agents/python/

# Run tests
pytest agents/python/tests/
```

### Building the Daemon
```bash
# Build Rust daemon
cd agents/daemon
cargo build --release

# Test build
cargo test

# Format code
rustfmt

# Lint code
clippy
```

## Performance Characteristics

- **Memory Usage**: ~100MB (Python) + ~50MB (Rust daemon)
- **CPU Usage**: <10% idle, <30% during operations
- **Network**: HTTP/1.1 with connection pooling
- **Response Time**: <500ms for URL processing
- **Throughput**: 50-100 URLs per minute

## Security Considerations

### Authentication
- OAuth2 for Blogger API
- Application passwords for WordPress
- Token storage in environment variables
- Secure configuration files

### Network Security
- TLS/SSL enforcement
- Certificate validation
- HTTP headers security
- Rate limiting

### File Security
- Secure credential storage
- Log file permissions
- Temporary file handling

## Deployment

### Systemd Service (Linux)
```ini
[Unit]
Description=WordPress Media Offload Daemon
After=network.target

[Service]
Type=simple
User=daemon
Group=daemon
WorkingDirectory=/opt/agents/python
Environment=RUST_LOG=info
ExecStart=/usr/local/bin/wp-offload-daemon
Restart=always
RestartSec=10
StandardOutput=journal
StandardError=journal

[Install]
WantedBy=multi-user.target
```

### Launchd Service (macOS)
```xml
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>Label</key>
    <string>com.agents.wpoffload.daemon</string>
    <key>ProgramArguments</key>
    <array>
        <string>/usr/local/bin/wp-offload-daemon</string>
    </array>
    <key>WorkingDirectory</key>
    <string>/opt/agents/python</string>
    <key>EnvironmentVariables</key>
    <dict>
        <key>RUST_LOG</key>
        <string>info</string>
    </dict>
    <key>RunAtLoad</key>
    <true/>
    <key>KeepAlive</key>
    <true/>
</dict>
</plist>
```

### Windows Service
```powershell
# Install Windows Service
nssm install wpOffloadDaemon "C:\Program Files\wp-offload-daemon\wp-offload-daemon.exe"

# Start service
sc start wpOffloadDaemon

# Stop service
sc stop wpOffloadDaemon
```

## Monitoring and Alerting

### Prometheus Metrics
```rust
// In Rust daemon
#[macro_use]
extern crate prometheus;

fn register_metrics() {
    let _ = prometheus::register(
        prometheus::Opts::new("wp_offload_requests_total", "Total requests processed")
            .consts(vec![("service", "wp_offload")]),
    );
}
```

### Alert Rules
```yaml
groups:
- name: wp-offload
  rules:
  - alert: HighErrorRate
    expr: rate(wp_offload_requests_total{status="error"}[5m]) / rate(wp_offload_requests_total[5m]) > 0.05
    for: 5m
    labels:
      severity: warning
    annotations:
      summary: "High error rate in WordPress offload"
      description: "Error rate is {{ $value | humanizePercentage }} over the last 5 minutes"
```

## Troubleshooting

### Common Issues

1. **Configuration Errors**
   ```bash
   # Validate configuration
   python -c "from pydantic import ValidationError; from config import DaemonConfig; DaemonConfig(config_path='config.json')"
   ```

2. **Connection Issues**
   ```bash
   # Test API connectivity
   curl -v https://your-wordpress.com/wp-json/wp/v2/media
   ```

3. **Python Import Issues**
   ```bash
   # Install in development mode
   pip install -e agents/python
   ```

4. **Rust Daemon Build Issues**
   ```bash
   # Clean and rebuild
   cd agents/daemon && cargo clean && cargo build --release
   ```

### Debug Commands
```bash
# Check daemon status
cd agents/daemon
cargo run --release -- --status

# Run Python monitor
python -m agents.python.monitor

# Test URL processing
python -c "from agents.python.media_processor import MediaProcessor; mp = MediaProcessor(800, 600); result = mp.process_url('https://example.com/image.jpg'); print(result)"

# View logs
tail -f agents/python/logs/daemon.log
```

## Future Enhancements

1. **Advanced Image Processing**: ML-based image optimization
2. **Distributed Processing**: Horizontal scaling capabilities
3. **WebSocket Notifications**: Real-time status updates
4. **Database Integration**: Persistent operation history
5. **Plugin Architecture**: Third-party plugin support
6. **Advanced Analytics**: Usage patterns and insights
7. **Multi-account Support**: Parallel processing for multiple WordPress sites
8. **Template Engine**: Customizable processing workflows
