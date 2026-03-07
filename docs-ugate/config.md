# ugate - Cấu hình

**File config:** `/etc/config/ugate` (UCI format)

Cấu hình được load tại startup từ UCI, hỗ trợ hot-reload qua Web API.

## Cấu trúc UCI

Mỗi **section** trong file config tương ứng với 1 cấu hình subsystem. Format:
```ini
config <section>
    option <key> '<value>'
    option <key2> '<value2>'
```

## Sections

### [general] - Tùy chọn chung

| Key | Kiểu | Default | Mô tả |
|-----|------|---------|--------|
| `device_name` | string | `ugate` | Tên thiết bị (hiển thị Web UI) |
| `interval_secs` | u64 | `3` | Interval đọc config (không dùng hiện tại) |

**Ví dụ:**
```ini
config general
    option device_name 'gateway-01'
    option interval_secs '3'
```

### [uart] - Cấu hình cổng UART

| Key | Kiểu | Default | Mô tả |
|-----|------|---------|--------|
| `enabled` | bool | `1` | Bật/tắt UART reader |
| `port` | string | `/dev/ttyS1` | Device path |
| `baudrate` | u32 | `115200` | Tốc độ baud |
| `data_bits` | u8 | `8` | Bit dữ liệu (7 or 8) |
| `parity` | enum | `none` | `none` \| `even` \| `odd` |
| `stop_bits` | u8 | `1` | Stop bits (1 or 2) |
| `frame_mode` | enum | `none` | `none` \| `frame` \| `modbus` |
| `frame_length` | u16 | `256` | Max frame size (bytes) |
| `frame_timeout_ms` | u16 | `50` | Timeout phát hiện frame (ms) |
| `gap_ms` | u16 | `20` | Thời gian gap giữa frames (ms) |

**Frame detection modes:**
- `none` — Không phát hiện, gửi byte khi có dữ liệu
- `frame` — Phát hiện frame by timeout/length
- `modbus` — Phát hiện Modbus RTU (CRC check, frame structure)

**Ví dụ Modbus RTU:**
```ini
config uart
    option enabled '1'
    option port '/dev/ttyS1'
    option baudrate '9600'
    option data_bits '8'
    option parity 'even'
    option stop_bits '1'
    option frame_mode 'modbus'
    option frame_length '256'
    option frame_timeout_ms '100'
    option gap_ms '30'
```

### [mqtt] - Kênh MQTT Publisher

| Key | Kiểu | Default | Mô tả |
|-----|------|---------|--------|
| `enabled` | bool | `0` | Bật/tắt MQTT |
| `broker` | string | `broker.emqx.io` | MQTT broker hostname |
| `port` | u16 | `8883` | MQTT port (8883 TLS, 1883 plain) |
| `tls` | bool | `1` | Bật TLS/SSL |
| `topic` | string | `ugate/data` | Topic publish dữ liệu UART |
| `sub_topic` | string | `ugate/cmd` | Topic subscribe lệnh (GPIO) |
| `client_id` | string | `ugate-01` | MQTT client ID |
| `username` | string | (empty) | Username (optional) |
| `password` | string | (empty) | Password (optional) |
| `qos` | u8 | `1` | QoS level (0, 1, 2) |

**Ví dụ với TLS + auth:**
```ini
config mqtt
    option enabled '1'
    option broker 'mqtt.example.com'
    option port '8883'
    option tls '1'
    option topic 'devices/gateway-01/data'
    option sub_topic 'devices/gateway-01/cmd'
    option client_id 'gateway-01'
    option username 'myuser'
    option password 'mypass'
    option qos '1'
```

**Subscribe payload (cmd):**
Gửi JSON hoặc raw bytes tới `sub_topic`:
```json
{"pin": 17, "value": 1}  // SetPin command
{"pin": 17}              // Toggle command
```

### [http] - Kênh HTTP POST

| Key | Kiểu | Default | Mô tả |
|-----|------|---------|--------|
| `enabled` | bool | `0` | Bật/tắt HTTP publisher |
| `url` | string | (empty) | HTTP endpoint (http://...) |
| `method` | enum | `post` | `post` \| `get` |

**Ví dụ:**
```ini
config http
    option enabled '1'
    option url 'http://api.example.com/gateway/data'
    option method 'post'
```

**Behavior:**
- Dữ liệu UART gửi raw bytes trong request body (POST)
- Lỗi network → drop message (lossy)

### [tcp] - Kênh TCP Relay

| Key | Kiểu | Default | Mô tả |
|-----|------|---------|--------|
| `enabled` | bool | `0` | Bật/tắt TCP |
| `mode` | enum | `server` | `server` \| `client` \| `both` |
| `server_port` | u16 | `9000` | TCP server listening port |
| `client_host` | string | (empty) | TCP client remote host |
| `client_port` | u16 | `9000` | TCP client remote port |

**Ví dụ Server mode:**
```ini
config tcp
    option enabled '1'
    option mode 'server'
    option server_port '9000'
```

**Ví dụ Client mode:**
```ini
config tcp
    option enabled '1'
    option mode 'client'
    option client_host '192.168.1.100'
    option client_port '5000'
```

**Ví dụ Both mode:**
```ini
config tcp
    option enabled '1'
    option mode 'both'
    option server_port '9000'
    option client_host '10.0.0.5'
    option client_port '5000'
```

### [gpio] - Điều khiển GPIO

| Key | Kiểu | Default | Mô tả |
|-----|------|---------|--------|
| `led_pin` | u8 | `44` | Chân LED heartbeat |
| `pins` | string | (empty) | Danh sách chân điều khiển (space-separated) |

**Ví dụ:**
```ini
config gpio
    option led_pin '44'
    option pins '17 18 23'
```

**Pin control via Web API:**
```
POST /api/gpio/{pin}
Body: {"value": 1}  or  {"action": "toggle"}
```

### [web] - Cấu hình Web Server

| Key | Kiểu | Default | Mô tả |
|-----|------|---------|--------|
| `port` | u16 | `8888` | HTTP server port |
| `password` | string | `admin` | Login password |
| `max_ws_connections` | u8 | `4` | Max WebSocket clients |

**Ví dụ:**
```ini
config web
    option port '8888'
    option password 'securepass123'
    option max_ws_connections '8'
```

## Complete Example Config

```ini
config general
    option device_name 'ugate-01'
    option interval_secs '3'

config uart
    option enabled '1'
    option port '/dev/ttyS1'
    option baudrate '115200'
    option data_bits '8'
    option parity 'none'
    option stop_bits '1'
    option frame_mode 'modbus'
    option frame_length '256'
    option frame_timeout_ms '50'
    option gap_ms '20'

config mqtt
    option enabled '1'
    option broker 'broker.emqx.io'
    option port '8883'
    option tls '1'
    option topic 'gateways/ugate-01/data'
    option sub_topic 'gateways/ugate-01/cmd'
    option client_id 'ugate-01'
    option qos '1'

config http
    option enabled '0'
    option url ''
    option method 'post'

config tcp
    option enabled '0'
    option mode 'server'
    option server_port '9000'

config gpio
    option led_pin '44'
    option pins '17 18'

config web
    option port '8888'
    option password 'admin'
    option max_ws_connections '4'
```

## Loading & Hot-Reload

**Startup:**
1. Read `/etc/config/ugate` via UCI
2. Apply defaults for missing fields
3. Log config summary

**Hot-reload (Web API):**
```bash
POST /api/config
Content-Type: application/json
{
    "mqtt": {"enabled": true, "broker": "new-broker.com", ...},
    "uart": {...},
    ...
}
```

**Behavior:**
- UART reader: reconnect to new port (if changed)
- MQTT: disconnect + reconnect with new creds
- HTTP/TCP: update endpoint, persist on next publish
- GPIO: update pin list immediately

## Validation Rules

| Field | Constraint |
|-------|-----------|
| `port` (UART) | Must exist and be TTY device |
| `baudrate` | 300-921600 |
| `data_bits` | 7 or 8 |
| `stop_bits` | 1 or 2 |
| `frame_length` | 1-4096 |
| `frame_timeout_ms` | 1-10000 |
| `mqtt.port` | 1-65535 |
| `mqtt.qos` | 0, 1, or 2 |
| `tcp.server_port` | 1-65535 |
| `web.port` | 1025-65535 (>1024) |
| `web.max_ws_connections` | 1-64 |

## OpenWrt Integration

**Location:** `/etc/config/ugate`
**Owner:** `uci` tool

**Edit config:**
```bash
# Via uci
uci set ugate.@mqtt[0].enabled=1
uci set ugate.@mqtt[0].broker='new-broker.com'
uci commit ugate

# Restart service
/etc/init.d/ugate restart

# Or edit directly
vi /etc/config/ugate
# Restart service after edit
```

**Backup/Restore:**
```bash
# Backup
tar czf /tmp/ugate-backup.tar.gz /etc/config/ugate

# Restore
tar xzf /tmp/ugate-backup.tar.gz -C /
/etc/init.d/ugate restart
```
