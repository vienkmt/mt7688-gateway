---
status: pending
created: 2026-03-07
brainstorm: ../reports/brainstorm-260307-0004-ugate-iot-gateway.md
---

# ugate IoT Gateway Implementation Plan

## Overview

Build complete IoT Gateway firmware for MT7688 hardware with UART serial reader, multi-channel fan-out (MQTT/HTTP/TCP), axum web server + WebSocket, Vue.js embedded frontend, and GPIO control.

## Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                         ugate (Rust)                             │
├─────────────────────────────────────────────────────────────────┤
│  #[tokio::main(flavor = "current_thread")]                       │
│                                                                   │
│  ┌──────────────┐     ┌──────────────┐     ┌──────────────┐     │
│  │  Web Server  │     │ UART Reader  │     │  GPIO Ctrl   │     │
│  │  (axum)      │     │ (AsyncFd)    │     │  (sysfs)     │     │
│  │  + WebSocket │     │              │     │  4 OUT + LED │     │
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

| Phase | Name | Status | File | Effort |
|-------|------|--------|------|--------|
| 1 | Core Infrastructure | pending | [phase-01-core-infrastructure.md](phase-01-core-infrastructure.md) | 2d |
| 2 | Channels | pending | [phase-02-channels.md](phase-02-channels.md) | 3d |
| 3 | Web Server | pending | [phase-03-web-server.md](phase-03-web-server.md) | 2d |
| 4 | GPIO Control | pending | [phase-04-gpio-control.md](phase-04-gpio-control.md) | 1d |
| 5 | Vue.js Frontend | pending | [phase-05-vue-frontend.md](phase-05-vue-frontend.md) | 3d |
| 6 | Integration & Testing | pending | [phase-06-integration-testing.md](phase-06-integration-testing.md) | 2d |

**Total:** ~13 days

## Key Decisions

| Aspect | Choice |
|--------|--------|
| HTTP Server | tiny-http + tungstenite (proven stack) |
| Frontend | Vue.js + Tailwind CSS embedded |
| TCP | Server + Client modes |
| GPIO | MCU + Server triggers, pins configurable via UCI |
| Auth | Simple password (UCI) |
| MQTT | Bidirectional (publish + subscribe commands), QoS configurable |
| Config | UCI chuẩn (`/etc/config/ugate`), auto-create if missing |
| UART | 115200 baud default |
| WebSocket | Real-time UART logs + system stats (CPU/RAM/Flash) |
| Reuse | Copy & refactor from vgateway |

## Success Criteria

- [ ] Binary <1.2MB
- [ ] All channels work independently
- [ ] WebSocket <100ms latency
- [ ] GPIO toggle <50ms
- [ ] Memory <20MB runtime
- [ ] 24h stress test pass

## Dependencies

See brainstorm report for full dependency list.

## Risks

| Risk | Mitigation |
|------|------------|
| Binary size >1MB | Optimize profile, acceptable up to 1.2MB |
| WebSocket memory | Limit 8 concurrent connections |
