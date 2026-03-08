# OpenWrt WiFi: AP ↔ STA

## Xem cấu hình hiện tại

```bash
cat /etc/config/wireless    # cấu hình wifi
cat /etc/config/network      # cấu hình mạng
iwinfo wlan0 info            # trạng thái wifi
iwinfo wlan0 scan            # scan wifi xung quanh
```

## Chuyển sang STA (kết nối WiFi)

```bash
uci set wireless.default_radio0.mode='sta'
uci set wireless.default_radio0.network='wwan'
uci set wireless.default_radio0.ssid='TEN_WIFI'
uci set wireless.default_radio0.encryption='psk2'
uci set wireless.default_radio0.key='MAT_KHAU'
uci commit wireless
wifi reload
```

> Cần có sẵn interface `wwan` trong `/etc/config/network`:
>
> ```
> config interface 'wwan'
>     option proto 'dhcp'
>     option device 'wlan0'
> ```

## Chuyển về AP (phát WiFi)

```bash
uci set wireless.default_radio0.mode='ap'
uci set wireless.default_radio0.network='lan'
uci set wireless.default_radio0.ssid='TEN_WIFI_PHAT'
uci set wireless.default_radio0.encryption='psk-mixed'
uci set wireless.default_radio0.key='MAT_KHAU'
uci commit wireless
wifi reload
```

## Kiểm tra

```bash
iwinfo wlan0 info    # xem mode, SSID, signal
ping 8.8.8.8         # test internet
```

## Lưu ý

- `uci commit` = lưu vĩnh viễn qua reboot
- 1 radio chỉ chạy được 1 mode (AP hoặc STA), trừ khi chip hỗ trợ multi-interface
- Khi chuyển STA, thiết bị không còn phát WiFi