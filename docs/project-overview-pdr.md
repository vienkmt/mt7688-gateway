# Project Overview & Product Development Requirements (PDR)

**Project:** MT7688AN IoT Gateway Firmware
**Version:** 1.0
**Last Updated:** 2026-02-12
**Status:** Active Development

---

## Executive Summary

The MT7688AN IoT Gateway is an embedded Rust application that transforms a MediaTek LinkIt Smart 7688 (MIPS 24KEc, 580MHz, 256MB RAM) into a capable IoT data aggregator and edge processing node. The firmware:

- Collects data from external devices via UART (e.g., Quectel 4G modem)
- Publishes to cloud platforms via MQTT and HTTP
- Provides a web-based management interface for configuration and monitoring
- Manages network connectivity (DHCP and Static IP modes)
- Operates reliably on highly resource-constrained hardware

**Target Users:**
- IoT device manufacturers integrating MT7688AN into products
- Edge computing deployments requiring local data aggregation
- IoT applications needing reliable firmware on cost-effective hardware

**Hardware:** MediaTek LinkIt Smart 7688 (MT7688AN)
**OS:** OpenWrt 21.02
**Architecture:** MIPS 32-bit (mipsel-unknown-linux-musl)
**Language:** Rust (for memory safety and performance)

---

## Core Features (Implemented)

### 1. System Monitoring Dashboard
- **Endpoint:** GET /
- **Displays:**
  - System uptime
  - CPU usage percentage
  - Memory (RAM) usage
  - Network interface statistics
- **Refresh:** On-demand (no polling required)

### 2. Configuration Management
- **Endpoint:** GET/POST /config
- **Manages:**
  - MQTT broker settings (URL, port, TLS, topic, client ID)
  - HTTP publisher endpoint
  - UART serial port configuration (port, baudrate)
  - Data collection interval (seconds)
- **Persistence:** In-memory (survives runtime restart)
- **UI:** HTML form with real-time validation

### 3. Network Configuration (NEW - Feb 2026)
- **Endpoints:**
  - HTML UI: GET/POST /network
  - JSON API: GET/POST /api/network
- **Manages:**
  - WAN interface (eth0.2) DHCP vs Static IP
  - Static IP, subnet mask, gateway
  - DNS servers (primary and secondary)
- **Persistence:** OpenWrt UCI (/etc/config/network)
- **Validation:**
  - IP address format validation
  - Subnet mask validity (contiguous bits)
  - Gateway in same subnet check
  - LAN conflict prevention (10.10.10.0/24 reserved)
  - DNS format validation
- **Application:** `ifdown wan` / `ifup wan` to activate
- **Status Monitoring:** Live IP, netmask, gateway, DNS via system commands

### 4. Data Acquisition & Publishing
- **UART Reader:** Reads serial data from /dev/ttyS1 (Quectel modem or any device)
- **MQTT Publisher:** Publishes to configurable broker (with TLS support)
- **HTTP Publisher:** POSTs to configurable endpoint
- **Threading:** Bounded channels (128 messages) prevent OOM on 64MB device
- **Error Handling:** Automatic reconnect with exponential backoff

### 5. Time Synchronization
- **Startup Routine:** Sync system clock before TLS operations
- **Purpose:** Prevent certificate validation failures due to incorrect time
- **Method:** NTP synchronization to OpenWrt time source

---

## Architecture & Components

| Component | File | Purpose | Complexity |
|-----------|------|---------|-----------|
| HTTP Server | main.rs | Request routing, response handling | Medium |
| Network Config | network_config.rs | UCI integration, IP validation | High |
| Network UI | html_network.rs | Form rendering, status display | Medium |
| UCI Wrapper | uci.rs | CLI command wrapper | Low |
| System Info | system_info.rs | Stats collection from /proc | Low |
| Config Manager | config.rs | Thread-safe config storage | Medium |
| UART Reader | uart_reader.rs | Serial data acquisition | Medium |
| MQTT Publisher | mqtt_publisher.rs | MQTT client with reconnect | Medium |
| HTTP Publisher | http_publisher.rs | HTTP POST with retry | Medium |
| Time Sync | time_sync.rs | NTP synchronization | Low |
| HTML Templates | html_*.rs | Web UI rendering (3 files) | Low |

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

### Phase 1: Core System (COMPLETE)
- HTTP server with basic routes
- System info collection
- Configuration management (MQTT/HTTP/UART)
- UART reader with background thread
- MQTT and HTTP publishers

### Phase 2: Web UI (COMPLETE)
- Dashboard (system stats)
- Configuration form
- Responsive HTML templates

### Phase 3: Network Management (COMPLETE - Feb 2026)
- Network configuration module (network_config.rs)
- Network UI page (html_network.rs)
- UCI wrapper (uci.rs)
- IP/netmask validation functions
- LAN conflict detection
- Gateway subnet checking
- /network and /api/network endpoints

### Phase 4: Advanced Features (PLANNED)
- Multi-interface support (LAN + WAN)
- Firewall rule configuration
- Static routes
- VLAN tagging
- Performance optimization
- OTA firmware updates

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
| 1.0 | 2026-02-12 | Initial release with Phase 3 (Network Config) complete |

---

## References

- **CLAUDE.md** - Hardware and development constraints
- **docs/codebase-summary.md** - Code structure and module overview
- **docs/system-architecture.md** - Detailed architecture and data flows
- **docs/mips-build-guide.md** - Build and deployment instructions
- **AGENTS.md** - Development team and responsibilities
