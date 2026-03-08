---
title: "Toolbox - Network Diagnostics (Ping/Traceroute/NSLookup)"
description: "Add Toolbox tab with streaming network diagnostic tools via WebSocket"
status: pending
priority: P2
effort: 3h
branch: main
tags: [web-ui, network, diagnostics, websocket]
created: 2026-03-08
---

# Toolbox - Network Diagnostics

## Summary

Add a "Toolbox" tab to ugate web UI with 3 network diagnostic tools (ping, traceroute, nslookup). Output streams in real-time via existing WebSocket broadcast channel.

## Architecture Overview

```
Browser (Toolbox tab)             ugate (Rust)
  |                                  |
  |-- POST /api/toolbox/run -------->|-- spawn child process (ping/traceroute/nslookup)
  |   {tool:"ping",target:"1.1.1.1"}|   read stdout line-by-line
  |                                  |   broadcast each line via ws_manager.broadcast_tx
  |<-- WS: {"type":"toolbox",...} ---|
  |<-- WS: {"type":"toolbox",...} ---|
  |<-- WS: {"type":"toolbox","done":true} --|
```

**Key design decisions:**
- Reuse existing WS broadcast channel (no new WS endpoint)
- One tool at a time (AtomicBool guard prevents concurrent runs)
- REST API to start tool, WS for streaming output
- POST /api/toolbox/stop to kill running process
- Messages tagged `{"type":"toolbox"}` so JS can filter them from UART data

## Phases

| # | Phase | Status | File |
|---|-------|--------|------|
| 1 | Backend: toolbox.rs module | pending | [phase-01](./phase-01-backend-toolbox-module.md) |
| 2 | Server: route wiring + HTML tab | pending | [phase-02](./phase-02-server-routes-and-ui.md) |

## Dependencies

- OpenWrt has `ping`, `traceroute`, `nslookup` installed by default (busybox)
- Existing: `ws_manager.broadcast_tx`, `tiny_http`, `tungstenite`
- No new crate dependencies needed

## Risk Assessment

- **OOM from large output**: mitigate with line count limit (max 200 lines) and process kill
- **Concurrent runs**: AtomicBool prevents multiple tools running simultaneously
- **Command injection**: strict validation — only alphanumeric, dots, hyphens, colons in target
- **Long-running process**: traceroute can take 30s+ — provide stop button + 60s timeout
