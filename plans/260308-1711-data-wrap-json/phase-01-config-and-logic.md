# Phase 1: Config + UCI + Fan-out Logic

## Overview
- **Priority:** High
- **Status:** TODO
- **Effort:** Small

## Related Files
- `ugate/src/config.rs` — GeneralConfig struct, load/save UCI
- `ugate/src/main.rs` — Fan-out task (lines 173-187)
- `ugate/src/channels/http_pub.rs` — HTTP publisher (detect wrap vs raw)
- `ugate/src/channels/mqtt.rs` — MQTT publisher (send as-is)

## Implementation Steps

### 1. Config struct (`config.rs`)
- Thêm `wrap_json: bool` vào `GeneralConfig`
- `load()`: đọc `uci_section_get("general", "wrap_json", "0")`, parse "1" → true
- `save_to_uci()`: `uci_set("general", "wrap_json", if wrap_json {"1"} else {"0"})`
- `to_json()`: thêm `"wrap_json":true/false` vào JSON output

### 2. Fan-out task (`main.rs`)
- Trong fan-out loop, kiểm tra `config.general.wrap_json`
- Nếu ON: tạo JSON string `{"device_name":"...","timestamp":...,"data":"hex"}`
  - `device_name` từ config
  - `timestamp` = `std::time::SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs()`
  - `data` = hex encode raw bytes
- Convert JSON string → `Vec<u8>` rồi gửi vào channels
- Nếu OFF: gửi raw bytes như hiện tại

### 3. HTTP publisher (`http_pub.rs`)
- Hiện tại POST body: `{"data":"hex","len":N}`
- Khi nhận wrapped JSON (detect bằng check byte đầu là `{` hoặc thêm enum/flag):
  - Gửi trực tiếp wrapped JSON, không wrap lại
- Khi nhận raw bytes: giữ nguyên logic cũ
- **Approach đơn giản:** Thêm 1 field `wrap_json: bool` vào HTTP task config, check trước khi format

### 4. MQTT publisher (`mqtt.rs`)
- Không cần thay đổi logic — `client.publish()` nhận `&[u8]`, wrapped JSON bytes sẽ tự nhiên là valid

## Key Decisions
- Wrap tại fan-out (không phải tại mỗi publisher) → DRY
- HTTP publisher cần detect để tránh double-wrap
- MQTT gửi as-is, không cần thay đổi

## Success Criteria
- [ ] `wrap_json` config load/save/to_json hoạt động
- [ ] Fan-out wrap data khi enabled
- [ ] HTTP publisher gửi wrapped JSON trực tiếp khi wrap_json ON
- [ ] MQTT publisher gửi wrapped JSON bytes khi wrap_json ON
- [ ] Compile thành công cho MIPS target
