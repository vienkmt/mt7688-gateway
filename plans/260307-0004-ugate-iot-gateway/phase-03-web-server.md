# Phase 3: Web Server + WebSocket

**Priority:** High
**Status:** pending
**Effort:** 2 days
**Depends on:** Phase 1

## Context

tiny-http + tungstenite web server (proven stack từ vgateway):
- Static file serving (Vue.js embedded)
- REST API cho config
- WebSocket cho real-time data + commands
- Simple password auth

## Module Structure

```
ugate/src/web/
├── mod.rs
├── server.rs     # tiny-http setup, request routing
├── ws.rs         # tungstenite WebSocket handler
├── auth.rs       # Simple password auth
└── status.rs     # SharedStats, StatusCollector, status push
```

## Implementation Steps

### 1. Create web/auth.rs

```rust
use std::collections::HashMap;

pub fn check_session(cookie: Option<&str>) -> bool {
    cookie.map(|c| c.contains("session=valid")).unwrap_or(false)
}

pub fn validate_password(body: &str, expected: &str) -> bool {
    // Parse from form: password=xxx
    let params: HashMap<_, _> = body.split('&')
        .filter_map(|p| p.split_once('='))
        .collect();
    params.get("password").map(|p| *p == expected).unwrap_or(false)
}

pub fn login_response(success: bool) -> tiny_http::Response<std::io::Cursor<Vec<u8>>> {
    if success {
        tiny_http::Response::from_string("OK")
            .with_header(
                tiny_http::Header::from_bytes(&b"Set-Cookie"[..], &b"session=valid; Path=/; HttpOnly"[..]).unwrap()
            )
    } else {
        tiny_http::Response::from_string("Unauthorized")
            .with_status_code(401)
    }
}
```

### 2. Create web/ws.rs

WebSocket handler với tungstenite — **thread per connection + broadcast channel** (không busy-loop):

```rust
use tungstenite::{accept, Message};
use std::net::TcpStream;
use std::sync::Arc;
use std::sync::atomic::{AtomicU8, Ordering};
use tokio::sync::broadcast;

pub struct WsManager {
    pub broadcast_tx: broadcast::Sender<String>,
    pub cmd_tx: std::sync::mpsc::Sender<Command>,
    pub connections: AtomicU8,
    pub max_connections: u8,
}

impl WsManager {
    pub fn new(cmd_tx: std::sync::mpsc::Sender<Command>) -> Self {
        let (broadcast_tx, _) = broadcast::channel(64);
        Self {
            broadcast_tx,
            cmd_tx,
            connections: AtomicU8::new(0),
            max_connections: 8,
        }
    }

    /// Broadcast data to all WS clients
    pub fn broadcast(&self, data: String) {
        let _ = self.broadcast_tx.send(data);
    }
}

/// Spawn per connection — 2 threads: reader + writer
pub fn handle_websocket(stream: TcpStream, manager: Arc<WsManager>) {
    if manager.connections.load(Ordering::Relaxed) >= manager.max_connections {
        return;
    }
    manager.connections.fetch_add(1, Ordering::Relaxed);

    let ws = match accept(stream) {
        Ok(ws) => ws,
        Err(_) => {
            manager.connections.fetch_sub(1, Ordering::Relaxed);
            return;
        }
    };

    let (mut ws_write, mut ws_read) = ws.split();
    let mut broadcast_rx = manager.broadcast_tx.subscribe();
    let cmd_tx = manager.cmd_tx.clone();
    let conn_count = manager.connections.clone();

    // Writer thread — receive from broadcast, send to client
    let writer_handle = std::thread::spawn(move || {
        while let Ok(data) = broadcast_rx.blocking_recv() {
            if ws_write.send(Message::Text(data)).is_err() {
                break;
            }
        }
    });

    // Reader thread — blocking read from client (NO busy-loop)
    loop {
        match ws_read.read() {
            Ok(Message::Text(text)) => {
                if let Some(cmd) = parse_json_command(&text) {
                    let _ = cmd_tx.send(cmd);
                }
            }
            Ok(Message::Close(_)) | Err(_) => break,
            _ => {}
        }
    }

    conn_count.fetch_sub(1, Ordering::Relaxed);
    // Writer thread will exit when broadcast_rx is dropped
}
```

**Key changes:**
- **Blocking read** — không CPU spin
- **broadcast::channel** — push data từ main thread
- **2 threads per connection** — reader (blocking) + writer (blocking recv)

### 3. Create web/server.rs

Main HTTP server (pattern từ vgateway):

```rust
use tiny_http::{Server, Request, Response, Method};
use std::sync::Arc;

pub fn run_server(
    state: Arc<AppState>,
    data_tx: std::sync::mpsc::Sender<String>,
    cmd_tx: std::sync::mpsc::Sender<Command>,
) {
    let config = state.get();
    let addr = format!("0.0.0.0:{}", config.web.port);

    let server = Server::http(&addr).expect("Failed to bind");
    println!("ugate running on http://{}", addr);

    for mut request in server.incoming_requests() {
        let url = request.url().to_string();
        let method = request.method().clone();

        // Check auth for protected routes
        if !url.starts_with("/api/login") && !url.starts_with("/ws") {
            let cookie = request.headers()
                .iter()
                .find(|h| h.field.as_str() == "Cookie")
                .map(|h| h.value.as_str());

            if !auth::check_session(cookie) && !url.starts_with("/assets") && url != "/" {
                let _ = request.respond(Response::from_string("Unauthorized").with_status_code(401));
                continue;
            }
        }

        let response = match (method, url.as_str()) {
            (Method::Get, "/") => serve_index(),
            (Method::Get, path) if path.starts_with("/assets/") => serve_asset(path),
            (Method::Post, "/api/login") => handle_login(&mut request, &state),
            (Method::Get, "/api/config") => handle_get_config(&state),
            (Method::Post, "/api/config") => handle_set_config(&mut request, &state),
            (Method::Get, "/api/status") => handle_get_status(),
            (Method::Post, path) if path.starts_with("/api/gpio/") => {
                handle_set_gpio(&mut request, path, &cmd_tx)
            }
            (Method::Post, "/api/password") => handle_change_password(&mut request, &state),
            (Method::Get, "/ws") => {
                // Upgrade to WebSocket
                handle_ws_upgrade(request, &state, &data_tx, &cmd_tx);
                continue;
            }
            _ => Response::from_string("Not Found").with_status_code(404),
        };

        let _ = request.respond(response);
    }
}

// Embedded static files
static INDEX_HTML: &[u8] = include_bytes!("../../frontend/dist/index.html");

fn serve_index() -> Response<std::io::Cursor<Vec<u8>>> {
    Response::from_data(INDEX_HTML.to_vec())
        .with_header(tiny_http::Header::from_bytes(&b"Content-Type"[..], &b"text/html"[..]).unwrap())
}

fn handle_ws_upgrade(
    request: Request,
    state: &Arc<AppState>,
    data_tx: &std::sync::mpsc::Sender<String>,
    cmd_tx: &std::sync::mpsc::Sender<Command>,
) {
    // Extract TCP stream and spawn WebSocket handler
    let stream = request.upgrade("websocket", Response::empty(101));
    if let Some(stream) = stream {
        let ws_state = Arc::new(WsState { ... });
        std::thread::spawn(move || ws::handle_websocket(stream, ws_state));
    }
}
```

### 4. Create web/mod.rs

```rust
pub mod server;
pub mod ws;
pub mod auth;
pub mod status;

pub use server::run_server;
pub use status::{SharedStats, StatusCollector, SystemStatus};
```

## WebSocket Status Push

Status broadcast mỗi 1 giây tới tất cả WebSocket clients.

### Status JSON Structure

```json
{
  "mac": "AA:BB:CC:DD:EE:FF",
  "ip": "10.10.10.1",
  "gateway": "10.10.10.1",
  "version": "1.0.0",
  "uptime": "2d 3h 45m",
  "cpu": 15,
  "ram_used": 12,
  "ram_total": 64,
  "flash_used": 8,
  "flash_total": 16,
  "wifi_state": "connected",
  "wifi_rssi": -45,
  "uart": {
    "rx_bytes": 12345,
    "rx_frames": 100,
    "tx_bytes": 5678,
    "tx_frames": 50,
    "failed": 0,
    "config": "115200 8N1"
  },
  "mqtt": {
    "enabled": true,
    "state": "connected",
    "published": 100,
    "failed": 0
  },
  "tcp": {
    "enabled": true,
    "mode": "server",
    "state": "listening",
    "connections": 2
  },
  "gpio": [false, true, false, true]
}
```

### Create web/status.rs

**SharedStats** — atomic counters shared giữa UART, MQTT, TCP tasks:

```rust
use std::sync::atomic::{AtomicU64, AtomicU8, Ordering};
use std::sync::Arc;

pub struct SharedStats {
    pub uart_rx_bytes: AtomicU64,
    pub uart_rx_frames: AtomicU64,
    pub uart_tx_bytes: AtomicU64,
    pub uart_tx_frames: AtomicU64,
    pub uart_failed: AtomicU64,
    pub mqtt_published: AtomicU64,
    pub mqtt_failed: AtomicU64,
    pub mqtt_state: AtomicU8,  // 0=disabled, 1=disconnected, 2=connected
    pub tcp_connections: AtomicU8,
    pub tcp_state: AtomicU8,   // 0=disabled, 1=listening, 2=connected
    pub gpio_states: [AtomicU8; 4],
}

impl SharedStats {
    pub fn new() -> Self {
        Self {
            uart_rx_bytes: AtomicU64::new(0),
            uart_rx_frames: AtomicU64::new(0),
            uart_tx_bytes: AtomicU64::new(0),
            uart_tx_frames: AtomicU64::new(0),
            uart_failed: AtomicU64::new(0),
            mqtt_published: AtomicU64::new(0),
            mqtt_failed: AtomicU64::new(0),
            mqtt_state: AtomicU8::new(0),
            tcp_connections: AtomicU8::new(0),
            tcp_state: AtomicU8::new(0),
            gpio_states: [AtomicU8::new(0); 4],
        }
    }
}
```

**StatusCollector** — collect system info từ /proc/* và shared stats:

```rust
use serde::Serialize;

#[derive(Serialize)]
pub struct SystemStatus {
    pub mac: String,
    pub ip: String,
    pub gateway: String,
    pub version: &'static str,
    pub uptime: String,
    pub cpu: u8,
    pub ram_used: u16,
    pub ram_total: u16,
    pub flash_used: u16,
    pub flash_total: u16,
    pub wifi_state: String,
    pub wifi_rssi: i8,
    pub uart: UartStatus,
    pub mqtt: MqttStatus,
    pub tcp: TcpStatus,
    pub gpio: [bool; 4],
}

#[derive(Serialize)]
pub struct UartStatus {
    pub rx_bytes: u64,
    pub rx_frames: u64,
    pub tx_bytes: u64,
    pub tx_frames: u64,
    pub failed: u64,
    pub config: String,
}

#[derive(Serialize)]
pub struct MqttStatus {
    pub enabled: bool,
    pub state: String,
    pub published: u64,
    pub failed: u64,
}

#[derive(Serialize)]
pub struct TcpStatus {
    pub enabled: bool,
    pub mode: String,
    pub state: String,
    pub connections: u8,
}

pub struct StatusCollector {
    stats: Arc<SharedStats>,
    config: Arc<AppState>,
}

impl StatusCollector {
    pub fn new(stats: Arc<SharedStats>, config: Arc<AppState>) -> Self {
        Self { stats, config }
    }

    pub fn collect(&self) -> SystemStatus {
        let cfg = self.config.get();

        SystemStatus {
            mac: read_mac_address(),
            ip: read_ip_address(),
            gateway: read_gateway(),
            version: env!("CARGO_PKG_VERSION"),
            uptime: read_uptime(),
            cpu: read_cpu_usage(),
            ram_used: read_mem_used(),
            ram_total: 64,
            flash_used: read_flash_used(),
            flash_total: 16,
            wifi_state: read_wifi_state(),
            wifi_rssi: read_wifi_rssi(),
            uart: UartStatus {
                rx_bytes: self.stats.uart_rx_bytes.load(Ordering::Relaxed),
                rx_frames: self.stats.uart_rx_frames.load(Ordering::Relaxed),
                tx_bytes: self.stats.uart_tx_bytes.load(Ordering::Relaxed),
                tx_frames: self.stats.uart_tx_frames.load(Ordering::Relaxed),
                failed: self.stats.uart_failed.load(Ordering::Relaxed),
                config: format!("{} {}{}{}", cfg.uart.baud, cfg.uart.data_bits,
                    cfg.uart.parity.chars().next().unwrap_or('N'), cfg.uart.stop_bits),
            },
            mqtt: MqttStatus {
                enabled: cfg.mqtt.enabled,
                state: mqtt_state_str(self.stats.mqtt_state.load(Ordering::Relaxed)),
                published: self.stats.mqtt_published.load(Ordering::Relaxed),
                failed: self.stats.mqtt_failed.load(Ordering::Relaxed),
            },
            tcp: TcpStatus {
                enabled: cfg.tcp.enabled,
                mode: cfg.tcp.mode.clone(),
                state: tcp_state_str(self.stats.tcp_state.load(Ordering::Relaxed)),
                connections: self.stats.tcp_connections.load(Ordering::Relaxed),
            },
            gpio: [
                self.stats.gpio_states[0].load(Ordering::Relaxed) != 0,
                self.stats.gpio_states[1].load(Ordering::Relaxed) != 0,
                self.stats.gpio_states[2].load(Ordering::Relaxed) != 0,
                self.stats.gpio_states[3].load(Ordering::Relaxed) != 0,
            ],
        }
    }
}

// Helper functions
fn read_uptime() -> String {
    std::fs::read_to_string("/proc/uptime")
        .ok()
        .and_then(|s| s.split_whitespace().next().map(String::from))
        .map(|s| format_uptime(s.parse::<f64>().unwrap_or(0.0)))
        .unwrap_or_else(|| "0s".into())
}

fn format_uptime(secs: f64) -> String {
    let s = secs as u64;
    let d = s / 86400;
    let h = (s % 86400) / 3600;
    let m = (s % 3600) / 60;
    if d > 0 { format!("{}d {}h {}m", d, h, m) }
    else if h > 0 { format!("{}h {}m", h, m) }
    else { format!("{}m", m) }
}

fn mqtt_state_str(state: u8) -> String {
    match state {
        0 => "disabled",
        1 => "disconnected",
        2 => "connected",
        _ => "unknown",
    }.into()
}

fn tcp_state_str(state: u8) -> String {
    match state {
        0 => "disabled",
        1 => "listening",
        2 => "connected",
        _ => "unknown",
    }.into()
}
```

### Status Push Thread (main.rs)

Broadcast status JSON mỗi 1 giây:

```rust
// In main.rs - after creating ws_manager
let shared_stats = Arc::new(SharedStats::new());

// Status broadcast thread - push mỗi 1 giây
let ws_broadcast_tx = ws_manager.broadcast_tx.clone();
let stats_clone = shared_stats.clone();
let config_clone = app_state.clone();
std::thread::spawn(move || {
    let collector = StatusCollector::new(stats_clone, config_clone);
    loop {
        std::thread::sleep(Duration::from_secs(1));
        let status = collector.collect();
        if let Ok(json) = serde_json::to_string(&status) {
            let _ = ws_broadcast_tx.send(json);
        }
    }
});
```

### Update WsManager

Thêm shared_stats reference để writer có thể access GPIO state:

```rust
pub struct WsManager {
    pub broadcast_tx: broadcast::Sender<String>,
    pub cmd_tx: std::sync::mpsc::Sender<Command>,
    pub connections: AtomicU8,
    pub max_connections: u8,
    pub shared_stats: Arc<SharedStats>,  // NEW
}
```

## Files to Create/Modify

| File | Action |
|------|--------|
| ugate/src/web/mod.rs | Create |
| ugate/src/web/server.rs | Create |
| ugate/src/web/ws.rs | Create |
| ugate/src/web/auth.rs | Create |
| ugate/src/web/status.rs | Create |
| ugate/src/main.rs | Modify |

## Dependencies to Add

```toml
tiny_http = "0.12"
tungstenite = "0.21"
```

## Todo

- [ ] Create web/auth.rs
- [ ] Create web/ws.rs
- [ ] Create web/server.rs
- [ ] Create web/status.rs — SharedStats, StatusCollector, SystemStatus
- [ ] Create web/mod.rs
- [ ] Wire SharedStats in main.rs — pass to UART, MQTT, TCP tasks
- [ ] Wire status push thread in main.rs — broadcast mỗi 1 giây
- [ ] Test login flow
- [ ] Test change password API
- [ ] Test API endpoints
- [ ] Test WebSocket connection
- [ ] Test WebSocket status broadcast (1s interval)

## Success Criteria

- [ ] Login works with password
- [ ] API returns config
- [ ] WebSocket connects
- [ ] WebSocket receives status JSON mỗi 1 giây
- [ ] Status JSON chứa đầy đủ: system info, UART, MQTT, TCP, GPIO
- [ ] WebSocket sends commands (GPIO control)

## Next Phase

Phase 4: GPIO Control
