# WordPress Offload Python Scripts

Collection of Python scripts that interface with the Rust daemon for media offloading operations.

## Overview

These scripts handle:
- Configuration loading and validation
- Monitoring and logging
- Offload orchestration
- Error handling and retries

## Usage

```bash
cd agents/python
python offload_manager.py
```

## Structure

- `config.py`: Configuration management
- `api_clients.py`: WordPress and Blogger API clients
- `media_processor.py`: Image transformation utilities
- `offload.py`: Main offload workflow orchestration
- `monitor.py`: Monitoring and logging
