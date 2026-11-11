# WordPress Media Offload Daemon

Daemon service for managing WordPress-to-Blogger media offloading across multiple platforms (Windows, macOS, Linux).

## Architecture

This daemon implements the core logic in Rust and provides a Python interface for configuration, monitoring, and orchestration.

Key components:
- Chrome automation
- WordPress API interactions
- Blogger API integration
- Media processing and transformation
- Cross-platform daemon service management

## Building

```bash
cd agents/daemon
cargo build --release

# Cross-platform builds (ensure targets are installed):
# Windows
cargo build --target x86_64-pc-windows-msvc --release
# macOS  
cargo build --target x86_64-apple-darwin --release
# Linux
cargo build --target x86_64-unknown-linux-gnu --release
```

## Python Interface

Python scripts can call Rust functionality via PyO3 extensions or through REST APIs provided by the daemon.