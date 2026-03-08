# Data Wrap JSON Feature

## Summary
Thêm tùy chọn wrap UART raw data thành JSON với metadata trước khi publish lên MQTT/HTTP.

- **Mặc định:** OFF (gửi raw data như hiện tại)
- **Khi ON:** `{"device_name":"ugate-001","timestamp":1709876543,"data":"<hex>"}`

## Phases

| # | Phase | Status | Effort |
|---|-------|--------|--------|
| 1 | Config + UCI + Fan-out logic | TODO | Small |
| 2 | Web UI checkbox | TODO | Small |

## Architecture

```
UART Reader → Broadcast → Fan-out task
                            ├─ wrap_json ON?  → JSON wrap → MQTT/HTTP
                            └─ wrap_json OFF? → raw bytes → MQTT/HTTP
```

**Wrap point:** Fan-out task trong `main.rs` (lines 173-187) — nơi data được clone và gửi vào MQTT/HTTP channels. Wrap tại đây để cả 2 channel đều nhận cùng format.

**JSON format khi wrap:**
```json
{"device_name":"ugate-001","timestamp":1709876543,"data":"0a1b2c3d"}
```
- `data` = hex-encoded raw bytes
- `timestamp` = Unix epoch seconds
- `device_name` = từ config

## Files to Modify

| File | Change |
|------|--------|
| `ugate/src/config.rs` | Thêm `wrap_json: bool` vào `GeneralConfig`, load/save UCI |
| `ugate/src/main.rs` | Fan-out task: wrap data nếu `wrap_json` enabled |
| `ugate/src/channels/mqtt.rs` | Gửi `Vec<u8>` as-is (đã wrap hoặc raw) |
| `ugate/src/channels/http_pub.rs` | Detect wrapped JSON vs raw, adjust Content-Type |
| `ugate/src/embedded_index.html` | Checkbox trong General card |

## Risk
- **Low risk** — thêm 1 config flag, logic đơn giản
- HTTP publisher hiện tạo JSON riêng (`{"data":"hex","len":N}`). Khi wrap_json ON, cần gửi wrapped JSON trực tiếp thay vì wrap lại lần nữa.
