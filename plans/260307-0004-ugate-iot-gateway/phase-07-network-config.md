# Phase 7: Network Configuration

**Priority:** Medium
**Status:** pending
**Effort:** 3 days
**Depends on:** Phase 3 (Web Server)

## Context

API cấu hình mạng qua Web UI: scan WiFi, connect WiFi, config LAN/WAN (DHCP/static IP).
Sử dụng UCI commands của OpenWrt.

## API Endpoints

| Method | Path | Description |
|--------|------|-------------|
| GET | `/api/wifi/scan` | Quét danh sách WiFi |
| GET | `/api/wifi/status` | Trạng thái kết nối WiFi |
| POST | `/api/wifi/connect` | Kết nối WiFi |
| POST | `/api/wifi/disconnect` | Ngắt kết nối WiFi |
| GET | `/api/network` | Lấy config LAN/WAN |
| POST | `/api/network` | Set config LAN/WAN |
| GET | `/api/ntp` | Lấy NTP settings |
| POST | `/api/ntp` | Set NTP server + timezone |
| POST | `/api/ntp/sync` | Trigger sync ngay |
| GET | `/api/routes` | Lấy routing table |
| POST | `/api/routes` | Thêm static route |
| DELETE | `/api/routes/{id}` | Xóa static route |
| POST | `/api/interface/metric` | Set metric cho interface |

## Module Structure

```
ugate/src/web/
├── ...
└── network.rs    # NEW: Network configuration handlers
```

## Implementation

### 1. WiFi Scan

```rust
// GET /api/wifi/scan
pub fn handle_wifi_scan() -> Response {
    let output = Command::new("iwinfo")
        .args(["wlan0", "scan"])
        .output()
        .ok();

    let networks = parse_iwinfo_scan(&output);
    json_response(&networks)
}

fn parse_iwinfo_scan(output: &[u8]) -> Vec<WifiNetwork> {
    // Parse ESSID, Signal, Encryption từ iwinfo output
    // ESSID: "MyWifi"
    // Signal: -45 dBm
    // Encryption: WPA2 PSK
}

#[derive(Serialize)]
struct WifiNetwork {
    ssid: String,
    signal: i32,      // dBm
    encryption: String,
}
```

### 2. WiFi Connect

```rust
// POST /api/wifi/connect
// Body: { "ssid": "MyWifi", "password": "secret", "encryption": "psk2" }
pub fn handle_wifi_connect(body: &str) -> Response {
    let req: WifiConnectRequest = serde_json::from_str(body)?;

    // UCI commands
    let commands = [
        format!("uci set wireless.@wifi-iface[0].ssid='{}'", req.ssid),
        format!("uci set wireless.@wifi-iface[0].key='{}'", req.password),
        format!("uci set wireless.@wifi-iface[0].encryption='{}'", req.encryption),
        "uci commit wireless".to_string(),
    ];

    for cmd in &commands {
        Command::new("sh").args(["-c", cmd]).status()?;
    }

    // Reload WiFi (async, không block response)
    Command::new("wifi").arg("reload").spawn()?;

    json_response(&StatusResponse { success: true })
}
```

### 3. WiFi Status

```rust
// GET /api/wifi/status
pub fn handle_wifi_status() -> Response {
    let output = Command::new("iwinfo")
        .args(["wlan0", "info"])
        .output()
        .ok();

    let status = WifiStatus {
        connected: check_connected(&output),
        ssid: parse_current_ssid(&output),
        signal: parse_signal(&output),
        ip: get_ip_address("wlan0"),
    };

    json_response(&status)
}
```

### 4. Network Config (LAN/WAN)

```rust
// GET /api/network
pub fn handle_get_network() -> Response {
    let lan = NetworkInterface {
        name: "lan".to_string(),
        proto: uci_get("network.lan.proto"),
        ipaddr: uci_get("network.lan.ipaddr"),
        netmask: uci_get("network.lan.netmask"),
        gateway: uci_get("network.lan.gateway"),
        dns: uci_get("network.lan.dns"),
    };

    let wan = NetworkInterface {
        name: "wan".to_string(),
        proto: uci_get("network.wan.proto"),
        ipaddr: uci_get("network.wan.ipaddr"),
        netmask: uci_get("network.wan.netmask"),
        gateway: uci_get("network.wan.gateway"),
        dns: uci_get_list("network.wan.dns"),  // Multiple DNS servers
    };

    // Also get global DNS if set
    let global_dns = uci_get_list("network.@globals[0].dns");

    json_response(&NetworkConfig { lan, wan })
}

// POST /api/network
// Body: { "interface": "lan", "proto": "static", "ipaddr": "192.168.1.100", ... }
pub fn handle_set_network(body: &str) -> Response {
    let req: NetworkSetRequest = serde_json::from_str(body)?;
    let iface = &req.interface; // "lan" or "wan"

    let commands = match req.proto.as_str() {
        "dhcp" => vec![
            format!("uci set network.{}.proto='dhcp'", iface),
            format!("uci delete network.{}.ipaddr 2>/dev/null || true", iface),
            format!("uci delete network.{}.netmask 2>/dev/null || true", iface),
        ],
        "static" => {
            // Clear old DNS
            Command::new("sh")
                .args(["-c", &format!("uci delete network.{}.dns 2>/dev/null || true", iface)])
                .status().ok();

            let mut cmds = vec![
                format!("uci set network.{}.proto='static'", iface),
                format!("uci set network.{}.ipaddr='{}'", iface, req.ipaddr),
                format!("uci set network.{}.netmask='{}'", iface, req.netmask),
                format!("uci set network.{}.gateway='{}'", iface, req.gateway),
            ];

            // Add multiple DNS servers
            for dns in &req.dns {
                cmds.push(format!("uci add_list network.{}.dns='{}'", iface, dns));
            }
            cmds
        }
        _ => return error_response("Invalid proto"),
    };

    for cmd in &commands {
        Command::new("sh").args(["-c", cmd]).status()?;
    }

    Command::new("sh").args(["-c", "uci commit network"]).status()?;

    // Delay restart — response trước, restart sau (tránh kill connection)
    std::thread::spawn(|| {
        std::thread::sleep(std::time::Duration::from_secs(2));
        Command::new("/etc/init.d/network").arg("restart").status().ok();
    });

    json_response(&StatusResponse {
        success: true,
        message: "Network will restart in 2 seconds..."
    })
}

fn uci_get(key: &str) -> String {
    Command::new("uci")
        .args(["get", key])
        .output()
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .unwrap_or_default()
        .trim()
        .to_string()
}
```

### 5. Routing & Metrics (Multi-WAN)

UCI config (`/etc/config/network`):
```
# Interface metrics (failover priority)
config interface 'wan_eth'
    option metric '10'      # Highest priority

config interface 'wan_wifi'
    option metric '20'      # Backup 1

# Static routes
config route 'office_route'
    option interface 'wan_eth'
    option target '10.0.0.0'
    option netmask '255.0.0.0'
    option gateway '192.168.1.1'

config route 'vpn_route'
    option interface 'wan_wifi'
    option target '172.16.0.0'
    option netmask '255.255.0.0'
    option gateway '192.168.2.1'
```

```rust
// GET /api/routes
pub fn handle_get_routes() -> Response {
    // Get from UCI
    let routes = get_uci_routes();

    // Also get current routing table
    let output = Command::new("ip")
        .args(["route", "show"])
        .output()?;
    let current = String::from_utf8_lossy(&output.stdout);

    json_response(&RoutingInfo { routes, current_table: current })
}

// POST /api/routes
// Body: { "name": "office", "target": "10.0.0.0", "netmask": "255.0.0.0",
//         "gateway": "192.168.1.1", "interface": "wan_eth" }
pub fn handle_add_route(body: &str) -> Response {
    let req: RouteRequest = serde_json::from_str(body)?;
    let name = format!("route_{}", req.name);

    let cmds = [
        format!("uci set network.{}=route", name),
        format!("uci set network.{}.interface='{}'", name, req.interface),
        format!("uci set network.{}.target='{}'", name, req.target),
        format!("uci set network.{}.netmask='{}'", name, req.netmask),
        format!("uci set network.{}.gateway='{}'", name, req.gateway),
    ];

    for cmd in &cmds {
        Command::new("sh").args(["-c", cmd]).status()?;
    }
    Command::new("sh").args(["-c", "uci commit network"]).status()?;

    // Apply immediately
    Command::new("ip")
        .args(["route", "add", &req.target, "via", &req.gateway])
        .status().ok();

    json_response(&StatusResponse { success: true })
}

// DELETE /api/routes/{name}
pub fn handle_delete_route(name: &str) -> Response {
    let route_name = format!("route_{}", name);
    Command::new("sh")
        .args(["-c", &format!("uci delete network.{}", route_name)])
        .status()?;
    Command::new("sh").args(["-c", "uci commit network"]).status()?;

    json_response(&StatusResponse { success: true })
}

// POST /api/interface/metric
// Body: { "interface": "wan_eth", "metric": 10 }
pub fn handle_set_metric(body: &str) -> Response {
    let req: MetricRequest = serde_json::from_str(body)?;

    Command::new("sh")
        .args(["-c", &format!("uci set network.{}.metric='{}'", req.interface, req.metric)])
        .status()?;
    Command::new("sh").args(["-c", "uci commit network"]).status()?;

    // Delay restart — response trước, restart sau
    std::thread::spawn(|| {
        std::thread::sleep(std::time::Duration::from_secs(2));
        Command::new("/etc/init.d/network").arg("restart").status().ok();
    });

    json_response(&StatusResponse {
        success: true,
        message: "Metric updated. Network will restart in 2 seconds..."
    })
}
```

### 6. NTP Settings

UCI config (`/etc/config/system`):
```
config timeserver 'ntp'
    option enabled '1'
    option enable_server '0'
    list server 'pool.ntp.org'
    list server 'time.google.com'

config system
    option timezone 'ICT-7'
    option zonename 'Asia/Ho_Chi_Minh'
```

```rust
// GET /api/ntp
pub fn handle_get_ntp() -> Response {
    let ntp = NtpConfig {
        enabled: uci_get("system.ntp.enabled") == "1",
        servers: uci_get_list("system.ntp.server"),
        timezone: uci_get("system.@system[0].timezone"),
        zonename: uci_get("system.@system[0].zonename"),
    };
    json_response(&ntp)
}

// POST /api/ntp
// Body: { "enabled": true, "servers": ["pool.ntp.org"], "timezone": "ICT-7" }
pub fn handle_set_ntp(body: &str) -> Response {
    let req: NtpSetRequest = serde_json::from_str(body)?;

    // Clear old servers
    Command::new("sh").args(["-c", "uci delete system.ntp.server"]).status().ok();

    // Add new servers
    for server in &req.servers {
        Command::new("sh")
            .args(["-c", &format!("uci add_list system.ntp.server='{}'", server)])
            .status()?;
    }

    // Set timezone
    Command::new("sh")
        .args(["-c", &format!("uci set system.@system[0].timezone='{}'", req.timezone)])
        .status()?;

    // Enable/disable
    let enabled = if req.enabled { "1" } else { "0" };
    Command::new("sh")
        .args(["-c", &format!("uci set system.ntp.enabled='{}'", enabled)])
        .status()?;

    Command::new("sh").args(["-c", "uci commit system"]).status()?;

    // Restart NTP service
    Command::new("/etc/init.d/sysntpd").arg("restart").spawn()?;

    json_response(&StatusResponse { success: true })
}

// POST /api/ntp/sync - Trigger manual sync
pub fn handle_ntp_sync() -> Response {
    // Force sync via ntpd or fallback to HTTP Date
    let result = Command::new("ntpd")
        .args(["-q", "-p", "pool.ntp.org"])
        .status();

    match result {
        Ok(s) if s.success() => json_response(&StatusResponse {
            success: true,
            message: "NTP synced"
        }),
        _ => {
            // Fallback to HTTP Date sync
            crate::time_sync::sync_time();
            json_response(&StatusResponse {
                success: true,
                message: "HTTP Date synced (NTP failed)"
            })
        }
    }
}
```

### 6. Wire in server.rs

```rust
// Thêm routes trong server.rs
(Method::Get, "/api/wifi/scan") => network::handle_wifi_scan(),
(Method::Get, "/api/wifi/status") => network::handle_wifi_status(),
(Method::Post, "/api/wifi/connect") => network::handle_wifi_connect(&body),
(Method::Post, "/api/wifi/disconnect") => network::handle_wifi_disconnect(),
(Method::Get, "/api/network") => network::handle_get_network(),
(Method::Post, "/api/network") => network::handle_set_network(&body),
```

### 6. Vue.js NetworkView

```vue
<!-- src/views/NetworkView.vue -->
<template>
  <div class="network">
    <!-- WiFi Section -->
    <section>
      <h3>WiFi</h3>
      <div v-if="wifiStatus.connected">
        Connected: {{ wifiStatus.ssid }} ({{ wifiStatus.signal }} dBm)
        <button @click="disconnect">Disconnect</button>
      </div>

      <button @click="scanWifi">Scan WiFi</button>
      <ul v-if="networks.length">
        <li v-for="net in networks" :key="net.ssid" @click="selectNetwork(net)">
          {{ net.ssid }} ({{ net.signal }} dBm) - {{ net.encryption }}
        </li>
      </ul>

      <div v-if="selectedNetwork">
        <input v-model="wifiPassword" type="password" placeholder="Password">
        <button @click="connectWifi">Connect</button>
      </div>
    </section>

    <!-- LAN/WAN Section -->
    <section>
      <h3>LAN</h3>
      <select v-model="lan.proto">
        <option value="dhcp">DHCP</option>
        <option value="static">Static IP</option>
      </select>
      <div v-if="lan.proto === 'static'">
        <input v-model="lan.ipaddr" placeholder="IP Address">
        <input v-model="lan.netmask" placeholder="Netmask">
        <input v-model="lan.gateway" placeholder="Gateway">
        <input v-model="lan.dns" placeholder="DNS">
      </div>
      <button @click="saveLan">Save LAN</button>
    </section>
  </div>
</template>
```

## Files to Create/Modify

| File | Action |
|------|--------|
| ugate/src/web/network.rs | Create |
| ugate/src/web/mod.rs | Modify - add network module |
| ugate/src/web/server.rs | Modify - add routes |
| ugate/frontend/src/views/NetworkView.vue | Create |
| ugate/frontend/src/router/index.ts | Modify - add route |

## Todo

- [ ] Create web/network.rs
- [ ] Add WiFi scan handler
- [ ] Add WiFi connect/disconnect handlers
- [ ] Add WiFi status handler
- [ ] Add network get/set handlers
- [ ] Wire routes in server.rs
- [ ] Create NetworkView.vue
- [ ] Add router entry
- [ ] Test WiFi scan on device
- [ ] Test WiFi connect on device
- [ ] Test LAN static IP config
- [ ] Test WAN DHCP config
- [ ] Add NTP get/set handlers
- [ ] Add NTP sync handler
- [ ] Test NTP config save
- [ ] Test manual NTP sync
- [ ] Add routing get/add/delete handlers
- [ ] Add metric set handler
- [ ] Test static route add
- [ ] Test static route delete
- [ ] Test metric priority (failover)

## Success Criteria

- [ ] WiFi scan returns list
- [ ] WiFi connect works
- [ ] WiFi status shows current connection
- [ ] LAN/WAN config saves and applies
- [ ] Network restart doesn't hang UI
- [ ] NTP config saves to UCI
- [ ] NTP sync works (with fallback to HTTP Date)
- [ ] Static routes add/delete works
- [ ] Metric failover works (eth down → wifi takes over)

## Security Notes

- API yêu cầu auth (session cookie)
- Password WiFi không được log
- Validate IP address format trước khi set

## Network Safety

**Problem:** `/etc/init.d/network restart` có thể kill connection của client.

**Solutions implemented:**
1. **Delay restart 2s** — Response trước, restart sau
2. **Async spawn** — Không block HTTP response

**Future enhancement (optional):**
```bash
# /etc/init.d/ugate-watchdog
# Nếu không ping được gateway sau 60s → revert config
#!/bin/sh
backup_config() {
    cp /etc/config/network /tmp/network.backup
}

restore_config() {
    if ! ping -c 3 -W 5 "$GATEWAY" > /dev/null 2>&1; then
        cp /tmp/network.backup /etc/config/network
        /etc/init.d/network restart
    fi
}
```

## Next Phase

Phase 8: OpenWrt Packaging
