# Project Changelog - ugate IoT Gateway

## Version 3.0.0 - Phases 1-6 Complete (2026-03-08)

### Phase 1: Core Infrastructure (Complete)
- **UART Reader:** AsyncFd + epoll, supports line/fixed/timeout frame modes
- **Config System:** UCI-based (/etc/config/ugate), hot-reload via watch<()>
- **App State:** RwLock-based thread-safe storage with async notifications
- **Time Sync:** HTTP-based NTP before TLS operations

### Phase 2a: MQTT & HTTP Channels (Complete)
- **MQTT Publisher:** std::thread + rumqttc sync Client (stable on MIPS)
  - Supports authentication, TLS (rustls), configurable QoS
  - Subscribes to command topic for bi-directional control
  - Cross-thread channel: std::sync::mpsc
- **HTTP POST Publisher:** Async task with ureq via spawn_blocking
  - GET/POST methods configurable
  - Response body parsed as JSON commands or raw UART TX
  - Async channel: tokio::sync::mpsc (capacity 64)
- **Offline Buffer:** RAM queue → /tmp/ugate_buffer disk overflow

### Phase 2b: TCP + Reliability (Complete)
- **TCP Server:** Accept Modbus/binary connections, broadcast UART data
- **TCP Client:** Connect to upstream gateway/server, bidirectional
- **Reconnect Logic:** Exponential backoff (2s → 4s → 8s → 60s cap)
- **Server + Client Mode:** Both simultaneously operational

### Phase 3: Web Server & WebSocket (Complete)
- **HTTP Server:** tiny-http at port 8888 (spawn_blocking)
- **WebSocket:** tungstenite for real-time UART logs and system stats
- **Embedded UI:** Vue.js + Tailwind CSS in binary (include_str!)
- **REST API:** /api/* endpoints for config, status, login, GPIO

### Phase 4: GPIO Control (Complete)
- **chardev ioctl:** Pure Rust GPIO (no DTS required)
- **32+ GPIO Lines:** Configurable per-pin via UCI
- **Command Dispatch:** GpioSet, GpioToggle, GpioPulse
- **Status Tracking:** SharedStats monitors GPIO operations

### Phase 5: Vue.js Frontend (Complete)
- **Single-Page App:** Vue.js framework with Tailwind CSS
- **Pages:** Dashboard, Config, GPIO Control, Status
- **Real-time Updates:** WebSocket for live stats and UART logs
- **Session Auth:** Cookie-based auth, 1h expiry (RAM-based)

### Phase 6: Integration & Testing (Complete)
- **Cross-platform:** Tested on MT7688 (OpenWrt 24.10)
- **Syslog Integration:** Logs to /dev/log for `logread`
- **Status API:** Real-time stats (uptime, CPU%, RAM%, channel states)
- **Config API:** Full CRUD for all settings via REST
- **UI Auth:** Password-protected dashboard with session management

### New Features in v3.0
- **Frame Modes:** Line (newline), Fixed (N bytes), Timeout (gap-based)
- **TCP Bi-directional:** Server and client modes with Modbus support
- **Command Dispatch:** Unified command routing from WS/TCP/MQTT
- **Offline Buffer:** Disk overflow when RAM full (/tmp/ugate_buffer)
- **WebSocket Real-time:** Live UART logs and system stats
- **GPIO API:** Set/toggle/pulse GPIO via HTTP endpoints
- **Vue.js SPA:** Modern web UI embedded in binary
- **Session Auth:** Multi-user support (max 4 sessions)
- **Syslog Integration:** OpenWrt native logging via syslog crate

### Architecture Changes
- **Channels:** Hybrid async/sync (tokio + std::thread)
  - UART → MQTT: std::sync::mpsc (cross-thread)
  - UART → HTTP: tokio::sync::mpsc (async)
  - Config notify: tokio::sync::watch<()> (lightweight)
- **MQTT:** std::thread + rumqttc sync Client (vs AsyncClient on MIPS)
- **Concurrency:** RwLock for read-heavy config access
- **I/O Multiplexing:** AsyncFd + epoll for non-blocking UART

### Bug Fixes
- Fixed MIPS async compatibility issue (std::thread for MQTT)
- Fixed config hot-reload notification (watch<()> for async, polling for std::thread)
- Fixed UART frame detection (timeout-based gap detection)
- Fixed WebSocket broadcast (proper channel cloning per client)
- Fixed GPIO chardev ioctl (proper error handling)

### Performance
- **Binary size:** ~800KB (release, stripped) [target: <1.2MB]
- **Startup time:** ~2s [target: <5s]
- **Memory (idle):** ~15MB [target: <30MB]
- **Memory (100 msg/s):** ~25MB [target: <50MB]
- **HTTP latency:** ~50ms [target: <100ms]
- **WebSocket latency:** ~30ms [target: <50ms]
- **GPIO toggle:** ~10ms [target: <50ms]

### Configuration
- **File:** `/etc/config/ugate` (UCI native, replaces TOML)
- **Sections:** mqtt, http, tcp, uart, gpio, web, general
- **Hot-reload:** Via watch channel notifications

### Breaking Changes
| Change | Old | New |
|--------|-----|-----|
| Config file | `/etc/vgateway.toml` | `/etc/config/ugate` |
| MQTT lib | AsyncClient | sync Client in std::thread |
| Frame detection | Simple newline | line/fixed/timeout modes |
| GPIO API | crate-based | chardev ioctl |
| Command dispatch | Direct calls | Command enum + dispatcher |

---

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
- UART serial communication with external devices (sensors, MCU)
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
