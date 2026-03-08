# Phase 08 Analysis: System Maintenance

**Date:** 2026-03-08
**Type:** Plan review & gap analysis
**Phase:** plans/260307-0004-ugate-iot-gateway/phase-08-system-maintenance.md

---

## 1. Trạng thái hiện tại

- **maintenance.rs** chưa tồn tại — phase 08 hoàn toàn chưa implement
- Server routing tại `web/server.rs` — tiny-http blocking, pattern match routes
- Config đã có hot-reload qua `AppState::update()` + `watch::Sender<()>`
- UCI wrapper (`uci.rs`) đã hoàn chỉnh: get/set/delete/commit/add_list
- Default config đã có trong `Config::ensure_uci_file()` (inline string)
- Frontend: Vue.js embedded trong `embedded_index.html` (inline SPA)

---

## 2. Vấn đề tồn đọng (Critical)

### 2.1 Multipart parsing — KHÔNG CÓ LIBRARY

Plan yêu cầu multipart/form-data cho restore config và upload IPK, nhưng:
- **Cargo.toml không có multipart parser** (không có `multer`, `multipart`, hay tương đương)
- `read_body()` giới hạn **4KB** — IPK file ~850KB sẽ bị cắt
- tiny-http không tự parse multipart

**Giải pháp đề xuất:** Dùng raw binary body thay multipart. Client gửi `Content-Type: application/octet-stream` + raw file bytes. Đơn giản hơn, không cần thêm dependency. Hoặc implement multipart parser thủ công (phức tạp, dễ bug).

### 2.2 SHA256 — KHÔNG CÓ LIBRARY

Plan yêu cầu `sha256_file()` cho verify checksum khi remote upgrade, nhưng:
- Cargo.toml không có `sha2`, `ring`, hay crypto library nào
- Thêm `sha2` crate sẽ tăng binary size đáng kể trên MIPS

**Giải pháp đề xuất:** Gọi `sha256sum` CLI (có sẵn trên OpenWrt via `coreutils-sha256sum` hoặc busybox). Không thêm dependency, giữ binary nhỏ.

### 2.3 build.rs — KHÔNG TỒN TẠI

Plan dùng `env!("BUILD_DATE")` và `env!("GIT_COMMIT")` nhưng chưa có `build.rs` để set các env vars này.

**Giải pháp:** Tạo `ugate/build.rs` đơn giản:
```rust
fn main() {
    println!("cargo:rustc-env=BUILD_DATE={}", chrono::Local::now().format("%Y-%m-%d"));
    // Hoặc không dùng chrono, dùng Command::new("date")
}
```

### 2.4 read_body() — BỎ GIỚI HẠN 4KB

`read_body()` tại `server.rs:372` giới hạn 4KB. Upload IPK (~850KB) và config file sẽ bị cắt.

**Giải pháp:** Bỏ limit 4KB, đọc toàn bộ body. Hoặc tạo `read_body_raw()` trả `Vec<u8>` cho binary upload.

---

## 3. Vấn đề OpenWrt-specific

### 3.1 Factory Reset — Chỉ reset ugate config

Scope đã xác nhận: chỉ reset settings của ugate, KHÔNG phải full OpenWrt firstboot. Approach trong plan (ghi default config → uci commit) là đúng.

### 3.2 Upgrade path — Plan không dùng sysupgrade

OpenWrt có 2 cách upgrade:
1. **opkg install** — upgrade package đơn lẻ (plan dùng cách này)
2. **sysupgrade** — flash toàn bộ firmware image

Plan chỉ dùng `opkg install --force-reinstall` cho IPK — đúng cho package upgrade.

**Đã xác nhận:** Sau opkg install chỉ cần restart service, KHÔNG reboot:
```rust
Command::new("/etc/init.d/ugate").arg("restart").status().ok();
```

### 3.3 Remote upgrade — TLS + DNS dependency

`ureq::get(url)` cho remote upgrade yêu cầu:
- **TLS** hoạt động (đã có `ureq` với feature `tls` + `rustls`)
- **DNS resolution** — cần `resolv.conf` đúng
- **NTP sync** — TLS cert validation cần thời gian chính xác (đã có `time_sync::sync_time()`)
- **RAM cho download** — ~850KB IPK load vào RAM, OK với 64MB

### 3.4 `/tmp` trên OpenWrt

Plan ghi IPK vào `/tmp/ugate.ipk`. Trên OpenWrt, `/tmp` là **tmpfs** (RAM-based). Với 64MB RAM:
- ~850KB IPK: OK
- Nhưng nếu có nhiều thứ khác trong /tmp, có thể hết RAM
- Nên check available RAM trước khi download

### 3.5 Config backup — đúng cách trên OpenWrt

`/etc/config/ugate` là plain text UCI → download trực tiếp OK.
Nhưng nên thêm file vào `/etc/sysupgrade.conf` để OpenWrt preserve config khi sysupgrade.

---

## 4. Vấn đề thiết kế code

### 4.1 Plan dùng serde_json nhưng codebase KHÔNG DÙNG

Toàn bộ JSON parsing hiện tại là **thủ công** (manual string parsing, `jval()`, `jbool()`). Plan code snippets dùng `serde_json::from_str()` — không nhất quán.

**Khuyến nghị:** Giữ manual JSON output (format! macro). Không thêm serde_json dependency — binary size quan trọng trên MIPS 16MB flash.

### 4.2 Reload config — ĐÃ CÓ SẴN

`AppState::update()` đã trigger `watch::Sender` + `mqtt_notify`. Plan thêm `/api/reload` nhưng thực chất `POST /api/config` đã làm reload rồi (save_to_uci + state.update).

**Khuyến nghị:** `/api/reload` chỉ cần re-read UCI và gọi `state.update()` — reload mà KHÔNG thay đổi config, chỉ re-apply. Hữu ích khi user sửa UCI trực tiếp qua SSH.

### 4.3 Vue.js MaintenanceView — OK

Frontend dùng Vue.js (inline SPA trong `embedded_index.html`). Thêm maintenance section/tab vào embedded Vue app.

### 4.4 `include_str!("../../default-ugate.config")` — DUP

Default config đã có trong `Config::ensure_uci_file()` dạng inline string. Tạo thêm file riêng là trùng lặp.

**Khuyến nghị:** Reuse `Config::default()` + `save_to_uci()` cho factory reset, không cần file riêng.

---

## 5. Đánh giá từng API endpoint

| Endpoint | Khả thi | Vấn đề |
|----------|---------|--------|
| `GET /api/backup` | OK | Đọc `/etc/config/ugate`, trả raw text |
| `POST /api/restore` | Cần sửa | Multipart → đổi sang raw body. Validate UCI format phức tạp |
| `POST /api/factory-reset` | OK | Dùng `Config::default()` + `save_to_uci()` |
| `POST /api/restart` | OK | Spawn thread, sleep 1s, gọi `reboot` hoặc service restart |
| `POST /api/reload` | OK | Re-read UCI → `state.update()` |
| `GET /api/version` | Cần build.rs | Cần tạo build.rs cho BUILD_DATE, GIT_COMMIT |
| `POST /api/upgrade` | Cần sửa | Multipart → raw body. read_body limit 4KB → cần tăng |
| `GET /api/upgrade/check` | Cần dep | ureq đã có. Cần thêm version comparison logic |
| `POST /api/upgrade/remote` | Cần sửa | SHA256 CLI thay lib. RAM check trước download |

---

## 6. Recommended Implementation Order

1. **Version info** (đơn giản nhất, tạo build.rs)
2. **Backup** (đọc file, trả response)
3. **Factory reset** (reuse Config::default)
4. **Restart** (spawn + reboot)
5. **Reload** (re-read UCI)
6. **Restore** (cần giải quyết upload body)
7. **Upgrade upload** (cần read_body_large)
8. **Upgrade check** (cần version compare)
9. **Upgrade remote** (cần download + checksum)

---

## 7. Dependency Impact

| Cần thêm | Tại sao | Binary size impact |
|----------|---------|-------------------|
| Không thêm serde_json | Manual JSON đủ | 0 |
| Không thêm sha2 | Dùng `sha256sum` CLI | 0 |
| Không thêm multipart lib | Dùng raw body | 0 |
| Tạo build.rs | Version info | ~0 (compile-time only) |

**Zero new dependencies** — giữ binary nhỏ, đây là ưu tiên #1 trên 16MB flash.

---

## 8. Unresolved Questions

1. ~~Factory reset nên chỉ reset ugate config hay full OpenWrt firstboot?~~ → Chỉ reset ugate config
2. ~~Sau opkg install, nên reboot hay chỉ restart service?~~ → Chỉ restart service
3. ~~Upload file dùng raw body hay multipart?~~ → Raw body (`application/octet-stream`). Đơn giản, zero deps, frontend dùng `fetch` + `Blob` trực tiếp.
4. ~~Upgrade check URL config ở đâu?~~ → Thêm UCI section `upgrade` mới (option url, option auto_check)
5. ~~Rate-limit endpoints nguy hiểm?~~ → Không cần. Đã có auth (session cookie), thiết bị IoT ít user (max 4 sessions). Các endpoint nguy hiểm (factory-reset, restart) có confirm dialog ở frontend là đủ.
