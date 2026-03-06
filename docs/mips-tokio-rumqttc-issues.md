# MIPS + Tokio + rumqttc Issues

## Vấn đề

Khi chạy trên MT7688AN (MIPS 24KEc), rumqttc `AsyncClient` kết hợp với tokio runtime gặp các vấn đề:

### 1. eventloop.poll() không return
- **Triệu chứng**: `eventloop.poll().await` block vô hạn, không bao giờ return
- **Ảnh hưởng**: Các branch khác trong `tokio::select!` không được thực thi
- **Cả TLS và non-TLS** đều bị

### 2. tokio::sync::broadcast không hoạt động
- **Triệu chứng**: `Sender::send()` báo "2 receivers" nhưng `Receiver::recv()` không bao giờ ready
- **Nguyên nhân**: Waker registration có vấn đề trên MIPS

### 3. tokio::time::timeout không hoạt động với rumqttc
- **Triệu chứng**: `timeout(Duration::from_ms(100), eventloop.poll())` vẫn block
- **Nguyên nhân**: Có thể do cách rumqttc handle internal futures

## Giải pháp

### MQTT: Dùng std::thread + sync Client

```rust
// main.rs
let (mqtt_tx, mqtt_rx) = std::sync::mpsc::channel::<String>();
std::thread::spawn(move || {
    mqtt_publisher::run_sync(state, mqtt_rx);
});

// mqtt_publisher.rs
use rumqttc::{Client, MqttOptions, QoS};  // sync Client, không phải AsyncClient

pub fn run_sync(state: Arc<AppState>, uart_rx: std::sync::mpsc::Receiver<String>) {
    let (client, mut connection) = Client::new(opts, 10);

    // Spawn connection handler trong thread riêng
    std::thread::spawn(move || {
        for notification in connection.iter() {
            // handle events
        }
    });

    // Main loop với try_recv()
    loop {
        while let Ok(json) = uart_rx.try_recv() {
            client.publish(&topic, QoS::AtLeastOnce, false, json)?;
        }
        std::thread::sleep(Duration::from_millis(50));
    }
}
```

### UART → MQTT: Dùng std::sync::mpsc

```rust
// Cross-thread compatible
let (mqtt_tx, mqtt_rx) = std::sync::mpsc::channel::<String>();

// UART gửi (trong tokio task)
let _ = mqtt_tx.send(json);

// MQTT nhận (trong std::thread)
while let Ok(json) = uart_rx.try_recv() { ... }
```

## Kiến trúc hoạt động

```
┌────────────────────────────────────────────────┐
│              vgateway Process                   │
├────────────────────────────────────────────────┤
│  std::thread          │  tokio current_thread  │
│  ├── rumqttc Client   │  ├── UART (AsyncFd)    │
│  └── Connection iter  │  ├── HTTP publisher    │
│                       │  └── OLED display      │
└────────────────────────────────────────────────┘
        ↑ std::sync::mpsc
```

## Kết quả

| Metric | Giá trị |
|--------|---------|
| VmRSS | 3.6 MB |
| VmSize | 22.9 MB |
| Threads | 2 (main tokio + MQTT) |
| UART → MQTT latency | ~50ms |

## 4. AsyncFd (epoll) gây CPU cao với partial data

### Triệu chứng
CPU 85% dù chỉ xử lý 1 message/giây từ UART

### Nguyên nhân chi tiết

```
MCU gửi: "xxxuuuuuu\n" (10 bytes)

Timeline:
  t=0ms:   MCU bắt đầu gửi byte đầu tiên 'x'
  t=1ms:   Kernel nhận 'x', epoll báo fd readable
  t=1ms:   App đọc 'x', không có '\n' → return None
  t=1ms:   App loop lại, gọi async_fd.readable()
  t=1ms:   Kernel vẫn có data 'x' trong buffer → epoll báo readable NGAY LẬP TỨC
  t=1ms:   App đọc, vẫn không có '\n' → return None
  ... lặp liên tục cho đến khi nhận đủ '\n' ...
  t=100ms: MCU gửi xong '\n'
  t=100ms: App đọc line hoàn chỉnh
```

**Vấn đề**: Trong 100ms chờ MCU gửi xong, app loop hàng ngàn lần vì:
- epoll level-triggered: báo readable khi buffer có data
- App không sleep → busy loop 100% CPU

### Giải pháp

```rust
match guard.try_io(|inner| read_line_nonblocking(...)) {
    Ok(Ok(Some(line))) => {
        // Có line hoàn chỉnh → xử lý
    }
    Ok(Ok(None)) => {
        // Có data nhưng chưa có '\n'
        // Sleep để chờ MCU gửi tiếp, tránh busy loop
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    Err(_would_block) => {
        // Không có data → epoll sẽ tự chờ
    }
}
```

### Kết quả
- Trước: CPU 85%
- Sau: CPU 9%

## Lưu ý

- **Không dùng** `rumqttc::AsyncClient` trên MIPS
- **Không dùng** `tokio::sync::broadcast` cho cross-task communication
- **Dùng** `std::thread` + `std::sync::mpsc` cho MQTT
- **Giữ** tokio `current_thread` cho I/O async (UART, HTTP)
- **Thêm sleep** khi AsyncFd trả về partial data để tránh CPU cao
