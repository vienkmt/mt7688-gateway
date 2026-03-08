# Planner Report: Toolbox Network Diagnostics

## Summary

Created implementation plan for adding a "Toolbox" tab with ping/traceroute/nslookup streaming via WebSocket.

## Research Findings

- WS broadcast channel (`ws_manager.broadcast_tx`) already fans out data to all clients — toolbox reuses this, zero new infra
- Server runs in `spawn_blocking` (tiny-http is blocking) — child process stdout reading is naturally blocking, fits perfectly
- UART terminal uses append-only `.stream` div with `_streamEl`/`_rendered` pattern — toolbox copies this for performance
- Messages differentiated by `type` field: `"status"`, `"uart"`, now `"toolbox"`
- No new crate dependencies needed — `std::process::Command` + `BufReader` sufficient
- OpenWrt busybox provides `ping`, `traceroute`, `nslookup` out of the box

## Plan Structure

| Phase | Description | Effort | Files |
|-------|-------------|--------|-------|
| 1 | Backend `toolbox.rs` module | 1.5h | Create `web/toolbox.rs`, modify `web/mod.rs` |
| 2 | Server routes + UI tab | 1.5h | Modify `server.rs`, `embedded_index.html` |

**Total effort: ~3h**

## Key Design Decisions

1. **Reuse WS broadcast** (not separate endpoint) — KISS, no new connection management
2. **AtomicBool guard** — prevents concurrent tool runs, avoids resource exhaustion on 64MB device
3. **Thread-per-run** — `std::thread::spawn` reads stdout lines, broadcasts each one; thread exits when process completes
4. **Safety**: strict target validation (alphanumeric + dots/hyphens/colons only), 200-line limit, 60s timeout
5. **REST to start, WS to stream** — client POSTs to start tool, receives output via existing WS

## Plan Location

`/Users/phonglinh/mt7688-gateway/plans/260308-1359-toolbox-network-diagnostics/`

- `plan.md` — overview
- `phase-01-backend-toolbox-module.md` — Rust backend with handle_run, handle_stop, is_safe_target
- `phase-02-server-routes-and-ui.md` — route wiring + JS UI with renderToolbox, runTool, stopTool
