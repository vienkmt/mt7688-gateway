# Brainstorm Report: ugate IoT Gateway

**Date:** 2026-03-07
**Status:** Agreed
**Next:** Create implementation plan

## Problem Statement

Build complete IoT Gateway firmware (ugate) for MT7688 hardware:
- UART serial reader with fan-out to multiple channels
- Channels: MQTT, HTTP POST, TCP Server/Client
- Web UI: Vue.js + WebSocket for real-time monitoring & control
- Bidirectional: Server commands → MCU via UART TX
- GPIO: 4 outputs + 1 LED heartbeat
- Config via UCI, auth via simple password

## Requirements Summary

| Aspect | Decision | Rationale |
|--------|----------|-----------|
| Frontend | Vue.js embedded (include_bytes!) | Single binary deploy |
| HTTP Server | axum + tokio | WebSocket native, modern async |
| TCP Channel | Server + Client modes | Flexible deployment |
| GPIO Trigger | MCU (UART) + Server (channels) | Full control |
| UART Frame | Plain text + newline | Simple parsing |
| Auth | Simple password (UCI) | LAN-only, trusted network |
| Binary Size | ~1MB OK | Flash 16MB sufficient |
| Code Reuse | Copy & refactor from vgateway | Proven patterns |

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
│                           │                                      │
│                           ▼                                      │
│                    ┌──────────────┐                              │
│                    │  UART TX     │                              │
│                    │  (to MCU)    │                              │
│                    └──────────────┘                              │
└─────────────────────────────────────────────────────────────────┘
```

## Module Structure

```
ugate/
├── Cargo.toml
├── src/
│   ├── main.rs              # Entry, task orchestration
│   ├── config.rs            # AppState, Config (from vgateway)
│   ├── uci.rs               # UCI wrapper (from vgateway)
│   ├── uart/
│   │   ├── mod.rs
│   │   ├── reader.rs        # AsyncFd + epoll
│   │   └── writer.rs        # TX to MCU
│   ├── channels/
│   │   ├── mod.rs
│   │   ├── mqtt.rs          # std::thread sync
│   │   ├── http_pub.rs      # HTTP POST
│   │   └── tcp.rs           # Server + Client
│   ├── web/
│   │   ├── mod.rs
│   │   ├── server.rs        # axum setup
│   │   ├── routes.rs        # API endpoints
│   │   ├── ws.rs            # WebSocket handler
│   │   └── auth.rs          # Simple password
│   ├── gpio.rs              # sysfs GPIO
│   └── commands.rs          # Command parser
└── frontend/                # Vue.js
    └── dist/                # → embed
```

## Channel Types

| Channel | Direction | Spawn Method | Channel Type |
|---------|-----------|--------------|--------------|
| MQTT | TX only | std::thread | std::sync::mpsc |
| HTTP POST | TX only | tokio::spawn | tokio::mpsc |
| TCP | Bidirectional | tokio::spawn | tokio::mpsc |
| WebSocket | Bidirectional | axum handler | tokio::broadcast + mpsc |

## Command Protocol

**UART → GPIO (from MCU):**
```
GPIO:1:ON\n
GPIO:2:OFF\n
GPIO:3:TOGGLE\n
```

**Server → GPIO (from WS/TCP/MQTT):**
```json
{"cmd":"gpio","pin":1,"state":"on"}
{"cmd":"uart","data":"raw data to MCU"}
```

## Risks & Mitigations

| Risk | Impact | Mitigation |
|------|--------|------------|
| axum fail on MIPS | High | Phase 0: test minimal axum first |
| Binary size >1MB | Medium | Optimize release profile, lazy static |
| WebSocket memory | Medium | Limit concurrent connections (8 max) |
| TCP reconnect storms | Low | Exponential backoff |

## Implementation Phases

1. **Phase 0** - axum MIPS test (1 day)
2. **Phase 1** - Core: config, uci, uart reader (2 days)
3. **Phase 2** - Channels: MQTT, HTTP, TCP (3 days)
4. **Phase 3** - Web server + WebSocket (2 days)
5. **Phase 4** - GPIO control (1 day)
6. **Phase 5** - Vue.js frontend (3 days)
7. **Phase 6** - Integration + testing (2 days)

**Total estimate:** ~14 days

## Success Criteria

- [ ] Binary <1.2MB
- [ ] All channels work independently
- [ ] WebSocket real-time <100ms latency
- [ ] GPIO toggle <50ms response
- [ ] Memory usage <20MB runtime
- [ ] Survive 24h stress test

## Dependencies

**Cargo.toml (tentative):**
```toml
[dependencies]
tokio = { version = "1", features = ["rt", "io-util", "sync", "time", "net"] }
axum = { version = "0.7", features = ["ws"] }
tower-http = { version = "0.5", features = ["fs", "cors"] }
rumqttc = "0.24"
ureq = { version = "2", features = ["tls"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
toml = "0.8"
```

## Open Questions

1. GPIO pin mapping cụ thể trên MT7688?
2. Vue.js component library (naive-ui, element-plus, or vanilla)?
3. MQTT QoS level default?
