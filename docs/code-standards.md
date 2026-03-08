# Code Standards - ugate IoT Gateway

**Last Updated:** 2026-03-08
**Version:** 1.6.0 (Phases 1-9 Complete)

## Project Structure

```
ugate/
├── Cargo.toml                  # Project manifest (workspace member)
├── src/
│   ├── main.rs                 # Startup, channel creation, task spawn (~300 lines)
│   ├── config.rs               # AppState, RwLock<Config>, watch channel
│   ├── commands.rs             # Command enum, parsing (JSON/binary)
│   ├── time_sync.rs            # HTTP-based NTP at startup
│   ├── uci.rs                  # UCI wrapper for config I/O (146 lines)
│   │
│   ├── channels/               # Data fan-out (outbound)
│   │   ├── mod.rs
│   │   ├── mqtt.rs             # MQTT pub/sub (std::thread, rumqttc sync) (~250 lines)
│   │   ├── http_pub.rs         # HTTP POST publisher (tokio::spawn) (~180 lines)
│   │   ├── tcp.rs              # TCP server + client (async)
│   │   ├── buffer.rs           # Offline buffer (RAM + disk)
│   │   └── reconnect.rs        # Exponential backoff logic
│   │
│   ├── uart/                   # UART I/O
│   │   ├── mod.rs
│   │   ├── reader.rs           # AsyncFd, frame detection (~180 lines)
│   │   └── writer.rs           # UART TX queue
│   │
│   ├── gpio.rs                 # GPIO control (chardev ioctl)
│   │
│   └── web/                    # HTTP server + WebSocket + API handlers
│       ├── mod.rs              # Shared helpers (json_resp, jval, json_escape) (75 LOC)
│       ├── server.rs           # HTTP routing, handlers (573 LOC)
│       ├── auth.rs             # Session manager, token-based (141 LOC)
│       ├── status.rs           # SharedStats (atomic counters) (206 LOC)
│       ├── ws.rs               # WebSocket, tungstenite (121 LOC)
│       ├── wifi.rs             # WiFi 4 modes, scan, status (209 LOC)
│       ├── netcfg.rs           # Network, NTP, routing (350 LOC)
│       ├── maintenance.rs      # Backup/restore/upgrade (362 LOC)
│       ├── toolbox.rs          # System tools and diagnostics (135 LOC)
│       ├── syslog.rs           # Syslog viewer (165 LOC)
│       └── embedded_index.html # Vanilla JS SPA (925 LOC)
│
│   ├── assets/                 # Frontend assets (174 LOC)
│   │   ├── style.css           # Responsive CSS styling (132 LOC)
│   │   └── preview-mock.js     # Local preview support (42 LOC)
│   │
│   └── modals/                 # Modal dialog system (56 LOC)
│       ├── modals-loader.js    # Modal injection + helper functions (42 LOC)
│       └── help-data-wrap-format.html  # Data Wrap format help modal (14 LOC)
│
└── Cargo.lock
```

**Web Modules (Total: 2,538+ LOC):**
- Server: 588 LOC (HTTP routing, REST API endpoints)
- WiFi: 209 LOC (4 modes, scanning, status, draft/apply)
- Network: 350 LOC (LAN/WAN, NTP, static routing, draft/apply/revert)
- Maintenance: 362 LOC (backup/restore, firmware upgrade, factory reset)
- Toolbox: 135 LOC (ping, traceroute, nslookup diagnostics)
- Syslog: 165 LOC (OpenWrt log viewer with filtering)
- Auth: 141 LOC (token-based sessions, max 4, 24h TTL, rate limiting)
- WebSocket: 121 LOC (tungstenite, live UART + stats streaming)
- Status: 210 LOC (atomic counters, UART stats, channel monitoring)
- Helpers: 75 LOC (json_resp, jval, json_escape, shared utilities)
- Frontend HTML: 925 LOC (vanilla JS SPA, no framework, no npm)
- Frontend CSS: 132 LOC (responsive styles, mobile-first)
- Modal System: 56 LOC (modal injection, help dialogs)

## Dependencies Overview

### Core Runtime
- **tokio** (v1): Single-thread async executor, channels, sync primitives
- **std::thread**: OS threads for MQTT (rumqttc sync Client avoids hang on MIPS)

### Networking
- **rumqttc** (v0.24): MQTT client (sync Client preferred on MIPS)
- **rustls** (v0.22): TLS (no OpenSSL dependency)
- **webpki-roots** (v0.26): Root certificates for TLS
- **ureq** (v2): Sync HTTP client (wrapped in spawn_blocking)
- **tiny-http** (v0.12): Minimal HTTP server
- **tungstenite** (v0.21): WebSocket (async-std compatible)

### Serialization
- **serde** (v1): Serialization framework (derive macros only, no_std)
- **Note:** No serde_json — manual JSON formatting with format!() to keep binary small

### System & Logging
- **log** (v0.4): Log facade
- **syslog** (v6): OpenWrt syslog integration
- **thiserror** (v1): Error handling macros

### Optional/Embedded
- **nom**: Parser combinator (optional, for binary protocol parsing)
- **heapless**: Fixed-capacity collections (optional, for no_std)

## Coding Conventions

### Naming

| Category | Convention | Example |
|----------|-----------|---------|
| **Modules** | kebab-case | `channels/mqtt.rs`, `uart/reader.rs` |
| **Files** | kebab-case | `http_pub.rs`, `time_sync.rs` |
| **Functions** | snake_case | `parse_config()`, `run_mqtt_thread()` |
| **Structs/Enums** | PascalCase | `AppState`, `MqttConfig`, `Command` |
| **Constants** | UPPER_SNAKE_CASE | `MAX_SESSIONS`, `DEFAULT_UART_BAUD` |
| **Variables** | snake_case | `config`, `mqtt_rx`, `uart_tx` |
| **Lifetimes** | 'a, 'b, ... | Generic lifetime parameters |
| **Type Parameters** | T, U, ... | Generic type parameters |

### Function Signatures

```rust
// ✓ Good: Clear purpose, documented errors, async where needed
pub async fn run(
    state: Arc<AppState>,
    data_rx: mpsc::Receiver<Vec<u8>>,
    cmd_tx: mpsc::Sender<Command>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // ...
}

// ✓ Good: std::thread function (no async)
pub fn run_sync(
    state: Arc<AppState>,
    data_rx: std::sync::mpsc::Receiver<Vec<u8>>,
) {
    // Loop until error or return signal
}

// ✓ Good: Private helper, clear intent
fn parse_frame(bytes: &[u8], mode: FrameMode) -> Option<Vec<u8>> {
    // ...
}
```

### Error Handling

**Principle:** Use `Result<T, E>` for recoverable errors; `panic!` only for unrecoverable bugs.

```rust
// ✓ Good: Explicit error propagation
fn load_config() -> Result<Config, String> {
    let uci = Uci::get("ugate.mqtt.broker")?;
    Ok(Config { ... })
}

// ✓ Good: Error context in logs
match connect_mqtt(&config).await {
    Ok(client) => {
        log::info!("[MQTT] Connected");
        Ok(client)
    }
    Err(e) => {
        log::error!("[MQTT] Connect failed: {}", e);
        Err(e)
    }
}

// ✓ Good: Graceful degradation (non-critical)
if let Err(e) = save_to_uci(&config) {
    log::warn!("[UCI] Save failed: {}", e);
    // Continue; config may be in-memory only
}

// ✓ Good: Panic only for truly unrecoverable
let config = state.get(); // Never panics (RwLock can't deadlock with single-thread)
```

### Async/Await Patterns

```rust
// ✓ Good: Avoid blocking in async context
tokio::spawn(async {
    loop {
        tokio::select! {
            _ = config_watch.changed() => {
                log::info!("Config changed, reloading...");
                return; // Restart from parent
            }
            Some(data) = data_rx.recv() => {
                // Spawn blocking for sync operations
                tokio::task::spawn_blocking(move || {
                    heavy_http_post(&url, &data); // sync ureq
                }).await.ok();
            }
        }
    }
});

// ✓ Good: std::thread for rumqttc sync Client
std::thread::spawn(move || {
    let mut client = rumqttc::Client::new(options, 10);
    loop {
        // recv_timeout is blocking, that's OK in OS thread
        match mqtt_rx.recv_timeout(Duration::from_secs(1)) {
            Ok(data) => {
                let _ = client.publish(&topic, QoS::AtLeastOnce, false, data);
            }
            Err(_) => {} // Timeout, continue
        }
    }
});

// ✗ Avoid: Blocking in tokio::spawn without spawn_blocking
tokio::spawn(async {
    ureq::get(&url).call(); // ✗ Blocks tokio worker! Use spawn_blocking.
});
```

### Channel Types

| Pattern | Channel | Reason |
|---------|---------|--------|
| **Async → Async** | `tokio::sync::mpsc` | Async-aware, supports select! |
| **Async → Async (broadcast)** | `tokio::sync::broadcast` | Multi-subscribe pattern |
| **Async → Config notify** | `tokio::sync::watch<()>` | Notify-only, no data clone |
| **Std thread ↔ Async** | `std::sync::mpsc` | Cross-boundary compatibility |
| **Std thread → Std thread** | `std::sync::mpsc` | Standard, no async overhead |
| **Web socket frames** | `tokio::sync::mpsc` (per connection) | Per-client buffering |

### Shared State Pattern

```rust
// ✓ Good: RwLock for read-heavy, watch for notifications
pub struct AppState {
    config: RwLock<Config>,
    config_tx: watch::Sender<()>,
}

impl AppState {
    pub fn get(&self) -> Config {
        self.config.read().unwrap().clone()
    }

    pub fn update(&self, new: Config) {
        {
            let mut cfg = self.config.write().unwrap();
            *cfg = new;
        }
        let _ = self.config_tx.send(()); // Notify subscribers
    }

    pub fn subscribe(&self) -> watch::Receiver<()> {
        self.config_tx.subscribe()
    }
}

// Usage: HTTP handler
let config = state.get(); // Read lock, clone
state.update(new_config); // Write lock, notify
```

### Logging Conventions

**Format:** `[COMPONENT] Message` (uppercase component code)

```rust
log::info!("[UART] Opened {} at 115200 baud", port);
log::warn!("[MQTT] Connection lost, retrying in 5s...");
log::error!("[HTTP] POST failed: {}", e);
log::debug!("[TCP] Server accepted connection from {}", addr);

// With context
log::error!("[MQTT] Connect error: {}. Details: {:?}", e.to_string(), e);
```

**Component Codes:**
- `[MAIN]` — Startup, shutdown
- `[UART]` — UART reader/writer
- `[MQTT]` — MQTT publisher/subscriber
- `[HTTP]` — HTTP server, publishers
- `[TCP]` — TCP server/client
- `[GPIO]` — GPIO control
- `[WS]` — WebSocket
- `[CONFIG]` — Config loading/saving
- `[AUTH]` — Session management

### File Size Management

**Target:** Each module ≤ 200 lines (UART reader is ~180, MQTT is ~250 exception)

```
ugate/src/
├── main.rs          ~300 lines (startup, multi-module, exception)
├── config.rs        ~150 lines
├── commands.rs      ~100 lines
├── time_sync.rs     ~50 lines
├── uci.rs           ~100 lines
├── gpio.rs          ~180 lines
│
├── channels/
│   ├── mqtt.rs      ~250 lines (exception: complex MQTT logic)
│   ├── http_pub.rs  ~180 lines
│   ├── tcp.rs       ~200 lines
│   └── buffer.rs    ~120 lines
│
├── uart/
│   ├── reader.rs    ~180 lines
│   └── writer.rs    ~80 lines
│
└── web/
    ├── server.rs    ~200 lines (exception: HTTP routing)
    ├── auth.rs      ~100 lines
    ├── status.rs    ~80 lines
    └── ws.rs        ~150 lines
```

## Modal System

Modal HTML templates tách riêng trong `ugate/src/modals/` — content HTML thuần, JS chỉ load + inject data.

### Cấu trúc

```
ugate/src/
├── assets/
│   ├── style.css            # CSS chính
│   └── preview-mock.js      # Mock data cho preview (không deploy)
├── modals/
│   ├── modals-loader.js     # openModal() helper + show*() functions
│   └── help-*.html          # HTML templates cho từng modal
```

### Thêm modal mới

1. **HTML template** — `ugate/src/modals/help-<context>-<topic>.html`
   - Dùng class `modal-pre` cho code blocks
   - Đặt `id` cho elements cần inject dynamic data
2. **Server route** — `web/server.rs`:
   ```rust
   const MODAL_HELP_FOO: &str = include_str!("../modals/help-foo-bar.html");
   // Trong match routes:
   (Get, "/modals/help-foo-bar") => Response::from_string(MODAL_HELP_FOO).with_header(content_type_html())
   ```
3. **JS function** — `modals/modals-loader.js`:
   ```js
   function showFooHelp() {
     openModal('foo-modal', 'Title', '/modals/help-foo-bar', function(body) {
       body.querySelector('#el-id').textContent = dynamicValue;
     });
   }
   ```
4. **Gọi từ UI**: `helpBtn(()=>showFooHelp())` — icon ⓘ tròn, hover xanh

### UI Helpers

| Helper | Mô tả |
|--------|--------|
| `helpBtn(fn)` | Nút icon ⓘ (SVG info circle), class `.btn-help` |
| `openModal(id, title, url, onLoaded)` | Fetch HTML template, tạo overlay + modal container |
| `.modal-pre` | CSS class cho code blocks trong modal |

### Local Preview (không cần device)

Preview Web UI trên máy local bằng Python HTTP server + mock data:

```bash
cd ugate/src && python3 -c "
import http.server
class H(http.server.SimpleHTTPRequestHandler):
    def do_GET(self):
        r={
          '/':('embedded_index.html','text/html'),
          '/style.css':('assets/style.css','text/css'),
          '/modals.js':('modals/modals-loader.js','application/javascript'),
          '/modals/help-data-wrap-format':('modals/help-data-wrap-format.html','text/html'),
          '/preview-mock.js':('assets/preview-mock.js','application/javascript'),
        }
        if self.path in r:
            f,ct=r[self.path];self.send_response(200)
            self.send_header('Content-Type',ct);self.end_headers()
            self.wfile.write(open(f,'rb').read())
        else:
            self.send_response(404);self.end_headers()
http.server.HTTPServer(('',8901),H).serve_forever()
"
```

Mở `http://localhost:8901`, Console:
```js
fetch('/preview-mock.js').then(r=>r.text()).then(eval)
```

Mock data file: `assets/preview-mock.js` — set `S.page`, `S.config`, `S.status` rồi gọi `render()`.
Thêm route mới vào dict `r` khi tạo thêm modal/asset.

## Configuration (UCI)

**File:** `/etc/config/ugate` (created automatically if missing)

**Format:** UCI (OpenWrt native)

```ini
# MQTT Configuration
config mqtt 'main'
    option enabled '1'
    option broker 'mqtt.example.com'
    option port '1883'
    option tls '0'
    option client_id 'ugate-device1'
    option username ''
    option password ''
    option topic 'device/sensor/data'
    option sub_topic 'device/sensor/cmd'
    option qos '1'

# HTTP Publisher
config http 'main'
    option enabled '1'
    option url 'https://api.example.com/data'
    option method 'post'

# TCP Configuration
config tcp 'main'
    option enabled '1'
    option mode 'both'
    option server_port '502'
    option client_host 'gateway.local'
    option client_port '502'

# UART Configuration
config uart 'main'
    option enabled '1'
    option port '/dev/ttyS0'
    option baudrate '115200'
    option data_bits '8'
    option parity 'none'
    option stop_bits '1'
    option frame_mode 'line'      # line, fixed, timeout
    option frame_length '128'     # for fixed mode
    option frame_timeout_ms '100' # for timeout mode
    option gap_ms '10'            # gap between bytes

# GPIO Configuration
config gpio 'main'
    option enabled '1'
    list line '17:out:high'    # pin:direction:initial_state
    list line '27:in:pull_up'

# Web Configuration
config web 'main'
    option enabled '1'
    option port '8888'
    option password 'admin123'
    option max_ws_connections '32'

# General Configuration
config general 'main'
    option log_level 'info'
    option buffer_ram_limit '64'
```

## Testing Strategy

### Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_frame_line_mode() {
        let data = b"hello\nworld\n";
        let frames = parse_frames(data, FrameMode::Line);
        assert_eq!(frames.len(), 2);
    }

    #[test]
    fn test_offline_buffer_overflow() {
        let mut buf = OfflineBuffer::new(2, "/tmp/test");
        buf.push(vec![1]);
        buf.push(vec![2]);
        buf.push(vec![3]); // → disk
        assert_eq!(buf.pop(), Some(vec![3])); // disk first
    }
}
```

### Integration Tests

- **Device test:** Deploy binary to MT7688, verify:
  - Web UI loads (port 8888)
  - MQTT publishes frames
  - HTTP POSTs send data
  - TCP server accepts connections
  - GPIO toggles
  - Config persists (UCI)

### Stress Tests

- **24h runtime:** Monitor RAM, CPU, channel queue lengths
- **100 msg/sec UART:** Verify buffer doesn't overflow
- **1000 WebSocket messages:** Check memory stability
- **Config hot-reload:** Verify no data loss during reconfig

## Performance Targets

| Metric | Target | Actual |
|--------|--------|--------|
| Binary size | <1.2MB | ~800KB |
| Startup time | <5s | ~2s |
| Memory (idle) | <30MB | ~15MB |
| Memory (100 msg/s) | <50MB | ~25MB |
| HTTP latency | <100ms | ~50ms |
| WebSocket latency | <50ms | ~30ms |
| GPIO toggle time | <50ms | ~10ms |

## Security Considerations

| Issue | Mitigation |
|-------|-----------|
| XSS (web UI) | HTML escaping in templates, Content-Type: text/html |
| Command injection (UCI) | Proper argument quoting, no shell exec |
| MQTT password | Base64 encoding (not encryption), assume LAN trust |
| Session hijack | Random token generation, 4-session limit, no persistence |
| UART buffer overflow | Bounded queue (64 frames), disk overflow to /tmp |
| TLS cert validation | System clock sync at startup, rustls verification |

## CI/CD & Build

```bash
# Check for syntax errors
cargo check --target mipsel-unknown-linux-musl

# Build release binary (MIPS)
cross +nightly build --target mipsel-unknown-linux-musl --release -p ugate

# Binary location
target/mipsel-unknown-linux-musl/release/ugate (~800KB)

# Deploy to device
scp target/mipsel-unknown-linux-musl/release/ugate root@10.10.10.1:/tmp/
ssh root@10.10.10.1 'chmod +x /tmp/ugate && nohup /tmp/ugate > /var/log/ugate.log 2>&1 &'

# Verify running
ssh root@10.10.10.1 'ps aux | grep ugate'
ssh root@10.10.10.1 'logread | grep ugate'
```

## Breaking Changes & Phase 7 Updates (v2.0 → v3.0)

| Feature | Old (v2.0) | New (v3.0 Phase 7) | Migration Notes |
|---------|-----------|------------------|-----------------|
| **Frontend** | Vue.js framework | Vanilla JS (no build step) | Remove npm dependencies, simpler deployment |
| **WiFi Config** | Simple on/off | 4 modes (STA/AP/STA+AP/Off) | Update UI, add mode selector |
| **Network Config** | WAN only | LAN+WAN+NTP+Routes | More complex, but full control |
| **Draft/Apply** | Immediate | Save to RAM → Apply to flash | User must click "Apply" to persist |
| **Session auth** | No token | Token-based (24h TTL) | Add login endpoint to client |
| **Upgrade** | Manual | Local IPK + Remote URL | Firmware update via web UI now |
| **Backup** | Manual UCI export | Web UI download | Full config backup in one click |

**Migration for Existing Deployments:**
1. Backup config: `cp /etc/config/ugate /tmp/ugate.backup`
2. Build Phase 7 binary
3. Deploy new binary
4. Review WiFi/Network settings (draft mode requires Apply)
5. Test WiFi mode switching and network config
6. Verify session login works (24h TTL)

## References

- **CLAUDE.md** — Hardware constraints and build commands
- **system-architecture.md** — Detailed architecture diagrams
- **project-overview-pdr.md** — Features and requirements
- **project-changelog.md** — Version history and breaking changes
- **./mips-rust-notes/bugs-and-gotchas.md** — MIPS-specific issues
