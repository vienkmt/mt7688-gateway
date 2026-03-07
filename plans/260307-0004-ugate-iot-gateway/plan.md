---
status: pending
created: 2026-03-07
brainstorm: ../reports/brainstorm-260307-0004-ugate-iot-gateway.md
---

# ugate IoT Gateway Implementation Plan

## Overview

IoT Gateway firmware cho MT7688: UART serial reader → multi-channel fan-out (MQTT/HTTP/TCP) + Web UI (tiny-http + tungstenite + Vue.js) + GPIO control.

## Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                         ugate (Rust)                             │
├─────────────────────────────────────────────────────────────────┤
│  #[tokio::main(flavor = "current_thread")]                       │
│                                                                   │
│  ┌──────────────┐     ┌──────────────┐     ┌──────────────┐     │
│  │  Web Server  │     │ UART Reader  │     │  GPIO Ctrl   │     │
│  │ (tiny-http)  │     │ (AsyncFd)    │     │ (chardev)    │     │
│  │ + tungstenite│     │              │     │  DTS defined │     │
│  └──────────────┘     └──────────────┘     └──────────────┘     │
│         │                    │                    ▲              │
│         │                    ▼                    │              │
│         │             ┌──────────────┐            │              │
│         │             │  Fan-Out Hub │────────────┤              │
│         │             └──────────────┘            │              │
│         │                    │                    │              │
│         │       ┌────────────┼────────────┐       │              │
│         │       ▼            ▼            ▼       │              │
│         │  ┌────────┐  ┌────────┐  ┌────────┐    │              │
│         │  │  MQTT  │  │  HTTP  │  │  TCP   │    │              │
│         │  │ (sync) │  │  POST  │  │ S + C  │    │              │
│         │  └────────┘  └────────┘  └────────┘    │              │
│         │                                         │              │
│         └─────────────────────────────────────────┘              │
│                    Command Merge (WS + TCP + MQTT)               │
└─────────────────────────────────────────────────────────────────┘
```

## Phases

### Stage 1: Foundation (4d)

| Phase | Name | Status | Effort | Depends |
|-------|------|--------|--------|---------|
| 1 | Core Infrastructure | pending | 2d | - |
| 2a | MQTT + HTTP Channels | pending | 2d | Phase 1 |

### Stage 2: Communication (4d)

| Phase | Name | Status | Effort | Depends |
|-------|------|--------|--------|---------|
| 2b | TCP + Reliability | pending | 2d | Phase 2a |
| 3 | Web Server + WebSocket | pending | 2d | Phase 1 |

### Stage 3: Control & UI (4d)

| Phase | Name | Status | Effort | Depends |
|-------|------|--------|--------|---------|
| 4 | GPIO Control | pending | 1d | Phase 3 |
| 5 | Vue.js Frontend | pending | 3d | Phase 3 |

### Stage 4: Advanced Features (5d)

| Phase | Name | Status | Effort | Depends |
|-------|------|--------|--------|---------|
| 7 | Network Configuration | pending | 3d | Phase 3 |
| 8 | System Maintenance | pending | 2d | Phase 3 |

### Stage 5: Finalization (3d)

| Phase | Name | Status | Effort | Depends |
|-------|------|--------|--------|---------|
| 6 | Integration & Testing | pending | 2d | All |
| Final | OpenWrt Packaging | pending | 1d | Phase 6 |

**Total:** ~20 days (5 stages)

## Phase Files

| Phase | File |
|-------|------|
| 1 | [phase-01-core-infrastructure.md](phase-01-core-infrastructure.md) |
| 2a | [phase-02a-mqtt-http.md](phase-02a-mqtt-http.md) |
| 2b | [phase-02b-tcp-reliability.md](phase-02b-tcp-reliability.md) |
| 3 | [phase-03-web-server.md](phase-03-web-server.md) |
| 4 | [phase-04-gpio-control.md](phase-04-gpio-control.md) |
| 5 | [phase-05-vue-frontend.md](phase-05-vue-frontend.md) |
| 6 | [phase-06-integration-testing.md](phase-06-integration-testing.md) |
| 7 | [phase-07-network-config.md](phase-07-network-config.md) |
| 8 | [phase-08-system-maintenance.md](phase-08-system-maintenance.md) |
| Final | [phase-final-openwrt-packaging.md](phase-final-openwrt-packaging.md) |

## Key Decisions

| Aspect | Choice |
|--------|--------|
| Web Server | tiny-http + tungstenite (proven stack từ vgateway) |
| Frontend | Vue.js + Tailwind CSS (embedded in binary) |
| TCP | Server + Client modes (separate channels) |
| GPIO | **chardev ioctl** (Rust thuần), line numbers từ UCI, không cần DTS |
| Auth | Session token + 1h expiry (stored in RAM) |
| MQTT | rumqttc sync thread, QoS configurable, auth + TLS (rustls) |
| HTTP POST | ureq + rustls (no OpenSSL dependency) |
| Config | UCI `/etc/config/ugate`, auto-create if missing |
| Time Sync | HTTP Date header (plain HTTP) **trước TLS** |
| UART | 115200 baud default, AsyncFd epoll |
| WebSocket | Real-time UART logs + system stats |
| Port | 8888 |
| Logging | `log` + `syslog` crates |
| Error Handling | `thiserror` crate |

## Execution Order

```
Stage 1: Foundation
├── Phase 1: Core Infrastructure (config, UCI, UART)
└── Phase 2a: MQTT + HTTP Channels

Stage 2: Communication (parallel possible)
├── Phase 2b: TCP + Reliability (OfflineBuffer, Reconnector)
└── Phase 3: Web Server + WebSocket

Stage 3: Control & UI (parallel possible)
├── Phase 4: GPIO Control
└── Phase 5: Vue.js Frontend

Stage 4: Advanced Features (parallel possible)
├── Phase 7: Network Configuration (WiFi, LAN/WAN, NTP)
└── Phase 8: System Maintenance (backup, upgrade)

Stage 5: Finalization
├── Phase 6: Integration & Testing
└── Phase Final: OpenWrt Packaging
```

## Success Criteria

- [ ] Memory <20MB runtime
- [ ] All channels work independently
- [ ] WebSocket <100ms latency
- [ ] GPIO toggle <50ms
- [ ] 24h stress test pass
- [ ] Graceful shutdown (no data loss)

## Risks

| Risk | Severity | Mitigation |
|------|----------|------------|
| WebSocket CPU busy-loop | High | Thread per connection + broadcast channel |
| Session hijack | Medium | Random token + expiry + rate limit |
| RAM overflow (buffer) | Medium | Disk overflow khi RAM full |
| Network restart kill connection | High | Delay 2s + watchdog fallback |

## Dependencies (Cargo.toml)

```toml
[dependencies]
tokio = { version = "1", features = ["rt", "net", "io-util", "sync", "time", "fs"] }
rumqttc = "0.24"
ureq = { version = "2", features = ["tls"] }
rustls = "0.22"
webpki-roots = "0.26"
tiny_http = "0.12"
tungstenite = "0.21"
serde = { version = "1", features = ["derive"] }
thiserror = "1"
log = "0.4"
syslog = "6"

[profile.release]
opt-level = "z"       # Optimize for size
lto = true            # Link-time optimization
codegen-units = 1     # Single codegen unit
panic = "abort"       # No unwinding
strip = true          # Strip symbols
```

## Config Format

**UCI only** — không dùng TOML. Native OpenWrt integration.
- Config file: `/etc/config/ugate`
- Commands: `uci get/set/commit`
