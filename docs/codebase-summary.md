# Codebase Summary - MT7688AN IoT Gateway

**Last Updated:** 2026-02-12
**Version:** 1.0

## Overview

This is a Rust-based IoT Gateway firmware for the MediaTek LinkIt Smart 7688 (MT7688AN) running OpenWrt 21.02. The gateway provides system monitoring, network configuration management, and data publishing via MQTT and HTTP to external servers.

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

**Responsibility:** HTTP server, request routing, configuration/network management API

**Key Features:**
- Listens on `0.0.0.0:8888`
- Routes:
  - `GET /` → Dashboard with system info
  - `GET/POST /config` → MQTT/HTTP config page
  - `GET/POST /network` → Network (WAN) configuration page
  - `GET/POST /api/network` → JSON API for network config
- Spawns three background threads on startup:
  1. UART reader (reads serial data from 4G module)
  2. MQTT publisher (sends messages to MQTT broker)
  3. HTTP publisher (sends data via HTTP POST)

**Notes:**
- Uses manual JSON/form parsing (no serde_json to keep binary small)
- Handles URL encoding/decoding for form data
- Validates network config before applying

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

### Background Publishers

**uart_reader.rs** - Reads UART serial data and sends to both MQTT and HTTP publishers via channels
**mqtt_publisher.rs** - Connects to MQTT broker, publishes UART data
**http_publisher.rs** - Posts UART data to HTTP endpoint
**time_sync.rs** - Syncs system clock before TLS operations (prevents cert validation failures)

**Channel Model:**
- Main thread → UART reader: (pass serial RX)
- UART reader → Publishers: sync_channel<String> (bounded 128 messages)
- Prevents OOM on 64MB device

## Dependencies

**Cargo.toml** specifies:
- `tiny-http` - Minimal HTTP server
- `serialport` - UART serial communication
- `paho-mqtt` - MQTT client
- `musl-libc` - Static linking for portability

**Size Optimization:**
- Avoid `tokio` (async runtime adds ~1MB)
- Use `heapless` collections where available
- Enable release strip: `[profile.release] strip = true`

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
scp target/mipsel-unknown-linux-musl/release/{binary} root@10.10.10.1:/tmp/
ssh root@10.10.10.1 'chmod +x /tmp/{binary} && nohup /tmp/{binary} > /var/log/gateway.log 2>&1 &'
```

**Target Size:** < 500KB (optimized release build with stripping)

## Key Constraints

| Constraint | Impact | Mitigation |
|-----------|--------|-----------|
| 580MHz CPU (single-core) | Limited processing | Avoid heavy computations, async runtime |
| 256MB RAM | OOM risk | Bounded channels (128 msgs), no large allocations |
| 25MB available flash | Binary size limit | Use release build + strip, avoid large deps |
| MIPS 32-bit | No AtomicU64 | Use AtomicU32 or Mutex<u64> |
| No std optional | Binary size | Prefer musl for static linking |

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
