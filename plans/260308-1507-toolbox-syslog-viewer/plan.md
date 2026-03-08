# Syslog Viewer - Tích hợp vào tab Toolbox

## Tổng quan
Thêm phần **Syslog Viewer** vào tab Toolbox, hiển thị log `logread | grep ugate` dạng streaming real-time qua WebSocket. Chỉ lấy thông tin hữu ích (bỏ phần trùng với serial UART), giới hạn 200 dòng, có filter.

## Trạng thái
- [ ] Phase 1: Backend - API + WS streaming

## Kiến trúc

### Flow
```
[OpenWrt logread -f] → pipe stdout → thread đọc line-by-line
  → parse + filter (bỏ UART raw data) → WS broadcast {"type":"syslog","line":"..."}
  → Frontend nhận → append vào log viewer (max 200 dòng)
```

### Backend (toolbox.rs hoặc file mới syslog.rs)
- **Endpoint:** `POST /api/syslog/start` — start streaming logread
- **Endpoint:** `POST /api/syslog/stop` — kill logread process
- **Process:** `logread -f -e ugate` (follow + filter by "ugate")
- **WS type:** `{"type":"syslog","line":"[Time] Clock synced: ..."}`
- **Parse mỗi dòng:** Bỏ prefix timestamp + daemon.info/warn + PID, chỉ giữ message
  - Input: `Sun Mar  8 14:53:18 2026 daemon.info ugate[2136]: [Config] Loaded: ...`
  - Output: `14:53:18 [Config] Loaded: ...` (giữ time ngắn + message)
- **Filter server-side:** Bỏ các dòng trùng với serial stream:
  - Bỏ `[Dispatch] UART TX:` (đã thấy trên UART tab)
  - Bỏ `[TCP] Nhận ... bytes` nếu chỉ là echo
  - Giữ: startup, connection, error, config, MQTT, HTTP, GPIO, WS events
- **Max 200 dòng** buffer, không cần atomic RUNNING vì syslog chạy song song với toolbox

### Frontend (embedded_index.html)
- Thêm card "System Log" bên dưới card "Network Diagnostics" trong renderToolbox()
- **State:** `S.syslog = {lines:[], running:false, filter:''}`
- **UI:**
  - Input filter (client-side grep trên lines đã nhận)
  - Toggle Start/Stop button
  - Clear button
  - Stream div `.stream` giống toolbox, max 200 dòng
  - Highlight: warn/error → màu vàng/đỏ
- **WS handler:** Thêm case `d.type==='syslog'` trong ws.onmessage

### Chi tiết parse log line
```rust
// Input: "Sun Mar  8 14:53:18 2026 daemon.info ugate[2136]: [Config] Loaded: ..."
// Tìm "ugate[" → skip tới ": " → lấy message
// Trích time từ đầu dòng (HH:MM:SS)
// Output: "14:53:18 [Config] Loaded: ..."

// Dòng warn: "daemon.warn" → prefix ⚠
// Dòng err: "daemon.err" → prefix ✕
```

### Lọc server-side (bỏ noise)
```rust
const SKIP_PREFIXES: &[&str] = &[
    "[Dispatch] UART TX:",    // đã thấy trên UART tab
    "[Dispatch] UART RX:",    // đã thấy trên UART tab
];
```
Chỉ skip các prefix trên. Giữ tất cả còn lại vì chúng hữu ích.

## Files cần sửa
1. `ugate/src/web/server.rs` — thêm 2 route `/api/syslog/start`, `/api/syslog/stop`
2. `ugate/src/web/toolbox.rs` — thêm syslog handling (hoặc tạo `syslog.rs` nếu >100 dòng)
3. `ugate/src/embedded_index.html` — thêm UI + WS handler

## Quyết định thiết kế
- **Tách riêng với toolbox RUNNING** — syslog chạy song song, không block ping/traceroute
- **`logread -f -e ugate`** — OpenWrt built-in, follow mode, filter by process name
- **Parse server-side** — giảm data gửi qua WS, frontend nhẹ hơn
- **Client-side filter** — user gõ keyword, JS filter trên lines đã nhận (không gửi lại server)
- **Max 200 dòng** — tránh lag trên browser mobile

## Risk
- `logread -f` process cần được kill khi WS disconnect → dùng `child.kill()` khi stop
- Nếu không có log mới → WS idle, không vấn đề gì
- Binary size: thêm ~2-3KB code, chấp nhận được
