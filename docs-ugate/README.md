# ugate - IoT Gateway Firmware

**Mục đích:** Gateway nhúng chạy trên OpenWrt MT7688, kết nối thiết bị UART với hạ tầng IoT qua MQTT/HTTP/TCP

## Đặc tính

| Thành phần | Mô tả |
|-----------|--------|
| **Nền tảng** | MediaTek MT7628DAN (MIPS 24KEc 580MHz) |
| **RAM** | 64MB DDR2 |
| **Bộ nhớ** | 16MB SPI-NOR Flash |
| **Hệ điều hành** | OpenWrt 24.10 (Kernel 6.6.x) |
| **Ngôn ngữ** | Rust (100% unsafe-free ngoài syscalls) |

## Chức năng chính

- **UART Reader:** Đọc dữ liệu từ MCU qua cổng serial (hỗ trợ frame detection: None/Frame/Modbus)
- **MQTT Publisher:** Gửi dữ liệu tới broker MQTT với TLS/auth tuỳ chọn
- **HTTP Publisher:** POST dữ liệu tới endpoint HTTP
- **TCP Server/Client:** Kết nối hai chiều với các thiết bị khác qua TCP
- **GPIO Controller:** Điều khiển các chân GPIO (LED heartbeat, relay control, v.v.)
- **Web UI:** Dashboard cấu hình qua trình duyệt (Vue.js, WebSocket real-time)
- **Session Management:** Bảo vệ API bằng login/cookie

## Cấu trúc dự án

```
ugate/
├── src/
│   ├── main.rs              # Entry point, khởi chạy tất cả tasks
│   ├── config.rs            # Quản lý config UCI, hot-reload
│   ├── uart/                # UART reader + frame detection
│   ├── channels/            # MQTT, HTTP, TCP publishers
│   ├── gpio.rs              # GPIO control via ioctl
│   ├── web/                 # HTTP server, WebSocket, auth
│   ├── commands.rs          # Command definitions (GPIO, UART)
│   ├── time_sync.rs         # Đồng bộ hệ thống
│   └── uci.rs               # OpenWrt config interface
├── Cargo.toml               # Dependencies
└── embedded_index.html      # Web UI

Config: /etc/config/ugate    # UCI config file
Binary: /usr/bin/ugate       # Deploy target
Init script: /etc/init.d/ugate # procd (auto-generated)
```

## Build

**Yêu cầu:**
- `cargo` + `cross` (`cargo install cross`)
- `nightly` toolchain

```bash
# Build ugate
cross +nightly build --target mipsel-unknown-linux-musl --release -p ugate

# Binary: target/mipsel-unknown-linux-musl/release/ugate (~500KB)
```

## Deploy

**Cách 1: Deploy script (tự động)**
```bash
./deploy.sh              # Deploy ugate (mặc định)
./deploy.sh ugate        # Deploy ugate tường minh
./deploy.sh vgateway     # Deploy vgateway
./deploy.sh --build-only # Chỉ build
./deploy.sh --skip-build # Chỉ deploy
```

**Cách 2: Thủ công**
```bash
# Copy binary
scp target/mipsel-unknown-linux-musl/release/ugate root@<device>:/usr/bin/

# Chạy (procd sẽ start tự động)
ssh root@<device> "/etc/init.d/ugate restart"
```

## Cấu hình

**File:** `/etc/config/ugate` (UCI format)

```ini
config general
    option device_name 'ugate'
    option interval_secs '3'

config uart
    option enabled '1'
    option port '/dev/ttyS1'
    option baudrate '115200'
    option frame_mode 'modbus'      # none | frame | modbus

config mqtt
    option enabled '1'
    option broker 'broker.emqx.io'
    option port '8883'
    option tls '1'
    option topic 'ugate/data'
    option qos '1'

config http
    option enabled '0'
    option url 'http://example.com/api/data'
    option method 'post'

config tcp
    option enabled '0'
    option mode 'server'            # server | client | both
    option server_port '9000'

config gpio
    option led_pin '44'
    option pins '17 18'             # Control pins (space-separated)

config web
    option port '8888'
    option password 'admin'
    option max_ws_connections '4'
```

## Truy cập

**Web UI:** `http://<device-ip>:8888`
- **Login:** Password từ config (mặc định: `admin`)
- **Tabs:** Status | Config | UART | Data
- **Real-time:** WebSocket (status mỗi 1s, UART data live)

## Logs

```bash
# Xem log (syslog hoặc stderr)
ssh root@<device> logread | grep ugate

# Debug: SSH và chạy trực tiếp
ssh root@<device> /usr/bin/ugate
```

## Tài liệu chi tiết

- [**Architecture**](./architecture.md) — Async runtime, channels, tasks
- [**Configuration**](./config.md) — UCI config fields, defaults
- [**Web UI**](./web-ui.md) — API endpoints, WebSocket, login
- [**Deployment**](./deployment.md) — Cross-compile, init script, troubleshooting
- [**Troubleshooting**](./troubleshooting.md) — Common issues, known bugs

## License

Proprietary — MT7688 Gateway Project
