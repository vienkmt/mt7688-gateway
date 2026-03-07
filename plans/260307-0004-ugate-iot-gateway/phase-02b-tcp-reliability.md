# Phase 2b: TCP + Reliability

**Priority:** High
**Status:** pending
**Effort:** 2 days
**Depends on:** Phase 2a

## Objective

1. TCP Server + Client channels (bidirectional)
2. Reliability features: OfflineBuffer, Reconnector, Graceful shutdown

## Module Structure

```
ugate/src/channels/
├── mod.rs          # Update
├── tcp.rs          # NEW
├── buffer.rs       # NEW: OfflineBuffer
└── reconnect.rs    # NEW: Reconnector
```

## Implementation

### 1. channels/tcp.rs

```rust
pub enum TcpMode { Server, Client, Both }

// TCP Server - listen for connections
pub async fn run_tcp_server(
    state: Arc<AppState>,
    mut data_rx: tokio::sync::mpsc::Receiver<String>,
    cmd_tx: tokio::sync::mpsc::Sender<Command>,
) {
    let config = state.get();
    if !config.tcp.enabled || config.tcp.mode == TcpMode::Client {
        return;
    }

    let listener = TcpListener::bind(format!("0.0.0.0:{}", config.tcp.server_port))
        .await.expect("TCP bind failed");

    loop {
        let (socket, _addr) = listener.accept().await.unwrap();
        let cmd_tx = cmd_tx.clone();
        tokio::spawn(handle_tcp_connection(socket, cmd_tx));
    }
}

// TCP Client - connect to remote server
pub async fn run_tcp_client(
    state: Arc<AppState>,
    mut data_rx: tokio::sync::mpsc::Receiver<String>,
    cmd_tx: tokio::sync::mpsc::Sender<Command>,
) {
    let mut reconnector = Reconnector::new(Duration::from_secs(1), Duration::from_secs(60));

    loop {
        let config = state.get();
        if !config.tcp.enabled || config.tcp.mode == TcpMode::Server {
            tokio::time::sleep(Duration::from_secs(5)).await;
            continue;
        }

        let addr = format!("{}:{}", config.tcp.client_host, config.tcp.client_port);
        match TcpStream::connect(&addr).await {
            Ok(stream) => {
                reconnector.reset();
                handle_tcp_connection(stream, cmd_tx.clone()).await;
            }
            Err(_) => {
                if let Some(delay) = reconnector.next() {
                    tokio::time::sleep(delay).await;
                }
            }
        }
    }
}

async fn handle_tcp_connection(
    stream: TcpStream,
    cmd_tx: tokio::sync::mpsc::Sender<Command>,
) {
    let (reader, _writer) = stream.into_split();
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

### 2. channels/buffer.rs (OfflineBuffer)

```rust
use std::collections::VecDeque;
use std::path::PathBuf;

pub struct OfflineBuffer {
    ram_queue: VecDeque<String>,
    ram_limit: usize,           // 1000 messages
    disk_path: PathBuf,         // /tmp/ugate_buffer/
    disk_limit: usize,          // 10000 messages
}

impl OfflineBuffer {
    pub fn new(ram_limit: usize, disk_path: PathBuf) -> Self {
        Self {
            ram_queue: VecDeque::new(),
            ram_limit,
            disk_path,
            disk_limit: 10000,
        }
    }

    pub fn push(&mut self, msg: String) {
        if self.ram_queue.len() < self.ram_limit {
            self.ram_queue.push_back(msg);
        } else {
            self.write_to_disk(&msg);
        }
    }

    pub fn pop(&mut self) -> Option<String> {
        self.ram_queue.pop_front().or_else(|| self.read_from_disk())
    }

    pub fn len(&self) -> usize {
        self.ram_queue.len()
    }

    fn write_to_disk(&self, msg: &str) {
        let path = self.disk_path.join("buffer.jsonl");
        if let Ok(mut file) = std::fs::OpenOptions::new()
            .create(true).append(true).open(&path) {
            let _ = writeln!(file, "{}", msg);
        }
    }

    fn read_from_disk(&mut self) -> Option<String> {
        // Read first line, rewrite rest
        let path = self.disk_path.join("buffer.jsonl");
        // ... implementation
        None
    }

    pub fn flush_to_disk(&mut self) {
        while let Some(msg) = self.ram_queue.pop_front() {
            self.write_to_disk(&msg);
        }
    }

    pub fn load_from_disk(&mut self) -> usize {
        // Load saved messages on startup
        0
    }
}
```

### 3. channels/reconnect.rs

```rust
use std::time::Duration;

pub struct Reconnector {
    delay: Duration,
    min_delay: Duration,
    max_delay: Duration,
    attempts: u32,
}

impl Reconnector {
    pub fn new(min: Duration, max: Duration) -> Self {
        Self {
            delay: min,
            min_delay: min,
            max_delay: max,
            attempts: 0,
        }
    }

    pub fn next(&mut self) -> Option<Duration> {
        self.attempts += 1;
        let current = self.delay;
        self.delay = (self.delay * 2).min(self.max_delay);
        Some(current)
    }

    pub fn reset(&mut self) {
        self.delay = self.min_delay;
        self.attempts = 0;
    }

    pub fn attempts(&self) -> u32 {
        self.attempts
    }
}
```

### 4. Graceful Shutdown

```rust
// main.rs
use tokio::signal;

async fn shutdown_signal() {
    let _ = signal::ctrl_c().await;
}

#[tokio::main(flavor = "current_thread")]
async fn main() {
    // ... setup ...

    tokio::select! {
        _ = run_main_loop() => {},
        _ = shutdown_signal() => {
            log::info!("Shutting down...");
            buffer.flush_to_disk();
            log::info!("Buffer flushed, exiting");
        }
    }
}
```

### 5. Update channels/mod.rs

```rust
pub mod mqtt;
pub mod http_pub;
pub mod tcp;
pub mod buffer;
pub mod reconnect;

pub use mqtt::run_mqtt;
pub use http_pub::run_http_publisher;
pub use tcp::{run_tcp_server, run_tcp_client};
pub use buffer::OfflineBuffer;
pub use reconnect::Reconnector;
```

## UCI Config

```
config tcp
    option enabled '1'
    option mode 'both'          # server, client, both
    option server_port '9000'
    option client_host '192.168.1.100'
    option client_port '9001'

config reliability
    option buffer_ram_limit '1000'
    option buffer_disk_path '/tmp/ugate_buffer'
    option reconnect_min '1'
    option reconnect_max '60'
```

## Files to Create

| File | Action |
|------|--------|
| ugate/src/channels/tcp.rs | Create |
| ugate/src/channels/buffer.rs | Create |
| ugate/src/channels/reconnect.rs | Create |
| ugate/src/channels/mod.rs | Update |
| ugate/src/main.rs | Update - graceful shutdown |

## Todo

- [ ] Create tcp.rs (Server + Client)
- [ ] Create buffer.rs (OfflineBuffer)
- [ ] Create reconnect.rs (Reconnector)
- [ ] Update channels/mod.rs
- [ ] Add graceful shutdown to main.rs
- [ ] Add startup recovery (load from disk)
- [ ] Test TCP Server accept
- [ ] Test TCP Client connect + reconnect
- [ ] Test OfflineBuffer RAM → disk overflow
- [ ] Test graceful shutdown (buffer flush)
- [ ] Test startup recovery

## Success Criteria

- [ ] TCP Server accepts connections
- [ ] TCP Client connects with reconnect
- [ ] Reconnect uses exponential backoff
- [ ] Buffer stores messages when offline
- [ ] Buffer overflows to disk
- [ ] Graceful shutdown flushes buffer
- [ ] Startup loads from disk

## Next Phase

Phase 3: Web Server + WebSocket
