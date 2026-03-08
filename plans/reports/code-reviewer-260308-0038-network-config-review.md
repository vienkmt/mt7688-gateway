# Code Review: Phase 7 - Network Configuration Module

**Date:** 2026-03-08
**Reviewer:** code-reviewer
**Build status:** PASS (cargo check -p ugate, 12 warnings — none from reviewed files)

## Scope

- `ugate/src/web/wifi.rs` — 163 lines, WiFi scan/connect/disconnect/status
- `ugate/src/web/netcfg.rs` — 266 lines, LAN/WAN, NTP, routes, interface metric
- `ugate/src/uci.rs` — 97 lines, new `get_list` + `add_list` methods
- `ugate/src/web/server.rs` — route wiring (lines 97-150)
- LOC reviewed: ~526

## Overall Assessment

Solid implementation that follows existing codebase patterns. Code is readable, appropriately sized for an embedded device, and correctly uses UCI CLI wrapper for OpenWrt config. Several security issues need attention before deploying to production.

---

## Critical Issues

### C1. UCI Command Injection via Unsanitized User Input

**Files:** `wifi.rs:96`, `netcfg.rs:168-173`, `netcfg.rs:105`, `netcfg.rs:205`

All user-supplied strings are passed directly into `Uci::set()` which constructs shell arguments as `key=value`. The UCI CLI itself provides some protection since values are passed as arguments (not shell-interpolated), but the **key path** is partially user-controlled.

**Specific vectors:**
- `wifi.rs:96` — SSID value is unvalidated; could contain shell metacharacters
- `netcfg.rs:168` — Route `name` is used to construct UCI key: `format!("network.route_{}", name)`. A name like `foo.bar` or `foo;rm` could manipulate UCI paths
- `netcfg.rs:105` — NTP server strings are unvalidated; any string is passed to `uci add_list`
- `netcfg.rs:205` — Interface name in metric handler is not whitelisted (only checked `!is_empty()`)

**Impact:** On OpenWrt, UCI commands run as root. While `Command::new("uci")` does not invoke a shell (safe from shell injection), malicious UCI key paths can corrupt `/etc/config/*` files.

**Fix:** Add input sanitization:

```rust
// For route names, interface names — alphanumeric + underscore only
fn is_safe_identifier(s: &str) -> bool {
    !s.is_empty() && s.len() <= 32
        && s.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
}

// For NTP servers — hostname/IP pattern
fn is_safe_hostname(s: &str) -> bool {
    !s.is_empty() && s.len() <= 253
        && s.chars().all(|c| c.is_ascii_alphanumeric() || c == '.' || c == '-' || c == ':')
}
```

Apply `is_safe_identifier` to: route name (netcfg.rs:156), interface name in metric handler (netcfg.rs:198), interface name in set_network (netcfg.rs:33 — already whitelisted, good).

### C2. WiFi SSID JSON Injection in Scan Response

**File:** `wifi.rs:27-28`

SSID values from `iwinfo scan` output are embedded directly into JSON strings without escaping. An SSID containing `"` or `\` will produce malformed JSON and potentially enable XSS if rendered in a browser.

```rust
// Current (vulnerable):
r#"{{"ssid":"{}","signal":{},"encryption":"{}"}}"#, ssid, signal, enc

// Fix — escape JSON special chars:
fn json_escape(s: &str) -> String {
    s.replace('\\', "\\\\").replace('"', "\\\"")
}
// Then use json_escape(&ssid) in the format string
```

Same issue in `wifi.rs:81` (status SSID) and `wifi.rs:68` (encryption string).

---

## High Priority

### H1. `handle_ntp_sync` Blocks HTTP Server Thread

**File:** `netcfg.rs:128-138`

`ntpd -q -p pool.ntp.org` can block for 10+ seconds on network timeout. Since the HTTP server runs single-threaded in `spawn_blocking`, this blocks ALL other HTTP requests during sync.

**Fix:** Spawn NTP sync in a separate thread and return immediately:
```rust
pub fn handle_ntp_sync() -> Resp {
    std::thread::spawn(|| {
        let result = Command::new("ntpd").args(["-q", "-p", "pool.ntp.org"]).status();
        if result.map(|s| s.success()).unwrap_or(false) {
            log::info!("[NTP] sync ok via ntp");
        } else {
            crate::time_sync::sync_time();
            log::info!("[NTP] sync ok via http_date");
        }
    });
    json_resp(r#"{"ok":true,"message":"sync started"}"#)
}
```

### H2. Missing Netmask Validation in `is_valid_ipv4`

**File:** `netcfg.rs:224-227`

`is_valid_ipv4` only checks that each octet parses as `u8`. This allows `0.0.0.0` as a target (unintended route) and does not validate netmask format. The function is used for both IP addresses and netmask values, but `255.255.255.0` is simply passed through without structural validation.

**Impact:** Low-medium. UCI/ip-route will reject most invalid values, but garbage data could persist in config.

### H3. No Body Size Limit in wifi.rs/netcfg.rs Handlers

**File:** `wifi.rs:87`, `netcfg.rs:31` etc.

The `read_body()` in `server.rs:423-428` limits to 4KB. But the body string is passed to handler functions — this is fine. However, verify that **all** POST routes go through `read_body()` in server.rs. Checked: yes, all POST routes in server.rs call `read_body(&mut request)` before passing to handlers. This is correct.

### H4. Duplicated Helper Functions Across Modules

**Files:** `wifi.rs:134-162` and `netcfg.rs:237-265`

`jval()`, `json_resp()`, `json_err()` are copy-pasted identically between `wifi.rs` and `netcfg.rs`. Also duplicated in `server.rs:279`.

**Fix:** Extract to a shared `web::helpers` module:
```rust
// ugate/src/web/helpers.rs
pub fn jval(json: &str, key: &str) -> Option<String> { ... }
pub fn json_resp(json: &str) -> Resp { ... }
pub fn json_err(code: u16, msg: &str) -> Resp { ... }
pub fn json_escape(s: &str) -> String { ... }
```

This violates DRY and increases binary size (~300 bytes duplicated x3 = ~900 bytes wasted, relevant for 16MB flash target).

---

## Medium Priority

### M1. Route Delete Does Not Validate Name Sufficiently

**File:** `netcfg.rs:186`

```rust
if name.is_empty() || name.contains('.') || name.contains('/') {
```

This blocks `.` and `/` but allows spaces, semicolons, brackets, and other characters that could create unexpected UCI paths. Should use the same `is_safe_identifier` recommended in C1.

### M2. `handle_set_network` Silently Ignores UCI Errors

**File:** `netcfg.rs:49-56`

All `Uci::set()` calls use `.ok()` — errors are silently discarded. If `Uci::set` for the proto succeeds but ipaddr fails, a partial config is committed and network restarts with broken config.

**Recommendation:** Collect errors and abort if any critical set fails, or at minimum log errors:
```rust
if let Err(e) = Uci::set(&format!("{}.ipaddr", prefix), &ipaddr) {
    log::error!("[NET] UCI set ipaddr failed: {}", e);
    return json_err(500, "failed to set ipaddr");
}
```

### M3. `wifi reload` vs `/etc/init.d/network restart` Inconsistency

**Files:** `wifi.rs:103` (uses `wifi reload` immediately), `netcfg.rs:76-82` (uses delayed `/etc/init.d/network restart`)

WiFi handlers spawn `wifi reload` without delay, while network config delays 2 seconds. The WiFi approach could kill the HTTP connection before response is sent if the gateway is connected via WiFi.

**Fix:** Apply the same delayed-restart pattern to WiFi:
```rust
std::thread::spawn(|| {
    std::thread::sleep(std::time::Duration::from_secs(2));
    Command::new("wifi").arg("reload").status().ok();
});
```

### M4. `handle_add_route` — `ip route add` Applied Without Netmask

**File:** `netcfg.rs:177-180`

```rust
Command::new("ip")
    .args(["route", "add", &target, "via", &gateway])
```

The netmask from UCI config is not used in the immediate `ip route add`. Should include netmask as CIDR:
```rust
let cidr = netmask_to_cidr(&netmask);
let target_cidr = format!("{}/{}", target, cidr);
Command::new("ip").args(["route", "add", &target_cidr, "via", &gateway]).status().ok();
```

---

## Low Priority

### L1. Hardcoded `wlan0` Interface Name

**File:** `wifi.rs:11, 56, 78`

Assumes WiFi interface is always `wlan0`. On some OpenWrt configs it could be `wlan1` or `ra0`. Consider making this configurable or reading from UCI `wireless.@wifi-iface[0].ifname`.

### L2. `jval` JSON Parser Does Not Handle Escaped Quotes

**Files:** `wifi.rs:139`, `netcfg.rs:241`

The simple `jval()` parser breaks if a JSON value contains escaped quotes (`\"`). For this embedded use case with controlled frontends, this is acceptable but worth noting.

### L3. Missing `Content-Length` Header

**Files:** All `json_resp` / `json_err` functions

tiny_http should set Content-Length automatically from the string, but explicit setting would be more correct for some HTTP clients.

---

## Edge Cases Found by Scouting

1. **DNS list parsing** (`netcfg.rs:59-65`): Uses comma-split + trim of `[]"` chars. If frontend sends JSON array format `["8.8.8.8","1.1.1.1"]`, the naive `jval()` parser will return `8.8.8.8","1.1.1.1` (stops at first quote). The trim/split workaround handles this but is fragile. Would break with DNS entries containing commas (unlikely but possible with IPv6).

2. **Concurrent network restarts**: If user sends two `POST /api/network` quickly, two `std::thread::spawn` will both call `/etc/init.d/network restart` after 2 seconds, potentially causing a race. Consider a static AtomicBool to debounce.

3. **`handle_delete_route` returns success even if route does not exist**: `Uci::delete` returns Ok for non-existent keys (line uci.rs:46 — exit code 1 treated as success). This is acceptable for idempotent DELETE but caller gets no feedback.

4. **WiFi scan on device without WiFi**: `iwinfo wlan0 scan` will fail and return empty array. Correct behavior, no issue.

5. **`is_valid_ipv4` accepts "0.0.0.0"**: Valid per parsing but semantically wrong for route targets or gateway addresses.

---

## Positive Observations

1. **Follows existing patterns** — Response types, route wiring, body reading all match established `server.rs` style
2. **Good interface whitelist** — `handle_set_network` correctly restricts to `"lan" || "wan"`
3. **Delayed network restart** — Smart pattern in `netcfg.rs` to avoid killing HTTP response
4. **Reasonable file sizes** — Both files under 200 lines, well within project guidelines
5. **Proper HTTP status codes** — 400 for client errors, 500 for server errors
6. **UCI operations are clean** — The `Uci` wrapper is simple, correct, and well-structured
7. **No panics** — All `.unwrap()` calls are on `Header::from_bytes` with static inputs (safe)
8. **Memory-appropriate** — No large allocations, Vec sizes bounded by iwinfo/ip output

---

## Recommended Actions (Priority Order)

1. **[CRITICAL]** Add `is_safe_identifier()` validation for route names, interface names in metric handler, and NTP server strings
2. **[CRITICAL]** Add `json_escape()` for SSID and encryption strings in WiFi scan/status responses
3. **[HIGH]** Make `handle_ntp_sync` non-blocking (spawn thread)
4. **[HIGH]** Extract shared helpers (`jval`, `json_resp`, `json_err`) to `web::helpers` module (DRY)
5. **[MEDIUM]** Add delayed restart to WiFi connect/disconnect handlers
6. **[MEDIUM]** Log UCI errors instead of silently discarding with `.ok()`
7. **[MEDIUM]** Include netmask/CIDR in `ip route add` command
8. **[LOW]** Make WiFi interface name configurable

## Metrics

- Type Coverage: N/A (Rust — compiler enforced)
- Test Coverage: 0% for new modules (no tests in wifi.rs/netcfg.rs)
- Linting Issues: 0 warnings from reviewed files
- Build: PASS

## Unresolved Questions

1. Should NTP server input be validated against a hostname regex, or is the current pass-through acceptable?
2. Is there a plan to add rate limiting for network restart operations?
3. Should WiFi password be stored encrypted in UCI, or is plaintext acceptable for this device class?
