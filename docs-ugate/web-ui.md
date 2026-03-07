# ugate - Web UI

**URL:** `http://<device-ip>:8888`

**Technology:** Vue.js (embedded HTML + WebSocket)

## Authentication

**Login Screen**

```
┌─────────────────────┐
│     ugate Login     │
│                     │
│ Password: [_____]   │
│  [Login Button]     │
└─────────────────────┘
```

**Process:**
1. User enters password from config (`web.password`)
2. POST `/api/login` with password hash
3. Server returns session cookie
4. Browser stores cookie → subsequent requests include it
5. Reload page → `/api/status` check (if 200 OK, auto-skip login)

**Session Management:**
- Cookie: `session=<token>` (server-side in-memory)
- Expires: When server restarts or 8 hours idle
- Protected: All `/api/*` routes except `/api/login`

## Dashboard Tabs

### Tab 1: Trạng thái (Status)

**Display:**
```
Device: ugate-01
Uptime: 12h 34m 10s
UART: ✓ Connected (ttyS1 @ 115200)
MQTT: ✓ Connected to broker.emqx.io:8883
HTTP: Disabled
TCP: ✓ Server listening 9000
GPIO: 3 pins configured (17, 18, 23)

Messages Processed:
  Total: 2,841
  MQTT published: 2,841
  HTTP posted: 0
  TCP relayed: 156
  Errors: 0
```

**Update:** WebSocket every 1 second
```json
{
  "type": "status",
  "device_name": "ugate-01",
  "uptime_secs": 45250,
  "uart_connected": true,
  "uart_port": "/dev/ttyS1",
  "uart_baudrate": 115200,
  "mqtt_connected": true,
  "mqtt_broker": "broker.emqx.io:8883",
  "http_enabled": false,
  "tcp_enabled": true,
  "tcp_mode": "server",
  "gpio_pins": [17, 18, 23],
  "stats": {
    "total_messages": 2841,
    "mqtt_published": 2841,
    "http_posted": 0,
    "tcp_relayed": 156,
    "errors": 0
  }
}
```

### Tab 2: Cấu hình (Config)

**Form Fields (Editable):**

**General Section:**
- Device Name: text input
- Interval: number input

**UART Section:**
- Enabled: checkbox
- Port: select dropdown (/dev/ttyS0, /dev/ttyS1, etc.)
- Baudrate: select (300, 1200, 9600, 19200, 38400, 57600, 115200)
- Data bits: select (7, 8)
- Parity: select (None, Even, Odd)
- Stop bits: select (1, 2)
- Frame Mode: select (None, Frame, Modbus)
- Frame Length: number (256)
- Frame Timeout (ms): number (50)
- Gap (ms): number (20)

**MQTT Section:**
- Enabled: checkbox
- Broker: text input
- Port: number (8883)
- TLS: checkbox
- Topic: text input
- Sub Topic: text input
- Client ID: text input
- Username: text input
- Password: password input
- QoS: select (0, 1, 2)

**HTTP Section:**
- Enabled: checkbox
- URL: text input
- Method: select (GET, POST)

**TCP Section:**
- Enabled: checkbox
- Mode: select (Server, Client, Both)
- Server Port: number (9000)
- Client Host: text input
- Client Port: number (9000)

**GPIO Section:**
- LED Pin: number (44)
- Control Pins: text input (space-separated, e.g., "17 18 23")

**Web Section:**
- Port: number (8888)
- Max WS Connections: number (4)
- Password: password input

**Action Buttons:**
```
[Save Config]  [Reset]  [Export JSON]  [Import JSON]
```

**Save Behavior:**
- POST `/api/config` with full config object
- On success → show "✓ Config saved"
- On error → show "✗ Error: {message}"
- Server restarts affected subsystems (MQTT reconnect, UART restart)

### Tab 3: UART (Live Data)

**Display:**

```
UART Data Monitor

Direction: RX / TX (toggle)
Format: Hex / ASCII / Mixed (radio buttons)

Frame Count: 1,234

┌────────────────────────────────────────┐
│ [RX] 2024-03-07 15:32:45.123           │
│ 01 03 00 00 00 10 44 07                │
│                                        │
│ [RX] 2024-03-07 15:32:46.245           │
│ 01 03 00 00 00 10 44 07                │
│ (ASCII: ...D.......)                   │
│                                        │
│ [CLEAR]  [COPY ALL]  [AUTO SCROLL ON]  │
└────────────────────────────────────────┘
```

**WebSocket Data Format:**
```json
{
  "type": "uart",
  "dir": "rx",
  "hex": "010300000010440700",
  "len": 9,
  "timestamp": 1704896565123
}
```

**Features:**
- Auto-scroll: Show latest frames at bottom
- Max buffer: 1000 frames (auto-clear old)
- Pause/Resume: Toggle auto-scroll
- Copy: Copy to clipboard
- Clear: Clear display

### Tab 4: Dữ liệu (Data Stats)

**Display:**

```
Data Flow Statistics

┌─ MQTT ────────────────┐
│ Enabled: Yes          │
│ Connected: Yes        │
│ Messages: 2,841       │
│ Bytes sent: 142 KB    │
│ Errors: 0             │
└───────────────────────┘

┌─ HTTP ────────────────┐
│ Enabled: No           │
│ URL: (empty)          │
│ Messages: 0           │
│ Bytes sent: 0 B      │
│ Errors: 0             │
└───────────────────────┘

┌─ TCP ────────────────┐
│ Enabled: Yes         │
│ Mode: Server         │
│ Clients: 2/4         │
│ Messages: 156        │
│ Bytes sent: 8.2 KB   │
│ Errors: 0            │
└──────────────────────┘
```

## GPIO Control

**Via Web UI (Config Tab):**
```
Control Pins: [17] [18] [23]

Pin 17: [OFF] [ON] [TOGGLE]
Pin 18: [OFF] [ON] [TOGGLE]
Pin 23: [OFF] [ON] [TOGGLE]

Status:
Pin 17: LOW (0.2V)
Pin 18: HIGH (3.3V)
Pin 23: LOW (0.2V)
```

**Via REST API:**
```bash
# Set pin 17 to HIGH
curl -X POST http://device:8888/api/gpio/17 \
  -H "Cookie: session=<token>" \
  -H "Content-Type: application/json" \
  -d '{"value": 1}'

# Response
{
  "pin": 17,
  "value": 1,
  "success": true
}

# Toggle pin 18
curl -X POST http://device:8888/api/gpio/18 \
  -H "Cookie: session=<token>" \
  -H "Content-Type: application/json" \
  -d '{"action": "toggle"}'

# Response
{
  "pin": 18,
  "value": 0,
  "success": true
}
```

## API Endpoints

### Authentication

**POST /api/login**
```json
Request:  {"password": "admin"}
Response: {"success": true, "session": "abc123xyz"}
```

### Status

**GET /api/status**
```json
Response: {
  "device_name": "ugate-01",
  "uptime_secs": 45250,
  "uart_connected": true,
  "mqtt_connected": true,
  "...": "..."
}
```
Auth: Not required (public)

### Configuration

**GET /api/config**
```json
Response: {
  "general": {...},
  "uart": {...},
  "mqtt": {...},
  "http": {...},
  "tcp": {...},
  "gpio": {...},
  "web": {...}
}
```
Auth: Required (cookie)

**POST /api/config**
```json
Request:  {
  "general": {...},
  "uart": {...},
  "mqtt": {...},
  "http": {...},
  "tcp": {...},
  "gpio": {...},
  "web": {...}
}
Response: {
  "success": true,
  "message": "Config updated, services restarting..."
}
```
Auth: Required (cookie)

### GPIO Control

**POST /api/gpio/{pin}**
```json
Request:  {"value": 1}  OR  {"action": "toggle"}
Response: {
  "pin": 17,
  "value": 1,
  "success": true
}
```
Auth: Required (cookie)

### Password Change

**POST /api/password**
```json
Request:  {
  "old_password": "admin",
  "new_password": "newsecure123"
}
Response: {
  "success": true,
  "message": "Password changed"
}
```
Auth: Required (cookie)

## WebSocket Connection

**URL:** `ws://device:8888/ws`

**Connection:**
```javascript
const ws = new WebSocket(`ws://${window.location.host}/ws`);

ws.onmessage = (event) => {
  const msg = JSON.parse(event.data);
  if (msg.type === 'status') {
    updateStatusTab(msg);
  } else if (msg.type === 'uart') {
    appendUartData(msg);
  }
};

ws.onclose = () => {
  console.log('WebSocket closed, retrying in 5s...');
  setTimeout(() => location.reload(), 5000);
};
```

**Message Types:**

1. **Status (every 1s)**
   ```json
   {"type": "status", "uptime_secs": 45250, ...}
   ```

2. **UART Data (live)**
   ```json
   {"type": "uart", "dir": "rx", "hex": "...", "len": 8}
   ```

## Responsive Design

- **Desktop (1024px+):** 4 tabs, full-width forms
- **Tablet (768px-1023px):** 4 tabs, scrollable forms
- **Mobile (< 768px):** Collapsible tabs, stacked inputs

## Session Timeout

- Idle 1 hour → auto-logout
- On logout → remove cookie
- On reload → check `/api/status` (if 401, show login)

## Performance

- **Page load:** ~500ms (fetch config + status)
- **WebSocket latency:** ~100-200ms (status updates)
- **UART data latency:** ~50-100ms (live to display)
- **API response:** ~50ms (config, GPIO control)
