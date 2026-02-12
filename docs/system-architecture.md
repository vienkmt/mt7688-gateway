# System Architecture - MT7688AN IoT Gateway

**Last Updated:** 2026-02-12
**Version:** 1.0

## Architecture Overview

The MT7688AN IoT Gateway is a Rust-based embedded system that collects sensor/device data via UART and publishes it to remote servers (MQTT, HTTP) while providing a web-based management interface for configuration and monitoring.

### High-Level Components

```
┌─────────────────────────────────────────────────────────────┐
│                    IoT Gateway (Rust)                       │
├─────────────────────────────────────────────────────────────┤
│                                                               │
│  ┌───────────────┐  ┌───────────────┐  ┌───────────────┐   │
│  │  Web Server   │  │  UART Reader  │  │   Time Sync   │   │
│  │  (tiny-http)  │  │  (Background) │  │ (Startup)     │   │
│  │   :8888       │  │   /dev/ttyS0  │  │               │   │
│  └───────────────┘  └───────────────┘  └───────────────┘   │
│         △                    △                                │
│         │                    │                                │
│    ┌────┴──────────┬─────────┴────┐                          │
│    │               │               │                          │
│    ▼               ▼               ▼                          │
│ ┌──────────┐ ┌──────────┐ ┌──────────┐                      │
│ │ Network  │ │  System  │ │ Config   │                      │
│ │ Config   │ │  Info    │ │ Manager  │                      │
│ └──────────┘ └──────────┘ └──────────┘                      │
│    △ │           △            △  │                           │
│    │ │           │            │  │                           │
│    │ ▼           ▼            │  ▼                           │
│    │ ┌─────────────────────┐  │ ┌──────────────────────┐    │
│    │ │   UCI Wrapper       │  │ │  AppState (Config)   │    │
│    │ │   /etc/config/net   │  │ │  Arc<Mutex<...>>     │    │
│    └─┤                     │  └─┤                      │    │
│      │  [get|set|delete|   │    │ MQTT/HTTP/UART       │    │
│      │   commit]           │    │ settings             │    │
│      └─────────────────────┘    └──────────────────────┘    │
│                                                               │
│  ┌──────────────────┐     ┌──────────────────┐              │
│  │  MQTT Publisher  │     │ HTTP Publisher   │              │
│  │  (Background)    │     │ (Background)     │              │
│  │  Paho MQTT       │     │ HTTP POST        │              │
│  └──────────────────┘     └──────────────────┘              │
│         △                          △                          │
└─────────┼──────────────────────────┼──────────────────────────┘
          │                          │
          │ (from UART reader)       │ (from UART reader)
          │                          │
    ┌─────▼──────────────────────────▼─────┐
    │     Bounded Channels (128 messages)   │
    │     (Prevents OOM on 64MB device)     │
    └─────────────────────────────────────────┘
          △
          │ (UART serial data)
          │
    ┌─────┴──────────┐
    │  4G Modem      │
    │  (Quectel)     │
    │  /dev/ttyS1    │
    └────────────────┘
```

## Module Architecture

### 1. HTTP Server (main.rs)

**Purpose:** Accept configuration changes and provide system monitoring UI

**Endpoints:**

| Endpoint | Method | Purpose | Response |
|----------|--------|---------|----------|
| `/` | GET | Dashboard with system stats | HTML |
| `/config` | GET | Configuration form | HTML |
| `/config` | POST | Save MQTT/HTTP/UART settings | HTML (with status) |
| `/network` | GET | Network configuration form | HTML |
| `/network` | POST | Save WAN settings | HTML (with validation errors) |
| `/api/network` | GET | Get WAN config as JSON | JSON |
| `/api/network` | POST | Set WAN config from JSON | JSON (with errors) |

**Request Handler Flow:**

```
HTTP Request
    │
    ▼
Parse URL + Method
    │
    ├─ "/" ──────────────▶ system_info::SystemInfo::collect() ──▶ html_template::render_page()
    │
    ├─ "/config" GET ────▶ config::AppState::get() ──▶ html_config::render_config_page()
    │
    ├─ "/config" POST ───▶ parse_config_form() ──▶ config::AppState::update() ──▶ html_config::render_config_page()
    │
    ├─ "/network" GET ───▶ NetworkConfig::load_from_uci() ──▶ NetworkStatus::get_current() ──▶ html_network::render_network_page()
    │
    ├─ "/network" POST ──▶ parse_network_form() ──▶ validate_config() ──▶ save_to_uci() ──▶ html_network::render_network_page()
    │
    ├─ "/api/network" GET ──▶ NetworkConfig::load_from_uci() ──▶ format_network_json()
    │
    └─ "/api/network" POST ─▶ parse_network_json() ──▶ validate_config() ──▶ save_to_uci() ──▶ format_network_json()
```

### 2. Network Configuration (network_config.rs + html_network.rs)

**Responsibility:** Manage WAN interface (eth0.2) configuration

**Architecture:**

```
User submits form (HTML or JSON API)
    │
    ▼
Parse input (URL-encoded or JSON)
    │
    ▼
Create NetworkConfig struct
    │
    ▼
validate_config()
    ├─ DHCP mode: No validation
    └─ Static mode:
       ├─ IP format check (is_valid_ipv4)
       ├─ Netmask validity (is_valid_netmask)
       ├─ Gateway in subnet (gateway_in_subnet)
       ├─ LAN conflict check (conflicts_with_lan: 10.10.10.0/24)
       └─ DNS format validation
    │
    ▼ (if valid)
NetworkConfig::save_to_uci()
    ├─ UCI::set("network.wan.proto", mode)
    ├─ (Static) UCI::set ipaddr, netmask, gateway, dns
    ├─ (DHCP) UCI::delete ipaddr, netmask, gateway, dns
    ├─ UCI::commit("network")
    └─ ifdown wan; ifup wan (restart interface)
    │
    ▼
Get live status via NetworkStatus::get_current()
    ├─ ip addr show eth0.2 (parse IP, netmask)
    ├─ ip route (parse gateway)
    └─ cat /tmp/resolv.conf.d/resolv.conf.auto (parse DNS)
    │
    ▼
Render response (HTML form or JSON)
```

**Configuration Flow (UCI):**

```
User sets Static IP: 192.168.1.100, Netmask: 255.255.255.0, GW: 192.168.1.1
    │
    ▼
UCI commands executed:
    uci set network.wan.proto=static
    uci set network.wan.ipaddr=192.168.1.100
    uci set network.wan.netmask=255.255.255.0
    uci set network.wan.gateway=192.168.1.1
    uci set network.wan.dns="8.8.8.8 8.8.4.4"
    uci commit network
    │
    ▼
/etc/config/network updated:
    config interface 'wan'
        option proto 'static'
        option ifname 'eth0.2'
        option ipaddr '192.168.1.100'
        option netmask '255.255.255.0'
        option gateway '192.168.1.1'
        option dns '8.8.8.8 8.8.4.4'
    │
    ▼
Interface restarted:
    ifdown wan
    ifup wan
    │
    ▼
eth0.2 now has static IP (verified via ip addr show eth0.2)
```

**Validation Rules (Static Mode):**

| Check | Rule | Error Message |
|-------|------|---------------|
| IP Format | `a.b.c.d` where 0≤a,b,c,d≤255 | "Invalid IP address format" |
| Netmask | Contiguous 1s followed by 0s | "Invalid subnet mask" |
| Gateway Format | Valid IPv4 | "Invalid gateway format" |
| Gateway in Subnet | GW & Mask = IP & Mask | "Gateway not in same subnet" |
| LAN Conflict | IP not in 10.10.10.0/24 | "IP conflicts with LAN" |
| Primary DNS | Valid IPv4 (if not empty) | "Invalid primary DNS" |
| Secondary DNS | Valid IPv4 (if not empty) | "Invalid secondary DNS" |

### 3. Configuration Manager (config.rs)

**Thread-Safe Config Storage:**

```rust
pub struct AppState {
    config: Mutex<Config>
}

pub struct Config {
    mqtt: MqttConfig,
    http: HttpConfig,
    uart: UartConfig,
    general: GeneralConfig,
}
```

**Access Pattern:**

```
Main Thread                Background Publishers
    │                              │
    ├─ state.get() ──────────▶ Acquire lock
    │  (read-only)            Get copy of Config
    │                         Release lock
    │                              │
    ├─ state.update() ──────▶ Acquire lock
    │  (write)                Update Config
    │                         Release lock
    │                              │
    └───────────────────────────────▶ Publishers read latest config
```

**Mutex Prevents:**
- Concurrent modification
- Partial reads (torn updates)
- Data races on MIPS 32-bit

### 4. UART & Data Publishing

**Three Background Threads:**

```
Startup (main.rs)
    │
    ├─ Create sync_channel(128) for MQTT
    ├─ Create sync_channel(128) for HTTP
    ├─ Spawn uart_reader (with both TX sides)
    ├─ Spawn mqtt_publisher (with RX side)
    └─ Spawn http_publisher (with RX side)
    │
    ▼
UART Reader Loop (uart_reader.rs)
    │
    ├─ Open /dev/ttyS1 (Quectel modem)
    ├─ Read line (blocking on UART)
    ├─ Parse data if valid
    ├─ Send to MQTT channel (mqtt_uart_tx.send())
    ├─ Send to HTTP channel (http_uart_tx.send())
    └─ Loop (bounded channel blocks if full, prevents OOM)
    │
    ▼
Publishers Loop
    ├─ MQTT: Receive ──▶ Publish to topic ──▶ If error, reconnect
    └─ HTTP: Receive ──▶ POST to endpoint ──▶ If error, retry with backoff
```

**Channel Bounds:**
- **128 messages max** per channel
- **Prevents OOM:** If publisher is slow, channel fills up → uart_reader blocks → no data lost, device stable
- **Configurable interval:** `general_config.interval_secs` throttles UART reads

### 5. Web UI Architecture

**HTML Templates:**

| File | Purpose | Route | Dynamic Content |
|------|---------|-------|-----------------|
| html_template.rs | Dashboard | GET / | Uptime, CPU%, RAM%, interface stats |
| html_config.rs | Config form | GET/POST /config | MQTT broker, HTTP URL, UART settings |
| html_network.rs | Network form | GET/POST /network | WAN IP mode, static fields, live status |

**Template Pattern:**

```rust
fn render_page(data: &Struct) -> String {
    format!(r#"<!DOCTYPE html>...{field1}...{field2}..."#,
        field1 = html_escape(&data.field1),  // Prevent XSS
        field2 = data.field2,
    )
}
```

**Client-Side Interactivity:**

```html
<!-- Example: Show/hide static IP fields based on mode selection -->
<input type="radio" name="mode" value="dhcp" onclick="toggleStatic(false)">
<input type="radio" name="mode" value="static" onclick="toggleStatic(true)">

<div id="static-fields" style="display:none">
  <!-- IP, Netmask, Gateway, DNS fields -->
</div>

<script>
function toggleStatic(show) {
  document.getElementById('static-fields').style.display = show ? 'block' : 'none';
}
</script>
```

### 6. UCI Integration (uci.rs)

**OpenWrt Unified Configuration Interface Wrapper:**

```
Rust Code
    │
    ├─ Uci::get("network.wan.ipaddr")
    │   └─▶ Command::new("uci").args(["get", "network.wan.ipaddr"])
    │       └─▶ /etc/config/network file read
    │           └─▶ Returns: "192.168.1.100"
    │
    ├─ Uci::set("network.wan.ipaddr", "192.168.1.100")
    │   └─▶ Command::new("uci").args(["set", "network.wan.ipaddr=192.168.1.100"])
    │       └─▶ Staging area updated
    │
    ├─ Uci::delete("network.wan.ipaddr")
    │   └─▶ Command::new("uci").args(["delete", "network.wan.ipaddr"])
    │       └─▶ Option removed from staging
    │
    └─ Uci::commit("network")
        └─▶ Command::new("uci").args(["commit", "network"])
            └─▶ /etc/config/network file written
```

**Error Handling:**

- UCI commands may fail (permission denied, syntax error)
- Each function returns `Result<T, String>`
- Caller decides to retry, log, or propagate error
- Network config validation happens **before** UCI calls to minimize failures

## Concurrency Model

**Threads:**

| Thread | Purpose | Shared State | Sync Mechanism |
|--------|---------|--------------|---|
| Main | HTTP server loop | AppState (Config) | Arc<Mutex<>> |
| UART Reader | Read serial, dispatch to publishers | Channel senders | sync_channel (bounded) |
| MQTT Publisher | Publish to broker | AppState (Config) | Arc<Mutex<>> + Channel |
| HTTP Publisher | POST to endpoint | AppState (Config) | Arc<Mutex<>> + Channel |

**No Data Races:**
- Mutex protects Config
- Channels ensure ordered delivery
- Bounded channels prevent OOM

## Memory & Storage

**RAM Usage (Target: 256MB total, ~100MB available for application):**

- HTTP server: ~5MB
- Config + state: <1MB
- UART/publishers: <5MB
- Channel buffers (128 × 1KB avg): ~1MB
- Total: ~12MB comfortable, <<100MB budget

**Storage (Target: 25MB available):**

- Binary: ~400KB
- /tmp logs: ~1MB
- /etc/config/: <1MB
- Total: ~2MB used, 23MB free for logs, firmware updates

## Error Handling Strategy

| Layer | Errors | Action |
|-------|--------|--------|
| **HTTP Request** | Parse error, invalid URL | 404 or 400 response |
| **Config Validation** | Bad network settings | Display errors in form, no apply |
| **UCI Commands** | Permission denied, syntax error | Log error, return to user |
| **UART/Publishers** | Connection lost, timeout | Retry in background, log |
| **Async Channels** | Channel full (backpressure) | UART reader blocks (safe) |

## Security Considerations

| Issue | Mitigation |
|-------|-----------|
| XSS (Web UI) | HTML escaping in templates |
| Command injection (UCI) | Proper argument quoting |
| Unauthorized access | No authentication (LAN-only, trusted network) |
| TLS/MQTT | Time sync at startup, certificate validation |
| UART data | Validation before storage/publishing |

## Performance Characteristics

**HTTP Requests:**
- Latency: <100ms (local network)
- Throughput: ~100 req/sec (tiny-http single-threaded)

**UART Publishing:**
- Latency: <1s (depends on interval setting)
- Throughput: 100 messages/sec (with 128-message buffer)

**MQTT/HTTP:**
- Connection time: ~2s (TLS handshake)
- Publish latency: <500ms (network dependent)
- Backoff: Exponential retry on disconnect

**System Info Collection:**
- Frequency: On-demand (per /dashboard request)
- Time: <10ms (file reads only, no heavy computation)

## Extensibility Points

**Add New Configuration:**
1. Add field to `Config` struct (config.rs)
2. Add form field to `html_config::render_config_page()`
3. Add parsing in `parse_config_form()`
4. Access in publishers via `state.get().new_field`

**Add New HTTP Endpoint:**
1. Add route match in main.rs HTTP loop
2. Create handler function
3. Return `tiny_http::Response::from_string()`

**Add New Network Interface:**
1. Extend `NetworkConfig` to support `eth0.1` (LAN)
2. Update UCI paths: `network.lan.proto`
3. Add validation rules for multiple interfaces

## Testing Approach

**Unit Tests:**
- Validation functions (is_valid_ipv4, gateway_in_subnet)
- Config parsing (parse_config_form, parse_network_json)

**Integration Tests:**
- HTTP endpoints (mock server)
- UCI commands (on device or docker)
- UART data flow (loopback)

**Device Tests:**
- Cross-compile and deploy to MT7688AN
- Verify web UI loads
- Test network config apply
- Monitor resource usage

**CI/CD:**
- Cross-compilation check (weekly)
- Binary size verification (<500KB)
- Documentation consistency check
