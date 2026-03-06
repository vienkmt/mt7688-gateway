# Phase 1: Core Infrastructure

**Priority:** High
**Status:** pending
**Effort:** 2 days
**Depends on:** Phase 0 (axum test pass)

## Context

Copy và refactor core modules từ vgateway. Đây là foundation cho tất cả phases sau.

## Objective

Setup project structure với:
- Config management (AppState, Config structs)
- UCI wrapper
- UART reader (AsyncFd + epoll)
- Channel infrastructure

## Module Structure

```
ugate/src/
├── main.rs
├── config.rs      # AppState, Config (from vgateway)
├── uci.rs         # UCI wrapper (from vgateway)
├── uart/
│   ├── mod.rs
│   ├── reader.rs  # AsyncFd + epoll
│   └── writer.rs  # TX to MCU
└── commands.rs    # Command parser
```

## Implementation Steps

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

### 3. Create uart/reader.rs

Copy từ vgateway/uart_reader.rs, refactor:
- Extract to separate module
- Add command detection (GPIO commands)
- Return parsed frames

```rust
pub enum UartFrame {
    Data(String),           // Regular data
    GpioCommand(u8, bool),  // GPIO pin, state
}

pub async fn run(
    state: Arc<AppState>,
    data_tx: broadcast::Sender<String>,
    gpio_tx: mpsc::Sender<GpioCommand>,
) { ... }
```

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
tokio = { version = "1", features = ["rt", "net", "io-util", "sync", "time", "fs"] }
serde = { version = "1", features = ["derive"] }
toml = "0.8"
```

## Todo

- [ ] Copy config.rs từ vgateway
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
- [ ] Config loads from /etc/ugate.toml
- [ ] UCI wrapper works
- [ ] UART reader starts without crash

## Next Phase

Phase 2: Channels (MQTT, HTTP, TCP)
