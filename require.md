Tôi chuẩn bị làm 1 dự án iot Gateway, viết bằng rust, phần cứng và phần mềm có trong claude.md.

## Tổng quan
- Ngôn ngữ rust, 1 app duy nhất, procd trên openwrt đã lo quản lý deamon rồi
- Cấu hình, setting dùng uci
- Frontend dùng vuejs, websocket, tích hợp trong dự án luôn. Dùng để login, cấu hình network, channel giao tiếp
- Dự án demo vgateway đã hoàn thiện 1 số thứ chạy khá ok, dự án ugate này sẽ là bản hoàn chỉnh

## Nhiệm vụ
- 1 task đọc serial1 qua tokio (tận dụng epoll). MCU sẽ đổ dữ liệu về đây, ngăn cách frame bằng charater đơn giản (\r, \n, \r\n)
- 1 task chính quản lý, nhận dữ liệu serial, kiểm tra xem channel nào được bật, fan out ra các channel: MQTT, HTTP, TCP Server
- channel nào dc bật thì sẽ có 1 task riêng quản lý kết nối, cấu hình
- 1 task làm web server, phục vụ cả static file. Sẽ phục vụ login, setting, montoring
- Truyền thông 2 chiều, server gửi xuống sẽ chuyển qua RX cho MCU
- Có 4 ngõ ra GPIO nữa, mcu hoặc server trigger 1 lệnh đặc biệt sẽ toogle dc các chân GPIO này

## Tips từ vgate
- Đọc dự án demo vgateway, sẽ học dc nhiều thứ, có cả bug phòng tránh

