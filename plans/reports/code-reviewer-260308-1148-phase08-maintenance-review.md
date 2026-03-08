# Code Review: Phase 8 System Maintenance

## Scope
- Files: `maintenance.rs` (NEW, 331 LOC), `build.rs` (NEW, 33 LOC), `server.rs` (MOD), `mod.rs` (MOD), `config.rs` (MOD), `embedded_index.html` (MOD)
- Focus: Security, memory safety, error handling, OpenWrt compat
- Build: `cargo check -p ugate` PASSES

## Overall Assessment

Solid implementation. Follows existing codebase patterns (manual JSON, `json_resp`/`json_err`, `spawn_blocking` style). Good body size limits. Two critical issues found, several medium concerns.

---

## Critical Issues

### C1. Unbounded memory allocation in remote IPK download (maintenance.rs:237-238)

```rust
let mut data = Vec::new();
std::io::Read::read_to_end(&mut reader, &mut data)
```

`read_to_end` has NO size limit. A malicious or misconfigured update server could return gigabytes, causing OOM on the 64MB device. The upload path correctly limits to 2MB, but the remote download path does not.

**Fix:** Use `.take(2 * 1024 * 1024)` on the reader before `read_to_end`:
```rust
let mut reader = resp.into_reader().take(2 * 1024 * 1024);
```

### C2. Checksum bypass returns `true` when sha256sum is unavailable (maintenance.rs:309)

```rust
log::warn!("[Maint] sha256sum not available, skipping checksum verify");
return true; // skip nếu không có sha256sum
```

If `sha256sum` binary is missing or fails to execute, the checksum verification is silently skipped and the (potentially tampered) IPK is installed anyway. This defeats the purpose of checksum verification entirely.

**Fix:** Return `false` when sha256sum is unavailable. Log an error, not a warning:
```rust
log::error!("[Maint] sha256sum not available, cannot verify checksum");
return false;
```

**Note:** On OpenWrt, `sha256sum` is part of busybox and always present. But defense-in-depth matters for a firmware upgrade path.

---

## High Priority

### H1. No concurrency guard on upgrade operations

Both `handle_upgrade_upload` and `handle_upgrade_remote` write to the same `/tmp/ugate.ipk` path and spawn threads to install. If two upgrade requests arrive concurrently, they race on the same file. One may delete the IPK while the other is installing it.

**Fix:** Use an `AtomicBool` (or `Arc<Mutex<()>>`) static to reject concurrent upgrade requests:
```rust
static UPGRADING: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
// At start of handler:
if UPGRADING.swap(true, Ordering::SeqCst) {
    return json_err(409, "upgrade already in progress");
}
// Clear in spawned thread's cleanup
```

### H2. `ureq::get(&url)` uses user-controlled URL without validation (maintenance.rs:165, 206, 228)

The `url` value comes from UCI config (`ugate.@upgrade[0].url`). While UCI is only writable by root/authenticated users, the URL is used directly in HTTP requests. If a user sets it to `file:///etc/shadow` or an internal network address, it becomes an SSRF vector.

**Fix:** Validate that the URL starts with `https://` or `http://`:
```rust
if !url.starts_with("http://") && !url.starts_with("https://") {
    return json_err(400, "upgrade URL must be HTTP(S)");
}
```

### H3. `handle_restore` trusts any file containing "config " substring (maintenance.rs:61)

The validation `text.contains("config ")` is too weak. Any text file containing the word "config " somewhere passes validation and overwrites `/etc/config/ugate`. A corrupted or partial config will break UCI parsing and could brick the device.

**Fix:** Add stricter validation. At minimum, require the file starts with `config ` on a line:
```rust
if !text.lines().any(|l| l.trim_start().starts_with("config ")) {
    return json_err(400, "invalid config: no UCI sections found");
}
```

---

## Medium Priority

### M1. `build.rs` missing `cargo:rerun-if-changed` directives

Without `rerun-if-changed`, the build script runs on every build, executing `date` and `git rev-parse` each time. This slows incremental builds.

**Fix:** Add:
```rust
println!("cargo:rerun-if-changed=Cargo.toml");
println!("cargo:rerun-if-changed=.git/HEAD");
```

### M2. `handle_backup` exposes full config including password (maintenance.rs:21)

`/api/backup` returns the raw UCI config file, which contains `option password 'admin'` (the web password). Anyone with an authenticated session can download the plaintext password.

**Mitigation:** This is acceptable for an IoT device admin panel where the user is already authenticated. Document this as a known behavior. Alternatively, strip the password line before serving.

### M3. Thread spawns without size-limited stack (maintenance.rs:98, 135, 226)

`std::thread::spawn` uses default stack size (typically 8MB on Linux, 2MB on musl). On a 64MB device running multiple threads, this adds up. Not a bug now, but worth noting.

**Consideration:** Use `std::thread::Builder::new().stack_size(64 * 1024)` for simple threads that only run shell commands.

### M4. `version_gt` does not handle pre-release or non-numeric segments (maintenance.rs:293-300)

`version_gt("1.2.3-beta", "1.2.3")` returns `false` because `parse::<u32>()` on "3-beta" fails and is filtered out, making it compare `[1,2]` vs `[1,2,3]`.

**Risk:** Low. Versions are controlled by the developer via `Cargo.toml`. But worth a comment.

### M5. Frontend does not disable buttons during async operations

`doRemoteUpgrade`, `doUploadIpk`, `doRestore` do not disable their buttons during the fetch. Users can click multiple times, triggering concurrent requests (see H1).

**Fix:** Set a loading state and disable buttons during operations, similar to `checkingUpdate` pattern already used for `checkUpgrade`.

---

## Low Priority

### L1. `_state` parameter unused in `handle_upgrade_upload` signature consideration

The function does not take `state` but the upgrade path restarts the service. This is fine since the service will re-read config on startup.

### L2. Consistent use of `#[allow(unused)]` for `Arc<AppState>` import

`maintenance.rs` line 4 imports `AppState` and `Config` but `Config` is only used in `handle_factory_reset` and `handle_restore`. Import is clean and correct.

---

## Positive Observations

1. **Body size limits** on upload endpoints (`read_body_raw` with 64KB/2MB caps) - good memory safety
2. **IPK format validation** via ar archive magic bytes - prevents random file uploads
3. **Backup before restore** (`/tmp/ugate.backup`) - recovery path exists
4. **Delayed reboot/install** via `thread::sleep(1s)` - ensures HTTP response reaches client
5. **`json_escape` used** for all user-controlled strings in JSON output - prevents injection
6. **Build info** via `env!()` compile-time macros - zero runtime cost
7. **Frontend confirmation dialogs** for destructive actions (restart, factory reset)
8. **Auth check** covers all maintenance endpoints via the existing `needs_auth` guard in `server.rs`

---

## Recommended Actions (Priority Order)

1. **[C1]** Add `.take(2MB)` limit on remote IPK download reader
2. **[C2]** Return `false` from `verify_checksum` when sha256sum unavailable
3. **[H1]** Add `AtomicBool` upgrade lock to prevent concurrent upgrades
4. **[H2]** Validate upgrade URL scheme (http/https only)
5. **[H3]** Strengthen UCI config validation in restore handler
6. **[M1]** Add `rerun-if-changed` to `build.rs`
7. **[M5]** Add loading states to frontend buttons for destructive operations

## Metrics

- Build: PASS
- Type safety: Good (no `unwrap()` on user input, all errors handled)
- Panic risk: None found (all `.unwrap()` are on `Header::from_bytes` with static values)
- New dependencies: 0 (uses existing `ureq`, `tiny-http`)
- LOC added: ~400 (Rust + HTML)

## Unresolved Questions

1. Should `handle_upgrade_check` cache the manifest briefly to avoid repeated fetches from UI? (User might click repeatedly)
2. Should the restore endpoint validate that all required UCI sections (`mqtt`, `uart`, `tcp`, etc.) are present, not just any `config ` line?
3. Is `/tmp` guaranteed to have enough space for 2MB IPK on this 16MB flash device? (Yes - `/tmp` is tmpfs on RAM, but worth confirming available RAM during upgrade)
