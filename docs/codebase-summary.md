# Codebase Summary - ugate IoT Gateway

**Generated:** 2026-03-08
**Version:** 3.0 (Phases 1-6 Complete)
**Total Lines of Code:** ~3,070 (ugate/src)
**Total Files:** 21

## Project Overview

**ugate** is a production-ready IoT Gateway firmware for MT7688 (MIPS 580MHz, 64MB RAM) running OpenWrt 24.10. It provides multi-channel data acquisition and command dispatch with the following capabilities:

- **Real-time UART data collection** with configurable frame detection (line-based, fixed-length, timeout-based)
- **Multi-channel fan-out** to MQTT (pub/sub), HTTP POST, and TCP (server/client)
- **Bi-directional command control** from WebSocket, TCP, and MQTT subscription
- **GPIO control** via chardev ioctl (32+ GPIO lines)
- **Web management UI** with Vue.js SPA and session authentication
- **Offline buffering** with RAM→disk overflow
- **Flexible UCI-based configuration** with hot-reload
- **Syslog integration** for OpenWrt logging

**Phases 1-6 complete.** Ready for production deployment.

## Project Structure

```
src/
├── main.rs                 # HTTP server (tiny-http), routing, request handlers
├── system_info.rs          # System stats collection (CPU, RAM, uptime, etc.)
├── config.rs               # MQTT/HTTP config storage & AppState
├── html_template.rs        # Dashboard HTML page rendering
├── html_config.rs          # Config page HTML rendering
├── network_config.rs       # Network interface configuration logic
├── html_network.rs         # Network config page HTML rendering
├── uci.rs                  # OpenWrt UCI command wrapper
├── uart_reader.rs          # UART serial data reader (background thread)
├── mqtt_publisher.rs       # MQTT client (background thread)
├── http_publisher.rs       # HTTP POST publisher (background thread)
└── time_sync.rs            # System clock synchronization

.cargo/
├── config.toml             # MIPS target linker configuration

Cross.toml                   # Cross-compilation settings for mipsel-unknown-linux-musl

Cargo.toml                   # Project dependencies

devicetree/
├── linkit.dts              # Device tree source (custom GPIO/peripheral mappings)
└── linkit.dtb              # Compiled device tree binary
```

## Core Modules

### Main Server (src/main.rs)

**Responsibility:** HTTP server, request routing, configuration/network management API (v2.0 Hybrid)

**Key Features:**
- Listens on `0.0.0.0:8889`
- Routes:
  - `GET /` → Dashboard with system info
  - `GET/POST /config` → MQTT/HTTP config page
  - `GET /network` → Redirects to /config
  - `GET/POST /api/network` → JSON API for network config
- Spawns tasks on startup (hybrid async/sync):
  1. `tokio::spawn(oled::display_loop)` - OLED display
  2. `std::thread::spawn(mqtt_publisher::run_sync)` - MQTT in OS thread (NOT tokio)
  3. `tokio::spawn(uart_reader::run)` - UART with AsyncFd
  4. `tokio::spawn(http_publisher::run)` - HTTP with spawn_blocking
  5. `spawn_blocking(run_http_server)` - tiny-http blocking server

**Architecture (v2.0):**
- Single-thread Tokio runtime: `#[tokio::main(flavor = "current_thread")]`
- Hybrid channels:
  - `std::sync::mpsc::channel` for UART → MQTT (cross-thread)
  - `tokio::sync::mpsc::channel` for UART → HTTP (async, capacity 64)
- `tokio::sync::watch<()>` for config change notifications (notify-only)
- Manual JSON/form parsing (no serde_json for small binary)
- URL encoding/decoding for form data
- Network config validation before applying

### System Info (src/system_info.rs)

**Responsibility:** Collect and expose system statistics

**Collects:**
- Uptime (from /proc/uptime)
- CPU usage (from /proc/stat)
- Memory usage (from /proc/meminfo)
- Network interface stats (via `ifconfig`)

**Returns:** `SystemInfo` struct consumed by HTML template

### Configuration Management (src/config.rs)

**Responsibility:** Runtime configuration storage and access with change notifications

**Contains:**
- `AppState` - Thread-safe config wrapper with watch notification
  - `config: RwLock<Config>` - Concurrent reads, exclusive writes
  - `config_tx: watch::Sender<()>` - Notify subscribers on update
- `Config` - MQTT, HTTP, UART, General settings
- `MqttConfig` - Broker URL, port, TLS, topic, client ID
- `HttpConfig` - Endpoint URL, enabled flag
- `UartConfig` - Serial port, baudrate, enabled flag
- `GeneralConfig` - Data collection interval (seconds)

**Thread Safety:**
- `RwLock<Config>` allows concurrent reads from multiple tasks
- `watch::Sender<()>` notifies async subscribers immediately on `state.update()`
- MQTT publisher (std::thread) polls `state.get()` every 2s instead of using watch

### Network Configuration (src/network_config.rs)

**Responsibility:** WAN interface (eth0.2) configuration management

**Features:**
- Supports DHCP and Static IP modes
- Reads/writes via OpenWrt UCI: `network.wan.proto`, `network.wan.ipaddr`, etc.
- Validation: IP format, netmask, gateway in subnet, LAN conflict check
- Applies changes via `ifdown wan` / `ifup wan` commands

**Key Structs:**
- `NetworkMode` - Enum: `Dhcp` or `Static`
- `NetworkConfig` - Configuration: mode, IP, netmask, gateway, DNS
- `NetworkStatus` - Live status: current IP, gateway, DNS servers, interface state

**Validation Functions:**
- `is_valid_ipv4()` - Checks IPv4 format (a.b.c.d)
- `is_valid_netmask()` - Validates subnet mask (contiguous 1s)
- `gateway_in_subnet()` - Confirms gateway is on same subnet
- `conflicts_with_lan()` - Prevents WAN IP from conflicting with LAN (10.10.10.0/24)

### UCI Wrapper (src/uci.rs)

**Responsibility:** Safe wrapper around OpenWrt `uci` CLI commands

**Methods:**
- `Uci::get(key)` → `Result<String, String>` - Get config value
- `Uci::set(key, value)` → `Result<(), String>` - Set config value
- `Uci::delete(key)` → `Result<(), String>` - Delete config option
- `Uci::commit(config)` → `Result<(), String>` - Apply changes

**Example:** `Uci::get("network.wan.ipaddr")` reads current WAN IP

### HTML Templates

**html_template.rs** - Dashboard page with system stats and config links
**html_config.rs** - MQTT/HTTP configuration form
**html_network.rs** - WAN network configuration form with live status display

All templates:
- Inline CSS for minimal HTML size
- Client-side JavaScript for form interactions (e.g., show/hide static IP fields)
- HTML escaping to prevent XSS

### Background Publishers (v2.0 Hybrid)

**uart_reader.rs** - Async UART reader (`tokio::spawn`)
- Uses `AsyncFd` with epoll for non-blocking serial I/O
- Opens port with `O_NONBLOCK`, configures via `libc::termios`
- Sends to MQTT via `std::sync::mpsc` (cross-thread)
- Sends to HTTP via `tokio::sync::mpsc` (async)
- Listens for config changes via `watch::Receiver<()>.changed()`

**mqtt_publisher.rs** - Sync MQTT publisher (`std::thread::spawn`)
- Uses `rumqttc::Client` (sync, NOT AsyncClient) due to MIPS compatibility issues
- Spawns separate connection thread for network I/O
- Receives UART data via `std::sync::mpsc::Receiver`
- Polls config every 2s (cannot use async watch in std::thread)
- Publishes system info at configurable interval

**http_publisher.rs** - Async HTTP publisher (`tokio::spawn`)
- Receives UART data via `tokio::sync::mpsc::Receiver`
- Uses `tokio::task::spawn_blocking` for ureq HTTP POST
- Listens for config changes via `watch::Receiver<()>.changed()`
- `tokio::select!` for multiplexing UART, config, and interval timer

**oled.rs** - Async OLED display (`tokio::spawn`)
**time_sync.rs** - Syncs system clock before TLS (prevents cert validation failures)

**Channel Model (v2.0 Actual):**
- `std::sync::mpsc::channel<String>` for UART → MQTT (cross-thread, unbounded)
- `tokio::sync::mpsc::channel<String>` for UART → HTTP (async, capacity 64)
- `tokio::sync::watch<()>` for config notifications (notify-only, no data)
- UART reader and HTTP publisher use `config_rx.changed()` in tokio::select!
- MQTT publisher polls config every 2s
- Non-blocking I/O via AsyncFd + epoll

## Dependencies

**Cargo.toml** specifies:
- `tokio` - Async runtime with single-thread executor and epoll backend
- `tiny-http` - Minimal HTTP server (blocking, wrapped in spawn_blocking)
- `libc` - Direct termios/serial configuration (no serialport crate)
- `rumqttc` - MQTT client (sync Client, not AsyncClient due to MIPS issues)
- `rustls` + `webpki-roots` - TLS for MQTT
- `ureq` - HTTP POST (with spawn_blocking wrapper)
- `toml` + `serde` - Config file parsing
- `musl-libc` - Static linking for portability

**Size Optimization:**
- Tokio single-thread executor (~1MB overhead, justified by resource efficiency on 256MB RAM)
- Use `heapless` collections where available
- Enable release strip: `[profile.release] strip = true`
- Binary target: <800KB (increased from 500KB due to async runtime)

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
   └────┬───────┬────┘
        │       │
        │       ├─▶ std::sync::mpsc ──▶ ┌────────────────┐
        │       │                       │ MQTT Publisher │ ──▶ MQTT Broker
        │       │                       │ (std::thread)  │
        │       │                       └────────────────┘
        │       │
        │       └─▶ tokio::sync::mpsc ──▶ ┌────────────────┐
        │                                 │ HTTP Publisher │ ──▶ HTTP Server
        │                                 │ (tokio::spawn) │
        │                                 └────────────────┘
        │
        ▼
  ┌───────────────────┐
  │  HTTP Server      │
  │  (spawn_blocking) │
  │  :8889            │
  └───────────────────┘
   ▲  ▲  ▲
   │  │  └──── /api/network (JSON)
   │  └─────── /config (HTML form)
   └────────── / (Dashboard)
```

## Configuration Files

- **CLAUDE.md** - Project constraints (CPU, memory, architecture)
- **.claude/rules/** - Development rules and protocols
- **Cross.toml** - Cross-compilation target image
- **.cargo/config.toml** - MIPS linker configuration

## Compilation & Deployment

**Build:**
```bash
cross +nightly build --target mipsel-unknown-linux-musl --release
```

**Deploy:**
```bash
scp -O target/mipsel-unknown-linux-musl/release/{binary} root@10.10.10.1:/tmp/
ssh root@10.10.10.1 'chmod +x /tmp/{binary} && nohup /tmp/{binary} > /var/log/gateway.log 2>&1 &'
```

**Target Size:** < 500KB (optimized release build with stripping)

## Key Constraints

| Constraint | Impact | Mitigation |
|-----------|--------|-----------|
| 580MHz CPU (single-core) | Limited processing | Tokio async avoids context switching overhead |
| 256MB RAM | OOM risk | Broadcast channels (128 msgs), tokio lightweight tasks |
| 25MB available flash | Binary size limit | Release build + strip, careful dep selection |
| MIPS 32-bit | No AtomicU64 | Use AtomicU32 or Mutex<u64> |
| No std optional | Binary size | Prefer musl for static linking, tokio has minimal overhead |

## v2.0 Async Refactor Key Changes

| Component | v1.0 (Blocking) | v2.0 (Hybrid Async) |
|-----------|-----------------|---------------------|
| Runtime | Standard threads | Tokio single-thread executor |
| UART I/O | BufReader blocking | AsyncFd with epoll (`tokio::spawn`) |
| MQTT | rumqttc::Client | rumqttc::Client in `std::thread::spawn` (NOT AsyncClient) |
| HTTP Server | tiny-http blocking | tiny-http in `spawn_blocking` |
| HTTP Publisher | std::thread::spawn | `tokio::spawn` + `spawn_blocking(ureq)` |
| UART→MQTT Channel | std::sync::mpsc | std::sync::mpsc (unchanged, cross-thread) |
| UART→HTTP Channel | std::sync::mpsc | tokio::sync::mpsc (async, capacity 64) |
| Config Notification | Polling | tokio::sync::watch<()> (MQTT still polls) |
| LED/OLED | thread::spawn | tokio::spawn |
| Config file | `/etc/v3s-monitor.toml` | `/etc/vgateway.toml` |
| HTTP Port | 8888 | 8889 |
| Binary name | v3s-system-monitor | vgateway |

**Why MQTT uses std::thread instead of tokio::spawn:**
- rumqttc AsyncClient has compatibility issues on MIPS architecture
- Sync Client with dedicated connection thread is more reliable
- Config polling (2s) is acceptable overhead for reliability

## Error Handling

- UCI commands: Check exit code, capture stderr
- Network commands: Validate before application (IP format, subnet mask, conflicts)
- UART: Bounded channels prevent blocking
- Publishers: Retry logic in background threads

## Security Considerations

- HTML escaping in templates prevents XSS
- UCI commands executed with proper quoting
- No hardcoded credentials (read from config files)
- TLS support in MQTT (via paho-mqtt)
- Clock sync before TLS operations (cert validation)

## Testing Strategy

- Manual testing on MT7688AN device
- Cross-compile verification (binary architecture check)
- Configuration validation (IP, netmask, gateway)
- UART data flow testing (loopback)
- MQTT connection testing (broker availability)

## Future Enhancements

- Device tree customization for additional GPIO/peripherals
- Multi-interface support (eth0.1 for LAN, eth0.2 for WAN)
- Advanced network features (static routes, firewall rules)
- Web UI improvements (Bootstrap CSS, responsiveness)
- OTA update mechanism
