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
