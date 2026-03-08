# Phase 7.1 — WiFi + Network UI Overhaul

## Status: IN PROGRESS — code done, chua deploy test

## Done (session 2)
- [x] WiFi 4 mode: sta/ap/sta_ap/off — backend + frontend
- [x] STA: input SSID go tay, quet WiFi ho tro dien, prefill pwd tu UCI, icon signal
- [x] AP: ten WiFi, password, kenh (chi hien khi mode=ap, STA+AP lock channel)
- [x] Bo dropdown ma hoa — backend tu quyet: co pwd=psk2, khong pwd=none
- [x] Password eye toggle (show/hide) cho ca STA + AP
- [x] Bo nut "Ngat" — dung dropdown mode thay the
- [x] Chi 1 nut "Luu nhap" trong WiFi card, "Ap dung" dung banner chung
- [x] WiFi status card tren trang Status (mode/STA signal+SSID/IP/AP)
- [x] Kenh truyen + WiFi status chuyen tu .grid sang .cf (can cot deu)
- [x] Label/value styling: label #64748b, value #e2e8f0 + semi-bold
- [x] LAN/WAN → chi giu ETH WAN, bo LAN khoi view
- [x] Metric tach ra card rieng "Uu tien mang"
- [x] Responsive mobile: .cf 2 col, .grid 1 col khi <520px
- [x] 0 warnings (cargo fix + #[allow(dead_code)] cho code du phong)

## Done (session 3 — UI overhaul + network apply)
- [x] NTP: toggle inline heading, dynamic server inputs (2 col), + Them/- Xoa cuoi
- [x] NTP: luu thang flash (uci commit system), k can qua apply flow
- [x] NTP: nut "Dong bo ngay" + "Luu NTP" ngang nhau
- [x] Datetime hien thi tren trang Trang thai (backend `date` command)
- [x] Tab "Cau hinh" → "Truyen thong", tab "Du lieu" gop vao tab "UART"
- [x] Thu tu tab: Trang thai → Truyen thong → UART → Mang → Dinh tuyen
- [x] Card "Cai dat UART" + "UART Real-time" gop trong 1 tab, stream flex height
- [x] Card He thong: 3 col 1 row (Phien ban | Uptime | Thoi gian)
- [x] Card Kenh truyen: 3 col 2 dong (UART RX/TX/config + MQTT/HTTP/TCP)
- [x] MQTT: Publish + Subscribe Topic cung 1 row
- [x] HTTP: Phuong thuc + URL cung 1 row
- [x] TCP: Che do + Cong/Dia chi cung 1 row
- [x] UART: Frame Mode + Gap/Timeout chia column
- [x] Max-width 800px → 1000px
- [x] Network apply: `ubus call network reload` thay `/etc/init.d/network restart`
- [x] Apply chi restart interface thay doi (netifd diff-based), WiFi rieng `wifi reload`
- [x] Uci::changed_sections() helper parse uci changes output

## TODO
- [ ] Deploy test len thiet bi
- [ ] Test 4 mode thuc te
- [ ] Dynamic WAN discovery: `ip route show default` parse toan bo WAN interface (cho 4G sau nay)
- [ ] Review: bo connectWifi/disconnectWifi cu (da thay bang saveWifiMode)

## UCI Mapping
| Mode   | wwan.disabled | default_radio0.disabled |
|--------|--------------|------------------------|
| STA    | 0            | 1                      |
| AP     | 1            | 0                      |
| STA+AP | 0            | 0                      |
| Off    | 1            | 1                      |

## Key Files
- `ugate/src/web/wifi.rs` — backend handlers
- `ugate/src/web/server.rs` — routing
- `ugate/src/embedded_index.html` — frontend

## Notes
- STA+AP chay OK tren MT7628 — cung phy0, AP lock channel theo STA
- radio0.country=VN, band=2g — can giu
- UCI draft/apply: save RAM → "Ap dung" commit flash + network reload/wifi reload
- Network apply: `ubus call network reload` (netifd diff, gian doan toi thieu)
- WiFi apply: `wifi reload` (rieng, netifd khong quan ly WiFi)
- NTP: commit truc tiep, khong can draft/apply flow
- Dynamic WAN: `ip route show default` → parse dev+metric → ho tro ETH/WiFi/4G
