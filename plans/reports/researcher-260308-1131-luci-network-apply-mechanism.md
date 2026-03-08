# LuCI Network Configuration Apply/Commit Mechanism

## Executive Summary

LuCI uses a **three-phase apply/confirm/rollback workflow** managed by rpcd (via ubus), NOT direct service restarts. This minimizes disruption by only reloading changed services and includes automatic rollback on connectivity loss.

---

## 1. How LuCI Applies Network Changes ("Save & Apply" Button)

**Workflow (3 phases):**

1. **UCI Commit**: `ubus call uci apply` — commits staging area to /etc/config files
2. **Async Reload**: rpcd spawns async service reloads (via ucitrack/hotplug)
3. **Polling Confirm**: Frontend polls `ubus call uci confirm` for 27-30 seconds
   - If confirm succeeds → rollback timer cancelled (safe)
   - If connectivity lost → rpcd auto-reverts config after timeout (protection)

**Code Flow:**
- User clicks "Save & Apply"
- LuCI calls `view.handleSave()` → stages changes via `uci set/add/delete`
- Then calls `ui.changes.apply()` → invokes `luci.model.uci.apply()`
- Which calls `ubus call uci apply`
- Frontend enters countdown loop, polling `ubus call uci confirm`

---

## 2. Commands Used: `ubus` vs `uci commit` vs Service Restart

| Command | Purpose | Disruption |
|---------|---------|-----------|
| `ubus call uci apply` | Commit staging + arm rollback timer | Low (async reload only) |
| `ubus call uci confirm` | Cancel rollback timer (confirm safe) | None |
| `uci commit` | Write staging to flash (legacy, no rollback) | Low (staging → file) |
| `ubus call network reload` | Reconfigure changed interfaces only | **Minimal** |
| `/etc/init.d/network restart` | Full restart all interfaces | **High** |
| `/sbin/luci-reload` | Read ucitrack, trigger service reloads | Varies by service |

**Modern LuCI Preference:**
- **Use `ubus call uci apply`** (with confirm) — best for network safety
- **Avoid direct `uci commit`** — no rollback protection
- **Avoid full restart** — use `ubus call network reload` for minimal impact

---

## 3. Rollback/Timeout Mechanism

**How It Works (rpcd-managed):**

1. `ubus call uci apply {"rollback": true, "timeout": 30}`
   - rpcd spawns rollback timer (default 30s, configurable)
   - Commits /tmp/uci.changes to /etc/config/
   - Returns immediately

2. Frontend polls `ubus call uci confirm` every 1-2 seconds
   - If confirm succeeds → rpcd cancels timer
   - Connection is alive → config is safe

3. If confirm fails (no response for 30s)
   - rpcd auto-reverts /etc/config/* to pre-apply state
   - Restores /tmp/uci.changes.* backup
   - Rollback complete (device not bricked)

**Timeout Config:**
```bash
uci set luci.apply.timeout=60  # Default 30s, can increase to 60s
uci commit luci
```

**Backup Storage:**
- Before apply: `/tmp/uci.changes.*` files backed up
- On rollback: rpcd restores from backup

---

## 4. `ubus call uci apply` vs `uci commit` + Service Restart

**`ubus call uci apply` (PREFERRED):**
- Atomically commits + arms rollback
- Automatic async service reload via ucitrack triggers
- **Includes safety timeout mechanism**
- Returns immediately, reload happens async
- Example: `ubus call uci apply '{"rollback":true,"timeout":30}'`

**`uci commit` + Manual Restart (LEGACY):**
- Commits but NO rollback protection
- Manual service restart needed
- Higher latency (synchronous)
- Risk of device becoming unreachable permanently
- Example: `uci commit && /etc/init.d/network restart`

**Network-Specific:**
- `ubus call network reload` — only reconfigures changed interfaces (netifd)
- `/etc/init.d/network restart` — full restart (more disruption)

---

## 5. Service Reload Triggering: ucitrack vs Hotplug

**Legacy (ucitrack):**
- `/etc/config/ucitrack` maps config files → service restart commands
- Format: `config <service>` → `list affects <related_service>`
- `/sbin/luci-reload` reads ucitrack and calls service init scripts

**Modern (procd service_triggers):**
- Service init scripts define reload hooks
- `reload_service() { ... }` handler in init script
- Example: dnsmasq reloads on dhcp config change

**Hotplug (for async reloads):**
- Network hotplug events: `ifup`, `ifdown`, `ifupdate`
- Scripts in `/etc/hotplug.d/iface/` triggered on interface changes
- Used for firewall reload, etc.

**Practical: Minimal Disruption Flow**
```
ubus call uci apply
  ↓
netifd receives config change event
  ↓
ubus call network reload (only changed interfaces)
  ↓
hotplug triggers /etc/hotplug.d/iface/* scripts
  ↓
firewall, dnsmasq, etc. reload as needed
```

---

## Key Findings for ugate Implementation

1. **Use `ubus call uci apply` with timeout** — not raw `uci commit`
   - Provides automatic rollback on connectivity loss
   - Safer than manual restart

2. **For network-only changes:**
   - `ubus call network reload` (async, minimal disruption)
   - NOT `/etc/init.d/network restart` (full restart, higher impact)

3. **Polling strategy:**
   - Frontend should poll `ubus call uci confirm` every 1-2 seconds for 27-30 seconds
   - If confirm succeeds → config is confirmed safe
   - If timeout → show "Apply without safety check" option (user responsibility)

4. **For custom services (e.g., ugate itself):**
   - Add service triggers in `/etc/config/ucitrack` OR
   - Use procd `service_triggers` in init script
   - Example: When network config changes, ugate can subscribe to hotplug events and reload

5. **Backup/Restore:**
   - rpcd handles backup automatically in `/tmp/uci.changes.*`
   - No manual backup needed for rollback mechanism

---

## References

- [UCI Apply/Rollback Workflow - LuCI PR#1769](https://github.com/openwrt/luci/pull/1769)
- [UBUS Network Reload vs Restart](https://forum.openwrt.org/t/ubus-call-uci-commit-config-network-is-not-reloading-the-network/91527)
- [OpenWrt RPCd UCI Implementation](https://github.com/git-openwrt-org-mirror/rpcd/blob/master/uci.c)
- [Synchronous Network Reload/Restart Discussion](https://openwrt-devel.openwrt.narkive.com/IQG7WWCm/synchronous-network-reload-restart)
- [ucitrack Configuration](https://github.com/openwrt/luci/blob/master/modules/luci-base/root/etc/init.d/ucitrack)
- [Hotplug Events Documentation](https://trac.gateworks.com/wiki/OpenWrt/hotplug)

---

## Unresolved Questions

1. **Exact timeout default in current rpcd**: Is it 30s or 60s? (sources vary; PR#4528 suggests default was updated)
2. **Procd service_triggers adoption**: How widely adopted in OpenWrt 24.10? ucitrack still present or replaced?
3. **Rollback edge case**: What if a service fails during reload? Does rollback still trigger after timeout?
