# ugate - Kiến trúc hệ thống

## Tổng quan

ugate sử dụng **Tokio single-thread async runtime** với **message passing** để điều phối các kênh truyền dữ liệu.

```
┌──────────────────────────────────────────────────────┐
│              UART Reader (AsyncFd + epoll)           │
│  Broadcast: Vec<u8> (raw frames, max 64 messages)   │
└──────────────────┬──────────────────────────────────┘
                   │ broadcast::channel
        ┌──────────┴──────────┬──────────────┬──────────────┐
        │                    │              │              │
   MQTT Thread           HTTP Task       TCP Tasks      WebSocket
   (OS thread)          (spawn)          (spawn)        Broadcast
   (sync rumqttc)       (async)          (async)        (broadcast)
```

## Async Runtime

```rust
#[tokio::main(flavor = "current_thread")]
async fn main() { ... }
```

**Lý do:** MIPS 580MHz single-core → Work-stealing overhead không cần thiết.

**Executor:** epoll (Linux I/O multiplexing)
- Non-blocking socket reads (AsyncFd)
- Timer resolution: ms
- Memory footprint: nhỏ

## Channel Architecture

### 1. UART Broadcast Channel
```rust
let (uart_broadcast_tx, _) = tokio::sync::broadcast::channel::<Vec<u8>>(64);
```
- **Type:** Broadcast (1 producer, N subscribers)
- **Capacity:** 64 messages (8KB buffer per subscriber)
- **Subscribers:**
  - WebSocket clients (fan-out UART data)
  - MQTT publisher (send to broker)
  - HTTP publisher (POST data)
  - TCP clients (relay over network)

### 2. MQTT Data Channel
```rust
let (mqtt_tx, mqtt_rx) = std::sync::mpsc::channel::<Vec<u8>>();
```
- **Type:** Synchronous MPSC (thread-safe)
- **Reason:** MQTT runs on **OS thread** (sync rumqttc Client, not async)
- **Flow:** UART → MQTT thread → broker

### 3. HTTP POST Channel
```rust
let (http_tx, http_rx) = tokio::sync::mpsc::channel::<Vec<u8>>(64);
```
- **Type:** Async MPSC
- **Flow:** UART → HTTP task → spawn_blocking(ureq::post)

### 4. Command Channel
```rust
let (cmd_tx, cmd_rx) = tokio::sync::mpsc::channel::<commands::Command>(32);
```
- **Sources:** WebSocket, TCP server, MQTT sub_topic
- **Consumer:** GPIO controller task
- **Commands:** SetPin { pin, value }, Toggle, etc.

### 5. Config Notify
```rust
let (config_notify_tx, config_notify_rx) =
    std::sync::mpsc::channel::<()>();
```
- **Purpose:** Thông báo config thay đổi cho MQTT thread
- **Trigger:** Web API `/api/config` POST

## Task Topology

### Main Tasks (tokio::spawn)

| Task | Runtime | Purpose |
|------|---------|---------|
| UART Reader | async | Read UART, detect frames, broadcast |
| HTTP Publisher | async | Subscribe UART broadcast, spawn_blocking POST |
| TCP Server | async | Listen 0.0.0.0:9000, relay UART data |
| TCP Client | async | Connect remote host:port, relay UART data |
| GPIO Controller | async | Poll cmd_rx, control GPIO pins |
| Status Broadcaster | std::thread | Push status JSON every 1s |
| UART→WS Forwarder | async | UART broadcast → WS broadcast (hex format) |
| Config Subscriber | async | Hot-reload support (future) |

### Blocking Tasks (spawn_blocking)

| Task | Purpose |
|------|---------|
| HTTP Server | tiny-http (blocking I/O) |
| MQTT Publisher | OS thread (sync rumqttc) |
| WS Handler | Handle WebSocket upgrade (per-connection) |

## MQTT Architecture

**sync rumqttc** không hoạt động tốt trên MIPS async. **Giải pháp:**

```rust
std::thread::spawn(move || {
    channels::mqtt::run_sync(state, mqtt_rx, config_notify_rx);
    // Vòng lặp vô hạn:
    // 1. Check enabled
    // 2. Connect broker
    // 3. Recv from mqtt_rx → publish
    // 4. Poll config_notify_rx → reconnect if changed
});
```

**Key:** MQTT thread hoạt động độc lập, không block async executor.

## GPIO Controller

```rust
pub async fn run(
    config: GpioConfig,
    cmd_rx: mpsc::Receiver<Command>,
    stats: Arc<SharedStats>,
)
```

**I/O Method:** ioctl via `/dev/gpiochipX`
- **No libgpiod dependency** → easier cross-compile
- **Pure Rust implementation** using libc ioctl
- **Per-pin file descriptor** → low overhead

**Commands:**
- `SetPin { pin, value }` — Write GPIO state
- `Toggle { pin }` — Toggle state and return new value
- `GetPin { pin }` — Read current state

**Heartbeat LED:**
- Blink pin 44 every 2s (if configured)
- Indicates system alive

## WebSocket Architecture

**Constraint:** tiny-http `Request::upgrade()` returns `Box<dyn ReadWrite>` (cannot split).

**Solution:** Single-thread with read timeout strategy

```rust
pub fn handle_websocket<S>(stream: S, manager: Arc<WsManager>)
where
    S: std::io::Read + std::io::Write + Send + 'static,
{
    // Send broadcast messages + sleep (bypass read timeouts)
    // Client sends commands via HTTP API, not WebSocket
}
```

**Message Flow:**
- **Server → Client:** Status JSON (1s interval), UART hex data (live)
- **Client → Server:** HTTP POST `/api/gpio/` (not WebSocket)

## Data Flow Example

**Scenario:** MCU sends "Hello" via UART → broadcast to MQTT broker + HTTP endpoint

```
1. UART Reader
   ├─ Open /dev/ttyS1 (nonblocking)
   ├─ AsyncFd wraps fd
   ├─ epoll_wait() → readable
   └─ Read bytes → detect frame (frame_mode=none)

2. Broadcast UART data
   ├─ uart_broadcast_tx.send(vec![...])
   └─ All subscribers get copy

3. Fan-out Task
   ├─ Subscribe uart_broadcast
   ├─ Send to mqtt_tx (MPSC)
   └─ Send to http_tx (try_send, non-blocking)

4. MQTT Thread
   ├─ Recv from mqtt_rx
   ├─ rumqttc::client.publish()
   ├─ Retry on error
   └─ Wait for config change

5. HTTP Task
   ├─ Recv from http_rx
   ├─ spawn_blocking()
   ├─ ureq::post(url).send(data)
   └─ Log result
```

## Config Hot-Reload

**Trigger:** Web API `/api/config` POST new config

```rust
// AppState.update()
→ RwLock::write(new_config)
→ watch::send()  // Notify UART reader
→ mqtt_notify.send()  // Notify MQTT thread → reconnect
```

**MQTT Reconnect:**
```rust
// MQTT thread polls config_notify_rx every 2s
// On notify: break current connection → reconnect with new broker/creds
```

## Memory Layout

- **Text (code):** ~500KB (release, stripped)
- **Data (config):** ~4KB
- **Heap (runtime):** ~8MB (Tokio buffer pools, connections)
- **Total RSS:** ~12MB (leaves 52MB for other processes)

## Error Handling

| Component | Error | Behavior |
|-----------|-------|----------|
| UART Reader | Port not found | Log error, retry every 5-60s |
| MQTT | Connection failed | Log error, retry every 10s |
| HTTP | Network error | Drop message (lossy), continue |
| TCP | Bind failed | Log error, skip server |
| GPIO | ioctl error | Log warning, set stats counter |

## Resource Limits

| Resource | Value |
|----------|-------|
| Max WS connections | config.web.max_ws_connections (default 4) |
| Broadcast buffer | 64 messages (auto-skip lagged) |
| MQTT publish interval | 2s (sync client main loop) |
| Status broadcast | 1s (std::thread sleep) |
| GPIO heartbeat | 2s blink |

## Threading Model

```
┌─ Tokio Runtime (single-thread) ─────────────────┐
│  Tasks: UART reader, HTTP pub, TCP, GPIO, etc.  │
└────────────────────────────────────────────────┘
         ↑ (std::sync::mpsc)      ↑ (broadcast)
         │                         │
    ┌─ MQTT Thread ──────┐    ┌─ Status Thread ─────┐
    │ (OS thread)         │    │ (sleep 1s loop)     │
    │ sync rumqttc       │    │ broadcast status    │
    └────────────────────┘    └─────────────────────┘
         ↑ (Channel)           (Broadcast channel)
         │
    ┌─ HTTP Server ──────────────────────┐
    │ (spawn_blocking)                   │
    │ tiny-http + WebSocket upgrade      │
    └────────────────────────────────────┘
```

## Performance Notes

- **UART read latency:** < 1ms (epoll)
- **Broadcast latency:** < 1ms (in-memory channel)
- **MQTT publish:** ~200ms (network dependent)
- **CPU usage:** ~5-10% idle (epoll sleep)
- **Context switches:** Minimal (single thread)
