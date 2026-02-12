# MT7688AN IoT Gateway - Documentation Index

Welcome to the MT7688AN IoT Gateway project documentation. This directory contains all you need to understand, develop, and deploy the firmware.

## Quick Start for New Developers

1. **Start here:** Read [Project Overview & PDR](./project-overview-pdr.md) (15 min)
   - Understand what the project does
   - Learn about core features
   - See the complete API specification

2. **Then study:** Read [System Architecture](./system-architecture.md) (20 min)
   - Understand how components interact
   - Learn data flow patterns
   - Review concurrency model and threading

3. **Finally dive in:** Read [Codebase Summary](./codebase-summary.md) (10 min)
   - Understand code organization
   - Review module responsibilities
   - See what each source file does

4. **Build & Deploy:** Follow [MIPS Build Guide](./mips-build-guide.md)
   - Set up cross-compilation environment
   - Build for MT7688AN target
   - Deploy to device

**Total time to full project understanding:** ~45 minutes

## Documentation Files

### Core Documentation (Updated 2026-02-12)

| File | Purpose | Audience | Time |
|------|---------|----------|------|
| **[project-overview-pdr.md](./project-overview-pdr.md)** | Project requirements, features, and API specification | Product managers, architects, developers | 15 min |
| **[system-architecture.md](./system-architecture.md)** | Detailed system design, data flows, and component interactions | Developers, code reviewers, architects | 20 min |
| **[codebase-summary.md](./codebase-summary.md)** | Code organization, module overview, and quick reference | Developers, onboarding | 10 min |

### Infrastructure Documentation

| File | Purpose | Audience | Time |
|------|---------|----------|------|
| **[mips-build-guide.md](./mips-build-guide.md)** | Cross-compilation setup for MIPS target | DevOps, release engineers | 20 min |

## Documentation by Topic

### For Understanding "What"
- **What does this project do?** → [Project Overview](./project-overview-pdr.md#executive-summary)
- **What are the main features?** → [Core Features](./project-overview-pdr.md#core-features-implemented)
- **What are the requirements?** → [Functional Requirements](./project-overview-pdr.md#functional-requirements)

### For Understanding "How"
- **How does the system work?** → [System Architecture](./system-architecture.md#architecture-overview)
- **How do modules interact?** → [Module Architecture](./system-architecture.md#module-architecture)
- **How is the code organized?** → [Codebase Summary](./codebase-summary.md#project-structure)
- **How do I build it?** → [MIPS Build Guide](./mips-build-guide.md)

### For Understanding "Why"
- **Why this architecture?** → [Constraints & Considerations](./system-architecture.md#memory--storage)
- **Why these design choices?** → [Technical Constraints](./project-overview-pdr.md#technical-constraints)
- **Why bounded channels?** → [Concurrency Model](./system-architecture.md#concurrency-model)

## Key Features

### System Monitoring
- View real-time system stats (uptime, CPU, memory, network)
- Dashboard at `GET /`

### Configuration Management
- Configure MQTT broker, HTTP endpoint, UART settings
- Form at `GET/POST /config`

### Network Configuration (NEW)
- Configure WAN interface (DHCP or Static IP)
- Validate IP, netmask, gateway, DNS
- Prevent LAN conflicts
- Persist to OpenWrt UCI
- Web form at `GET/POST /network`
- JSON API at `GET/POST /api/network`

### Data Publishing
- Read device data via UART
- Publish to MQTT broker (with TLS)
- POST to HTTP endpoint
- Bounded channels prevent OOM

## Architecture at a Glance

```
4G Modem → UART → UART Reader ┐
                               ├→ MQTT Publisher → MQTT Broker
                               │
                               └→ HTTP Publisher → HTTP Server
                               ↓
                    HTTP Server (:8888)
                    ├─ / (Dashboard)
                    ├─ /config (Config form)
                    └─ /network (Network form)
                    └─ /api/network (JSON API)
```

## REST API Quick Reference

| Method | Endpoint | Purpose |
|--------|----------|---------|
| GET | `/` | System dashboard |
| GET | `/config` | Configuration form |
| POST | `/config` | Update configuration |
| GET | `/network` | Network config form |
| POST | `/network` | Update network config |
| GET | `/api/network` | Get network config (JSON) |
| POST | `/api/network` | Update network config (JSON) |

Full API spec: [API Specification](./project-overview-pdr.md#api-specification)

## Module Overview

| Module | Lines | Purpose |
|--------|-------|---------|
| main.rs | 287 | HTTP server, routing |
| network_config.rs | 321 | WAN config + validation |
| html_network.rs | 140 | Network UI form |
| html_config.rs | ~150 | Config UI form |
| html_template.rs | ~100 | Dashboard template |
| system_info.rs | ~100 | System stats collection |
| config.rs | ~100 | Config storage |
| uci.rs | 67 | UCI wrapper |
| uart_reader.rs | ~80 | Serial reader |
| mqtt_publisher.rs | ~100 | MQTT client |
| http_publisher.rs | ~100 | HTTP POST client |
| time_sync.rs | ~40 | Clock sync |

See [Codebase Summary](./codebase-summary.md#core-modules) for detailed descriptions.

## Development Information

### Build
```bash
cross +nightly build --target mipsel-unknown-linux-musl --release
```

### Deploy
```bash
scp target/mipsel-unknown-linux-musl/release/gateway root@10.10.10.1:/tmp/
ssh root@10.10.10.1 'chmod +x /tmp/gateway && nohup /tmp/gateway > /var/log/gateway.log 2>&1 &'
```

### Test
```bash
curl http://10.10.10.1:8888/
curl -X POST http://10.10.10.1:8888/api/network -H "Content-Type: application/json" -d '{...}'
```

Full deployment guide: [MIPS Build Guide](./mips-build-guide.md)

## Hardware Target

| Spec | Value |
|------|-------|
| SoC | MediaTek MT7688AN |
| CPU | MIPS 24KEc @ 580MHz |
| RAM | 256MB DDR2 |
| Flash | 32MB SPI-NOR (25MB available) |
| OS | OpenWrt 21.02 |
| Network | Ethernet, Wi-Fi 2.4GHz |
| Interfaces | 2x UART, SPI, I2C, GPIO |

Details: [CLAUDE.md](../CLAUDE.md)

## Key Design Decisions

1. **Rust + tiny-http:** Memory safety without garbage collection, minimal HTTP framework for embedded
2. **Bounded channels (128 msgs):** Prevents OOM on 256MB device, natural backpressure
3. **OpenWrt UCI:** Leverages existing OpenWrt config system, survives reboots
4. **No async runtime:** Tokio adds ~1MB; single-threaded HTTP adequate for LAN
5. **HTML templates over assets:** Inline CSS, no external JS, keeps binary <500KB
6. **Manual JSON parsing:** Avoids serde_json dependency overhead

See [System Architecture](./system-architecture.md) for rationale.

## Testing

### Unit Tests
- Validation functions (IP format, netmask, subnet checks)
- Form parsing (URL-encoded, JSON)
- Config serialization

### Integration Tests
- HTTP endpoints (mock server)
- UCI commands (on device)
- UART data flow (loopback)

### Device Tests
- Cross-compile verification
- Deploy to MT7688AN
- Web UI functionality
- Network config persistence
- 7-day soak test (no memory leaks)

See [Project Overview](./project-overview-pdr.md#testing-requirements) for complete test plan.

## Common Tasks

### I want to add a new configuration option
1. Add field to `Config` struct (config.rs)
2. Add form field to HTML template (html_config.rs)
3. Add parsing in `parse_config_form()` (main.rs)
4. Use in publishers via `state.get().new_field`

See [Extensibility Points](./system-architecture.md#extensibility-points)

### I want to add a new HTTP endpoint
1. Add route match in main loop (main.rs)
2. Create handler function
3. Return `tiny_http::Response::from_string()`

### I want to understand the network config validation
1. Read [Network Configuration](./system-architecture.md#2-network-configuration-network_configrs--html_networkrs) section
2. Review validation rules table
3. Check implementation in network_config.rs lines 200-320

### I want to deploy to a device
1. Follow [MIPS Build Guide](./mips-build-guide.md)
2. Verify binary: `file target/mipsel-unknown-linux-musl/release/gateway`
3. Deploy with scp/ssh as shown in [Deploy](#deploy)

### I want to monitor the running gateway
1. SSH to device: `ssh root@10.10.10.1`
2. Check logs: `tail -f /var/log/gateway.log`
3. Check process: `ps aux | grep gateway`
4. Check connectivity: `curl http://localhost:8888/`

## Troubleshooting

| Issue | Check | Solution |
|-------|-------|----------|
| Web UI not responding | Network connectivity to 10.10.10.1:8888 | Verify device is on, check LAN |
| Static IP not applying | Form validation errors | Review error messages, check input format |
| MQTT not publishing | Broker reachability | Test with mosquitto_sub, check logs |
| HTTP POST failing | Endpoint URL and network | Verify URL is reachable, check firewall |
| Device crashes | Memory usage | Check logs, enable core dumps |

See [Project Overview](./project-overview-pdr.md#maintenance--support) for full troubleshooting guide.

## Compliance & Standards

- **Memory Safety:** Rust eliminates buffer overflows, use-after-free
- **Error Handling:** Result types throughout, no unwrap() in critical paths
- **Security:** HTML escaping prevents XSS, UCI quoting prevents injection
- **Code Style:** Modular design, clear naming, <300 lines per file
- **Documentation:** Architecture documented, API specified, examples provided

## References

- **CLAUDE.md** - Project constraints and guidelines
- **AGENTS.md** - Development team and responsibilities
- **Cross.toml** - Cross-compilation configuration
- **.cargo/config.toml** - Rust target configuration
- **Cargo.toml** - Dependencies

## Documentation History

| Date | Version | Changes |
|------|---------|---------|
| 2026-02-12 | 1.0 | Initial comprehensive documentation with Phase 3 (Network Config) complete |

## Getting Help

1. **Questions about architecture?** → Read [System Architecture](./system-architecture.md)
2. **Questions about a module?** → Read [Codebase Summary](./codebase-summary.md)
3. **Questions about requirements?** → Read [Project Overview PDR](./project-overview-pdr.md)
4. **Questions about building?** → Read [MIPS Build Guide](./mips-build-guide.md)
5. **Questions about code?** → Check source files (well-commented, clear logic)

---

**Last Updated:** 2026-02-12
**Maintained By:** Development Team
**Status:** Production Ready
