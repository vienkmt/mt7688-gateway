# Phase 7.1 — WiFi + Network + System UI

## Status: DONE (chua deploy test thuc te)

---

## Tong quan

Phase nay overhaul toan bo giao dien web cua ugate: WiFi 4 che do, Network config (ETH WAN, metric, NTP), Routing, System maintenance (backup/restore, upgrade, factory reset). Frontend dung vanilla JS voi DOM builder `h()`, backend dung tiny-http + UCI CLI wrapper.

---

## Backend Modules

### web/wifi.rs — Quan ly WiFi
- `handle_scan()` — GET /api/wifi/scan: quet WiFi bang `iwinfo phy0-sta0 scan`, parse ESSID/Signal/Encryption
- `handle_status()` — GET /api/wifi/status: doc UCI disabled flags de detect mode (sta/ap/sta_ap/off), lay config + runtime info ca STA lan AP (signal, IP, connected, ssid, key, enc, channel)
- `handle_set_mode(body)` — POST /api/wifi/mode: chuyen 4 che do WiFi, set disabled flags + goi set_sta_config/set_ap_config tuong ung. Luu RAM (draft), can Apply de commit flash
- `handle_connect(body)` — POST /api/wifi/connect: ket noi STA toi 1 SSID cu the (legacy, frontend van goi)
- `handle_disconnect()` — POST /api/wifi/disconnect: xoa SSID + key cua STA (legacy)
- Helpers: `set_sta_config()`, `set_ap_config()`, `parse_signal()`, `get_iface_ip()`

### web/netcfg.rs — Network, NTP, Routing, WAN Discovery
- `handle_get_network()` — GET /api/network: doc config LAN + WAN tu UCI (proto, ipaddr, netmask, gateway, metric, dns) + wwan_metric
- `handle_set_network(body)` — POST /api/network: set LAN hoac WAN config (dhcp hoac static voi IP/netmask/gateway/DNS). Luu RAM
- `handle_apply()` — POST /api/network/apply: commit cac package da thay doi (network, wireless, system) + restart chi interface bi thay doi (`ubus call network reload` cho net, `wifi reload` cho wireless). Delay 1s trong thread rieng
- `handle_revert()` — POST /api/network/revert: huy tat ca thay doi chua commit (network + wireless + system)
- `handle_changes()` — GET /api/network/changes: kiem tra co thay doi pending khong (tra boolean cho tung package)
- `handle_get_ntp()` — GET /api/ntp: doc NTP config (enabled, servers list, timezone, zonename)
- `handle_set_ntp(body)` — POST /api/ntp: set NTP servers + timezone. Commit truc tiep (`uci commit system`), khong qua draft/apply flow
- `handle_ntp_sync()` — POST /api/ntp/sync: trigger dong bo thu cong bang `ntpd -q` (timeout 5s), fallback `time_sync::sync_time()` (HTTP Date header)
- `handle_get_routes()` — GET /api/routes: parse `ip route show` thanh JSON array (dest, via, dev, metric, scope)
- `handle_add_route(body)` — POST /api/routes: them static route vao UCI + apply ngay bang `ip route add`
- `handle_delete_route(name)` — DELETE /api/routes/{name}: xoa route khoi UCI
- `handle_wan_discover()` — GET /api/wan/discover: parse `ip route show default` de list cac WAN interface (dev, uci name, label, gateway, metric). Ho tro ETH/WiFi/4G
- `handle_set_metric(body)` — POST /api/interface/metric: set metric cho 1 interface UCI
- Helpers: `uci_get()`, `json_str_array()`, `netmask_to_cidr()`, `dev_to_uci()`

### web/maintenance.rs — Bao tri he thong
- `handle_version()` — GET /api/version: tra version, build_date, git_commit (compile-time env)
- `handle_backup()` — GET /api/backup: download file `/etc/config/ugate` dang binary
- `handle_restore(req, state)` — POST /api/restore: upload config file (max 64KB), validate UTF-8 + UCI format, backup cu truoc khi ghi de, re-read config
- `handle_factory_reset(state)` — POST /api/factory-reset: reset config ve default, save UCI + update state
- `handle_restart()` — POST /api/restart: reboot device (delay 1s)
- `handle_upgrade_upload(req)` — POST /api/upgrade: upload file IPK (max 10MB), validate ar archive format, install bang `opkg install --force-reinstall`, restart service. Guard `UPGRADING` AtomicBool chong concurrent
- `handle_get_upgrade_url()` — GET /api/upgrade/url: doc upgrade URL tu UCI
- `handle_set_upgrade_url(body)` — POST /api/upgrade/url: luu upgrade URL vao UCI + commit
- `handle_upgrade_check()` — GET /api/upgrade/check: fetch manifest JSON tu remote URL (ureq), parse version/changelog/size/url, so sanh voi version hien tai
- `handle_upgrade_remote()` — POST /api/upgrade/remote: download IPK tu remote, verify SHA256 checksum, install + restart. Chay trong thread rieng

### web/auth.rs — Xac thuc
- `SessionManager`: quan ly session trong RAM, max 4 session, TTL 24h, rate limit 2s cooldown giua cac lan login fail
- `validate_password()`: kiem tra password tu JSON body
- Token tao tu `/dev/urandom` (16 bytes = 32 hex chars), fallback timestamp + PID
- Session check bang cookie header `session=<token>`

### web/ws.rs — WebSocket
- `WsManager`: broadcast channel (tokio broadcast, capacity 64) + command channel (std mpsc)
- `handle_websocket()`: single-thread loop — gui broadcast data + doc lenh tu client. Max connections guard (atomic fetch_add). Idle timeout 120s
- tiny-http 101 Upgrade → tungstenite `from_raw_socket` (khong handshake lai)

### web/mod.rs — Shared helpers
- `json_resp()`, `json_err()` — tra JSON response voi Content-Type
- `jval()` — parse JSON value don gian (string hoac number/bool)
- `json_escape()` — escape string cho JSON output
- `is_safe_identifier()` — validate UCI key (alphanumeric + underscore, max 64 chars)
- `is_valid_ipv4()` — validate IPv4 format

### web/status.rs — Thu thap trang thai
- `SharedStats`: atomic counters chia se giua UART/MQTT/HTTP/TCP/GPIO tasks
- `to_status_json()`: thu thap CPU% (delta /proc/stat), RAM (/proc/meminfo), uptime (/proc/uptime), datetime (`date` command), counters cac kenh

### web/server.rs — HTTP Router
- Routes toan bo API endpoints (xem chi tiet tren)
- Auth middleware: tat ca `/api/*` (tru `/api/login`) yeu cau session cookie
- WebSocket upgrade tai `/ws` (yeu cau auth)
- Static file: `embedded_index.html` tai `/`
- Config CRUD: GET/POST `/api/config` (doc/ghi UCI ugate)
- GPIO: POST `/api/gpio/{pin}/{state}`
- Password change: POST `/api/password`

### uci.rs — UCI Wrapper
- `Uci::get/set/delete/get_list/add_list` — CRUD thao tac UCI
- `Uci::commit(config)` — commit thay doi vao flash
- `Uci::revert(config)` — huy thay doi chua commit
- `Uci::has_changes(config)` — kiem tra co pending changes
- `Uci::changed_sections(config)` — parse output `uci changes` de lay danh sach section bi thay doi

---

## Frontend (embedded_index.html — 870 dong)

### Tabs (thu tu)
1. **Trang thai** — He thong (version/uptime/datetime), Kenh truyen (UART/MQTT/HTTP/TCP stats), WiFi status, GPIO
2. **Truyen thong** — Config MQTT/HTTP/TCP (enable/disable + params)
3. **UART** — Cai dat UART (baudrate/parity/frame mode) + UART Real-time stream
4. **Mang** — WiFi card (4 mode + STA/AP config), ETH WAN card, Uu tien mang card, NTP card
5. **Dinh tuyen** — Bang dinh tuyen (parse tu `ip route`), them static route
6. **He thong** — Version/build info, Backup/Restore, Factory Reset, Restart, Upgrade (upload IPK hoac remote URL)

### WiFi UI
- Dropdown 4 mode: STA / AP / STA+AP / Tat WiFi
- STA section: input SSID (go tay hoac quet WiFi dien), password voi eye toggle, hien thi trang thai ket noi (signal bars + IP) khi da ket noi
- AP section: ten WiFi, password voi eye toggle, kenh (chi hien khi mode=AP, STA+AP lock channel tu dong)
- 1 nut "Luu nhap" trong WiFi card — luu vao RAM
- Banner "Ap dung" khi co pending changes — commit flash + wifi reload

### Network UI
- ETH WAN: proto DHCP/Static, IP/netmask/gateway/DNS khi static
- Uu tien mang: dynamic WAN discovery, hien thi tat ca WAN interfaces voi metric editable
- NTP: toggle enable, timezone dropdown (17 timezone), dynamic server inputs (them/xoa), 2 nut (Dong bo ngay + Luu NTP)

### Routing UI
- Bang dinh tuyen hien tai (dest/gateway/interface/metric/scope)
- Form them static route (ten/interface/target/netmask/gateway)

### System UI
- Phien ban firmware (version/build date/git commit)
- Backup config (download file)
- Restore config (upload file)
- Factory reset (reset ve mac dinh)
- Khoi dong lai thiet bi
- Upgrade: upload IPK hoac remote URL (kiem tra + cai dat tu dong voi checksum verify)

### Draft/Apply Pattern
- Thay doi WiFi/Network/Metric luu vao RAM (`uci set` khong commit)
- Banner "Ap dung" xuat hien khi co pending changes
- "Ap dung" → commit flash + restart chi interface bi thay doi
- "Huy" → revert tat ca thay doi chua commit
- Ngoai le: NTP commit truc tiep (khong qua draft flow)

---

## UCI Mapping

### WiFi Modes
| Mode   | wwan.disabled | default_radio0.disabled |
|--------|--------------|------------------------|
| STA    | 0            | 1                      |
| AP     | 1            | 0                      |
| STA+AP | 0            | 0                      |
| Off    | 1            | 1                      |

### WiFi Interfaces
- `wireless.wwan` = STA iface (mode=sta, network=wwan)
- `wireless.default_radio0` = AP iface (mode=ap, network=lan)
- `wireless.radio0` = radio config (channel, country=VN, band=2g)

### Network Interfaces
- `network.lan` = br-lan, static 192.168.10.1/24
- `network.wan` = eth0.2, proto dhcp, metric 100
- `network.wwan` = WiFi WAN, proto dhcp, metric 10

### Device → UCI Mapping (WAN Discovery)
- `eth0.2` → wan (ETH WAN)
- `phy0-sta0` → wwan (WiFi WAN)
- `br-lan` → lan (LAN)
- Other → pass-through (ho tro 4G/USB sau nay)

---

## Key Files
- `ugate/src/web/wifi.rs` — WiFi handlers (210 dong)
- `ugate/src/web/netcfg.rs` — Network/NTP/Routes/WAN/Metric handlers (351 dong)
- `ugate/src/web/maintenance.rs` — Backup/Restore/Upgrade handlers (363 dong)
- `ugate/src/web/auth.rs` — Session manager (142 dong)
- `ugate/src/web/ws.rs` — WebSocket handler (115 dong)
- `ugate/src/web/status.rs` — System status collector (207 dong)
- `ugate/src/web/server.rs` — HTTP router (529 dong)
- `ugate/src/web/mod.rs` — Shared helpers (74 dong)
- `ugate/src/uci.rs` — UCI CLI wrapper (147 dong)
- `ugate/src/embedded_index.html` — Frontend SPA (870 dong)

---

## Hoan thanh

### Session 1 — WiFi backend co ban
- [x] WiFi scan, status, connect/disconnect API
- [x] UCI draft/apply pattern

### Session 2 — WiFi 4 mode + UI
- [x] WiFi 4 mode: sta/ap/sta_ap/off — backend + frontend
- [x] STA: input SSID go tay, quet WiFi ho tro dien, prefill pwd tu UCI, signal bars
- [x] AP: ten WiFi, password, kenh (chi hien khi mode=AP, STA+AP lock channel)
- [x] Bo dropdown ma hoa — backend tu quyet: co pwd=psk2, khong pwd=none
- [x] Password eye toggle (show/hide) cho ca STA + AP
- [x] Bo nut "Ngat" — dung dropdown mode thay the
- [x] Chi 1 nut "Luu nhap" trong WiFi card, "Ap dung" dung banner chung
- [x] WiFi status card tren trang Trang thai (mode/STA signal+SSID/IP/AP)
- [x] Kenh truyen + WiFi status chuyen tu .grid sang .cf (can cot deu)
- [x] Label/value styling: label #64748b, value #e2e8f0 + semi-bold
- [x] LAN/WAN → chi giu ETH WAN, bo LAN khoi view
- [x] Metric tach ra card rieng "Uu tien mang"
- [x] Responsive mobile: .cf 2 col, .grid 1 col khi <520px
- [x] 0 warnings (cargo fix + #[allow(dead_code)] cho code du phong)

### Session 3 — UI overhaul + Network apply
- [x] NTP: toggle inline heading, dynamic server inputs (2 col), + Them/- Xoa cuoi
- [x] NTP: luu thang flash (uci commit system), k can qua apply flow
- [x] NTP: nut "Dong bo ngay" + "Luu NTP" ngang nhau
- [x] Datetime hien thi tren trang Trang thai (backend `date` command)
- [x] Tab "Cau hinh" → "Truyen thong", tab "Du lieu" gop vao tab "UART"
- [x] Thu tu tab: Trang thai → Truyen thong → UART → Mang → Dinh tuyen → He thong
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

### Session 4 — System maintenance + Upgrade
- [x] Dynamic WAN discovery: `ip route show default` parse toan bo WAN interface
- [x] Maintenance module: backup/restore config, factory reset, restart
- [x] Upgrade: upload IPK file + remote upgrade tu URL
- [x] Upgrade check: fetch manifest JSON, so sanh version, hien thi changelog
- [x] Checksum verify (SHA256) cho remote upgrade
- [x] UPGRADING guard (AtomicBool) chong concurrent upgrade
- [x] System tab: version/build info, backup/restore, factory reset, restart, upgrade
- [x] Password change API (POST /api/password)

---

## TODO (con lai)
- [ ] Deploy test len thiet bi thuc te
- [ ] Test 4 mode WiFi tren device
- [ ] Cleanup: bo `handle_connect`/`handle_disconnect` trong wifi.rs + routes tuong ung trong server.rs + `connectWifi`/`disconnectWifi` trong frontend (da thay bang `saveWifiMode` voi 4 mode dropdown)

---

## Ghi chu ky thuat
- STA+AP chay OK tren MT7628 — cung phy0, AP lock channel theo STA
- radio0.country=VN, band=2g — can giu
- UCI draft/apply: save RAM → "Ap dung" commit flash + network reload/wifi reload
- Network apply: `ubus call network reload` (netifd diff, gian doan toi thieu)
- WiFi apply: `wifi reload` (rieng, netifd khong quan ly WiFi)
- NTP: commit truc tiep, khong can draft/apply flow
- Dynamic WAN: `ip route show default` → parse dev+metric → ho tro ETH/WiFi/4G
- Upgrade: file IPK la ar archive, bat dau bang "!<arch>"
- Remote upgrade: fetch manifest JSON → download IPK → verify SHA256 → opkg install
- Session: token 32 hex chars tu /dev/urandom, max 4 session, TTL 24h
- WebSocket: broadcast channel capacity 64, idle timeout 120s, max connections configurable
