# Phase 2: Channels

**Priority:** High
**Status:** pending
**Effort:** 3 days
**Depends on:** Phase 1

## Context

Implement 3 outbound channels: MQTT, HTTP POST, TCP (Server + Client).
Copy patterns từ vgateway, extend cho TCP.

## Module Structure

```
ugate/src/channels/
├── mod.rs
├── mqtt.rs       # std::thread sync (from vgateway)
├── http_pub.rs   # tokio::spawn (from vgateway)
└── tcp.rs        # NEW: Server + Client modes
```

## Implementation Steps

### 1. Create channels/mod.rs

```rust
pub mod mqtt;
pub mod http_pub;
pub mod tcp;

pub use mqtt::run_mqtt;
pub use http_pub::run_http_publisher;
pub use tcp::{run_tcp_server, run_tcp_client};
```

### 2. Copy & adapt channels/mqtt.rs

From vgateway/mqtt_publisher.rs:
- Same pattern: std::thread::spawn + rumqttc sync Client
- Add command subscription (receive from MQTT → cmd_tx)

```rust
pub fn run_sync(
    state: Arc<AppState>,
    data_rx: std::sync::mpsc::Receiver<String>,
    config_notify_rx: std::sync::mpsc::Receiver<()>,
    cmd_tx: std::sync::mpsc::Sender<Command>,  // NEW: for incoming commands
) { ... }
```

### 3. Copy & adapt channels/http_pub.rs

From vgateway/http_publisher.rs:
- Same pattern: tokio::spawn + spawn_blocking(ureq)
- No changes needed

### 4. Create channels/tcp.rs (NEW)

TCP Server + Client với bidirectional support:

```rust
pub enum TcpMode {
    Server,
    Client,
    Both,
}

// TCP Server - listen for connections
pub async fn run_tcp_server(
    state: Arc<AppState>,
    data_rx: mpsc::Receiver<String>,
    cmd_tx: mpsc::Sender<Command>,
) {
    let config = state.get();
    if !config.tcp.enabled || config.tcp.mode == TcpMode::Client {
        return;
    }

    let listener = TcpListener::bind(format!("0.0.0.0:{}", config.tcp.server_port))
        .await.unwrap();

    loop {
        let (socket, addr) = listener.accept().await.unwrap();
        let cmd_tx = cmd_tx.clone();
        tokio::spawn(handle_tcp_client(socket, cmd_tx));
    }
}

// TCP Client - connect to remote server
pub async fn run_tcp_client(
    state: Arc<AppState>,
    data_rx: mpsc::Receiver<String>,
    cmd_tx: mpsc::Sender<Command>,
) {
    let config = state.get();
    if !config.tcp.enabled || config.tcp.mode == TcpMode::Server {
        return;
    }

    loop {
        match TcpStream::connect(&config.tcp.client_host).await {
            Ok(stream) => {
                handle_tcp_connection(stream, &data_rx, &cmd_tx).await;
            }
            Err(_) => {
                tokio::time::sleep(Duration::from_secs(5)).await; // Reconnect delay
            }
        }
    }
}

async fn handle_tcp_client(
    mut socket: TcpStream,
    cmd_tx: mpsc::Sender<Command>,
) {
    let (reader, writer) = socket.split();
    let mut reader = BufReader::new(reader);
    let mut line = String::new();

    while reader.read_line(&mut line).await.is_ok() {
        if line.is_empty() { break; }
        if let Some(cmd) = parse_json_command(&line) {
            let _ = cmd_tx.send(cmd).await;
        }
        line.clear();
    }
}
```

### 5. Wire up in main.rs

```rust
// MQTT in std::thread
let mqtt_state = state.clone();
std::thread::spawn(move || {
    channels::run_mqtt(mqtt_state, mqtt_rx, config_notify_rx, cmd_tx_sync);
});

// HTTP publisher
tokio::spawn(channels::run_http_publisher(state.clone(), http_rx));

// TCP Server
tokio::spawn(channels::run_tcp_server(state.clone(), tcp_rx.clone(), cmd_tx.clone()));

// TCP Client
tokio::spawn(channels::run_tcp_client(state.clone(), tcp_rx, cmd_tx.clone()));
```

## Files to Create/Modify

| File | Action | Source |
|------|--------|--------|
| ugate/src/channels/mod.rs | Create | New |
| ugate/src/channels/mqtt.rs | Create | Copy from vgateway |
| ugate/src/channels/http_pub.rs | Create | Copy from vgateway |
| ugate/src/channels/tcp.rs | Create | New |
| ugate/src/main.rs | Modify | Wire channels |
| ugate/Cargo.toml | Modify | Add rumqttc, ureq |

## Dependencies to Add

```toml
rumqttc = "0.24"
ureq = { version = "2", features = ["tls"] }
```

## Todo

- [ ] Create channels/mod.rs
- [ ] Copy mqtt.rs từ vgateway, add command subscription
- [ ] Copy http_pub.rs từ vgateway
- [ ] Create tcp.rs với Server + Client modes
- [ ] Wire channels in main.rs
- [ ] Test MQTT publish
- [ ] Test HTTP POST
- [ ] Test TCP Server accept
- [ ] Test TCP Client connect

## Success Criteria

- [ ] MQTT publishes UART data
- [ ] HTTP POSTs UART data
- [ ] TCP Server accepts connections
- [ ] TCP Client connects to remote
- [ ] Commands received from all channels

## Next Phase

Phase 3: Web Server + WebSocket
