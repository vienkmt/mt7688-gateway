# Project Changelog - MT7688AN IoT Gateway

## Version 2.0.0 - Async Runtime Refactor (2026-03-05)

### Architecture Changes
- **Runtime:** Migrated from multi-thread blocking model to Tokio single-thread async executor with epoll
- **UART I/O:** BufReader blocking → AsyncFd with epoll multiplexing (non-blocking)
- **MQTT Client:** rumqttc::Client → rumqttc::AsyncClient
- **HTTP Publisher:** thread::spawn → spawn_blocking(ureq) under async runtime
- **Channels:** std::sync::mpsc → tokio::sync::broadcast + tokio::sync::watch
- **LED/OLED Tasks:** thread::spawn → tokio::spawn (lightweight async tasks)

### Configuration Updates
- **Config File:** `/etc/v3s-monitor.toml` → `/etc/vgateway.toml`
- **Binary Name:** v3s-system-monitor → vgateway
- **HTTP Port:** 8888 → 8889
- **Target Binary Size:** <500KB → <800KB (due to Tokio runtime overhead, justified by resource efficiency)

### Features Added
- WiFi LED heartbeat indicator (GPIO toggle via async task)
- OLED 0.91" display integration (I2C async driver, dynamic updates)
- Network configuration module (WAN DHCP/Static IP management)
- System information dashboard (uptime, CPU%, RAM%, interface stats)

### Technical Improvements
- Single-thread executor reduces context switching on 580MHz MIPS CPU
- Epoll-based I/O multiplexing for efficient resource usage on 256MB RAM
- Broadcast channels enable multi-subscriber patterns (UART data → MQTT, HTTP, OLED)
- Watch channels for efficient config change notifications
- Non-blocking UART communication prevents main thread blocking
- Memory footprint optimized: lightweight tokio tasks vs. OS threads

### Breaking Changes
- Config file path changed (`/etc/v3s-monitor.toml` → `/etc/vgateway.toml`)
- HTTP API port changed (8888 → 8889)
- Binary name changed (v3s-system-monitor → vgateway)
- Build artifacts now require `tokio` dependency (previously avoided)

### Known Limitations
- tiny-http remains single-threaded; concurrent requests handled via async tasks
- ureq POST operations use spawn_blocking (blocking thread pool) to maintain compatibility
- MIPS 32-bit architecture limits some atomic operations (use Mutex<T> for u64)

---

## Version 1.0.0 - Initial Release (2026-02-12)

### Features
- HTTP web server for configuration and monitoring (port 8888)
- MQTT publisher for sensor data
- HTTP publisher for external endpoint integration
- Network configuration management via OpenWrt UCI
- UART serial communication with 4G Quectel modem
- Web UI with system dashboard and configuration pages
- Support for static and DHCP WAN modes
- TLS support for MQTT and HTTP with system clock synchronization

### Architecture
- Multi-threaded blocking I/O model
- std::sync::mpsc bounded channels (128 message capacity)
- Mutex-based config storage (Arc<Mutex<Config>>)
- UCI wrapper for OpenWrt integration

### Target Specifications
- MT7688AN (MIPS 24KEc, 580MHz, single-core)
- 256MB DDR2 RAM, 25MB available flash
- OpenWrt 21.02 (Kernel 5.4.171)
- Cross-compilation target: mipsel-unknown-linux-musl
- Binary size: <500KB (release build with stripping)

### Documentation
- System architecture overview
- Codebase module documentation
- Code standards and conventions
- MIPS build guide
- Peripheral guides (GPIO, I2C RTC, OLED display)
