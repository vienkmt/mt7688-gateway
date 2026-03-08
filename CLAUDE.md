# CLAUDE.md

## Project Overview

IoT Gateway firmware written in **Rust** for embedded Linux (OpenWrt).

## Hardware Target

| Component | Specification |
|-----------|---------------|
| Board | MediaTek MT7628 |
| SoC | MT7628DAN (MIPS 24KEc, 580MHz, single-core) |
| RAM | 64MB DDR2 |
| Flash | 16MB SPI-NOR |
| Wi-Fi | 2.4GHz 802.11b/g/n (150Mbps) |
| Interfaces | Ethernet, USB Host, UART x2, SPI, I2C, GPIO |

## Firmware Environment

- **OS:** OpenWrt 24.10 stable (Kernel 6.6.x)
- **Architecture:** ramips/mt76x8 (MIPS)
- **Network:** LAN 10.10.10.1/24, WAN DHCP
- **Peripherals:** 2x UART ready, I2C (OLED), GPIO

## Development Constraints

- **CPU:** 580MHz single-core — avoid heavy processing
- **Memory:** Limited RAM — optimize allocations
- **Cross-compile:** Target `mipsel-unknown-linux-musl`
- **No std optional:** Consider `#![no_std]` for smaller binaries
- **Static linking:** Prefer musl for portable binaries

## Build Commands

```bash
# Cross-compile for MIPS
cargo build --target mipsel-unknown-linux-musl --release

# Check without building
cargo check --target mipsel-unknown-linux-musl
```

## Code Guidelines

- Keep binaries small (target <800KB with Tokio runtime)
- Tokio single-thread executor (v0.2.0+) provides efficient async/await with epoll
- Use `heapless` collections where possible
- Handle UART with AsyncFd (non-blocking epoll) or `serialport`
- GPIO via tokio::spawn for LED heartbeat, OLED display updates
- Log via `syslog` or file (no stdout in daemon mode)

## Async Runtime (v2.0)

- **Runtime:** `#[tokio::main(flavor = "current_thread")]` with epoll backend
- **UART I/O:** AsyncFd wraps serial file descriptor for non-blocking epoll
- **MQTT:** `std::thread::spawn` with sync rumqttc::Client (NOT AsyncClient due to MIPS issues)
- **HTTP Publisher:** `tokio::spawn` with `spawn_blocking(ureq)` for HTTP POST
- **Channels:**
  - UART → MQTT: `std::sync::mpsc::channel` (cross-thread compatible)
  - UART → HTTP: `tokio::sync::mpsc::channel` (async, capacity 64)
  - Config changes: `tokio::sync::watch<()>` (notify-only, MQTT polls every 2s)
- **HTTP Server:** `spawn_blocking` wrapping tiny-http
- **Port:** 8888 (ugate), 8889 (vgateway)
- **Config:** UCI `/etc/config/ugate` (ugate), TOML `/etc/vgateway.toml` (vgateway)

## Workspace

- **ugate/** — IoT Gateway (active development)
- **vgateway/** — demo/reference only, không cần cho production

```bash
cross +nightly build --target mipsel-unknown-linux-musl --release -p ugate
```

## Documentation

Tất cả docs nằm trong `./docs/`:

**Phần mềm:**
- `project-overview-pdr.md` — Requirements
- `system-architecture.md` — Kiến trúc hệ thống
- `code-standards.md` — Coding conventions
- `codebase-summary.md` — Tổng quan codebase
- `development-roadmap.md` — Roadmap & progress
- `project-changelog.md` — Changelog
- `uci-config-reference.md` — UCI config chi tiết
- `deployment-guide.md` — Build, deploy, init script
- `troubleshooting.md` — Known issues & fixes

**Hardware/Platform (docs/other-docs/):**
- OLED, I2C, GPIO guides, MIPS build notes, OpenWrt config

**MIPS/Rust bugs:** `./mips-rust-notes/bugs-and-gotchas.md`
