# Code Review: auth.rs + server.rs

**Reviewer:** code-reviewer | **Date:** 2026-03-08
**Files:** `ugate/src/web/auth.rs`, `ugate/src/web/server.rs` (+ edge-case scout of `mod.rs`, `maintenance.rs`, `config.rs`)
**LOC:** ~180 (auth.rs: 101, server.rs: 485 relevant)

---

## Overall Assessment

Solid embedded-appropriate code. Good body size limits, simple session model, proper error returns. However, several security issues need attention, most critically the token generation and password handling.

---

## Critical Issues

### C1. Predictable Session Tokens (auth.rs:62-69)

`generate_token()` uses `timestamp + pid` -- both values are deterministic/guessable. An attacker who knows approximate login time can brute-force the token space.

```rust
// Current: predictable
fn generate_token() -> String {
    let ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos()).unwrap_or(0);
    let pid = std::process::id();
    format!("{:016x}{:08x}", ts, pid)
}
```

**Fix:** Use `/dev/urandom` (always available on OpenWrt):

```rust
fn generate_token() -> String {
    let mut buf = [0u8; 16];
    if let Ok(mut f) = std::fs::File::open("/dev/urandom") {
        use std::io::Read;
        let _ = f.read_exact(&mut buf);
    }
    buf.iter().map(|b| format!("{:02x}", b)).collect()
}
```

### C2. Timing Side-Channel in Password Comparison (auth.rs:54)

`&val[..end] == expected` uses short-circuit string comparison. Attacker can infer password length/content by measuring response time.

```rust
// Current: vulnerable to timing attack
return &val[..end] == expected;
```

**Fix:** Constant-time comparison:

```rust
fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() { return false; }
    a.iter().zip(b.iter()).fold(0u8, |acc, (x, y)| acc | (x ^ y)) == 0
}
```

### C3. WebSocket Bypasses Authentication (server.rs:38-41)

WebSocket endpoint `/ws` is explicitly skipped from auth check. Any unauthenticated client can connect and receive real-time data or send commands.

```rust
// Comment says "khong can auth cho don gian" -- this is a security gap
if url == "/ws" {
    handle_ws_upgrade(request, &ws_manager);
    continue;
}
```

**Fix:** Check session cookie before upgrading WebSocket, or at minimum verify cookie during WS handshake.

### C4. Password Exposed in Config API Response (server.rs:288)

`handle_get_config` returns MQTT password and web password is stored in config but the MQTT password field is serialized in the JSON response to any authenticated user.

```rust
c.mqtt.password, c.mqtt.qos,  // MQTT password sent to browser
```

**Fix:** Mask or omit sensitive fields in API responses. Return `"password":"***"` instead of the actual value.

---

## High Priority

### H1. No Session Expiry (auth.rs)

Sessions persist until server restart or eviction by new sessions. No TTL, no logout endpoint. A stolen token remains valid indefinitely.

**Fix:** Store `(token, created_at)` tuples. Check age in `check_session()`. Add `DELETE /api/logout` to invalidate.

### H2. No Rate Limiting on Login (server.rs:70-72)

No protection against brute-force login attempts. With only password auth and no rate limit, automated attacks are trivial.

**Fix:** Track failed attempts per IP (or global counter). After N failures, add delay or lock out temporarily:

```rust
static FAIL_COUNT: AtomicU32 = AtomicU32::new(0);
// In handle_login, if fail: increment and sleep(2^n seconds, max 30s)
```

### H3. handle_change_password is a No-Op (server.rs:460-468)

The endpoint exists, passes auth, but does nothing -- just logs and returns `{"ok":true}`. User thinks password changed but it is not.

**Fix:** Either implement it or return `501 Not Implemented` / remove the route.

### H4. Config JSON Output Missing json_escape (server.rs:284-294)

User-controlled strings (device_name, broker, topic, client_id, etc.) are interpolated directly into JSON without escaping. A device name containing `"` breaks JSON or enables injection.

```rust
// Vulnerable: if device_name = 'test","evil":true'
r#"{{"general":{{"device_name":"{}",..."#, c.general.device_name, ...
```

**Fix:** Use `json_escape()` (already in `mod.rs`) for all string fields:

```rust
json_escape(&c.general.device_name), c.general.interval_secs,
json_escape(&c.mqtt.broker), ...
```

### H5. RwLock::unwrap() Can Panic on Poison (auth.rs:25,40)

If any thread panics while holding the lock, subsequent `.unwrap()` calls will panic the entire server.

**Fix:** Use `.unwrap_or_else(|e| e.into_inner())` or `.read().ok()` pattern.

---

## Medium Priority

### M1. Duplicate jval() Implementation (server.rs:326-337 vs mod.rs:32-45)

`handle_set_config` defines a local `jval()` identical to the shared one in `mod.rs`.

**Fix:** Use `crate::web::jval()` instead of the local copy.

### M2. No Input Validation on Config Values (server.rs:342-408)

Parsed config values are applied without bounds checking. A baudrate of 0, negative interval, or port 99999 would be accepted.

**Fix:** Validate critical fields before applying:
- `interval_secs > 0`
- `baudrate` in allowed set (9600, 19200, 38400, 57600, 115200)
- `port` in 1..65535
- `data_bits` in 5..8

### M3. Cookie Header Match is Case-Sensitive (server.rs:50)

Checks for `"Cookie"` or `"cookie"` but HTTP headers can have mixed case like `"COOKIE"`. tiny_http may normalize this, but safer to use case-insensitive comparison.

**Fix:** Use `.field.as_str().eq_ignore_ascii_case("cookie")`.

### M4. read_body Silently Swallows Errors (server.rs:470-476)

If body read fails (broken connection), returns empty string. Handlers proceed with empty input, potentially making unintended changes.

**Fix:** Return `Result<String>` or at minimum log the error.

---

## Low Priority

### L1. content_type_json/html Duplicated (server.rs:478-484)

These helpers duplicate the logic in `mod.rs::json_resp`. Could use the shared helpers throughout.

### L2. INDEX_HTML as &str Allocates on Response

`Response::from_string(INDEX_HTML)` copies the static string into a Vec. For a large HTML file this is wasteful. Consider `Response::from_data(INDEX_HTML.as_bytes())` if the response type allows it, or pre-allocate.

### L3. Missing Upgrade WebSocket Header (server.rs:224-231)

The WS handshake response is missing the `Upgrade: websocket` header. Some clients may reject this. Only `Connection: Upgrade` and `Sec-WebSocket-Accept` are set.

---

## Edge Cases Found by Scout

1. **maintenance.rs: Unrestricted system commands** -- `handle_restart`, `handle_factory_reset`, `handle_upgrade_*` execute system commands. All behind auth, but combined with C3 (WS bypass) and predictable tokens, these become high-risk.
2. **maintenance.rs:124 comment/code mismatch** -- Comment says "IPK max 2MB" but code sets limit to `10 * 1024 * 1024` (10MB). On 64MB RAM device, a 10MB upload could cause memory pressure.
3. **maintenance.rs:257-258 same mismatch** -- Remote download also limited to 10MB with comment saying 2MB.
4. **config.rs:193** -- Default web password is `"admin"`. No forced password change on first login.
5. **handle_delete_route** (server.rs:155-156) -- Route name extracted from path without validation via `is_safe_identifier()`, potential UCI injection if malicious path is crafted.

---

## Positive Observations

- Body size limit (4KB) in `read_body` -- good for embedded target
- Session eviction (MAX_SESSIONS=4) prevents unbounded memory growth
- Proper 401 responses with JSON content-type
- Clean routing pattern with match arms
- Good error logging throughout
- IPK format validation before install
- Checksum verification for remote upgrades

---

## Recommended Actions (Priority Order)

1. **[Critical]** Replace token generation with `/dev/urandom`
2. **[Critical]** Add auth check to WebSocket upgrade
3. **[Critical]** Add constant-time password comparison
4. **[Critical]** Stop exposing MQTT password in config response
5. **[High]** Add `json_escape()` to all string interpolation in config JSON
6. **[High]** Add login rate limiting
7. **[High]** Add session TTL (e.g., 1 hour)
8. **[High]** Fix or remove handle_change_password no-op
9. **[Medium]** Remove duplicate jval(), use shared helper
10. **[Medium]** Add config value bounds validation
11. **[Low]** Fix IPK size limit comment/code mismatch in maintenance.rs

---

## Metrics

| Metric | Value |
|--------|-------|
| Type Coverage | N/A (Rust -- compiler-enforced) |
| Test Coverage | Low -- only 2 unit tests in auth.rs, 0 for server.rs |
| Linting Issues | Not checked (cross-compile target) |
| Security Issues | 4 critical, 4 high |
| Code Smells | 2 (duplicate code, no-op endpoint) |

---

## Unresolved Questions

1. Is the device accessible only on LAN or also via WAN? If WAN-exposed, all Critical items become urgent.
2. Does tiny_http normalize header case? If yes, M3 is a non-issue.
3. What is the intended behavior for `/api/password` -- should it be removed or implemented?
