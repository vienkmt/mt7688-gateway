# Code Review: wifi.rs + status.rs

**Reviewer:** code-reviewer | **Date:** 2026-03-08
**Files:** `ugate/src/web/wifi.rs` (209 LOC), `ugate/src/web/status.rs` (206 LOC)
**Focus:** Security (command injection), error handling, memory, edge cases

## Overall Assessment

Good quality for an embedded target. Uses `Command::new()` with `.args()` throughout (no shell invocation), which eliminates classic command injection. UCI values are passed as arguments, not interpolated into shell strings. Several medium-priority issues worth addressing.

---

## Critical Issues

**None found.** No command injection vectors. UCI wrapper uses `Command::new("uci").args(["set", &arg])` which passes arguments directly to execve, not through a shell. WiFi SSID/password values cannot escape the argument boundary.

---

## High Priority

### H1. WiFi password exposed in status API (Information Disclosure)

**File:** `wifi.rs:65, 80, 91, 93`
`handle_status()` returns `sta_key` and `ap_key` (WiFi passwords) in the JSON response. Any authenticated user (or unauthenticated if auth is weak) can read all WiFi passwords.

**Impact:** Password disclosure. If the web UI is exposed on WAN or auth is bypassed, all WiFi credentials leak.

**Fix:** Return masked passwords or omit them entirely:
```rust
// Instead of returning the actual key:
json_escape(&sta_key)
// Return a masked indicator:
if sta_key.is_empty() { "" } else { "••••••••" }
```

### H2. `json_err()` does not escape the error message

**File:** `mod.rs:24`
`json_err` uses `format!(r#"{{"error":"{}"}}"#, msg)` but `msg` is not passed through `json_escape()`. If an error message contains quotes or backslashes (e.g., from UCI stderr), it produces malformed JSON.

**Impact:** Broken JSON responses, potential XSS if error messages are rendered in browser.

**Fix:**
```rust
pub(crate) fn json_err(code: u16, msg: &str) -> Resp {
    tiny_http::Response::from_string(format!(r#"{{"error":"{}"}}"#, json_escape(msg)))
```

### H3. `jval()` JSON parser does not handle escaped quotes in values

**File:** `mod.rs:37`
`jval()` finds the first `"` after the opening quote to determine value end. Input like `{"ssid":"test\"evil"}` will parse incorrectly, returning `test\` instead of `test"evil`.

**Impact:** Incorrect parsing of SSIDs or passwords containing escaped quotes. Could cause UCI to receive truncated values.

**Fix:** For this embedded context, document the limitation. If fixing: scan forward skipping `\"` sequences before finding the closing quote.

---

## Medium Priority

### M1. No input validation on `encryption` field in `handle_connect`

**File:** `wifi.rs:139`
User-supplied `encryption` value is passed directly to `Uci::set("wireless.wwan.encryption", &encryption)`. While not a command injection (args-based), it allows setting invalid UCI encryption values (e.g., arbitrary strings), which could break WiFi configuration.

**Fix:** Validate against allowed values:
```rust
let valid_enc = ["none", "psk", "psk2", "psk-mixed", "sae", "sae-mixed"];
let encryption = jval(body, "encryption")
    .filter(|e| valid_enc.contains(&e.as_str()))
    .unwrap_or_else(|| "psk2".into());
```

### M2. No validation on `ap_channel` in `set_ap_config`

**File:** `wifi.rs:189-191`
`ap_channel` accepts any string. Could set invalid channel values in UCI.

**Fix:** Validate as integer in range 1-14 (2.4GHz) or use "auto":
```rust
if let Some(ch) = jval(body, "ap_channel") {
    if ch == "auto" || ch.parse::<u8>().map_or(false, |n| (1..=14).contains(&n)) {
        Uci::set("wireless.radio0.channel", &ch).ok();
    }
}
```

### M3. Mutex poison panic in `read_cpu_percent`

**File:** `status.rs:65`
`self.cpu_prev.lock().unwrap()` will panic if a previous thread panicked while holding the lock. On an embedded device, this crashes the entire gateway.

**Fix:** Handle the poisoned lock:
```rust
let mut prev_lock = self.cpu_prev.lock().unwrap_or_else(|e| e.into_inner());
```

### M4. `handle_scan()` allocates unbounded Vec for networks

**File:** `wifi.rs:17`
In theory, a WiFi scan could return many networks. Each network entry is a heap-allocated String pushed to a Vec. On 64MB RAM this is unlikely to be a problem in practice, but worth noting.

**Fix (optional):** Cap at 50 networks: `if networks.len() >= 50 { break; }`

### M5. Silently discarding UCI errors with `.ok()`

**File:** `wifi.rs:107-108, 141, 145-146, etc.`
Most `Uci::set()` calls use `.ok()` to discard errors. If UCI fails (e.g., disk full, config corrupted), the user gets a success response while the config was not actually saved.

**Fix:** At minimum, log errors. For critical paths like `handle_connect`, propagate the error:
```rust
if let Err(e) = Uci::set("wireless.wwan.disabled", "0") {
    log::warn!("UCI set failed: {}", e);
}
```

---

## Low Priority

### L1. `read_datetime()` spawns a process instead of using Rust

**File:** `status.rs:132`
Spawns `date` command every status poll. Could use `libc::time()` + manual formatting to avoid process spawn overhead.

### L2. `to_status_json` format string is very long (single line)

**File:** `status.rs:88-117`
The 30-field format string is hard to read/maintain. Consider building it in sections or using a helper macro.

### L3. CPU idle calculation excludes iowait

**File:** `status.rs:203`
Comment says "idle + iowait if available" but only `vals[3]` (idle) is used. For accuracy on embedded: `let idle = vals[3] + vals.get(4).copied().unwrap_or(0);`

---

## Positive Observations

1. **No shell injection surface** - All external commands use `Command::new().args()`, never `sh -c`. This is the correct approach.
2. **Good use of atomics** - `SharedStats` uses `AtomicU32`/`AtomicU8` (not `AtomicU64`, which is known to fail on MIPS 32-bit). Well-designed for the target.
3. **Memory-conscious design** - No serde_json dependency, manual JSON formatting keeps binary small.
4. **Proper `saturating_sub`** - Used in CPU and RAM calculations to prevent underflow.
5. **Clean separation** - WiFi handlers are well-organized by function (scan/status/mode/connect/disconnect).
6. **`json_escape()` is solid** - Handles quotes, backslashes, newlines, carriage returns, tabs, and strips control chars.

---

## Recommended Actions (Priority Order)

1. **H2** - Escape error messages in `json_err()` (1-line fix, prevents malformed JSON)
2. **H1** - Stop exposing WiFi passwords in status endpoint
3. **M1/M2** - Validate encryption and channel values
4. **M3** - Handle mutex poison in CPU reading
5. **M5** - Log UCI errors instead of silently discarding
6. **H3** - Document or fix `jval()` escaped-quote limitation

---

## Metrics

| Metric | Value |
|--------|-------|
| Total LOC reviewed | ~415 + 74 (mod.rs) + 147 (uci.rs) |
| Command injection vectors | 0 |
| Error handling gaps | 5 (mostly `.ok()` discards) |
| Input validation gaps | 2 (encryption, channel) |
| Potential panics | 1 (mutex poison) |
| Information disclosure | 1 (password in API) |

## Unresolved Questions

- Is the WiFi status API behind authentication? If not, H1 becomes Critical.
- Is there rate limiting on `/api/wifi/scan`? Each scan spawns `iwinfo` which takes several seconds and blocks the interface.
- Should `handle_disconnect` also set `wireless.wwan.disabled=1`? Currently it only clears SSID/key but leaves interface enabled.
