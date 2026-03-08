# System Architecture - ugate IoT Gateway

**Last Updated:** 2026-03-08
**Version:** 1.6.0 (Phases 1-9 Complete)

## Architecture Overview

**ugate** is a hybrid async/sync IoT Gateway for MT7688 that collects binary/text data via UART and fan-outs to MQTT, HTTP, and TCP channels while accepting commands from multiple sources (WebSocket, TCP, MQTT) to drive GPIO and UART TX. The design prioritizes resource efficiency on 64MB RAM using Tokio single-thread async executor with epoll I/O multiplexing.

### High-Level Components (Phase 1-7: WiFi + Network + System Management)

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

### 1. HTTP Server & WebSocket (web_api/server.rs + web_api/ws.rs + embedded HTML)

**Purpose:** REST API (WiFi/Network/System), WebSocket (real-time updates), embedded Vue 3 SPA

**Key Endpoints (9 pages in Vue 3 SPA):**

| Page | Endpoints | Purpose |
|------|-----------|---------|
| **Status** | `/api/status`, `/api/wifi/status` | System info, WiFi signal, channel stats, GPIO |
| **Communication** | `/api/config` (MQTT/HTTP/TCP) | Config MQTT/HTTP/TCP settings |
| **UART** | `/api/config` (UART section) | UART baudrate, frame mode, real-time stream (WS) |
| **Network** | `/api/network`, `/api/ntp`, `/api/wan/discover` | LAN/WAN IP config, NTP servers, timezone |
| **Routing** | `/api/routes` | Show/add/delete static routes |
| **Toolbox** | `/api/toolbox` | System diagnostics (ping, nslookup, etc.) |
| **System** | `/api/version`, `/api/backup`, `/api/upgrade` | Version, backup/restore, firmware upgrade |

**Auth Endpoints:**

| Endpoint | Method | Purpose |
|----------|--------|---------|
| `/api/login` | POST | Get session token (cookie) |
| `/api/password` | POST | Change password |

**Server Details:**
- Runtime: Tokio single-thread executor (`#[tokio::main(flavor = "current_thread")]`)
- HTTP Server: `spawn_blocking(tiny-http::Server::http)`
- WebSocket: tungstenite in async task, broadcasts UART data & system stats
- Port: 8888 (configurable via UCI: config.web.port)
- Frontend: Embedded Vue 3 SPA (built from 10 modular JS files + HTML)
  - Loaded at build time via `build.rs` concatenation into `html-bundle/embedded_index.html`
  - Vue 3 served via CDN bundle (00-vue.min.js)
  - Modular pages: Status, Communication, UART, Network, Routing, Toolbox, System
  - Modal dialogs for help, data format reference, system info
  - No npm build needed, templates written as JS/Vue syntax
- Auth: Session tokens (32 hex chars, max 4 concurrent, 24h TTL), rate limiting

**Request Handler Flow (Summary):**

| Method | Endpoint | Handler | Purpose |
|--------|----------|---------|---------|
| GET | / | - | Embedded Vue 3 SPA (built from 10 JS modules) |
| GET | /style.css | - | External CSS stylesheet (asset) |
| POST | /api/login | SessionManager | Authenticate with password, get token |
| GET | /api/session | SessionManager | Check session validity |
| GET/POST | /api/wifi/* | wifi module | WiFi modes, scanning, status |
| GET/POST | /api/network* | netcfg module | LAN/WAN IP, apply/revert |
| GET/POST | /api/ntp* | netcfg module | NTP servers, timezone, sync |
| GET/POST | /api/routes* | netcfg module | Static routing |
| GET/POST | /api/version | maintenance module | Version/build info |
| GET/POST | /api/backup | maintenance module | Config backup/restore |
| POST | /api/upgrade* | maintenance module | Local/remote firmware upgrade |
| GET | /api/status | status module | Real-time stats |
| GET | /ws | ws module | WebSocket upgrade |

**WebSocket (tungstenite):**
- Upgrades from tiny-http 101 Upgrade
- Broadcasts UART frames (capacity 64)
- Sends system stats every 1s
- Supports command input (JSON) from client
- Max 32 concurrent connections (configurable)

### 1.5 WiFi Management (web_api/wifi.rs - 209 lines)

**Phase 7 Feature: WiFi 4-Mode Configuration**

**Handlers:**

| Endpoint | Function | Purpose |
|----------|----------|---------|
| `GET /api/wifi/scan` | `handle_scan()` | Parse `iwinfo phy0-sta0 scan`, extract ESSID/Signal/Encryption |
| `GET /api/wifi/status` | `handle_status()` | Read UCI disabled flags to detect mode, get STA/AP signal/IP/SSID |
| `POST /api/wifi/mode` | `handle_set_mode(body)` | Switch 4 modes (STA/AP/STA+AP/Off) by setting UCI disabled flags |
| `POST /api/wifi/connect` | `handle_connect(body)` | Legacy: manual SSID+password (now use mode endpoint) |
| `POST /api/wifi/disconnect` | `handle_disconnect()` | Legacy: clear STA SSID (now use mode endpoint) |

**WiFi Mode Mapping (UCI disabled flags):**

| Mode | wwan.disabled | default_radio0.disabled |
|------|---------------|------------------------|
| STA | 0 (enabled) | 1 (disabled) |
| AP | 1 (disabled) | 0 (enabled) |
| STA+AP | 0 (enabled) | 0 (enabled) |
| Off | 1 (disabled) | 1 (disabled) |

**UCI Interfaces:**
- `wireless.wwan` = STA interface (mode=sta, network=wwan)
- `wireless.default_radio0` = AP interface (mode=ap, network=lan)
- `wireless.radio0` = Radio config (channel, country=VN, band=2g)

**Helpers:**
- `set_sta_config(ssid, key, enc)` — Set STA SSID, password, encryption
- `set_ap_config(ssid, key, channel)` — Set AP SSID, password, channel
- `parse_signal(s)` — Convert dBm to signal percentage
- `get_iface_ip(iface)` — Get IP address of interface

**Draft/Apply Pattern:** Changes saved to RAM with `uci set` (no commit), require `/api/network/apply` to persist.

---

### 1.6 Network Configuration (web_api/netcfg.rs - 350 lines)

**Phase 7 Feature: Complete Network Stack Management**

**Handlers:**

| Endpoint | Function | Purpose |
|----------|----------|---------|
| `GET /api/network` | `handle_get_network()` | Get LAN/WAN config (proto, ipaddr, netmask, gateway, metric, dns) |
| `POST /api/network` | `handle_set_network(body)` | Set LAN/WAN config (DHCP or static) — draft to RAM |
| `POST /api/network/apply` | `handle_apply()` | Commit `uci commit network/wireless`, reload interfaces smartly |
| `POST /api/network/revert` | `handle_revert()` | Discard all pending changes (uci revert) |
| `GET /api/network/changes` | `handle_changes()` | Check if pending changes exist in network/wireless/system |
| `GET /api/ntp` | `handle_get_ntp()` | Get NTP servers list, timezone, enabled flag |
| `POST /api/ntp` | `handle_set_ntp(body)` | Set NTP servers + timezone, commit directly (no draft) |
| `POST /api/ntp/sync` | `handle_ntp_sync()` | Manually trigger time sync (ntpd -q or HTTP Date fallback) |
| `GET /api/routes` | `handle_get_routes()` | Parse `ip route show`, format as JSON array (dest/via/dev/metric) |
| `POST /api/routes` | `handle_add_route(body)` | Add static route to UCI, apply with `ip route add` |
| `DELETE /api/routes/{name}` | `handle_delete_route()` | Delete route from UCI |
| `GET /api/wan/discover` | `handle_wan_discover()` | Parse `ip route show default`, list available WAN interfaces + metrics |
| `POST /api/interface/metric` | `handle_set_metric(body)` | Set metric for interface (WAN priority) |

**Network Config Structure:**
- `network.lan` = LAN bridge (static 192.168.10.1/24 default)
- `network.wan` = ETH WAN (DHCP or static, metric 100)
- `network.wwan` = WiFi WAN (DHCP or static, metric 10 — higher priority)

**Smart Apply Mechanism:**
1. `uci changes network` → diff pending changes
2. `uci commit network` → persist to /etc/config/network
3. If wireless changed: `wifi reload` (separate from network)
4. If network changed: `ubus call network reload` (netifd diff, minimal downtime)
5. Delay 1s in separate thread to avoid blocking

**Helpers:**
- `uci_get(key)` — Get single UCI value
- `netmask_to_cidr(mask)` — Convert netmask to CIDR notation
- `dev_to_uci(dev)` — Map device name (eth0.2) to UCI section (wan)
- `json_str_array(list)` — Format string array as JSON ["a","b","c"]

---

### 1.7 System Maintenance (web_api/maintenance.rs - 362 lines)

**Phase 7 Feature: Firmware Management & Config Backup**

**Handlers:**

| Endpoint | Function | Purpose |
|----------|----------|---------|
| `GET /api/version` | `handle_version()` | Return version, build_date, git_commit (compile-time env) |
| `GET /api/backup` | `handle_backup()` | Stream `/etc/config/ugate` as binary download |
| `POST /api/restore` | `handle_restore()` | Upload config file, validate UTF-8 + UCI format, restore |
| `POST /api/factory-reset` | `handle_factory_reset()` | Reset config to defaults, save UCI |
| `POST /api/restart` | `handle_restart()` | Reboot device (1s delay) |
| `POST /api/upgrade` (local) | `handle_upgrade_upload()` | Upload IPK file (max 10MB), validate ar archive, install |
| `GET /api/upgrade/url` | `handle_get_upgrade_url()` | Get remote upgrade URL from UCI |
| `POST /api/upgrade/url` | `handle_set_upgrade_url()` | Save remote upgrade URL to UCI, commit |
| `GET /api/upgrade/check` | `handle_upgrade_check()` | Fetch manifest JSON from remote URL, check version/changelog |
| `POST /api/upgrade/remote` | `handle_upgrade_remote()` | Download IPK from remote, verify SHA256, install, restart |

**Upgrade Flow (Local):**
1. User selects IPK file
2. POST to `/api/upgrade` with file body
3. Validate: check ar archive magic ("!<arch>")
4. `opkg install --force-reinstall /tmp/ugate-*.ipk`
5. Guard: `UPGRADING.compare_exchange()` prevents concurrent upgrade
6. Restart service

**Upgrade Flow (Remote):**
1. GET `/api/upgrade/check` to fetch manifest
2. Parse JSON: `{version, changelog, size, url, sha256}`
3. Compare version vs current
4. POST `/api/upgrade/remote` → spawn_blocking to:
   - Download IPK via ureq
   - Verify SHA256 checksum
   - `opkg install --force-reinstall`
5. Restart service

**Backup/Restore:**
- Backup: Read `/etc/config/ugate` → stream binary
- Restore: Upload file → validate UTF-8 + contains "config " → backup old → write → reload state

---

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

### 2.5 Session Authentication (web/auth.rs - 141 lines)

**Phase 7 Feature: Token-Based Session Management**

**SessionManager:**
- Max 4 concurrent sessions
- Token: 32 hex chars (16 bytes from /dev/urandom)
- TTL: 24 hours (per-process, tokens discarded on restart)
- Rate limit: 2s cooldown between failed login attempts
- Cookie header: `session=<token>`

**Handlers:**
- `validate_password(body)` — Parse JSON {"password":"..."}, check vs config
- Tokens stored in VecDeque with expiry timestamp
- Failed attempt increments cooldown counter

**Security:**
- Tokens are random, not based on timestamp
- No persistent session storage (RAM only)
- Max 4 sessions prevents brute-force bombing
- Rate limiting (2s) further protects against attacks

---

### 2.6 WebSocket Manager (web/ws.rs - 121 lines)

**Phase 7 Feature: Real-Time Status Streaming**

**WsManager:**
- Broadcast channel: tokio::sync (capacity 64)
- Idle timeout: 120s (disconnect inactive clients)
- Max connections: Configurable (atomic counter)

**Handler:**
- Accepts WebSocket upgrade from `/ws`
- Broadcasts UART frames from `uart_broadcast_tx` channel
- Broadcasts system status every 1s
- Accepts JSON command input from client
- Single-thread loop per client (epoll-based)

---

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

### 5. Toolbox & Syslog Modules

**Toolbox (web/toolbox.rs — 135 LOC):**
- System commands: reboot, factory reset, shell commands
- Device diagnostics and debug tools
- Maintenance operations

**Syslog Viewer (web/syslog.rs — 165 LOC):**
- View OpenWrt syslog in real-time
- Log filtering and search
- Integration with system status

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

**Task Architecture (Hybrid Async/Sync):**

| Task | Method | Why |
|------|--------|-----|
| HTTP Server | spawn_blocking | tiny-http is blocking |
| UART Reader | tokio::spawn | AsyncFd/epoll non-blocking |
| MQTT Publisher | std::thread::spawn | rumqttc sync Client (stable on MIPS) |
| HTTP Publisher | tokio::spawn | async with spawn_blocking for ureq |
| TCP Server/Client | tokio::spawn | async I/O |
| GPIO Heartbeat | tokio::spawn | async task |
| WebSocket | tokio::spawn | real-time broadcast |

**Config Changes:**
- UART/HTTP: `tokio::sync::watch<()>.changed()` in tokio::select!
- MQTT: Polls state every 2s (can't use async watch in std::thread)
- AppState: `RwLock<Config>` + `watch::Sender<()>` for notifications

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
