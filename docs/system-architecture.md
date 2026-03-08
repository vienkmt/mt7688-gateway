# System Architecture - ugate IoT Gateway

**Last Updated:** 2026-03-08
**Version:** 3.0 (Phase 1-6 Complete)

## Architecture Overview

**ugate** is a hybrid async/sync IoT Gateway for MT7688 that collects binary/text data via UART and fan-outs to MQTT, HTTP, and TCP channels while accepting commands from multiple sources (WebSocket, TCP, MQTT) to drive GPIO and UART TX. The design prioritizes resource efficiency on 64MB RAM using Tokio single-thread async executor with epoll I/O multiplexing.

### High-Level Components (Phase 1-6)

```
┌──────────────────────────────────────────────────────────────────────┐
│                     ugate - Tokio (single_thread)                     │
├──────────────────────────────────────────────────────────────────────┤
│                                                                        │
│  ┌─────────────────┐  ┌──────────────────┐  ┌────────────────┐      │
│  │  Web Server     │  │  UART Reader     │  │  Time Sync     │      │
│  │  (tiny-http)    │  │  (AsyncFd)       │  │  (HTTP NTP)    │      │
│  │  :8888          │  │  /dev/ttyS*      │  │  (Startup)     │      │
│  │  spawn_blocking │  │  epoll + select! │  │                │      │
│  └─────────────────┘  └──────────────────┘  └────────────────┘      │
│         │                     │ (broadcast 64)                         │
│         │                     ├─────────────────────┐                 │
│         │                     ▼                     ▼                 │
│  ┌──────▼────────────────┐                    ┌────────────┐         │
│  │  WebSocket Manager    │                    │  UART TX   │         │
│  │  (tungstenite)        │                    │  Writer    │         │
│  │  Real-time logs/stats │                    │  (async)   │         │
│  └───────────────────────┘                    └────────────┘         │
│                                                                        │
│  ┌──────────────────────────────────────────────────────────────┐   │
│  │                    Fan-Out Hub                                │   │
│  │  ┌──────────┐  ┌──────────┐  ┌──────────┐  ┌──────────┐    │   │
│  │  │  MQTT    │  │  HTTP    │  │  TCP Srv │  │  TCP Cli │    │   │
│  │  │  Pub     │  │  POST    │  │  (async) │  │  (async) │    │   │
│  │  │(std:thr) │  │  (async) │  │          │  │          │    │   │
│  │  └──────────┘  └──────────┘  └──────────┘  └──────────┘    │   │
│  │     [Sub]         [Response]    [Bi-dir]     [Bi-dir]       │   │
│  └──────────────────────────────────────────────────────────────┘   │
│         │ (cmd)              │ (cmd)           │ (cmd)                │
│         └──────────┬─────────┴──────────┬──────┘                     │
│                    │                    │                            │
│                    ▼                    ▼                            │
│            ┌──────────────────────────────────┐                     │
│            │   Command Merge + Dispatch       │                     │
│            │   (tokio::mpsc 32 capacity)      │                     │
│            └──────────────────────────────────┘                     │
│                    │                    │                            │
│         ┌──────────┘                    └──────────┐                │
│         ▼                                          ▼                │
│  ┌────────────────┐                        ┌────────────────┐      │
│  │  GPIO Control  │                        │  UART TX Queue │      │
│  │  (chardev io)  │                        │  (async)       │      │
│  │  32+ GPIO      │                        │  (serial write)│      │
│  └────────────────┘                        └────────────────┘      │
│                                                                       │
│  ┌────────────────────────────────────────────────────────────────┐ │
│  │              Shared State Management                           │ │
│  │  ┌────────────────────────────────────────────────────────┐  │ │
│  │  │  AppState: RwLock<Config> + watch<()> notifier       │  │ │
│  │  │  - MQTT config (broker, auth, topic, QoS)            │  │ │
│  │  │  - HTTP config (URL, method)                         │  │ │
│  │  │  - TCP config (server port, client host:port)        │  │ │
│  │  │  - UART config (port, baud, frame mode, timeout)     │  │ │
│  │  │  - GPIO config (32+ line definitions)                │  │ │
│  │  │  - Web config (port, auth password, ws max conn)     │  │ │
│  │  └────────────────────────────────────────────────────────┘  │ │
│  │  ┌────────────────────────────────────────────────────────┐  │ │
│  │  │  SharedStats: Atomic counters (status API)            │  │ │
│  │  │  - UART frame count, MQTT/HTTP/TCP sent/received      │  │ │
│  │  │  - Channel state (connected=2, connecting=1, down=0)  │  │ │
│  │  │  - Uptime, CPU%, RAM%, GPIO toggle count              │  │ │
│  │  └────────────────────────────────────────────────────────┘  │ │
│  │  ┌────────────────────────────────────────────────────────┐  │ │
│  │  │  SessionManager: VecDeque<token> (auth)               │  │ │
│  │  │  - Max 4 concurrent sessions, token expiry via reload │  │ │
│  │  └────────────────────────────────────────────────────────┘  │ │
│  └────────────────────────────────────────────────────────────────┘ │
│                                                                       │
│  ┌────────────────────────────────────────────────────────────────┐ │
│  │              Offline Buffer (RAM + Disk)                       │ │
│  │  - RAM queue (64 messages) → /tmp/ugate_buffer/buffer.hex     │ │
│  │  - On reconnect: read disk first (FIFO), then RAM             │ │
│  │  - HEX encoding for binary data preservation                  │ │
│  └────────────────────────────────────────────────────────────────┘ │
└──────────────────────────────────────────────────────────────────────┘
                                │
                                │ (UART RX)
                                ▼
                        ┌──────────────────┐
                        │  External MCU    │
                        │  (Modbus/binary) │
                        │  /dev/ttyS0      │
                        └──────────────────┘
```

## Module Architecture

### 1. HTTP Server & WebSocket (main.rs + web/server.rs + web/ws.rs)

**Purpose:** REST API (config, login, GPIO), WebSocket (real-time logs/stats), static UI

**Endpoints:**

| Endpoint | Method | Purpose | Response | Auth |
|----------|--------|---------|----------|------|
| `/` | GET | Index.html (Vue SPA) | HTML | No |
| `/api/login` | POST | Authenticate, get session | JSON (token) | No |
| `/api/config` | GET | Get all config | JSON | Yes |
| `/api/config` | POST | Update config | JSON + save to UCI | Yes |
| `/api/status` | GET | Real-time stats | JSON | No |
| `/api/gpio/{pin}` | POST | Control GPIO (set/toggle) | JSON | Yes |
| `/api/uart/tx` | POST | Send data to UART TX | JSON | Yes |
| `/ws` | UPGRADE | WebSocket (logs/stats) | Binary frames | No |

**Server Details:**
- Runtime: Tokio single-thread executor (`#[tokio::main(flavor = "current_thread")]`)
- HTTP Server: `spawn_blocking(tiny-http::Server::http)`
- WebSocket: tungstenite in async task, broadcasts UART data & system stats
- Port: 8888 (configurable via UCI: config.web.port)
- Static UI: Embedded Vue.js binary in include_str!("embedded_index.html")
- Auth: Session cookies (1h expiry in RAM), password in config

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

### 2. UART Reader (uart/mod.rs + uart/reader.rs + uart/writer.rs)

**Responsibility:** Non-blocking serial I/O with multiple frame detection modes

**Architecture:**

```
Startup: open /dev/ttyS* (e.g., /dev/ttyS0)
    │
    ▼
AsyncFd::new(fd) ← Wrap in AsyncFd for epoll
    │
    ▼
tokio::select! {
    _ = config_watch.changed() => reconnect with new settings
    readable = async_fd.readable() => {
        read frame(s)
        broadcast to all subscribers (64 capacity)
    }
}
    │
    ├─ Frame Mode: Line (delimited by \n or \r\n)
    ├─ Frame Mode: Fixed length (e.g., 128 bytes) + timeout fallback
    └─ Frame Mode: Timeout (collect bytes until gap_ms with no data)
    │
    ▼
Parse frame data (binary or text)
    │
    ├─ Format option 1: Raw bytes (keep as-is)
    ├─ Format option 2: Hex string (encode bytes to "aabbcc...")
    └─ Format option 3: ASCII (text-only, skip non-printable)
    │
    ▼
Broadcast<Vec<u8>> to all subscribers:
    ├─ TCP server ──────▶ Send to all clients
    ├─ TCP client ──────▶ Send upstream
    ├─ MQTT tx ─────────▶ std::sync::mpsc (to MQTT publisher OS thread)
    └─ HTTP tx ─────────▶ tokio::sync::mpsc (to HTTP publisher async task)
```

**Configuration (from /etc/config/ugate):**

```ini
config uart 'main'
    option enabled '1'
    option port '/dev/ttyS0'       # UART device
    option baudrate '115200'       # 9600, 19200, 38400, 57600, 115200
    option data_bits '8'           # 7, 8
    option parity 'none'           # none, even, odd
    option stop_bits '1'           # 1, 2
    option frame_mode 'line'       # line, fixed, timeout
    option frame_length '128'      # for fixed mode: bytes
    option frame_timeout_ms '100'  # for timeout mode: ms
    option gap_ms '10'             # between bytes before EOF
```

### 3. Configuration Management (config.rs)

**Responsibility:** UCI-based config with hot-reload notification

**Architecture:**

```
AppState (Arc<_>)
    │
    ├─ config: RwLock<Config>     ← Thread-safe read-heavy access
    └─ config_tx: watch::Sender   ← Notify UART/HTTP on change
         │
         └─ config_rx: watch::Receiver (for MQTT: polling every 2s)
    │
    ▼
Config struct contains:
    ├─ mqtt: MqttConfig (broker, port, auth, tls, topic, qos)
    ├─ http: HttpConfig (url, method POST/GET)
    ├─ tcp: TcpConfig (mode: server/client/both, ports, host)
    ├─ uart: UartConfig (port, baud, frame mode, timeout)
    ├─ gpio: GpioConfig (32+ GPIO line definitions)
    ├─ web: WebConfig (port, password, max_ws_conn)
    └─ general: GeneralConfig (log_level, buffer_ram_limit)
    │
    ▼
Load from UCI:
    uci get ugate.mqtt.broker
    uci get ugate.http.enabled
    → Defaults if missing or invalid
    │
    ▼
On HTTP POST /api/config:
    1. Parse JSON payload
    2. Update AppState::config (RwLock write lock)
    3. Save back to UCI with uci set + uci commit
    4. Broadcast config_tx.send(()) ← Wake UART/HTTP
    5. MQTT polls every 2s (can't use watch in std::thread)
```

### 4. Command Dispatch (commands.rs)

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

**Responsibility:** Convert incoming commands to GPIO/UART TX actions

**Architecture:**

```
Command Sources:
    ├─ WebSocket: /ws → json_parse_command() → Command enum
    ├─ TCP: binary/JSON from server/client
    ├─ HTTP Response: from POST response body
    ├─ MQTT Sub: message on config.mqtt.sub_topic
    └─ API: POST /api/gpio/{pin}
    │
    ▼
Command enum variants:
    ├─ GpioSet { pin: u8, state: bool }
    ├─ GpioToggle { pin: u8 }
    ├─ GpioPulse { pin: u8, ms: u16 }
    ├─ UartTx { data: String }
    └─ UartTxHex { data: Vec<u8> }
    │
    ▼
Command merge (tokio::mpsc) → dispatcher:
    │
    ├─ GPIO command → gpio_tx (async channel to GPIO task)
    ├─ UART command → uart_writer::queue (async enqueue)
    └─ Echo back to WebSocket clients (via broadcast)
    │
    ▼
GPIO task (gpio.rs):
    ├─ Apply chardev ioctl for GPIO control
    ├─ Queue GPIO state changes
    └─ Count GPIO operations (SharedStats)
    │
    ▼
UART Writer (uart/writer.rs):
    └─ Async write to /dev/ttyS* (queued, non-blocking)

**Responsibility:** Async publish UART frames to MQTT broker, subscribe to command topic

**Architecture (std::thread + rumqttc sync Client):**

**Why std::thread?** rumqttc AsyncClient causes hangs on MIPS; sync Client in OS thread is more stable.

```
std::thread::spawn(mqtt::run_sync)
    │
    ├─ Create rumqttc::Client (with auth, TLS, client_id)
    ├─ Connect to broker (with exponential backoff on failure)
    ├─ Subscribe to config.mqtt.sub_topic (for command RX)
    │
    ├─ Main loop:
    │   ├─ tokio/std select! (polling style):
    │   │   ├─ UART RX: uart_rx.recv_timeout(1s) → publish to config.mqtt.topic
    │   │   ├─ MQTT RX: client.poll(100ms) → parse command → send via mqtt_cmd_tx
    │   │   └─ Periodic: every N seconds publish system info
    │   │
    │   └─ On config change (via config_notify_rx): return and reconnect
    │
    ├─ Offline buffer: on connection loss
    │   ├─ Queue messages in OfflineBuffer (RAM → /tmp/ugate_buffer on overflow)
    │   ├─ On reconnect: pop buffer first (FIFO), then new messages
    │   └─ HEX encoding for binary data safety
    │
    └─ QoS handling: 0 (fire-forget), 1 (at least once), 2 (exactly once)
```

**Configuration:**

```ini
config mqtt 'main'
    option enabled '1'
    option broker 'mqtt.example.com'
    option port '1883'
    option tls '0'
    option client_id 'ugate-123'
    option username 'user'
    option password 'pass'
    option topic 'device/sensor/data'
    option sub_topic 'device/sensor/cmd'
    option qos '1'
```

### 6. HTTP Publisher (channels/http_pub.rs)

**Responsibility:** POST UART frames to HTTP endpoint, parse response as commands

**Architecture (async + spawn_blocking):**

```
tokio::spawn(http_pub::run)
    │
    ├─ Create ureq::Agent (timeout=10s)
    │
    ├─ Main loop (tokio::select!):
    │   ├─ config_watch.changed() → reload URL/method
    │   │
    │   └─ data_rx.recv() → {
    │       ├─ Format: hex or JSON {"data":"aabbcc","len":3}
    │       ├─ spawn_blocking(ureq POST/GET)
    │       │   ├─ POST to config.http.url
    │       │   ├─ Read response body (max 10KB to avoid OOM)
    │       │   └─ Parse response: JSON command or raw UART TX data
    │       └─ Send response command via cmd_tx
    │   }
    │
    └─ Offline buffer: not implemented (HTTP 200 = success)
```

**Configuration:**

```ini
config http 'main'
    option enabled '1'
    option url 'https://api.example.com/sensor/data'
    option method 'post'              # post or get
```

### 7. TCP Channels (channels/tcp.rs)

**Responsibility:** Bi-directional TCP server/client for Modbus and custom protocols

**Architecture (separate async tasks):**

```
TCP Server: tokio::spawn(tcp::run_server)
    │
    ├─ Bind 0.0.0.0:config.tcp.server_port
    ├─ Accept connections in async loop
    ├─ Per-connection: AsyncFd for epoll (non-blocking)
    │   ├─ On RX: parse frame (binary, JSON, or Modbus RTU)
    │   ├─ Parse as Command (if recognized)
    │   ├─ Send via cmd_tx → dispatcher
    │   │
    │   ├─ On broadcast_rx: send UART data to client
    │   ├─ Buffer frames (OfflineBuffer on client slow)
    │   └─ Handle disconnect gracefully
    │
    └─ Track connection count (for connection pooling, max=32)

TCP Client: tokio::spawn(tcp::run_client)
    │
    ├─ Connect to config.tcp.client_host:client_port
    ├─ Exponential backoff on connection failure (2s, 4s, 8s, max 60s)
    │
    ├─ Main loop:
    │   ├─ On RX: parse frame → parse command → send via cmd_tx
    │   ├─ On broadcast_rx: send UART data upstream
    │   │
    │   └─ On config change: reconnect
    │
    └─ Offline buffer: queue messages during disconnect
```

**Configuration:**

```ini
config tcp 'main'
    option enabled '1'
    option mode 'both'                # server, client, both
    option server_port '502'          # Modbus TCP default
    option client_host 'gateway.local'
    option client_port '502'
```

### 8. Hybrid Async/Sync Task Architecture

**Channel Architecture (Actual Implementation):**
- **UART → MQTT:** `std::sync::mpsc::channel<String>` (cross-thread compatible, required for std::thread)
- **UART → HTTP:** `tokio::sync::mpsc::channel<String>` (async, capacity 64)
- **Config notifications:** `tokio::sync::watch<()>` (notify-only, no data payload)
  - UART reader and HTTP publisher use `config_rx.changed()` in tokio::select!
  - MQTT publisher polls config every 2s (cannot use async watch in std::thread)
- **AsyncFd epoll:** Efficient I/O multiplexing for single-thread executor

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

## Concurrency Model (v2.0 Hybrid Async/Sync)

**Task Architecture (v2.0 refactor):**

| Task | Spawn Method | Shared State | Channel Type | Why |
|------|--------------|--------------|--------------|-----|
| HTTP Server | `spawn_blocking` | Arc<AppState> | - | tiny-http is blocking |
| UART Reader | `tokio::spawn` | Arc<AppState> | std::mpsc + tokio::mpsc senders | AsyncFd for epoll |
| MQTT Publisher | `std::thread::spawn` | Arc<AppState> | std::sync::mpsc receiver | rumqttc sync Client on MIPS |
| HTTP Publisher | `tokio::spawn` | Arc<AppState> | tokio::sync::mpsc receiver | async with spawn_blocking for ureq |
| OLED Display | `tokio::spawn` | - | - | async display loop |

**Config Change Notification:**

| Component | Mechanism | Reason |
|-----------|-----------|--------|
| UART Reader | `tokio::sync::watch<()>.changed()` | Async, reconnect on UART settings change |
| HTTP Publisher | `tokio::sync::watch<()>.changed()` | Async, reload on config change |
| MQTT Publisher | Polling `state.get()` every 2s | std::thread cannot use async watch |

**AppState Implementation:**
```rust
pub struct AppState {
    config: RwLock<Config>,           // Thread-safe config storage
    config_tx: watch::Sender<()>,     // Notify-only (no data payload)
}
```

**Hybrid Architecture Benefits:**
- Single-thread Tokio executor (epoll) reduces context switching
- std::thread for MQTT avoids rumqttc async issues on MIPS
- RwLock allows concurrent reads, exclusive writes
- watch<()> is lightweight (no data cloning)
- std::sync::mpsc for cross-thread communication is simple and reliable

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
