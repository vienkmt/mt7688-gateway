# Phase 2: Web UI Checkbox

## Overview
- **Priority:** High
- **Status:** TODO
- **Effort:** Small
- **Depends on:** Phase 1

## Related Files
- `ugate/src/embedded_index.html` — General card (line ~192)

## Implementation Steps

### 1. Thêm checkbox vào General card
- Trong HTML template, tìm General card section
- Thêm checkbox sau input `device_name`:
  ```javascript
  chk('Wrap JSON', c.general, 'wrap_json')
  ```
- Kiểm tra helper `chk()` đã tồn tại trong template (dùng cho MQTT enabled, HTTP enabled, etc.)

### 2. Verify save/load
- Checkbox value bind tới `config.general.wrap_json`
- Save gửi JSON `{"general":{"wrap_json":true/false,...}}`
- Config parse từ JSON body đã handle boolean

## UI Layout
```
┌─ General ─────────────────────┐
│ Tên thiết bị: [ugate-001   ] │
│ ☐ Wrap JSON                   │
└───────────────────────────────┘
```

## Success Criteria
- [ ] Checkbox hiển thị trong General card
- [ ] Check/uncheck lưu đúng vào UCI config
- [ ] Reload page hiển thị đúng trạng thái
