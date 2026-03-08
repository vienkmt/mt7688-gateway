# Code Review: netcfg.rs, ws.rs, mod.rs

**Reviewer:** code-reviewer | **Date:** 2026-03-08
**Files:** `ugate/src/web/netcfg.rs` (351 LOC), `ugate/src/web/ws.rs` (106 LOC), `ugate/src/web/mod.rs` (74 LOC)
**Related:** `ugate/src/uci.rs`, `ugate/src/web/server.rs`, `ugate/src/web/wifi.rs`, `ugate/src/web/auth.rs`

## Overall Assessment

Solid embedded-appropriate code. Good use of `is_safe_identifier()` for UCI key injection prevention and `is_valid_ipv4()` for IP validation. Manual JSON approach is pragmatic for binary size. Several security and robustness issues found, mostly medium severity.

---

## Critical Issues

### C1. Session Token Generation is Predictable (auth.rs, affects all API)

```rust
fn generate_token() -> String {
    let ts = std::time::SystemTime::now().duration_since(...).as_nanos();
    let pid = std::process::id();
    format!("{:016x}{:08x}", ts, pid)
}
```

**Impact:** PID is constant for daemon lifetime. Timestamp is guessable. An attacker who knows approximate login time can brute-force the token space. On an IoT device exposed to LAN, this is realistic.

**Fix:** Use `/dev/urandom` directly:
```rust
fn generate_token() -> String {
    let mut buf = [0u8; 16];
    std::fs::File::open("/dev/urandom")
        .and_then(|mut f| { use std::io::Read; f.read_exact(&mut buf) })
        .unwrap_or_else(|_| { /* fallback to timestamp */ });
    buf.iter().map(|b| format!("{:02x}", b)).collect()
}
```

### C2. WebSocket Bypasses Authentication (server.rs:38-41)

```rust
if url == "/ws" {
    handle_ws_upgrade(request, &ws_manager);  // no auth check
    continue;
}
```

**Impact:** Any unauthenticated client can open a WebSocket and receive all broadcast data (sensor readings, device status). The comment says "khong can auth cho don gian" but this is a real data leak on shared networks.

**Fix:** Add session check before upgrade, or at minimum check Origin header.

---

## High Priority

### H1. NTP Timezone/Zonename Not Validated (netcfg.rs:165-170)

```rust
if let Some(tz) = jval(body, "timezone") {
    Uci::set("system.@system[0].timezone", &tz).ok();
}
if let Some(zn) = jval(body, "zonename") {
    Uci::set("system.@system[0].zonename", &zn).ok();
}
```

`Uci::set` passes the value as `key=value` to CLI. While `Command::new("uci").args(["set", &arg])` uses args (not shell), UCI values containing newlines or special chars could corrupt the config file. The `jval()` parser stops at `"` so embedded quotes are unlikely, but no explicit validation exists.

**Fix:** Validate timezone string (alphanumeric, `/`, `-`, `+`, digits) similar to NTP server validation.

### H2. WebSocket Connection Counter Race Condition (ws.rs:53-57)

```rust
if manager.connections.load(Ordering::Relaxed) >= manager.max_connections {
    return;  // reject
}
manager.connections.fetch_add(1, Ordering::Relaxed);  // TOCTOU gap
```

Between `load` and `fetch_add`, another thread could also pass the check, exceeding `max_connections`. With `max_connections=4` on 64MB device, this is low risk but still incorrect.

**Fix:** Use `fetch_add` first, then check and rollback:
```rust
let prev = manager.connections.fetch_add(1, Ordering::Relaxed);
if prev >= manager.max_connections {
    manager.connections.fetch_sub(1, Ordering::Relaxed);
    return;
}
```

### H3. WebSocket Thread Never Exits on Idle Client (ws.rs:72-104)

The WS loop has no read from client and no ping/pong. If a client connects and goes silent, the thread loops forever (sleeping 100ms per iteration). With `max_connections=4`, 4 idle clients permanently exhaust the WS pool.

**Impact:** Denial of service -- no new WS clients can connect.

**Fix:** Add an idle timeout counter. If no broadcast sent for N seconds and no client message received, close the connection:
```rust
let mut idle_count = 0;
// in loop:
if sent == 0 {
    idle_count += 1;
    if idle_count > 3000 { break; } // ~5 min at 100ms
    std::thread::sleep(Duration::from_millis(100));
} else {
    idle_count = 0;
}
```

### H4. `json_err` Does Not Escape Message (mod.rs:23-29)

```rust
pub(crate) fn json_err(code: u16, msg: &str) -> Resp {
    tiny_http::Response::from_string(format!(r#"{{"error":"{}"}}"#, msg))
```

If `msg` contains quotes or backslashes, the JSON output is malformed. Currently all callers use static strings, but this is fragile.

**Fix:** Use `json_escape(msg)` in the format string.

---

## Medium Priority

### M1. `json_str_array` Does Not Escape Items (netcfg.rs:336-342)

```rust
let inner: Vec<String> = items.iter().map(|s| format!("\"{}\"", s)).collect();
```

DNS server values from UCI are not escaped. If UCI returns a value with quotes (unlikely but possible from manual config editing), this produces malformed JSON.

**Fix:** `format!("\"{}\"", json_escape(s))`

### M2. `handle_apply` Spawns Threads Without Join (netcfg.rs:99-112)

Two `std::thread::spawn` calls with 1-second sleep each. These threads are detached. If `handle_apply` is called rapidly, threads accumulate. Each thread is lightweight but still consumes a stack (default 2MB on Linux, 128KB on musl).

**Recommendation:** Use a flag or debounce to prevent rapid re-applies, or reuse a single worker thread.

### M3. `jval` Parser Finds First Match, Not Nested Match (mod.rs:32-45)

`jval(body, "key")` searches for `"key":` anywhere in the JSON string. For nested JSON like `{"a":{"key":"inner"},"key":"outer"}`, it finds the inner one first.

This works only because current callers parse flat or known-structured JSON. If the structure changes, this will silently return wrong values. Already partially mitigated in `server.rs` by `section_body()` extracting sub-objects first.

**Note:** Not a bug today, but a latent fragility. Document the limitation.

### M4. `netmask_to_cidr` Gives Wrong Results for Invalid Netmasks (netcfg.rs:345-350)

`255.255.128.128` would return 18 (count of 1-bits) even though it's not a valid contiguous netmask. The IP is validated by `is_valid_ipv4` but netmask contiguity is not checked.

**Impact:** Low -- would create a wrong route, but input is already validated as a proper IP and typically comes from a dropdown UI.

### M5. WiFi SSID/Password Passed to UCI Without Sanitization (wifi.rs:142-146)

```rust
Uci::set("wireless.wwan.ssid", &ssid).ok();
Uci::set("wireless.wwan.key", &password).ok();
```

SSIDs can contain special characters (spaces, quotes, unicode). UCI handles quoting internally for `uci set`, and `Command::new().args()` prevents shell injection. However, SSIDs with single quotes may cause UCI parsing issues.

**Risk:** Low, UCI CLI handles quoting, but worth testing with edge-case SSIDs.

### M6. AP Channel Not Validated (wifi.rs:189-191)

```rust
if let Some(ch) = jval(body, "ap_channel") {
    Uci::set("wireless.radio0.channel", &ch).ok();
}
```

No validation that channel is a valid WiFi channel number (1-14 for 2.4GHz). An invalid value could cause `wifi reload` to fail silently.

---

## Low Priority

### L1. `_cmd_tx` Cloned But Never Used (ws.rs:69)

```rust
let _cmd_tx = manager.cmd_tx.clone();
```

Dead code. The comment says "Client gui lenh qua HTTP API thay vi WS" -- remove the clone or implement client-to-server WS commands.

### L2. `Ordering::Relaxed` on Connection Counter (ws.rs)

`Relaxed` ordering is technically sufficient for a counter but `Ordering::SeqCst` would be safer for correctness on MIPS (which has weak memory ordering). The performance difference is negligible at 4 max connections.

### L3. `handle_change_password` is a No-Op (server.rs:460-468)

```rust
fn handle_change_password(...) {
    let _body = read_body(request);
    // TODO: Parse va save password qua UCI
```

Returns `{"ok":true}` without actually changing anything. The client thinks password changed successfully.

**Fix:** Either implement or return 501 Not Implemented.

### L4. Missing `Upgrade: websocket` Header in WS Handshake Response (server.rs:224-232)

The 101 response includes `Connection: Upgrade` but not `Upgrade: websocket`. Per RFC 6455, both are required. Some clients may reject the handshake.

---

## Positive Observations

1. **Command injection prevention:** `Command::new().args()` used throughout (never `sh -c`). No shell expansion possible.
2. **Input validation:** `is_safe_identifier()` consistently applied to UCI keys. `is_valid_ipv4()` for IP addresses. Interface names whitelisted to `lan`/`wan`.
3. **Body size limit:** `read_body` caps at 4KB -- appropriate for 64MB device.
4. **Draft/apply pattern:** Network changes staged in RAM before commit -- prevents bricking on bad config.
5. **WS connection limit:** `AtomicU8` with `max_connections` prevents unbounded WS threads.
6. **Memory-conscious design:** No serde, no allocator-heavy deps, manual JSON formatting.
7. **`json_escape()`** handles quotes, backslashes, control chars -- good for output safety.

---

## Recommended Actions (Priority Order)

1. **[Critical]** Replace predictable session token with `/dev/urandom`
2. **[Critical]** Add auth check to WebSocket upgrade path
3. **[High]** Fix WS connection counter TOCTOU race
4. **[High]** Add idle timeout to WS handler to prevent thread exhaustion
5. **[High]** Use `json_escape` in `json_err` and `json_str_array`
6. **[Medium]** Validate timezone/zonename input
7. **[Medium]** Validate WiFi channel number range
8. **[Low]** Remove dead `_cmd_tx` clone in ws.rs
9. **[Low]** Add `Upgrade: websocket` header to handshake response
10. **[Low]** Return 501 from `handle_change_password` stub

## Metrics

| Metric | Value |
|--------|-------|
| Files reviewed | 3 primary + 3 related |
| Total LOC | ~531 (primary), ~757 (related) |
| Critical issues | 2 |
| High issues | 4 |
| Medium issues | 6 |
| Low issues | 4 |

## Unresolved Questions

- Is the device intended to be exposed beyond LAN? If internet-facing, C1/C2 become urgent.
- What is the expected WS client behavior -- do clients send heartbeat/ping? If not, H3 is guaranteed to occur.
- Is there rate limiting on `/api/login`? Without it, the predictable token (C1) is even more exploitable.
