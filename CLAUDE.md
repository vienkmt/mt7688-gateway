# CLAUDE.md

## Project Overview

IoT Gateway firmware written in **Rust** for embedded Linux (OpenWrt).

## Hardware Target

| Component | Specification |
|-----------|---------------|
| Board | MediaTek LinkIt Smart 7688 |
| SoC | MT7688AN (MIPS 24KEc, 580MHz, single-core) |
| RAM | 256MB DDR2 |
| Flash | 32MB SPI-NOR (25MB available) |
| Wi-Fi | 2.4GHz 802.11b/g/n (150Mbps) |
| Interfaces | Ethernet, USB Host, UART x2, SPI, I2C, GPIO |

## Firmware Environment

- **OS:** OpenWrt 21.02 (Kernel 5.4.171)
- **Architecture:** ramips/mt76x8 (MIPS)
- **Network:** LAN 10.10.10.1/24, WAN DHCP
- **Peripherals:** 2x UART ready, Quectel 4G module

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

- Keep binaries small (target <500KB)
- Use async runtime carefully (tokio adds ~1MB)
- Prefer `heapless` collections where possible
- Handle UART/GPIO with `serialport` or `embedded-hal`
- Log via `syslog` or file (no stdout in daemon mode)

## Documentation

See `./docs` for detailed documentation:
- `project-overview-pdr.md` — Requirements
- `system-architecture.md` — Architecture
- `code-standards.md` — Coding conventions
