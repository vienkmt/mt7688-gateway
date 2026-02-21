# Phân tích cấu hình OpenWRT cho MT7688 LinkIt Smart 7688

> **File cấu hình**: `01 OpenWRT SDK/openwrt_remove_mmc_driver/config_bak.txt`
> **Ngày phân tích**: 2026-02-15

## Thông tin Target

| Thuộc tính | Giá trị |
|------------|---------|
| Target | ramips / mt76x8 |
| Device | MediaTek LinkIt Smart 7688 |
| Kernel | Linux 5.4 |
| Architecture | mipsel_24kc |
| Libc | musl |
| GCC | 8.4.0 |
| Binutils | 2.34 |

---

## Các gói được cài đặt (phân loại)

### 1. Hệ thống cơ bản

| Gói | Mô tả |
|-----|-------|
| base-files | File hệ thống cơ bản |
| busybox | Tiện ích shell |
| procd | Process daemon |
| fstools | Công cụ filesystem |
| logd | Log daemon |
| mtd | MTD utilities |
| ubox, ubus, ubusd | OpenWRT bus system |
| uci | Unified Configuration Interface |
| opkg | Package manager |
| urandom-seed, urngd | Random number generator |

### 2. Mạng & WiFi

| Gói | Mô tả |
|-----|-------|
| netifd | Network interface daemon |
| dnsmasq | DNS + DHCP server |
| firewall | Firewall |
| swconfig | Switch configuration |
| kmod-mt7603 | Driver WiFi MT7603 |
| kmod-mac80211 | 802.11 stack |
| kmod-cfg80211 | Wireless config |
| wpad-basic | WPA supplicant/hostapd |
| hostapd-common | WiFi AP |
| iwinfo, iw | WiFi tools |
| wireless-regdb | Wireless regulatory database |

### 3. USB & 3G/4G Modem

| Gói | Mô tả |
|-----|-------|
| kmod-usb-core, kmod-usb2, kmod-usb-ohci | USB drivers |
| kmod-usb-serial, kmod-usb-serial-option | USB serial |
| kmod-usb-serial-qualcomm | Qualcomm modem |
| kmod-usb-net-qmi-wwan | QMI protocol |
| kmod-usb-net-cdc-mbim/ncm/ether | CDC drivers |
| kmod-usb-net-huawei-cdc-ncm | Huawei modem |
| kmod-usb-net-rndis | RNDIS protocol |
| uqmi | QMI utility |
| wwan | WWAN scripts |
| chat | Modem AT commands |
| kmod-usb-storage | USB storage |

### 4. PPP & Tunneling

| Gói | Mô tả |
|-----|-------|
| ppp, ppp-mod-pppoe, ppp-mod-pppol2tp | PPP protocols |
| ppp-mod-pppoa | PPPoA (DSL) |
| kmod-l2tp | L2TP tunnel |
| linux-atm, kmod-atm | ATM (DSL) |

### 5. Netfilter/Firewall

| Gói | Mô tả |
|-----|-------|
| iptables, ip6tables | Firewall rules |
| kmod-ipt-offload | Hardware offload |
| kmod-ipt-conntrack(-extra) | Connection tracking |
| ipset, kmod-ipt-ipset | IP sets |
| kmod-nf-nat, kmod-ipt-nat | NAT |

### 6. QoS (Quality of Service)

| Gói | Mô tả |
|-----|-------|
| qos-scripts | QoS scripts |
| tc-tiny, tc-mod-iptables | Traffic control |
| kmod-sched-core, kmod-sched-connmark | Scheduler |
| kmod-ifb | Intermediate Functional Block |

### 7. LuCI Web Interface

| Gói | Mô tả |
|-----|-------|
| luci, luci-base | Web UI core |
| luci-mod-admin-full | Admin interface |
| luci-mod-network/status/system | Các module |
| luci-app-firewall | Firewall app |
| luci-app-qos | QoS app |
| luci-app-opkg | Package manager app |
| **luci-app-myapplication** | App custom (tự phát triển) |
| luci-theme-argon | Theme Argon |
| luci-theme-bootstrap | Theme Bootstrap |
| luci-theme-material | Theme Material |
| luci-i18n-*-zh-cn | Gói ngôn ngữ tiếng Trung |
| uhttpd | Web server |
| rpcd, rpcd-mod-luci | RPC daemon |

### 8. Phần cứng khác

| Gói | Mô tả |
|-----|-------|
| kmod-i2c-core, kmod-i2c-mt7628 | I2C bus |
| kmod-rtc-pcf8563 | RTC chip PCF8563 |
| kmod-gpio-button-hotplug | GPIO buttons |
| kmod-leds-gpio | GPIO LEDs |
| kmod-gpio-dev | GPIO device |
| kmod-scsi-core, kmod-scsi-generic | SCSI support |

### 9. Thư viện

| Gói | Mô tả |
|-----|-------|
| libcurl, curl | HTTP client |
| libwolfssl | SSL/TLS library |
| libjson-c | JSON library |
| liblua, lua | Lua scripting |
| ca-bundle | SSL certificates |
| libubox, libubus, libuci | OpenWRT core libs |

---

## Đánh giá cấu hình

### Điểm tốt

1. **Target đúng** - MT76x8/LinkIt Smart 7688
2. **WiFi đầy đủ** - MT7603 driver + mac80211 + wpad-basic
3. **Hỗ trợ 3G/4G modem tốt** - QMI, MBIM, NCM, Huawei, Qualcomm
4. **Security cơ bản** - Stack protector, ASLR PIE, RELRO enabled
5. **QoS có sẵn** - Phù hợp cho gateway
6. **I2C + RTC** - PCF8563 cho đồng hồ thời gian thực
7. **LuCI đầy đủ** - Nhiều theme đẹp (Argon, Material)
8. **Hardware offload** - kmod-ipt-offload cho hiệu năng tốt hơn

### Lưu ý / Cần cải thiện

| Vấn đề | Chi tiết |
|--------|----------|
| MMC driver đã bị remove | OK nếu không dùng SD card |
| IPv6 không đầy đủ | Thiếu `odhcp6c`, `odhcpd-ipv6only` |
| Không có SSL stream | Thiếu `libustream-wolfssl` |
| USB ATM/DSL drivers | Có thể không cần, tốn flash |
| Chỉ có tiếng Trung | Có thể thêm tiếng Việt/Anh |
| Thiếu editor | Không có nano hoặc vim |

### Gói có thể loại bỏ (nếu không dùng DSL)

```
# Các gói ATM/DSL - tốn dung lượng flash
kmod-usb-atm
kmod-usb-atm-cxacru
kmod-usb-atm-speedtouch
kmod-usb-atm-ueagle
ppp-mod-pppoa
linux-atm
kmod-atm
```

### Gói nên thêm

```bash
# IPv6 đầy đủ
CONFIG_PACKAGE_odhcp6c=y
CONFIG_PACKAGE_odhcpd-ipv6only=y

# SSL stream cho HTTPS
CONFIG_PACKAGE_libustream-wolfssl=y

# Ngôn ngữ tiếng Việt (nếu cần)
CONFIG_PACKAGE_luci-i18n-base-vi=y

# Editor (tùy chọn)
CONFIG_PACKAGE_nano=y
```

---

## Kết luận

Cấu hình này **khá tốt** cho một gateway 3G/4G với MT7688:

- Đầy đủ driver modem USB (QMI, MBIM, NCM, Huawei, Qualcomm)
- WiFi hoạt động tốt với MT7603
- QoS và firewall đầy đủ
- Giao diện web LuCI với nhiều theme đẹp
- Hỗ trợ I2C và RTC cho các ứng dụng IoT

**Đề xuất**: Có thể tối ưu bằng cách loại bỏ các driver ATM/DSL không cần thiết để tiết kiệm flash (~200-500KB) và thêm các gói IPv6 nếu cần hỗ trợ IPv6 đầy đủ.

---

## Tham khảo

- [OpenWRT Wiki - MT76x8](https://openwrt.org/docs/techref/targets/ramips)
- [LinkIt Smart 7688 Wiki](https://openwrt.org/toh/mediatek/linkit_smart_7688)
