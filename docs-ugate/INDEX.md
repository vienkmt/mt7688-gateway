# ugate Documentation Index

**Quick navigation guide for ugate IoT Gateway documentation**

---

## Documentation Structure

```
docs-ugate/
├── INDEX.md                 ← You are here
├── README.md               ← Start here: Project overview
├── architecture.md         ← System design & internals
├── config.md              ← Configuration reference
├── web-ui.md              ← Frontend & API documentation
├── deployment.md          ← Build & deployment guide
└── troubleshooting.md     ← Known issues & solutions
```

---

## Quick Start by Role

### I'm a New Developer
1. Read: [README.md](./README.md) (5 min)
2. Read: [architecture.md](./architecture.md) (15 min)
3. Follow: [deployment.md](./deployment.md) → **Build** section
4. Deploy: [deployment.md](./deployment.md) → **Method 1: Deploy Script**

**Total time:** ~30 minutes to build and deploy

---

### I'm a System Operator
1. Read: [README.md](./README.md) (5 min)
2. Configure: [config.md](./config.md) → Find your section
3. Deploy: [deployment.md](./deployment.md) → **Deployment Methods**
4. Access: [README.md](./README.md) → **Truy cập** section (Web UI)
5. Troubleshoot: [troubleshooting.md](./troubleshooting.md) (as needed)

**Total time:** ~20 minutes to deploy and verify

---

### I'm a Web Developer
1. Read: [web-ui.md](./web-ui.md) (10 min)
2. API Reference: [web-ui.md](./web-ui.md) → **API Endpoints**
3. WebSocket Details: [web-ui.md](./web-ui.md) → **WebSocket Connection**
4. Architecture Context: [architecture.md](./architecture.md) → **WebSocket Architecture**

**Total time:** ~15 minutes to understand API

---

### I'm Debugging an Issue
1. Go to: [troubleshooting.md](./troubleshooting.md)
2. Check: **Known Issues** section (8 MIPS/Rust issues)
3. Or find: **Common Problems** (10 problems with solutions)
4. Reference: [config.md](./config.md) for configuration details
5. Run: **Debugging** section scripts

**Total time:** ~10 minutes to diagnose

---

### I'm Deploying to Production
1. Plan: [deployment.md](./deployment.md) → **Requirements**
2. Build: [deployment.md](./deployment.md) → **Build Process**
3. Deploy: [deployment.md](./deployment.md) → **Method 1: Deploy Script** OR **Method 2**
4. Verify: [deployment.md](./deployment.md) → **Verify Deployment**
5. Backup: [deployment.md](./deployment.md) → **Backup & Restore**

**Total time:** ~45 minutes total

---

## Document Purposes

| Document | Purpose | Audience |
|----------|---------|----------|
| [README.md](./README.md) | Project overview, features, quick start | Everyone |
| [architecture.md](./architecture.md) | System design, async runtime, channels | Developers, Architects |
| [config.md](./config.md) | Complete UCI configuration reference | Operators, DevOps |
| [web-ui.md](./web-ui.md) | Frontend interface, API, WebSocket | Web devs, Integrators |
| [deployment.md](./deployment.md) | Build, cross-compile, deploy procedures | DevOps, Operators |
| [troubleshooting.md](./troubleshooting.md) | Known issues, common problems, recovery | Support, Operators |

---

## Key Topics Quick Reference

### Architecture & Design
- **Async Runtime:** [architecture.md](./architecture.md) → Async Runtime
- **Channel Architecture:** [architecture.md](./architecture.md) → Channel Architecture
- **Task Topology:** [architecture.md](./architecture.md) → Task Topology
- **MQTT Design:** [architecture.md](./architecture.md) → MQTT Architecture
- **GPIO Control:** [architecture.md](./architecture.md) → GPIO Controller
- **WebSocket:** [architecture.md](./architecture.md) → WebSocket Architecture

### Configuration
- **UART Settings:** [config.md](./config.md) → [uart]
- **MQTT Broker:** [config.md](./config.md) → [mqtt]
- **HTTP Endpoint:** [config.md](./config.md) → [http]
- **TCP Relay:** [config.md](./config.md) → [tcp]
- **GPIO Pins:** [config.md](./config.md) → [gpio]
- **Web Server:** [config.md](./config.md) → [web]
- **Complete Example:** [config.md](./config.md) → Complete Example Config

### Web UI & API
- **Dashboard Tabs:** [web-ui.md](./web-ui.md) → Dashboard Tabs
- **Status Tab:** [web-ui.md](./web-ui.md) → Tab 1: Trạng thái
- **Config Tab:** [web-ui.md](./web-ui.md) → Tab 2: Cấu hình
- **UART Tab:** [web-ui.md](./web-ui.md) → Tab 3: UART
- **API Endpoints:** [web-ui.md](./web-ui.md) → API Endpoints
- **GPIO Control:** [web-ui.md](./web-ui.md) → GPIO Control

### Deployment
- **Build Process:** [deployment.md](./deployment.md) → Build Process
- **Deploy Script:** [deployment.md](./deployment.md) → Method 1: Deploy Script
- **Manual Deploy:** [deployment.md](./deployment.md) → Method 2: Manual SCP + SSH
- **Init Script:** [deployment.md](./deployment.md) → Init Script (procd)
- **Configuration:** [deployment.md](./deployment.md) → Configuration & First Run
- **Verification:** [deployment.md](./deployment.md) → Verify Deployment

### Troubleshooting
- **MIPS Issues:** [troubleshooting.md](./troubleshooting.md) → Known Issues
- **AtomicU64:** [troubleshooting.md](./troubleshooting.md) → Issue 1
- **WebSocket:** [troubleshooting.md](./troubleshooting.md) → Issue 3, 4
- **Deploy Script:** [troubleshooting.md](./troubleshooting.md) → Issue 6
- **MQTT Issues:** [troubleshooting.md](./troubleshooting.md) → Problem: MQTT Connection Fails
- **GPIO Issues:** [troubleshooting.md](./troubleshooting.md) → Problem: GPIO Control Not Working

---

## File Statistics

| File | Lines | Size | Sections |
|------|-------|------|----------|
| README.md | 150 | 4.4K | 8 |
| architecture.md | 266 | 8.6K | 12 |
| config.md | 314 | 7.6K | 9 |
| web-ui.md | 401 | 8.3K | 8 |
| deployment.md | 418 | 8.2K | 11 |
| troubleshooting.md | 499 | 11K | 15 |
| **TOTAL** | **2,048** | **48K** | **63** |

---

## Language

**All documentation written in Vietnamese** with technical terms in English:
- Configuration keys: English (ugate, mqtt, broker, etc.)
- Code examples: English (Rust, JSON, bash)
- Comments: Vietnamese
- Section headers: Vietnamese
- Technical concepts: English when no Vietnamese equivalent

---

## How to Use This Documentation

### Reading Order

**For first-time setup:**
1. README.md (5 min) → Overview
2. deployment.md (10 min) → Build & deploy
3. config.md (10 min) → Configure your device
4. README.md "Truy cập" (2 min) → Access Web UI
5. web-ui.md (5 min) → Understand tabs

**For deep understanding:**
1. architecture.md (20 min) → System internals
2. config.md (15 min) → All configuration options
3. web-ui.md (15 min) → API & frontend
4. deployment.md (15 min) → Complete deployment flow
5. troubleshooting.md (10 min) → Known issues & solutions

**For problem solving:**
1. troubleshooting.md → Find your problem
2. config.md → Check configuration if needed
3. deployment.md → Verify setup steps
4. architecture.md → Understand root cause

---

## Documentation Features

### Tables
Every document includes lookup tables for quick reference:
- Hardware specs (README)
- Channel types (architecture)
- Configuration fields (config)
- API endpoints (web-ui)
- Troubleshooting problems (troubleshooting)

### Code Examples
Real, working examples provided for:
- Build commands
- Deploy scripts
- Configuration files
- API calls (CURL)
- Bash diagnostic scripts

### ASCII Diagrams
System visualization includes:
- Channel flow (architecture)
- Task topology (architecture)
- Dashboard layouts (web-ui)

### Cross-References
Documents link to each other for:
- Configuration details → config.md
- API endpoints → web-ui.md
- Known issues → troubleshooting.md
- System design → architecture.md

---

## Updates & Maintenance

**Last Updated:** 2026-03-07

**Next Review:** After major feature changes

**Maintenance:**
- Update on: Feature additions, major bug fixes, config changes
- Review: Quarterly or after deployment
- Verify: Links, code examples, configuration accuracy

---

## Support Resources

### External References
- [OpenWrt Documentation](https://openwrt.org/docs)
- [Rust Book](https://doc.rust-lang.org/book/)
- [Tokio Runtime](https://tokio.rs/tokio/tutorial)
- [MQTT Protocol](https://mqtt.org/)
- [MediaTek MT7628](https://www.mediatek.com/)

### Internal References
- `mips-rust-notes/bugs-and-gotchas.md` — MIPS-specific issues
- `CLAUDE.md` — Project guidelines
- `src/` — Actual implementation code
- `Cargo.toml` — Dependencies

---

## Quick Commands Reference

### Build
```bash
cross +nightly build --target mipsel-unknown-linux-musl --release -p ugate
```

### Deploy
```bash
./deploy.sh              # Automated
# or
./deploy.sh --skip-build # Skip build if binary exists
```

### Verify
```bash
ssh root@device pgrep ugate
curl http://device:8888/api/status | jq
```

### Configure
```bash
ssh root@device uci show ugate
ssh root@device vi /etc/config/ugate
ssh root@device /etc/init.d/ugate restart
```

### Debug
```bash
ssh root@device logread | grep ugate
ssh root@device /usr/bin/ugate           # Run in foreground
curl http://device:8888/api/status | jq .stats
```

---

## Contact & Support

**For documentation issues:**
- Check: [troubleshooting.md](./troubleshooting.md)
- Review: [config.md](./config.md) for configuration questions
- Verify: [deployment.md](./deployment.md) for setup issues

**For code issues:**
- Check: `mips-rust-notes/bugs-and-gotchas.md`
- Read: [architecture.md](./architecture.md) for system design
- Search: Source code in `ugate/src/`

---

**Start with [README.md](./README.md) or choose your role above.**

Last updated: 2026-03-07
