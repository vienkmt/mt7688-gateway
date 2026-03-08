# Project Overview & Product Development Requirements (PDR)

**Project:** ugate - MT7688 IoT Gateway Firmware
**Version:** 1.6.0 (Phases 1-9 Complete)
**Last Updated:** 2026-03-08
**Status:** Production Ready — All phases complete

---

## Executive Summary

**ugate** is a high-performance IoT Gateway firmware written in Rust for the MT7688 (MIPS 580MHz, 64MB RAM) running OpenWrt. Phases 1-9 of development are complete, providing:

- **Real-time data acquisition** via UART with multiple frame formats (line-based, fixed-length, timeout-based)
- **Multi-channel fan-out:** MQTT (sync), HTTP POST (async), TCP (server/client modes)
- **Command merge:** Bi-directional control via WebSocket, TCP, and MQTT subscription
- **GPIO control:** 32+ GPIO lines with configurable modes and timing
- **Web management UI:** Vanilla JavaScript SPA with 6 tabs (Status, Communication, UART, Network, Routing, System)
- **WiFi Management:** 4 modes (STA/AP/STA+AP/Off) with scan, status, and mode switching
- **Network Configuration:** LAN/WAN, ETH/WiFi interfaces, NTP sync, static routing
- **System Maintenance:** Backup/restore, factory reset, firmware upgrade (local IPK + remote URL)
- **Session Authentication:** Token-based (max 4 sessions, 24h TTL, rate-limited)
- **Offline buffering:** RAM→Disk overflow with disk→RAM priority on reconnect
- **Flexible configuration:** UCI-based (/etc/config/ugate) with hot-reload and draft/apply pattern
- **Embedded HTML:** Zero external dependencies for UI (870-line SPA embedded in binary)

**Target Users:**
- IoT manufacturers integrating MT7688 with Modbus/proprietary MCU protocols
- Industrial gateways requiring local processing and cloud sync
- Edge nodes in IoT deployments with limited bandwidth

**Hardware:** MediaTek MT7628DAN (LinkIt Smart 7688)
**OS:** OpenWrt 24.10 (Kernel 6.6.x)
**Architecture:** MIPS 32-bit (mipsel-unknown-linux-musl)
**Language:** Rust with Tokio async runtime

---

## Core Features (Phases 1-9 Complete)

### Phase 1: Core Infrastructure
- **UART Reader (AsyncFd):** Non-blocking I/O with epoll, supports line/fixed/timeout frame modes
- **Configuration System:** UCI-based (/etc/config/ugate), hot-reload via watch channel
- **App State:** RwLock-based thread-safe storage with tokio::sync::watch notifications
- **Time Sync:** HTTP-based NTP before TLS operations

### Phase 2a: MQTT & HTTP Channels
- **MQTT Publisher:** std::thread + rumqttc sync Client (avoids async issues on MIPS)
  - Supports auth, TLS (rustls), QoS configurable
  - Subscribes to command topic for bi-directional control
  - Channel: std::sync::mpsc (cross-thread compatible)
- **HTTP POST Publisher:** async task with ureq via spawn_blocking
  - GET/POST methods configurable
  - Response body parsed as JSON commands or raw UART TX
  - Channel: tokio::sync::mpsc (capacity 64)
- **Offline Buffer:** RAM queue → disk overflow (/tmp/ugate_buffer/) on reconnect

### Phase 2b: TCP + Reliability
- **TCP Server:** Accept Modbus/binary connections, broadcast UART data to all clients
- **TCP Client:** Connect to upstream gateway/server, send UART data, receive commands
- **Reconnect Logic:** Exponential backoff, configurable retry intervals
- **Both Mode:** Server + Client simultaneous operation

### Phase 3: Web Server & WebSocket
- **HTTP Server:** tiny-http in spawn_blocking, port 8888
- **WebSocket:** Real-time UART logs, system stats, status streaming via tungstenite
- **Embedded UI:** Vanilla JavaScript SPA embedded in binary (include_str!)
- **REST API:** /api/* endpoints for config, status, login, network, WiFi

### Phase 4: GPIO Control
- **chardev ioctl:** Pure Rust GPIO control (no DTS required)
- **32+ GPIO Lines:** Configure direction (in/out), pull-up/down, edge triggers
- **Command Dispatch:** GPIO set/toggle/pulse via Command enum
- **Status Tracking:** SharedStats tracks GPIO state and publish counts

### Phase 5: Vanilla JavaScript Frontend
- **Single-Page App:** Vanilla JavaScript (no framework, no npm)
- **Pages:** Status, Communication, UART, Network, Routing, System, Help
- **Real-time Updates:** WebSocket for live stats, UART logs
- **Session Auth:** Token-based (32 hex chars, max 4 concurrent sessions, 24h TTL)

### Phase 6: Integration & Testing
- **Cross-platform Testing:** Deployed and tested on MT7688 (OpenWrt 24.10)
- **Syslog Integration:** Logs to /dev/log for `logread` access
- **Status API:** Real-time stats (uptime, CPU%, RAM%, channel states)
- **Config API:** Full CRUD for all settings
- **UI Auth:** Password protection with session management

### Phase 7: WiFi + Network + System Management (COMPLETE)
- **WiFi Management:** 4 modes (STA/AP/STA+AP/Off), scan, dynamic mode switching
- **Network Configuration:** ETH WAN (DHCP/static), WiFi WAN, LAN, metric priority
- **NTP Synchronization:** Server list, timezone support, manual sync trigger
- **Static Routing:** Create/delete static routes, dynamic route display
- **System Maintenance:** Backup/restore UCI config, factory reset, restart
- **Firmware Upgrade:** Local IPK upload, remote URL with SHA256 verification
- **Draft/Apply Pattern:** WiFi and network changes saved to RAM, apply commits flash
- **Embedded SPA:** Vanilla JS with 6 tabs, no external CDN dependencies

---

## Architecture & Components

| Component | File | Purpose | LOC | Complexity |
|-----------|------|---------|-----|-----------|
| Main Entry | main.rs | Tokio init, task spawning | 270 | Medium |
| Config Manager | config.rs | UCI config + hot-reload | 486 | High |
| UART Reader | uart/reader.rs | AsyncFd epoll, frame detection | 233 | Medium |
| UART Writer | uart/writer.rs | UART TX queue | 73 | Low |
| Web Server | web/server.rs | tiny-http routing, handlers | 588 | High |
| WiFi Manager | web/wifi.rs | 4-mode WiFi control, scanning | 209 | Medium |
| Network Config | web/netcfg.rs | LAN/WAN/NTP/routes (draft/apply) | 350 | High |
| Auth Manager | web/auth.rs | Session tokens, rate limiting | 141 | Medium |
| Maintenance | web/maintenance.rs | Backup/restore/upgrade | 362 | Medium |
| Status Collector | web/status.rs | System stats, channel monitoring | 210 | Low |
| Syslog Viewer | web/syslog.rs | OpenWrt logging integration | 165 | Low |
| Toolbox API | web/toolbox.rs | Ping, traceroute, DNS lookup | 135 | Low |
| WebSocket | web/ws.rs | tungstenite live streaming | 121 | Medium |
| Web Helpers | web/mod.rs | json_resp, jval, json_escape | 75 | Low |
| MQTT Publisher | channels/mqtt.rs | std::thread + rumqttc sync | 202 | Medium |
| HTTP Publisher | channels/http_pub.rs | spawn_blocking + ureq | 139 | Medium |
| TCP Channel | channels/tcp.rs | Server/client with reconnect | 195 | High |
| Offline Buffer | channels/buffer.rs | RAM + disk overflow | 222 | High |
| Reconnect Logic | channels/reconnect.rs | Exponential backoff | 66 | Low |
| UCI Wrapper | uci.rs | OpenWrt config CLI wrapper | 146 | Low |
| GPIO Control | gpio.rs | chardev ioctl, LED heartbeat | 171 | Medium |
| Time Sync | time_sync.rs | HTTP-based NTP | 83 | Low |
| Commands | commands.rs | Command enum, parsing | 139 | Low |
| Embedded SPA | embedded_index.html | Vanilla JS UI | 925 | Medium |
| Asset Pipeline | assets/ | CSS, JS modules, preview | 174 | Low |
| Modal System | modals/ | Help dialogs, loader | 56 | Low |

---

## Functional Requirements

### FR-1: System Monitoring
**Requirement:** Display real-time system statistics on dashboard
- **Details:**
  - Read uptime from /proc/uptime
  - Parse CPU usage from /proc/stat
  - Parse memory usage from /proc/meminfo
  - Display network interface stats via ifconfig
- **Success Criteria:**
  - Dashboard loads in <200ms
  - Stats update on each page load
  - No resource leaks after 7-day runtime

### FR-2: Configuration Management
**Requirement:** Allow users to configure data publishers and UART
- **Details:**
  - Accept MQTT broker URL, port, TLS flag, topic, client ID
  - Accept HTTP endpoint URL
  - Accept UART port and baudrate
  - Accept collection interval (seconds)
- **Persistence:** Store in application memory (survives restarts)
- **Success Criteria:**
  - Config form validates inputs
  - Changes apply immediately to publishers
  - UART reconnects with new baudrate

### FR-3: Network Configuration
**Requirement:** Allow users to configure WAN interface (eth0.2)
- **Details:**
  - Support DHCP mode (automatic IP assignment)
  - Support Static IP mode (manual IP, netmask, gateway, DNS)
  - Apply settings via OpenWrt UCI (/etc/config/network)
  - Validate settings before application
  - Restart interface without full network reboot
- **Persistence:** In /etc/config/network (survives device reboot)
- **Success Criteria:**
  - Static IP configuration applies and persists
  - DHCP mode returns to automatic IP
  - Interface status reflects actual configuration
  - All validation errors clearly displayed
  - No invalid configurations applied

### FR-4: UART Data Acquisition
**Requirement:** Read serial data from external devices and publish
- **Details:**
  - Open UART device at configured port (/dev/ttyS0, /dev/ttyS1)
  - Read at configured baudrate
  - Handle line-by-line data (newline-delimited)
  - Send to both MQTT and HTTP publishers
- **Success Criteria:**
  - Data arrives within 1 second of UART reception
  - No data loss with 100 msg/sec throughput
  - Baudrate changes apply after config update

### FR-5: MQTT Publishing
**Requirement:** Publish UART data to MQTT broker
- **Details:**
  - Connect to configurable broker (host:port)
  - Support TLS connections
  - Publish to configurable topic
  - Use configurable client ID
  - Reconnect with backoff on disconnection
- **Success Criteria:**
  - Connects within 5 seconds
  - Publishes each message atomically
  - Handles broker disconnection gracefully
  - Recovers automatically

### FR-6: HTTP Publishing
**Requirement:** POST UART data to HTTP endpoint
- **Details:**
  - POST JSON payload to configurable URL
  - Include timestamp and data
  - Retry on network failures
  - Respect HTTP response codes
- **Success Criteria:**
  - Successful POSTs acknowledged
  - Failed requests retried with backoff
  - No data loss due to temporary network issues

### FR-7: Web User Interface
**Requirement:** Provide responsive HTML-based management interface
- **Details:**
  - Dashboard at /
  - Configuration form at /config
  - Network form at /network
  - All forms with inline CSS (no external assets)
  - Mobile-responsive design
- **Success Criteria:**
  - Pages load in <500ms
  - Forms validate client-side
  - Navigation intuitive
  - No JavaScript errors

---

## Non-Functional Requirements

### NFR-1: Performance
- **HTTP Request Latency:** <100ms for local network requests
- **System Info Collection:** <10ms per request
- **Network Config Apply:** <5 seconds end-to-end
- **Data Throughput:** Support 100+ UART messages per second

### NFR-2: Resource Constraints
- **Binary Size:** <500KB (with release optimization + strip)
- **RAM Usage:** <50MB at runtime (comfortable on 256MB device)
- **Startup Time:** <2 seconds (time sync + thread spawning)
- **Disk Space:** <2MB (binary + logs)

### NFR-3: Reliability
- **Uptime:** No crashes on 30-day continuous operation
- **Memory Leaks:** None detected after 7-day soak test
- **Publisher Reconnection:** Automatic with exponential backoff
- **Data Delivery:** No loss under normal network conditions

### NFR-4: Security
- **Authentication:** None (LAN-only, trusted network assumed)
- **Encryption:** TLS support for MQTT and HTTP
- **Input Validation:** All user inputs sanitized (IP, domain, port)
- **XSS Prevention:** HTML escaping in all templates
- **Command Injection:** Safe UCI quoting

### NFR-5: Maintainability
- **Code Size:** Modular, each module <250 lines
- **Documentation:** Architecture and API docs complete
- **Testing:** Unit tests for validation functions
- **Logging:** Via syslog or /var/log/gateway.log

### NFR-6: Compatibility
- **Platforms:** OpenWrt 21.02+ on MT7688AN
- **Dependencies:** Minimal (tiny-http, paho-mqtt, serialport)
- **Cross-Compilation:** Via cross-rs for MIPS target
- **Backwards Compatibility:** Configuration format stable

---

## Technical Constraints

| Constraint | Value | Mitigation |
|-----------|-------|-----------|
| CPU | 580MHz single-core | No heavy computation, bounded channels |
| RAM | 256MB total, ~100MB app space | Bounded channels (128 msgs), efficient allocators |
| Flash | 25MB available | Release build <500KB, musl static linking |
| Architecture | MIPS 32-bit | cross-rs, no AtomicU64 |
| Network | 2.4GHz Wi-Fi 150Mbps | Reasonable for IoT (low bandwidth) |
| Interfaces | 1x Eth, 1x USB, 2x UART | Use UART for external devices |

---

## Development Phases

### Phase 1: Core System (COMPLETE - Jan 2026)
- HTTP server with basic routes (tiny-http)
- System info collection from /proc
- Configuration management (MQTT/HTTP/UART)
- UART reader with AsyncFd (epoll-based)
- MQTT and HTTP publishers (std thread + tokio async)

### Phase 2: Web UI (COMPLETE - Jan 2026)
- Dashboard (system stats, uptime, CPU, RAM)
- Configuration form (MQTT/HTTP/TCP/UART)
- Responsive HTML templates

### Phase 3: Network Management (COMPLETE - Feb 2026)
- Network configuration (LAN/WAN IP, DHCP/static)
- UCI wrapper (uci.rs) for /etc/config/network
- IP/netmask/gateway validation
- Interface restart without full reboot

### Phase 4-6: Advanced Features (COMPLETE - Feb 2026)
- TCP server + client with bi-directional data
- GPIO control via chardev ioctl
- WebSocket real-time logs and stats
- Offline buffering (RAM + disk)
- Session authentication (token-based, 24h TTL)

### Phase 7: WiFi + Network + System (COMPLETE - Mar 2026)
- **WiFi:** STA/AP/STA+AP/Off modes, scan, status, mode switching
- **Network:** Dynamic WAN discovery, metric priority, NTP servers, static routes
- **System:** Backup/restore, factory reset, version/build info, firmware upgrade
- **Draft/Apply:** UCI changes staged in RAM, apply commits flash + restarts services
- **Frontend:** 6 tabs (Status, Communication, UART, Network, Routing, System)

---

## API Specification

### REST Endpoints

#### GET / - System Dashboard
```
Response: 200 OK
Content-Type: text/html; charset=utf-8
Body: HTML page with system stats
```

#### GET /config - Configuration Form
```
Response: 200 OK
Content-Type: text/html; charset=utf-8
Body: HTML form for MQTT/HTTP/UART settings
```

#### POST /config - Update Configuration
```
Content-Type: application/x-www-form-urlencoded
Body: mqtt_enabled=on&mqtt_broker=broker.example.com&...

Response: 200 OK
Content-Type: text/html; charset=utf-8
Body: Form with success message or errors
```

#### GET /network - Network Configuration Form
```
Response: 200 OK
Content-Type: text/html; charset=utf-8
Body: HTML form for WAN settings + live status
```

#### POST /network - Update Network Configuration
```
Content-Type: application/x-www-form-urlencoded
Body: mode=static&ipaddr=192.168.1.100&netmask=255.255.255.0&gateway=192.168.1.1&dns_primary=8.8.8.8&dns_secondary=8.8.4.4

Response: 200 OK
Content-Type: text/html; charset=utf-8
Body: Form with success message or validation errors
```

#### GET /api/network - Get Network Configuration (JSON)
```
Response: 200 OK
Content-Type: application/json
Body: {
  "config": {
    "mode": "static",
    "ipaddr": "192.168.1.100",
    "netmask": "255.255.255.0",
    "gateway": "192.168.1.1",
    "dns_primary": "8.8.8.8",
    "dns_secondary": "8.8.4.4"
  },
  "status": {
    "ip": "192.168.1.100",
    "netmask": "255.255.255.0",
    "gateway": "192.168.1.1",
    "dns": ["8.8.8.8", "8.8.4.4"],
    "is_up": true
  },
  "saved": false,
  "errors": []
}
```

#### POST /api/network - Update Network Configuration (JSON)
```
Content-Type: application/json
Body: {
  "mode": "static",
  "ipaddr": "192.168.1.100",
  "netmask": "255.255.255.0",
  "gateway": "192.168.1.1",
  "dns_primary": "8.8.8.8",
  "dns_secondary": "8.8.4.4"
}

Response: 200 OK
Content-Type: application/json
Body: (same as GET /api/network)
```

---

## Configuration Storage

### Runtime Config (In-Memory)
- **Location:** AppState (Arc<Mutex<Config>>)
- **Lifetime:** Application runtime only
- **Survives:** Reload of web pages
- **Lost on:** Process restart

### Network Config (Persistent)
- **Location:** /etc/config/network (OpenWrt UCI)
- **Format:** UCI configuration file
- **Survives:** Device reboot
- **Loss:** Only if device factory reset

---

## Success Metrics

### Phase 3 (Network Configuration)
- [ ] Network config form loads without errors
- [ ] Static IP configuration applies and persists
- [ ] DHCP mode returns to automatic assignment
- [ ] Invalid IPs rejected with clear error messages
- [ ] Gateway subnet validation working
- [ ] LAN conflict (10.10.10.0/24) detected and rejected
- [ ] DNS format validation enforced
- [ ] Interface status reflects actual configuration
- [ ] /api/network JSON API fully functional
- [ ] Binary size remains <500KB
- [ ] No new memory leaks introduced
- [ ] Documentation updated and complete

---

## Known Issues & Limitations

### Current Limitations
1. **Single WAN Interface:** Only eth0.2 supported (no multi-WAN failover)
2. **No Auth:** Web UI assumes trusted LAN-only access
3. **Manual Time Sync:** Relies on OpenWrt NTP (not customizable)
4. **Limited Logging:** Basic syslog only (no structured logging)
5. **UART Limitations:** Line-based protocol only (no binary data)

### Future Considerations
1. **VLAN Support:** Tagged interfaces for network segmentation
2. **Firewall Integration:** Configure iptables from web UI
3. **Advanced Routing:** Static routes, policy-based routing
4. **Performance Tuning:** Memory usage optimization, faster startup
5. **Kubernetes Ready:** Support as edge node in K8s clusters

---

## Testing Requirements

### Unit Tests
- [ ] is_valid_ipv4() with valid/invalid IPs
- [ ] is_valid_netmask() with valid/invalid masks
- [ ] gateway_in_subnet() with various scenarios
- [ ] conflicts_with_lan() for LAN overlap detection
- [ ] parse_network_form() with URL-encoded input
- [ ] parse_network_json() with JSON input

### Integration Tests
- [ ] Network config form submission
- [ ] API network endpoint (GET/POST)
- [ ] UCI commands on device
- [ ] Interface restart without full reboot
- [ ] Persistent config survives device reboot

### Device Tests
- [ ] Deploy to MT7688AN
- [ ] Web UI loads on device
- [ ] Network config changes apply
- [ ] Status reflects actual configuration
- [ ] No resource leaks (7-day soak test)

---

## Deployment

### Build
```bash
cross +nightly build --target mipsel-unknown-linux-musl --release
```

### Deploy
```bash
scp target/mipsel-unknown-linux-musl/release/gateway root@10.10.10.1:/tmp/
ssh root@10.10.10.1 'chmod +x /tmp/gateway && nohup /tmp/gateway > /var/log/gateway.log 2>&1 &'
```

### Verify
```bash
ssh root@10.10.10.1 'curl http://localhost:8888/'
```

---

## Maintenance & Support

### Monitoring
- Check `/var/log/gateway.log` for errors
- Monitor process via `ps aux | grep gateway`
- Test publishers via `mosquitto_sub` (MQTT) or `curl` (HTTP)

### Updates
- Code updates: Recompile, redeploy
- Configuration updates: Modify form, apply via web UI
- Firmware updates: Planned for Phase 4

### Troubleshooting
- Network not responding: Check 10.10.10.1:8888 connectivity
- Static IP not applying: Verify form input, check /etc/config/network
- Data not publishing: Check MQTT/HTTP endpoint reachability, review logs

---

## Document History

| Version | Date | Changes |
|---------|------|---------|
| 3.0 | 2026-03-08 | Phase 7 complete: WiFi management (4 modes), network config (WAN/NTP/routing), system maintenance (backup/upgrade), draft/apply pattern |
| 2.0 | 2026-02-26 | Phase 1-6 complete: Core system, UART/MQTT/HTTP/TCP channels, GPIO, WebSocket, session auth |
| 1.0 | 2026-02-12 | Initial release with Phase 3 (Network Config) complete |

---

## References

- **CLAUDE.md** - Hardware and development constraints
- **docs/codebase-summary.md** - Code structure and module overview
- **docs/system-architecture.md** - Detailed architecture and data flows
- **docs/mips-build-guide.md** - Build and deployment instructions
- **AGENTS.md** - Development team and responsibilities
