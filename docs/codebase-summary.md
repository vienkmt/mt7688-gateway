# Codebase Summary - MT7688AN IoT Gateway

**Last Updated:** 2026-03-05
**Version:** 2.0 (Tokio Async Refactor)

## Overview

This is a Rust-based IoT Gateway firmware for the MediaTek LinkIt Smart 7688 (MT7688AN) running OpenWrt 21.02. The gateway provides system monitoring, network configuration management, and data publishing via MQTT and HTTP to external servers.

**v0.2.0 Refactor (2026-03-05):** Migrated from multi-thread blocking I/O to Tokio async/await with single-thread executor (epoll). Binary renamed from `v3s-system-monitor` to `vgateway`. HTTP port 8888 → 8889. Config file `/etc/v3s-monitor.toml` → `/etc/vgateway.toml`.

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

**Responsibility:** HTTP server, request routing, configuration/network management API (Async v0.2.0)

**Key Features:**
- Listens on `0.0.0.0:8889`
- Routes:
  - `GET /` → Dashboard with system info
  - `GET/POST /config` → MQTT/HTTP config page
  - `GET/POST /network` → Network (WAN) configuration page
  - `GET/POST /api/network` → JSON API for network config
- Spawns async tasks on startup:
  1. UART reader (AsyncFd non-blocking, epoll multiplexing)
  2. MQTT publisher (tokio async, AsyncClient)
  3. HTTP publisher (spawn_blocking for ureq)
  4. LED heartbeat (tokio::spawn GPIO blink)
  5. OLED display (tokio::spawn display updates)

**Architecture (v0.2.0):**
- Single-thread Tokio runtime with epoll
- Uses tokio::sync::broadcast for UART data multicast
- Uses tokio::sync::watch for config change notifications
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

**Responsibility:** Runtime configuration storage and access

**Contains:**
- `AppState` - Thread-safe config wrapper (Arc<Mutex<Config>>)
- `Config` - MQTT, HTTP, UART, General settings
- `MqttConfig` - Broker URL, port, TLS, topic, client ID
- `HttpConfig` - Endpoint URL
- `UartConfig` - Serial port, baudrate
- `GeneralConfig` - Data collection interval (seconds)

**Thread Safety:** `Arc<Mutex<Config>>` allows safe concurrent access from main and publisher threads

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

### Background Publishers (Async v0.2.0)

**uart_reader.rs** - Async UART reader (AsyncFd with epoll), broadcasts data to all subscribers
**mqtt_publisher.rs** - Async MQTT task (rumqttc::AsyncClient), subscribes to UART broadcast
**http_publisher.rs** - Async HTTP task (spawn_blocking for ureq POST), subscribes to UART broadcast
**led_controller.rs** - Async LED heartbeat task (GPIO toggle), watches config for blink rate changes
**oled_controller.rs** - Async OLED display task (I2C), watches config for updates
**time_sync.rs** - Syncs system clock before TLS (prevents cert validation failures)

**Channel Model (v0.2.0):**
- tokio::sync::broadcast<String> (capacity 128) for UART data
- tokio::sync::watch<Config> for configuration change notifications
- All tasks spawn via tokio::spawn (lightweight, single-thread executor)
- Non-blocking I/O via AsyncFd + epoll
- Prevents OOM on 256MB device with efficient resource usage

## Dependencies

**Cargo.toml** specifies:
- `tokio` (v0.2.0) - Async runtime with epoll backend (now included for better resource efficiency)
- `tiny-http` - Minimal HTTP server
- `serialport` or `AsyncFd` - UART async communication
- `rumqttc::AsyncClient` - Async MQTT client
- `ureq` - HTTP POST (with spawn_blocking wrapper)
- `musl-libc` - Static linking for portability

**Size Optimization:**
- Tokio single-thread executor (~1MB overhead, justified by resource efficiency on 256MB RAM)
- Use `heapless` collections where available
- Enable release strip: `[profile.release] strip = true`
- Binary target: <800KB (increased from 500KB due to async runtime)

## Data Flow

```
┌──────────────┐
│  4G Module   │
│  (Quectel)   │
└───────┬──────┘
        │ (UART)
        ▼
   ┌─────────┐
   │  UART   │
   │ Reader  │ ──────┐
   └────┬────┘       │
        │            ├─▶ ┌────────────┐
        │            │   │   MQTT     │ ──▶ MQTT Broker
        │            │   │ Publisher  │
        │            │   └────────────┘
        │            │
        │            └─▶ ┌────────────┐
        │                │   HTTP     │ ──▶ HTTP Server
        │                │ Publisher  │
        │                └────────────┘
        │
        ▼
  ┌──────────────┐
  │  HTTP Server │
  │  (:8888)     │
  └──────────────┘
   ▲  ▲  ▲
   │  │  └──── /api/network, /network (JSON/HTML)
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

## v0.2.0 Async Refactor Key Changes

| Component | v0.1 (Blocking) | v0.2 (Async) |
|-----------|-----------------|------------|
| Runtime | Standard threads | Tokio single-thread executor |
| UART I/O | BufReader blocking | AsyncFd with epoll |
| MQTT | rumqttc::Client | rumqttc::AsyncClient |
| HTTP Server | tiny-http blocking | tiny-http with async handling |
| HTTP Publisher | std::thread::spawn | spawn_blocking(ureq) |
| Channels | std::sync::mpsc | tokio::sync::broadcast + watch |
| LED/OLED | thread::spawn | tokio::spawn |
| Config file | `/etc/v3s-monitor.toml` | `/etc/vgateway.toml` |
| HTTP Port | 8888 | 8889 |
| Binary name | v3s-system-monitor | vgateway |

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
