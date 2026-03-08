# Bugs & Gotchas - MT7688 / OpenWrt / Rust

Ghi chú các lỗi hay gặp khi phát triển firmware Rust cho MIPS embedded.

---

## 1. MIPS 32-bit không hỗ trợ AtomicU64

- **Triệu chứng:** Compile error `AtomicU64 not available on this platform`
- **Nguyên nhân:** MIPS 24KEc là 32-bit, không có 64-bit atomic instructions
- **Fix:** Dùng `AtomicU32` thay thế. Nếu cần 64-bit counter, dùng `Mutex<u64>`

## 2. ioctl type khác nhau trên MIPS vs x86_64

- **Triệu chứng:** Type mismatch khi dùng `libc::ioctl`
- **Nguyên nhân:** `libc::Ioctl` = `i32` trên MIPS, `c_ulong` trên x86_64
- **Fix:** Khai báo constant với kiểu `libc::Ioctl` và cast: `0xC16CB403u32 as libc::Ioctl`

## 3. WebSocket qua tiny-http thiếu Sec-WebSocket-Accept

- **Triệu chứng:** Browser báo `WebSocket connection failed`, WS không kết nối
- **Nguyên nhân:** `tiny_http::Request::upgrade()` gửi 101 nhưng KHÔNG tự thêm header `Sec-WebSocket-Accept`. Browser yêu cầu header này theo RFC 6455
- **Fix:** Tính accept key từ `Sec-WebSocket-Key` bằng `tungstenite::handshake::derive_accept_key()`, thêm vào response trước khi upgrade:
  ```rust
  let accept_key = tungstenite::handshake::derive_accept_key(ws_key.as_bytes());
  let response = tiny_http::Response::empty(101)
      .with_header(Header::from_bytes("Connection", "Upgrade").unwrap())
      .with_header(Header::from_bytes("Sec-WebSocket-Accept", accept_key.as_bytes()).unwrap());
  let stream = request.upgrade("websocket", response);
  ```
- Sau đó dùng `WebSocket::from_raw_socket(stream, Role::Server, None)` (KHÔNG dùng `accept()` vì handshake đã xong)

## 4. WebSocket blocking read chặn broadcast write

- **Triệu chứng:** WS kết nối OK nhưng client không nhận được data (status, UART)
- **Nguyên nhân:** `ws.read()` blocking vô thời hạn trên stream từ tiny-http upgrade (Box<dyn ReadWrite> không có set_nonblocking)
- **Fix:** Dùng writer thread riêng với `broadcast_rx.blocking_recv()`, hoặc single-thread loop chỉ gửi broadcast + sleep (bỏ qua read từ client, client dùng HTTP API để gửi lệnh)

## 5. Frontend mất session khi F5

- **Triệu chứng:** Reload trang phải login lại dù cookie vẫn còn
- **Nguyên nhân:** JS khởi tạo `S.page='login'` mà không check session cookie có hợp lệ
- **Fix:** Khi load page, gọi `/api/status` — nếu 200 thì skip login:
  ```js
  async function checkSession(){
    try{const r=await fetch('/api/status');if(r.ok){S.page='status';connectWS()}}catch(_){}
    render();
  }
  checkSession();
  ```

## 6. Deploy script báo lỗi giả trên MIPS

- **Triệu chứng:** `deploy.sh` báo "ugate không chạy được" nhưng thực tế đang chạy
- **Nguyên nhân:** MIPS 580MHz khởi động Tokio runtime + bind port mất 5-8 giây. `sleep 2` không đủ
- **Fix:** Tăng `sleep 8` trước khi `pgrep` kiểm tra

## 7. rumqttc AsyncClient không hoạt động trên MIPS

- **Triệu chứng:** MQTT async client panic hoặc hang
- **Nguyên nhân:** Vấn đề với async runtime trên MIPS (epoll quirks)
- **Fix:** Dùng sync `rumqttc::Client` trong `std::thread::spawn`, giao tiếp qua `std::sync::mpsc`

## 8. tiny-http Cookie header case-sensitive

- **Triệu chứng:** Auth check fail dù cookie đúng
- **Nguyên nhân:** Browser gửi `Cookie` nhưng code check `cookie` (lowercase)
- **Fix:** Check cả hai: `h.field.as_str() == "Cookie" || h.field.as_str() == "cookie"`

## 9. Network Apply: dùng `ubus call network reload` thay vì `network restart`

- **Triệu chứng:** Ấn "Áp dụng" → mất kết nối 5-10s, toàn bộ interface tắt/bật
- **Nguyên nhân:** `/etc/init.d/network restart` restart TẤT CẢ interface kể cả interface không thay đổi
- **Fix:** Dùng `ubus call network reload` — netifd tự diff config cũ/mới, chỉ restart interface thay đổi
- **So sánh:**
  - `network restart` → gián đoạn cao (tắt/bật tất cả)
  - `ifup <iface>` → trung bình (restart 1 interface chỉ định)
  - `ubus call network reload` → thấp nhất (netifd diff-based)
- **WiFi:** netifd không quản lý WiFi → vẫn cần `wifi reload` riêng
- **NTP/System:** chỉ `uci commit system`, không cần reload gì
- **Học từ LuCI:** LuCI dùng `ubus call uci apply` + rollback timer 30s + `ubus call network reload`

## 10. NTP commit trực tiếp, không cần draft/apply

- **Triệu chứng:** Lưu NTP config → phải ấn "Áp dụng & Lưu flash" → trigger network restart không cần thiết
- **Fix:** NTP `handle_set_ntp()` gọi `Uci::commit("system")` ngay → ghi flash trực tiếp, không cần qua apply flow

## 11. Channel disable toggle không có tác dụng (MQTT, TCP)

- **Triệu chứng:** Tắt channel qua web UI switch → lưu → channel vẫn hoạt động. MQTT subscribe vẫn nhận message, TCP connection vẫn echo data. Chỉ HTTP channel tắt đúng.
- **Nguyên nhân (MQTT):** `run_publish_loop` spawn IO thread xử lý `connection.iter()`. Khi config thay đổi, publish loop return nhưng IO thread cũ **không bị dừng** — vẫn subscribe và forward message tới dispatcher qua `cmd_tx`. Mỗi lần reconnect tích lũy thêm 1 IO thread → 1 message MQTT nhân bản thành N message TX.
- **Nguyên nhân (TCP Server):** `handle_connection` được `tokio::spawn` → không bị cancel khi config thay đổi, connection cũ vẫn hoạt động.
- **Nguyên nhân (TCP Client):** `handle_connection.await` block trực tiếp không có `select!` với config watch → không phát hiện thay đổi cho đến khi remote đóng connection.
- **HTTP hoạt động đúng vì:** Dùng `tokio::sync::watch` + `config_watch.changed()` trong `select!` → phản hồi config change ngay lập tức.
- **Fix (MQTT):** Thêm `Arc<AtomicBool>` flag (`io_stop`) + gọi `client.disconnect()` tại mọi return path của `run_publish_loop`. IO thread check flag mỗi vòng lặp → break khi flag = true.
- **Fix (TCP):** Thêm `watch::Receiver<()>` shutdown signal vào `handle_connection`, `select!` trên `shutdown_rx.changed()`. Server truyền `state.subscribe()` cho mỗi spawned connection. Client tương tự.
- **Bài học:** Khi spawn thread/task xử lý network I/O, luôn có cơ chế shutdown signal. Đặc biệt `std::thread::spawn` không tự cancel như async task — cần flag hoặc disconnect explicit.

---

## 6. macOS SCP mặc định dùng SFTP — OpenWrt không có sftp-server

- **Triệu chứng:** `scp file root@device:/tmp/` → `ash: /usr/libexec/sftp-server: not found`
- **Nguyên nhân:** macOS mới (OpenSSH 9+) mặc định dùng SFTP protocol, OpenWrt chỉ có Dropbear SSH (không có `openssh-sftp-server`)
- **Fix:** Dùng `scp -O` (legacy SCP protocol) thay vì SFTP

---

## 7. macOS bsdtar tạo PaxHeader — busybox tar/opkg không hiểu

- **Triệu chứng:** `opkg install *.ipk` → hàng loạt `Unknown typeflag: 0x78`, `PaxHeader` errors
- **Nguyên nhân:** macOS bsdtar tự thêm PaxHeader entries (extended attributes, metadata) vào tar archive. busybox tar trên OpenWrt không hỗ trợ PAX format (typeflag `x` = 0x78)
- **Fix:** Dùng GNU tar (`gtar`) thay vì macOS bsdtar: `brew install gnu-tar`
- **Lưu ý:** `COPYFILE_DISABLE=1` chỉ tắt `._*` resource fork files, KHÔNG tắt PaxHeader — phải dùng `gtar`

---

## 8. opkg "up to date" khi nhiều IPK file cùng tồn tại

- **Triệu chứng:** `opkg install /tmp/ugate_*.ipk` → `Package ugate installed in root is up to date` hoặc `Multiple packages providing same name`
- **Nguyên nhân:** Glob `*.ipk` match nhiều file (bản cũ + mới), opkg so sánh version sai
- **Fix:** Xoá IPK cũ trước khi upload: `rm -f /tmp/ugate_*.ipk`, hoặc chỉ định file chính xác thay vì dùng glob

---

## 9. UCI named section mismatch gây init script không start service

- **Triệu chứng:** `opkg install` thành công nhưng service không chạy, `/etc/init.d/ugate status` → `active with no instances`
- **Nguyên nhân:** Init script dùng `config_get enabled main enabled 0` nhưng UCI config dùng section name khác (vd: `general`, `ugate`). `config_get` trả default `0` → không start
- **Fix:** Đồng bộ section name giữa UCI config (`config ugate 'ugate'`) và init script (`config_get enabled ugate enabled 0`)
- **Bài học:** Luôn test init script sau khi thay đổi UCI config structure. Dùng `uci show <package>` để verify section names
