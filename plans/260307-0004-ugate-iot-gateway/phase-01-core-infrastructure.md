# Phase 1: Core Infrastructure

**Priority:** High
**Status:** pending
**Effort:** 2 days
**Depends on:** -

## Context

Copy và refactor core modules từ vgateway. Foundation cho tất cả phases sau.

## Objective

Setup project structure với:
- Config management (AppState, Config structs) — UCI only
- UCI wrapper
- UART reader (AsyncFd + epoll)
- Channel infrastructure

## Module Structure

```
ugate/src/
├── main.rs
├── config.rs      # AppState, Config (from vgateway)
├── uci.rs         # UCI wrapper (from vgateway)
├── time_sync.rs   # Sync clock via HTTP Date (from vgateway) - MUST run before TLS!
├── uart/
│   ├── mod.rs
│   ├── reader.rs  # AsyncFd + epoll
│   └── writer.rs  # TX to MCU
└── commands.rs    # Command parser
```

## Implementation Steps

### 0. Copy time_sync.rs (CRITICAL for TLS)

**Vấn đề:** TLS certificate validation cần system time chính xác. Nếu time sai → TLS handshake fail.

**Giải pháp vgateway:** Sync time via HTTP Date header (plain HTTP, không TLS):
```rust
// main.rs - PHẢI chạy đầu tiên!
time_sync::sync_time();  // Sync clock trước khi kết nối TLS

// time_sync.rs
pub fn sync_time() {
    // Plain HTTP (no TLS) - tránh chicken-and-egg
    let resp = ureq::head("http://www.google.com").call()?;
    let date_str = resp.header("date")?;  // "Thu, 06 Feb 2026 11:30:00 GMT"

    // Parse → unix timestamp → settimeofday()
    let ts = parse_http_date(&date_str)?;
    unsafe { libc::settimeofday(&tv, null()); }
}
```

**Flow:**
```
Startup → time_sync() → [TLS ready] → MQTT/HTTP connections
```

### 1. Copy & refactor config.rs

From vgateway, adapt for ugate:
- Add TCP config (enabled, mode, port, remote_host)
- Add GPIO config (pins array)
- Add WebSocket config (max_connections)
- Keep MQTT, HTTP, UART, General configs

```rust
pub struct Config {
    pub mqtt: MqttConfig,
    pub http: HttpConfig,
    pub tcp: TcpConfig,      // NEW
    pub uart: UartConfig,
    pub gpio: GpioConfig,    // NEW
    pub web: WebConfig,      // NEW
    pub general: GeneralConfig,
}

pub struct TcpConfig {
    pub enabled: bool,
    pub mode: TcpMode,  // Server | Client | Both
    pub server_port: u16,
    pub client_host: String,
    pub client_port: u16,
}

pub struct GpioConfig {
    pub pins: [u8; 4],      // GPIO pin numbers
    pub led_pin: u8,        // Heartbeat LED
}

pub struct WebConfig {
    pub port: u16,
    pub password: String,   // Simple auth
    pub max_ws_connections: u8,
}
```

### 2. Copy uci.rs

Direct copy từ vgateway, no changes needed.

### 3. UART Frame Detection — Technical Spec

#### Protocol Modes

| Mode | Trigger | Use case |
|------|---------|----------|
| none | Gap silence | Legacy, debug, unknown protocol |
| frame | Length/Timeout/Delimiter | Fixed protocol, custom MCU |
| modbus | State machine + CRC | Modbus RTU devices |

#### Module Structure

```
uart/
├── mod.rs
├── reader.rs      # AsyncFd + byte pump
├── framer.rs      # Frame detection trait
│   ├── GapFramer      (none)
│   ├── FixedFramer    (frame)
│   └── ModbusFramer   (modbus)
├── modbus.rs      # Modbus state machine + CRC16
└── writer.rs      # TX to MCU
```

#### Mode 1: None (Gap-based)
```
Byte → buffer → reset gap timer
Gap timer expire → flush → fan-out
Buffer full (512B) → flush + warn
```

#### Mode 2: Frame (Fixed-length + Delimiter)
```
Complete when ANY:
  - buf.len() >= frame_length
  - elapsed >= frame_timeout
  - tag_enabled && byte == tag_tail

If tag_head enabled: skip bytes until tag_head
```

#### Mode 3: Modbus RTU
```
State: IDLE → ADDR → FUNC → DATA → CRC → fan-out/drop
Gap time: 3.5T based on baudrate (9600→4ms, 115200→1ms)
CRC: CRC-16/IBM (0xA001, little-endian)
Supported: 0x03, 0x04 (Read Registers)
```

#### Common Rules
- Raw termios (no canonical, echo, flow)
- Non-blocking read + 1ms tick
- Buffer overflow → flush + warn
- Partial frame timeout → drop + warn

### 4. Create uart/writer.rs

New module for TX to MCU:

```rust
pub struct UartWriter {
    fd: std::fs::File,
}

impl UartWriter {
    pub fn new(port: &str) -> io::Result<Self> { ... }
    pub fn write(&mut self, data: &[u8]) -> io::Result<()> { ... }
}
```

### 5. Create commands.rs

Parse commands from all sources:

```rust
pub enum Command {
    Gpio { pin: u8, state: GpioState },
    UartTx { data: String },
}

pub enum GpioState { On, Off, Toggle }

// Parse from UART: "GPIO:1:ON\n"
pub fn parse_uart_command(line: &str) -> Option<Command> { ... }

// Parse from JSON: {"cmd":"gpio","pin":1,"state":"on"}
pub fn parse_json_command(json: &str) -> Option<Command> { ... }
```

### 6. Setup channel infrastructure in main.rs

```rust
// Channels
let (uart_broadcast_tx, _) = broadcast::channel::<String>(64);
let (mqtt_tx, mqtt_rx) = std::sync::mpsc::channel::<String>();
let (http_tx, http_rx) = tokio::sync::mpsc::channel::<String>(64);
let (tcp_tx, tcp_rx) = tokio::sync::mpsc::channel::<String>(64);
let (cmd_tx, cmd_rx) = tokio::sync::mpsc::channel::<Command>(32);
let (gpio_tx, gpio_rx) = tokio::sync::mpsc::channel::<GpioCommand>(16);
```

## Files to Create/Modify

| File | Action | Source |
|------|--------|--------|
| ugate/src/config.rs | Create | Copy from vgateway, extend |
| ugate/src/uci.rs | Create | Copy from vgateway |
| ugate/src/uart/mod.rs | Create | New |
| ugate/src/uart/reader.rs | Create | Refactor from vgateway |
| ugate/src/uart/writer.rs | Create | New |
| ugate/src/commands.rs | Create | New |
| ugate/src/main.rs | Modify | Setup channels |
| ugate/Cargo.toml | Modify | Add dependencies |

## Dependencies to Add

```toml
[dependencies]
tokio = { version = "1", features = ["rt", "net", "io-util", "sync", "time", "fs"] }
serde = { version = "1", features = ["derive"] }
log = "0.4"
thiserror = "1"

[profile.release]
opt-level = "z"
lto = true
codegen-units = 1
panic = "abort"
strip = true
```

## Todo

- [ ] Setup Cargo.toml với profile.release
- [ ] Copy config.rs từ vgateway, adapt for UCI
- [ ] Extend Config với TcpConfig, GpioConfig, WebConfig
- [ ] Copy uci.rs từ vgateway
- [ ] Create uart/mod.rs
- [ ] Create uart/reader.rs (refactor từ vgateway)
- [ ] Create uart/writer.rs
- [ ] Create commands.rs
- [ ] Setup channel infrastructure in main.rs
- [ ] Compile check
- [ ] Unit test command parsing

## Success Criteria

- [ ] Project compiles
- [ ] Config loads from UCI `/etc/config/ugate`
- [ ] UCI wrapper works
- [ ] UART reader starts without crash

## Next Phase

Phase 2a: MQTT + HTTP Channels
