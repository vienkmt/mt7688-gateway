# Post-Fix Re-Review: Web Auth & WebSocket Security

**Reviewer:** code-reviewer | **Date:** 2026-03-08
**Scope:** auth.rs, server.rs, ws.rs, mod.rs (+ maintenance.rs edge cases)
**Context:** LAN-only IoT device, MT7628 MIPS 64MB RAM

---

## Fix Verification

| ID | Fix | Status | Notes |
|----|-----|--------|-------|
| C1 | urandom token generation | PASS | 16 bytes from /dev/urandom, hex-encoded. Fallback to timestamp+pid is acceptable for LAN. |
| C2 | WS requires Cookie auth | PASS | Auth check before `handle_ws_upgrade` in server.rs:39-54. |
| H2 | Session TTL 2h + cleanup | PASS | TTL checked in both `check_session` and `create_session` (retain). |
| H3 | Login rate limit 2s cooldown | PASS | `check_rate_limit()` called before password check. `record_fail()` on bad password. |
| H5 | WS idle timeout 120s | PASS | `last_activity` updated on broadcast send, checked each loop iteration. |
| H6 | TOCTOU fix fetch_add-first | PASS | Atomic increment before check, rollback on overflow. Correct pattern. |
| H7 | json_err uses json_escape | PASS | All callers pass literal strings, but escape is correct defense-in-depth. |

All seven fixes are correctly implemented.

---

## New Issues Found

### HIGH: Missing json_escape in handle_get_config (server.rs:307-317)

`handle_get_config` formats user-controlled config values (device_name, broker, topic, client_id, username, password, url, client_host) directly into JSON via `format!()` without `json_escape()`. If any UCI config value contains a double-quote or backslash, the JSON response becomes malformed.

**Impact:** Broken JSON response, potential XSS if frontend uses innerHTML (unlikely but preventable).

**Fix:** Wrap all string fields in `json_escape()`:
```rust
json_escape(&c.general.device_name), ...
json_escape(&c.mqtt.broker), ...
// etc for all string fields
```

This is the same class of bug that H7 fixed for `json_err` -- needs the same treatment for `handle_get_config`.

### HIGH: WS send failure does not exit the loop (ws.rs:90-91)

When `ws.send()` fails (line 90), the inner loop `break`s but the outer loop continues. The connection is dead but the thread keeps looping, sleeping 100ms each iteration until idle timeout (120s). This wastes a thread and the connection slot for up to 2 minutes.

**Fix:** Set a flag or break out of the outer loop:
```rust
Ok(data) => {
    if ws.send(Message::Text(data)).is_err() {
        log::info!("[WS] Send failed, closing");
        break; // inner break -- need outer break too
    }
    ...
}
```
Simplest: use a `let mut alive = true;` flag, set `alive = false` on send error, check after inner loop.

### MEDIUM: generate_token fallback leaves 4 bytes zeroed (auth.rs:100-107)

In the fallback path (urandom unavailable), only bytes 0..12 are filled (8 from timestamp, 4 from pid). Bytes 12..16 remain zero, reducing entropy. Not critical since fallback is nearly impossible on Linux, but easy to fix.

**Fix:** Fill remaining bytes, e.g.:
```rust
buf[12..16].copy_from_slice(&(ts >> 64).to_le_bytes()[..4]);
// or just XOR-fold
```

### MEDIUM: read_exact error silently ignored (auth.rs:98)

`let _ = f.read_exact(&mut buf);` -- if urandom read fails mid-way (theoretically possible), `buf` contains partial random data mixed with zeros. The token still gets generated from partial entropy.

**Fix:** Check the result and fall through to fallback on error:
```rust
let mut f = std::fs::File::open("/dev/urandom");
let ok = f.as_mut().map(|f| f.read_exact(&mut buf)).is_ok();
if !ok { /* fallback */ }
```

### MEDIUM: validate_password does not handle escaped quotes (auth.rs:79-91)

The simple JSON parser for `{"password":"xxx"}` does not handle escaped quotes in the password value. A password containing `\"` would cause incorrect parsing. Edge case for LAN admin, but worth noting since `handle_change_password` allows setting arbitrary passwords.

### LOW: Cookie header matching is case-sensitive for value (server.rs:43,63)

The code checks `h.field.as_str() == "Cookie" || h.field.as_str() == "cookie"` which handles the two common cases. Per HTTP/1.1, header field names are case-insensitive, so `COOKIE` or `cOOKIE` would be missed. In practice, no browser sends these variants -- acceptable for LAN.

### LOW: Rate limit is global, not per-IP (auth.rs)

A single failed login blocks ALL login attempts for 2 seconds. On a LAN-only device with few users this is acceptable, but an attacker could deny legitimate admin login by continuously sending bad passwords (1 request every 1.9s keeps the lock active). Low risk for LAN context.

---

## Positive Observations

1. **TOCTOU fix is textbook correct** -- fetch_add then rollback is the right atomic pattern
2. **Session TTL checked in both read and write paths** -- no stale session can survive past TTL
3. **Mutex poison recovery** (`unwrap_or_else(|e| e.into_inner())`) -- prevents panic propagation, good for embedded
4. **read_body 4KB limit** -- appropriate OOM protection for 64MB device
5. **Upgrade guard with AtomicBool** -- clean concurrent upgrade prevention
6. **json_escape covers all JSON-unsafe characters** including control chars
7. **Test coverage** for session flow and password validation

---

## Summary

| Severity | Count | Action Required |
|----------|-------|-----------------|
| Critical | 0 | -- |
| High | 2 | Fix before next deploy |
| Medium | 3 | Fix in next iteration |
| Low | 2 | Accept for LAN context |

**Overall:** The security fixes are all correctly implemented. The main remaining gap is **missing json_escape in handle_get_config** (same bug class as H7) and the **WS send-failure thread leak**. Both are straightforward fixes.

---

## Recommended Actions

1. Add `json_escape()` to all string fields in `handle_get_config` format string
2. Break outer WS loop on send failure instead of only breaking inner loop
3. (Optional) Handle `read_exact` error in `generate_token` more explicitly
