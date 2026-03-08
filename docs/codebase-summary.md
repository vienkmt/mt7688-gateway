# Codebase Summary - ugate IoT Gateway

**Generated:** 2026-03-08
**Version:** 1.6.0 (Phases 1-9 Complete)
**Total Lines of Code:** ~4,795 (Rust) + 1,113 (HTML/CSS/JS)
**Total Files:** 34 (core + web modules + channels + assets)

## Project Overview

**ugate** is a production-ready IoT Gateway firmware for MT7688 (MIPS 580MHz, 64MB RAM) running OpenWrt 24.10. It provides multi-channel data acquisition with MQTT, HTTP, TCP, and WebSocket support.

**Core Capabilities:**
- Real-time UART data collection with configurable frame detection (line/fixed/timeout modes)
- Multi-channel fan-out to MQTT, HTTP POST, TCP (server/client)
- Bi-directional command control from WebSocket, TCP, MQTT
- WiFi Management (4 modes: STA/AP/STA+AP/Off), scanning, dynamic switching
- Network Configuration (LAN/WAN IP, NTP, static routing, metrics)
- System Maintenance (backup/restore, factory reset, firmware upgrade)
- GPIO control via chardev ioctl (32+ lines)
- Web management UI (9 pages, Vue 3, draft/apply pattern)
- Offline buffering with RAM→disk overflow
- Session Authentication (token-based, 4 max sessions, 24h TTL)
- WebSocket real-time updates via tungstenite
- Syslog integration for OpenWrt logging

**Phases 1-9 complete.** Production ready with full feature set.

## Project Structure

```
ugate/
├── Cargo.toml                  # Project manifest (v1.6.0)
├── build.rs                    # Concatenate frontend into embedded_index.html
├── src/                        # Rust source (core + API only)
│   ├── main.rs (270 LOC)       # Startup, runtime setup, task spawning
│   ├── config.rs (486 LOC)     # AppState, RwLock<Config>, watch channel
│   ├── uci.rs (146 LOC)        # UCI CLI wrapper
│   ├── commands.rs (139 LOC)   # Command enum, parsing
│   ├── time_sync.rs (83 LOC)   # HTTP-based NTP at startup
│   ├── gpio.rs (171 LOC)       # GPIO control via chardev ioctl
│   │
│   ├── channels/               # Data routing (929 LOC)
│   │   ├── mod.rs (12 LOC)
│   │   ├── mqtt.rs (202 LOC)   # MQTT pub/sub (std::thread, rumqttc)
│   │   ├── http_pub.rs (139 LOC) # HTTP POST publisher (spawn_blocking)
│   │   ├── tcp.rs (195 LOC)    # TCP server + client (async)
│   │   ├── buffer.rs (222 LOC) # Offline buffer (RAM + disk)
│   │   └── reconnect.rs (66 LOC) # Exponential backoff
│   │
│   ├── uart/                   # UART I/O (308 LOC)
│   │   ├── mod.rs (2 LOC)
│   │   ├── reader.rs (233 LOC) # AsyncFd/epoll, frame detection
│   │   └── writer.rs (73 LOC)  # UART TX queue
│   │
│   └── web_api/                # HTTP + WebSocket API (2,538 LOC)
│       ├── mod.rs (75 LOC)     # Shared helpers (json_resp, jval, etc.)
│       ├── server.rs (588 LOC) # tiny-http routing, handlers
│       ├── auth.rs (141 LOC)   # Session manager (token-based)
│       ├── status.rs (210 LOC) # SharedStats (atomic counters)
│       ├── ws.rs (121 LOC)     # WebSocket (tungstenite)
│       ├── wifi.rs (209 LOC)   # WiFi modes (STA/AP/STA+AP/Off)
│       ├── netcfg.rs (350 LOC) # Network/NTP/routing config
│       ├── maintenance.rs (362 LOC) # Backup/restore/upgrade
│       ├── toolbox.rs (135 LOC) # Toolbox API
│       └── syslog.rs (165 LOC) # Syslog viewer
│
├── frontend/                   # Web frontend (Vue 3 SPA + assets)
│   ├── index-template.html     # Base HTML template
│   ├── js/                     # Vue 3 + modular JS (10 files)
│   │   ├── 00-vue.min.js       # Vue 3 CDN bundle
│   │   ├── 01-core.js          # Core functionality, API helpers
│   │   ├── 02-components.js    # Vue components
│   │   ├── 03-page-status.js   # Status page component
│   │   ├── 04-page-channels.js # Communication/channels page
│   │   ├── 05-page-uart.js     # UART config page
│   │   ├── 06-page-network.js  # Network config page
│   │   ├── 07-page-routing.js  # Routing page
│   │   ├── 08-page-toolbox.js  # Toolbox page
│   │   ├── 09-page-system.js   # System page
│   │   └── 10-app.js           # Vue app initialization
│   ├── css/
│   │   └── style.css           # Responsive CSS styling
│   └── modals/
│       ├── help-data-wrap-format.html # Data wrap help modal
│       └── modals-loader.js    # Modal injection system
│
└── html-bundle/                # Build output
    └── embedded_index.html     # Concatenated HTML (from build.rs)
```

## Core Components

### Main Entry Point (src/main.rs - 270 LOC)

**Responsibility:** Initialize Tokio runtime, spawn all async tasks, manage shared state

**Architecture:**
- Single-thread executor: `#[tokio::main(flavor = "current_thread")]` with epoll
- Spawns tasks:
  1. HTTP server (spawn_blocking with tiny-http)
  2. UART reader (AsyncFd/epoll)
  3. MQTT publisher (std::thread, sync rumqttc Client)
  4. HTTP publisher (tokio::spawn with spawn_blocking ureq)
  5. TCP server/client (async tasks)
  6. GPIO heartbeat task (tokio::spawn)
  7. WebSocket manager (async)

**Channels (hybrid):**
- UART → MQTT: `std::sync::mpsc` (cross-thread compatible)
- UART → HTTP: `tokio::sync::mpsc` (async, capacity 64)
- Config changes: `tokio::sync::watch<()>` (notify-only)

### Web Server (src/web_api/server.rs - 588 LOC)

**Responsibility:** HTTP routing, REST API, request handling

**Routes:**
- `GET /` → Embedded SPA (HTML + Vue 3 SPA with 10 JS modules)
- `POST /api/login`, `GET /api/session` → Authentication (SessionManager)
- `GET/POST /api/*` → REST API handlers (auth required)
- **Endpoints:** login, session, status, wifi/{status,mode,scan}, network/{apply,revert,changes}, syslog, config, password, reboot, upgrade, backup, factory-reset, toolbox
- **Port:** 8888 (configurable via UCI)
- **Frontend:** Vue 3 (CDN-delivered via 00-vue.min.js) with modular page components

### Configuration Management (src/config.rs - 486 LOC)

**Responsibility:** UCI-based config with hot-reload

**Key Components:**
- `AppState`: RwLock<Config> + watch::Sender<()> for notifications
- `Config`: MQTT, HTTP, TCP, UART, GPIO, Web settings
- Thread-safe reads via RwLock; watch<()> notifies async tasks on change
- MQTT publisher polls every 2s (can't use async watch in std::thread)

### Key Modules (Concise Summary)

| Module | LOC | Responsibility |
|--------|-----|-----------------|
| config.rs | 480 | UCI config + hot-reload |
| uci.rs | 146 | UCI CLI wrapper |
| commands.rs | 139 | Command enum + parsing |
| gpio.rs | 171 | GPIO control (chardev) |
| time_sync.rs | 83 | HTTP-based NTP |
| web_api/server.rs | 588 | HTTP routing + handlers |
| web_api/auth.rs | 141 | Session management |
| web_api/wifi.rs | 209 | WiFi 4-mode control |
| web_api/netcfg.rs | 350 | Network/NTP/routes |
| web_api/maintenance.rs | 362 | Backup/restore/upgrade |
| web_api/toolbox.rs | 135 | Toolbox API |
| web_api/syslog.rs | 165 | Syslog viewer |
| web_api/ws.rs | 121 | WebSocket (tungstenite) |
| web_api/status.rs | 210 | SharedStats collector |
| uart/reader.rs | 233 | AsyncFd/epoll UART RX |
| uart/writer.rs | 73 | UART TX queue |
| channels/mqtt.rs | 202 | MQTT pub/sub (std::thread) |
| channels/http_pub.rs | 139 | HTTP POST (spawn_blocking) |
| channels/tcp.rs | 195 | TCP server/client (async) |
| channels/buffer.rs | 222 | Offline buffer (RAM+disk) |

## Dependencies

**Key Crates:**
- tokio (1.x) — async runtime, single-thread + epoll
- tiny_http (0.12) — HTTP server, blocking
- rumqttc (0.24) — MQTT client (sync, not async)
- rustls (0.22) + webpki-roots (0.26) — TLS
- tungstenite (0.21) — WebSocket
- ureq (2.x) — HTTP POST
- serde + syslog (6) — logging
- libc (0.2) — GPIO/UART system calls

**Build Profile:**
```toml
[profile.release]
opt-level = "z"        # size optimization
lto = true             # link-time optimization
codegen-units = 1      # single codegen unit
panic = "abort"        # smaller binary
strip = true           # strip symbols
```

**Target:** ~800KB binary (release, stripped)

## Data Flow

```
┌──────────────┐
│ External MCU │
│  (Sensors)   │
└───────┬──────┘
        │ (UART)
        ▼
   ┌─────────────────┐
   │  UART Reader    │
   │  (tokio::spawn) │
   │  AsyncFd/epoll  │
   └───┬─────┬─────┬──┐
       │     │     │  │
       │     │     │  ├─▶ tokio::sync::broadcast ──▶ TCP clients
       │     │     │  │
       │     │     │  └─▶ tokio::sync::broadcast ──▶ WebSocket clients
       │     │     │
       │     │     └─▶ tokio::sync::mpsc (cap 64) ──▶ ┌─────────────────┐
       │     │                                        │ HTTP Publisher  │ ──▶ HTTP Server
       │     │                                        │ (tokio::spawn)  │
       │     │                                        └─────────────────┘
       │     │
       │     └─▶ std::sync::mpsc ──▶ ┌─────────────────┐
       │                             │ MQTT Publisher  │ ──▶ MQTT Broker
       │                             │ (std::thread)   │
       │                             └─────────────────┘
       │
       └─▶ tokio::sync::mpsc (cap 64) ──▶ Offline buffer (RAM+disk)

        ┌─────────────────────────────────────────┐
        │     HTTP Server (spawn_blocking)        │
        │     tiny-http :8888                     │
        ├─────────────────────────────────────────┤
        │ GET  /                → Vue 3 SPA (HTML) │
        │ POST /api/login       → Auth            │
        │ GET  /api/session     → Session check   │
        │ GET/POST /api/*       → REST handlers   │
        │ GET  /ws              → WebSocket       │
        └─────────────────────────────────────────┘
             ▲       ▲           ▲
             │       │           │
    Dashboard UI  REST APIs   WebSocket live
```

## Configuration Files

- **CLAUDE.md** - Project constraints (CPU, memory, architecture)
- **.claude/rules/** - Development rules and protocols
- **Cross.toml** - Cross-compilation target image
- **.cargo/config.toml** - MIPS linker configuration

## Build & Deploy

**Cross-compile for MIPS:**
```bash
cross +nightly build --target mipsel-unknown-linux-musl --release
```

**Target:** ~800KB binary size (release, stripped)

## Runtime Architecture

**Hybrid Async/Sync Model:**
- Tokio single-thread executor (epoll-based I/O)
- std::thread for MQTT (rumqttc sync Client, more stable on MIPS)
- spawn_blocking for tiny-http and ureq HTTP

**Channels:**
- UART → MQTT: `std::sync::mpsc` (cross-thread)
- UART → HTTP: `tokio::sync::mpsc` (async, capacity 64)
- Config change: `tokio::sync::watch<()>` (notify-only)

**Why this design:**
- Avoids rumqttc AsyncClient hangs on MIPS
- epoll reduces context switching on 580MHz CPU
- RwLock enables concurrent reads of config
- Non-blocking UART via AsyncFd prevents blocking
