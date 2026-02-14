# OpenWrt: Tạo Init Script khởi động app cùng hệ thống

## Tạo init script

```bash
cat > /etc/init.d/v3s-monitor << 'EOF'
#!/bin/sh /etc/rc.common
START=99

start() {
    (sleep 3 && /v3s-system-monitor > /dev/null 2>&1) &
}

stop() {
    killall v3s-system-monitor
}
EOF

chmod +x /etc/init.d/v3s-monitor
/etc/init.d/v3s-monitor enable
```

## Các lệnh quản lý

```bash
/etc/init.d/v3s-monitor start     # chạy thủ công
/etc/init.d/v3s-monitor stop      # dừng
/etc/init.d/v3s-monitor enable    # bật khởi động cùng boot
/etc/init.d/v3s-monitor disable   # tắt khởi động cùng boot
```

## Kiểm tra

```bash
ls /etc/rc.d/ | grep v3s          # xem đã enable chưa
ps | grep v3s-system-monitor      # xem app có đang chạy không
```

## Lưu ý

- `START=99` → chạy cuối cùng, sau khi các service khác sẵn sàng
- `sleep 3` → đợi UART/device sẵn sàng
- `> /dev/null 2>&1` → ẩn log trên console
- Bọc trong `(...) &` → không block quá trình boot
- App binary phải nằm ở partition persistent (không phải `/tmp`)