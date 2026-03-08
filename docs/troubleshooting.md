# ugate - Xử lý sự cố

## Known Issues (MIPS + Rust)

Refer to: `mips-rust-notes/bugs-and-gotchas.md`

### Issue 1: MIPS 32-bit không hỗ trợ AtomicU64

**Triệu chứng:** Compile error
```
error[E0432]: cannot find crate `std` in `no_std` code
error: AtomicU64 not available on this platform
```

**Nguyên nhân:** MIPS 24KEc là 32-bit, không có 64-bit atomic instructions

**Fix:** ugate sử dụng `AtomicU32` cho counter. Nếu cần 64-bit counter, dùng `Mutex<u64>`

**Status:** ✓ Fixed in code

---

### Issue 2: ioctl Type Mismatch

**Triệu chứng:** Compile error
```
error: mismatched types
  expected: `libc::Ioctl`
  found: `u32`
```

**Nguyên nhân:** `libc::Ioctl` = `i32` trên MIPS, `c_ulong` trên x86_64

**Fix:** Cast constants to `libc::Ioctl`:
```rust
const GPIO_GET_LINEHANDLE_IOCTL: libc::Ioctl = 0xC16CB403u32 as libc::Ioctl;
```

**Status:** ✓ Fixed in `gpio.rs`

---

### Issue 3: WebSocket not Connecting

**Triệu chứng:**
- Browser console: `WebSocket connection failed`
- No WS data received
- Network tab shows 101 response but connection doesn't persist

**Nguyên nhân:** tiny-http `Request::upgrade()` gửi 101 nhưng KHÔNG tự thêm header `Sec-WebSocket-Accept`. Browser yêu cầu header này theo RFC 6455.

**Fix:** Calculate accept key and add to response:
```rust
let accept_key = tungstenite::handshake::derive_accept_key(ws_key.as_bytes());
let response = tiny_http::Response::empty(101)
    .with_header(Header::from_bytes("Connection", "Upgrade").unwrap())
    .with_header(Header::from_bytes("Sec-WebSocket-Accept", accept_key.as_bytes()).unwrap());
let stream = request.upgrade("websocket", response);
```

**Status:** ✓ Fixed in `web/server.rs`

---

### Issue 4: WebSocket Data Not Received

**Triệu chứng:**
- WebSocket connects but client doesn't receive status/UART data
- `ws.send()` succeeds but client-side `onmessage` never fires

**Nguyên nhân:** `ws.read()` blocking vô thời hạn trên tiny-http stream (Box<dyn ReadWrite> không có set_nonblocking)

**Fix:** Single-thread loop: send broadcast first, then short sleep, skip client read
```rust
loop {
    // Send pending broadcast messages
    while let Ok(data) = broadcast_rx.try_recv() {
        ws.send(Message::Text(data)).ok();
    }
    // Short sleep instead of blocking read
    std::thread::sleep(Duration::from_millis(100));
}
```

**Status:** ✓ Fixed in `web/ws.rs`

---

### Issue 5: Frontend Loses Session on F5

**Triệu chứng:**
- Reload page (F5) → must login again
- Cookie still present in browser
- Expected: Skip login and show dashboard

**Nguyên nhân:** JS initializes page to login without checking session validity

**Fix:** On page load, call `/api/status` — if 200 OK, skip login
```javascript
async function checkSession(){
  try {
    const r = await fetch('/api/status');
    if (r.ok) {
      S.page = 'status';
      connectWS();
    }
  } catch(_) {}
  render();
}
checkSession();
```

**Status:** ✓ Fixed in frontend Vue.js code

---

### Issue 6: Deploy Script False Negative

**Triệu chứng:**
```
❌ ugate không chạy được!
```
But SSH shows process running, Web UI accessible

**Nguyên nhân:** MIPS 580MHz khởi động Tokio + bind port mất 5-8 giây. `sleep 2` không đủ

**Fix:** Increase sleep before pgrep check:
```bash
sleep 8  # Was: sleep 2
if pgrep ugate > /dev/null; then
    echo "✓ Service started"
fi
```

**Status:** ✓ Fixed in `deploy.sh` (line 127)

---

### Issue 7: rumqttc AsyncClient Panics

**Triệu chứng:**
- MQTT panic or hang on startup
- Tokio runtime not responsive
- Error log: `[MQTT] thread panicked`

**Nguyên nhân:** rumqttc::AsyncClient problematic on MIPS (epoll quirks, async runtime incompatibility)

**Fix:** Use sync `rumqttc::Client` in `std::thread::spawn`
```rust
std::thread::spawn(move || {
    let (client, mut connection) = Client::new(opts, 10);
    // Recv from channel, publish synchronously
});
```

**Status:** ✓ Fixed in `channels/mqtt.rs`

---

### Issue 8: Cookie Header Case Sensitivity

**Triệu chứng:**
- Auth check fails dù cookie đúng
- GET `/api/config` returns 401 Unauthorized
- Browser sends cookie but server rejects

**Nguyên nhân:** tiny-http headers case-sensitive. Browser gửi `Cookie` nhưng code check `cookie`

**Fix:** Check both cases:
```rust
let needs_auth = /* ... */;
if needs_auth {
    let cookie = request.headers()
        .iter()
        .find(|h| h.field.as_str() == "Cookie" || h.field.as_str() == "cookie")
        .map(|h| h.value.as_str().to_string());
}
```

**Status:** ✓ Fixed in `web/server.rs`

---

## Common Problems

### Problem: Can't Connect to Device via SSH

**Check:**
```bash
# Is device reachable?
ping 192.168.2.171

# Is SSH open?
ssh -v root@192.168.2.171

# Check SSH key permissions
ls -la ~/.ssh/id_rsa
# Should be: -rw------- (600)
```

**Fix:**
```bash
# Fix SSH key permissions
chmod 600 ~/.ssh/id_rsa

# Or use password auth
ssh -o PubkeyAuthentication=no root@192.168.2.171
```

### Problem: UART Port Not Found

**Triệu chứng:**
```
[UART] Error: Cannot open /dev/ttyS1: No such file or directory
```

**Check:**
```bash
# On device: list available UART devices
ssh root@192.168.2.171 ls -la /dev/tty*
# Should show: ttyS0, ttyS1, etc.

# Check device tree overlay
ssh root@192.168.2.171 dmesg | grep -i uart

# Check config
ssh root@192.168.2.171 uci get ugate.@uart[0].port
```

**Fix:**
1. Verify UART enabled in OpenWrt device tree
2. Update config to correct port
3. Restart service

### Problem: Web UI Not Accessible

**Symptoms:**
```bash
curl http://device:8888/
# Connection refused
```

**Check:**
```bash
# Is process running?
ssh root@device pgrep ugate

# Is port listening?
ssh root@device netstat -tlnp | grep 8888

# Check logs
ssh root@device logread | tail -50 | grep ugate
```

**Fixes:**
```bash
# Restart service
ssh root@device /etc/init.d/ugate restart

# Check if port is in use
ssh root@device netstat -tlnp | grep :8888

# If stuck, kill process
ssh root@device pkill -9 ugate
ssh root@device /etc/init.d/ugate start

# Check firewall
ssh root@device ufw status
ssh root@device iptables -L -n | grep 8888
```

### Problem: MQTT Connection Fails

**Triệu chứng:**
```
[MQTT] Error: Connection failed: network error
[MQTT] Retrying in 10s...
```

**Check:**
```bash
# Test broker connectivity
ssh root@device timeout 5 nc -zv broker.emqx.io 8883

# Check DNS
ssh root@device nslookup broker.emqx.io

# Check firewall outbound
ssh root@device netstat -tn | grep 8883
```

**Fixes:**
```bash
# Verify config
ssh root@device uci show ugate.@mqtt[0]

# Test with plaintext (no TLS)
ssh root@device uci set ugate.@mqtt[0].tls=0
ssh root@device uci set ugate.@mqtt[0].port=1883
ssh root@device uci commit ugate
ssh root@device /etc/init.d/ugate restart

# Or use different broker
ssh root@device uci set ugate.@mqtt[0].broker=test.mosquitto.org
ssh root@device uci commit ugate
ssh root@device /etc/init.d/ugate restart
```

### Problem: GPIO Control Not Working

**Triệu chứng:**
```
curl -X POST http://device:8888/api/gpio/17 \
  -H "Cookie: session=abc" \
  -d '{"value":1}'

# Response: {"success": false, "error": "..."}
```

**Check:**
```bash
# Is pin number valid?
ssh root@device cat /etc/config/ugate | grep gpio

# Check GPIO chip access
ssh root@device ls -la /dev/gpiochip*
# Should exist: /dev/gpiochip0

# Check kernel module
ssh root@device lsmod | grep gpio

# Verify ioctl access
ssh root@device getfacl /dev/gpiochip0
```

**Fixes:**
```bash
# Add ugate to gpio group (if available)
ssh root@device usermod -aG gpio ugate 2>/dev/null || true

# Run as root (default)
ssh root@device /etc/init.d/ugate stop
ssh root@device nohup /usr/bin/ugate &

# Or rebuild without GPIO support (temporary)
ssh root@device uci set ugate.@gpio[0].pins=''
ssh root@device uci commit ugate
ssh root@device /etc/init.d/ugate restart
```

### Problem: High CPU Usage

**Symptoms:**
```bash
ssh root@device top
# ugate using 80-90% CPU
```

**Causes:**
1. Tight loop in UART reader (no frame timeout)
2. WebSocket send loop not sleeping
3. MQTT reconnect loop

**Fixes:**
```bash
# Check UART frame timeout (should be >20ms)
ssh root@device uci get ugate.@uart[0].frame_timeout_ms

# Increase if low
ssh root@device uci set ugate.@uart[0].frame_timeout_ms=100
ssh root@device uci commit ugate
ssh root@device /etc/init.d/ugate restart

# Check MQTT logs
ssh root@device logread | grep MQTT

# If reconnect loop, check broker connectivity
ping broker.emqx.io
```

### Problem: Memory Leak / OOM

**Symptoms:**
```bash
ssh root@device free
# Available memory decreasing over time
```

**Check:**
```bash
# Monitor memory
ssh root@device while true; do free; sleep 5; done

# Check process memory
ssh root@device ps aux | grep ugate
# RSS column increasing?

# Check for stuck connections
ssh root@device netstat -tn | grep 8888 | wc -l
```

**Fixes:**
```bash
# Restart service (temporary fix)
ssh root@device /etc/init.d/ugate restart

# Check for broadcast channel overflow
# (auto-skip lagged messages, but may accumulate)

# Reduce WS max connections
ssh root@device uci set ugate.@web[0].max_ws_connections=2
ssh root@device uci commit ugate
ssh root@device /etc/init.d/ugate restart

# Disable unused features
ssh root@device uci set ugate.@mqtt[0].enabled=0
ssh root@device uci set ugate.@tcp[0].enabled=0
ssh root@device uci commit ugate
ssh root@device /etc/init.d/ugate restart
```

## Debugging

### Enable Verbose Logging

```bash
# Run in foreground with stderr
ssh root@device /usr/bin/ugate 2>&1 | grep -E 'UART|MQTT|WS|GPIO'
```

### Check Live Status

```bash
# Monitor logs in real-time
ssh root@device logread -f | grep ugate

# Check process info
ssh root@device ps aux | grep ugate

# Monitor network
ssh root@device watch -n 1 'netstat -tn | grep 8883'
```

### Capture UART Data

```bash
# Monitor UART data (if accessible)
ssh root@device strace -p $(pgrep ugate) -e trace=read,write 2>&1 | grep ttyS

# Or monitor via Web API
curl -s http://device:8888/api/status | jq .stats
```

## Recovery

### Full Reset

```bash
# Stop service
ssh root@device /etc/init.d/ugate stop

# Remove binary
ssh root@device rm /usr/bin/ugate

# Remove config
ssh root@device rm /etc/config/ugate

# Redeploy
./deploy.sh
```

### Factory Reset Config

```bash
# Restore defaults
ssh root@device rm /etc/config/ugate
ssh root@device /etc/init.d/ugate restart
# Will use hardcoded defaults
```

### Rollback Binary

```bash
# If backup exists
ssh root@device cp /usr/bin/ugate.old /usr/bin/ugate
ssh root@device /etc/init.d/ugate restart

# Or recompile and redeploy
./deploy.sh
```

## WebSocket (ws.rs) Troubleshooting

**Issue:** No real-time updates in Web UI
1. Check: Browser console for WS connection errors
2. Verify: Session auth token is valid (check /api/status)
3. Test: `wscat -c ws://device:8888/ws` from Linux
4. Check: Firewall allows WebSocket on port 8888

**Issue:** WebSocket drops frequently
1. Increase: `tcp_keepalive` on router
2. Check: Device CPU/memory load (free, top commands)
3. Reduce: WebSocket message frequency if device is slow

## Phase 7+ Network Configuration Troubleshooting

### WiFi Mode Switching Issues

**Issue:** WiFi mode change fails with error "Failed to set mode"
1. Check: `uci show wireless` — verify `wwan` and `default_radio0` interfaces exist
2. Verify: Mode value is valid (STA, AP, STA+AP, Off)
3. Test: Manual UCI change: `uci set wireless.wwan.disabled=0; uci set wireless.default_radio0.disabled=1; uci commit`
4. Logs: `logread | grep -i wifi` or `logread | grep -i wireless`

**Issue:** WiFi connection drops after mode switch
1. Wait: 2-3 seconds for interface to stabilize after mode change
2. Check: Radio is not disabled globally: `uci show wireless.radio0.disabled`
3. Verify: STA SSID and password are correct in config
4. Test: Try mode switch without STA connection first (AP-only mode)

**Issue:** STA+AP mode not working as expected
1. Verify: Both interfaces enabled: `uci show wireless.wwan.disabled` (should be 0)
2. Verify: AP SSID visible: `iw phy0 channels` (see if country/channel valid)
3. Check: IP addresses assigned: `ifconfig phy0-sta0 phy0-ap0 eth0.1`
4. Logs: Check for conflicts: `logread | grep -E "wwan|ap0"`

### Network Configuration Draft/Apply Issues

**Issue:** Network changes not persisting after apply
1. Check: `/api/network/apply` returned success (HTTP 200)
2. Verify: `uci changes network` shows empty (changes were committed)
3. Confirm: `cat /etc/config/network` has new values
4. Reboot: If still no persistence, may be filesystem issue

**Issue:** Draft/Apply button disabled in Web UI
1. Check: No pending changes: `uci changes` returns empty
2. Verify: All required fields are filled in form
3. Test: Make simple change (e.g., add/remove space in IP)
4. Note: Some changes apply immediately without draft/apply (NTP, routing)

**Issue:** Interface doesn't reconnect after network config change
1. Check: `ubus call network reload` succeeded
2. Verify: Interface got IP: `ip addr show dev <interface>`
3. Test: Manual reload: `ubus call network reload` on device
4. Wait: netifd may take 5-10s for full reload
5. Logs: `logread | grep -i network` or `logread | grep -i dhcp`

### NTP & Time Sync Issues

**Issue:** NTP sync fails, device time incorrect
1. Check: Device can reach NTP server: `ping pool.ntp.org`
2. Verify: NTP servers configured: `uci show system.ntp`
3. Test: Manual sync: `ntpd -q` on device
4. Fallback: HTTP Date header used if ntpd unavailable
5. Logs: `logread | grep -i ntp` or `logread | grep -i time`

**Issue:** Timezone changes not reflected
1. Verify: `/etc/config/system.@system[0].timezone` has valid zone
2. Test: Manual set: `uci set system.@system[0].timezone=Asia/Ho_Chi_Minh`
3. Check: Date format uses correct zone: `date`
4. Note: Requires system reboot or ntpd restart to apply

### Static Routing Issues

**Issue:** Static route not working
1. Verify: Route exists: `ip route show | grep <destination>`
2. Check: UCI entry: `uci show network.route` has correct dest/gateway/dev
3. Test: Manual add: `ip route add <dest> via <gateway> dev <interface>`
4. Confirm: Target is reachable from LAN: `ping <destination>`
5. Logs: `logread | grep -i route`

**Issue:** Multiple routes conflict
1. Check: Metric values are correct (lower = higher priority)
2. Verify: No overlapping destinations
3. Test: Remove one route and test again
4. Use: `ip route show` to verify kernel route table

## Toolbox (toolbox.rs) Troubleshooting

**Issue:** Toolbox commands fail
1. Verify: `/root` directory is writable
2. Check: ugate process has proper permissions (ps aux | grep ugate)
3. Test: `uci show` command manually on device
4. Logs: `logread | grep toolbox` for errors

## Syslog (syslog.rs) Troubleshooting

**Issue:** No logs appearing in Web UI
1. Verify: Syslog daemon is running: `ps aux | grep syslog`
2. Check: `/dev/log` socket exists: `ls -la /dev/log`
3. Test: Send test message: `logger -t test "hello"`
4. Check: Read permissions on log files: `ls -la /var/log/`

## Session Authentication (auth.rs) Troubleshooting

**Issue:** "Invalid session" after login
1. Check: Device time (NTP sync): `date`
2. Verify: Password is correct in /etc/config/ugate [web] section
3. Clear: Browser cookies and try again
4. Max sessions: Only 4 concurrent sessions allowed

**Issue:** Login rate limiting triggered
1. Wait: 2 seconds after failed login attempt
2. Check: No brute-force attempt in network logs
3. Reset: By restarting service (kills all sessions)

## Support

**For issues not listed here:**
1. Check logs: `logread | grep ugate`
2. Verify config: `cat /etc/config/ugate`
3. Test API: `curl -X GET http://device:8888/api/status`
4. Check hardware: `lsusb`, `lsmod`
5. Monitor process: `ps aux | grep ugate`, `free`, `top`
6. Report with: config file, device logs (logread output), reproduce steps
