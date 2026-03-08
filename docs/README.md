# ugate IoT Gateway — Documentation

**Last Updated:** 2026-03-08 | **Version:** 1.6.0 (Phases 1-9 Complete)

## Quick Start

1. [Project Overview](./project-overview-pdr.md) — requirements & features
2. [System Architecture](./system-architecture.md) — async runtime, channels, data flow
3. [UCI Config Reference](./uci-config-reference.md) — cấu hình chi tiết
4. [Deployment Guide](./deployment-guide.md) — build, deploy, init script
5. [Troubleshooting](./troubleshooting.md) — known issues & fixes

## Documentation Index

### Phần mềm (Software)

| File | Nội dung |
|------|----------|
| [project-overview-pdr.md](./project-overview-pdr.md) | Requirements, features, API spec |
| [system-architecture.md](./system-architecture.md) | Kiến trúc: async runtime, channels, threading |
| [code-standards.md](./code-standards.md) | Coding conventions, module structure |
| [codebase-summary.md](./codebase-summary.md) | Tổng quan code, module responsibilities |
| [uci-config-reference.md](./uci-config-reference.md) | UCI config sections chi tiết (UART/MQTT/HTTP/TCP/GPIO/Web) |
| [deployment-guide.md](./deployment-guide.md) | Cross-compile, deploy, init script, rollback |
| [troubleshooting.md](./troubleshooting.md) | Known issues MIPS+Rust, common problems, debugging |
| [development-roadmap.md](./development-roadmap.md) | Roadmap & progress tracking |
| [project-changelog.md](./project-changelog.md) | Version history & breaking changes |

### Hardware/Platform (other-docs/)

| File | Nội dung |
|------|----------|
| [gpio-wifi-led-guide.md](./other-docs/gpio-wifi-led-guide.md) | GPIO pinout, WiFi LED control |
| [i2c-rtc-guide.md](./other-docs/i2c-rtc-guide.md) | I2C RTC setup |
| [oled-display-guide.md](./other-docs/oled-display-guide.md) | OLED display integration |
| [OLED_0in91/](./other-docs/OLED_0in91/) | OLED driver source (C++ reference) |
| [mips-build-guide.md](./other-docs/mips-build-guide.md) | MIPS cross-compilation setup |
| [mips-tokio-rumqttc-issues.md](./other-docs/mips-tokio-rumqttc-issues.md) | Tokio + rumqttc trên MIPS |
| [openwrt-config-analysis.md](./other-docs/openwrt-config-analysis.md) | OpenWrt UCI config analysis |
| [InitScript-openWrt.md](./other-docs/InitScript-openWrt.md) | procd init script guide |
| [wifi-openwrt.md](./other-docs/wifi-openwrt.md) | WiFi config trên OpenWrt |

## Hardware

| Spec | Value |
|------|-------|
| SoC | MT7628DAN (MIPS 24KEc, 580MHz) |
| RAM | 64MB DDR2 |
| Flash | 16MB SPI-NOR |
| OS | OpenWrt 24.10 (Kernel 6.6.x) |

## Build & Deploy

```bash
cross +nightly build --target mipsel-unknown-linux-musl --release -p ugate
./deploy.sh  # build + scp + restart service
```

Chi tiết: [Deployment Guide](./deployment-guide.md)

## MIPS/Rust Bugs

Xem `../mips-rust-notes/bugs-and-gotchas.md` — AtomicU64, ioctl type, WS handshake, Cookie case-sensitivity, rumqttc async panic...
