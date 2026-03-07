# ugate - Triển khai (Deployment)

## Yêu cầu

### Development Machine
- `rustup` + `cargo`
- `cross` tool: `cargo install cross`
- `nightly` toolchain: `rustup toolchain install nightly`
- Docker hoặc OrbStack (để chạy containers)

### Target Device
- OpenWrt 24.10 (Kernel 6.6.x)
- Architecture: ramips/mt76x8 (MIPS)
- Connectivity: SSH access
- 16MB Flash (ugate binary ~500KB)

## Build Process

### Step 1: Cross-compile

```bash
cd /path/to/mt7688-gateway

# Build ugate
cross +nightly build --target mipsel-unknown-linux-musl --release -p ugate

# Binary location
ls -lh target/mipsel-unknown-linux-musl/release/ugate
```

**Output Example:**
```
-rwxr-xr-x 512K ugate
```

**Build time:** ~2-5 minutes (depending on cache)

### Step 2: Binary Verification

```bash
# Check architecture
file target/mipsel-unknown-linux-musl/release/ugate
# Output: ELF 32-bit LSB executable, MIPS, MIPS32 rel2 version 1...

# Check stripped
nm target/mipsel-unknown-linux-musl/release/ugate | head
# Output: (mostly empty, symbols stripped)
```

## Deployment Methods

### Method 1: Deploy Script (Recommended)

**Automatic build + deploy + verify**

```bash
./deploy.sh              # Build & deploy ugate (default)
./deploy.sh ugate        # Explicit ugate
./deploy.sh vgateway     # Deploy vgateway instead
./deploy.sh --build-only # Only build, no deploy
./deploy.sh --skip-build # Only deploy, assume binary exists
```

**What it does:**
1. Check Docker running (for cross)
2. Build with `cross +nightly`
3. Verify binary exists
4. SCP binary to device `/usr/bin/`
5. SSH: create procd init script
6. SSH: start service
7. Wait 8 seconds
8. Verify process running with `pgrep`

**Configuration in deploy.sh:**
```bash
HOST="root@192.168.2.171"       # SSH address
TARGET="mipsel-unknown-linux-musl"
REMOTE_DIR="/usr/bin"           # Installation directory
```

**Example output:**
```
┌─────────────────────────────────┐
│  Deploy ugate → root@192.168.2.171
└─────────────────────────────────┘

▶ [1/4] Building ugate...
✓ Build successful

▶ [2/4] Checking binary...
✓ Found: ugate (~512KB)

▶ [3/4] Uploading to device...
ugate 100% |████████| 512K

▶ [4/4] Creating init script & starting...
✓ Service ugate started
✓ Verified running (PID 1234)

Done in 45 seconds!
```

### Method 2: Manual SCP + SSH

**Step-by-step:**

```bash
# 1. Copy binary to device
scp target/mipsel-unknown-linux-musl/release/ugate \
    root@192.168.2.171:/usr/bin/

# 2. Make executable
ssh root@192.168.2.171 chmod +x /usr/bin/ugate

# 3. Start service (creates init script if needed)
ssh root@192.168.2.171 /etc/init.d/ugate restart

# 4. Verify running
ssh root@192.168.2.171 pgrep -a ugate
# Output: 1234 /usr/bin/ugate
```

### Method 3: Manual Binary Run (Debugging)

**Direct execution (non-daemon):**

```bash
# SSH to device
ssh root@192.168.2.171

# Run directly (see logs on stdout)
/usr/bin/ugate

# Or with explicit config
UCI_CONFIG=/etc/config/ugate /usr/bin/ugate
```

**Exit:** Ctrl+C

## Init Script (procd)

**Automatic:** Created when service first starts via deploy.sh

**Location:** `/etc/init.d/ugate`

**Manual creation:**
```bash
cat > /etc/init.d/ugate << 'EOF'
#!/bin/sh /etc/rc.common
START=99
STOP=15

start() {
    procd_open_instance
    procd_set_param command /usr/bin/ugate
    procd_set_param stdout 1
    procd_set_param stderr 1
    procd_set_param respawn 3600 5 5
    procd_close_instance
}

stop() {
    procd_send_signal ugate
}
EOF
chmod +x /etc/init.d/ugate
```

**Service Control:**
```bash
# Start
/etc/init.d/ugate start

# Stop
/etc/init.d/ugate stop

# Restart
/etc/init.d/ugate restart

# Enable on boot
/etc/init.d/ugate enable

# Disable on boot
/etc/init.d/ugate disable

# Status
/etc/init.d/ugate status
```

## Configuration & First Run

### Step 1: Create Config File

```bash
ssh root@192.168.2.171

cat > /etc/config/ugate << 'EOF'
config general
    option device_name 'ugate-01'
    option interval_secs '3'

config uart
    option enabled '1'
    option port '/dev/ttyS1'
    option baudrate '115200'
    option frame_mode 'modbus'

config mqtt
    option enabled '1'
    option broker 'broker.emqx.io'
    option port '8883'
    option tls '1'
    option topic 'devices/gateway/data'

config gpio
    option led_pin '44'
    option pins '17 18'

config web
    option port '8888'
    option password 'admin'
EOF
```

### Step 2: Start Service

```bash
/etc/init.d/ugate restart
```

### Step 3: Verify

```bash
# Check process
ps aux | grep ugate

# Check logs
logread | grep ugate

# Check network (if MQTT enabled)
netstat -tn | grep 8883

# Check port listening (Web UI)
netstat -tlnp | grep 8888
```

## Cross-compilation Setup

### Requirement: Docker/OrbStack

**Why:** `cross` runs compiler inside container to avoid musl library conflicts.

**Option 1: OrbStack (Recommended for Mac)**
```bash
brew install orbstack
orbstack start

# Verify
docker ps
```

**Option 2: Docker Desktop**
```bash
# Download from https://www.docker.com/products/docker-desktop/
# Start Docker Desktop

# Verify
docker ps
```

### Install cross

```bash
cargo install cross

# Verify
cross --version
# Output: cross 0.2.5
```

### Nightly Toolchain

```bash
rustup toolchain install nightly

# Verify
rustup +nightly --version
```

## Troubleshooting Deployment

| Problem | Cause | Solution |
|---------|-------|----------|
| `cross` fails to run | Docker not running | Start Docker/OrbStack first |
| Binary not found after build | Wrong target dir | Check `target/mipsel-unknown-linux-musl/release/` |
| SSH connection fails | Wrong IP/SSH key | Verify device IP, check SSH config |
| Service won't start | Binary permission | `chmod +x /usr/bin/ugate` |
| Port 8888 not responding | Firewall/config | Check `netstat -tlnp`, restart service |
| UART device not found | Device doesn't exist | `ls /dev/ttyS*` on device |

## Verify Deployment

### Quick Check

```bash
# Device IP
DEVICE=192.168.2.171

# 1. Binary exists
ssh root@$DEVICE ls -lh /usr/bin/ugate

# 2. Service running
ssh root@$DEVICE pgrep ugate

# 3. Web UI accessible
curl -s http://$DEVICE:8888/ | head -5

# 4. Status API
curl -s http://$DEVICE:8888/api/status | jq .

# 5. Logs
ssh root@$DEVICE logread | tail -20
```

### Full Health Check

```bash
#!/bin/bash
DEVICE=192.168.2.171
echo "Checking ugate on $DEVICE..."

# Check binary
echo -n "Binary: "
ssh root@$DEVICE test -x /usr/bin/ugate && echo "✓" || echo "✗"

# Check process
echo -n "Process: "
ssh root@$DEVICE pgrep ugate > /dev/null && echo "✓" || echo "✗"

# Check port
echo -n "Port 8888: "
ssh root@$DEVICE netstat -tlnp | grep :8888 > /dev/null && echo "✓" || echo "✗"

# Check UART (if enabled)
echo -n "UART port: "
ssh root@$DEVICE test -c /dev/ttyS1 && echo "✓" || echo "✗"

# Check config
echo -n "Config exists: "
ssh root@$DEVICE test -f /etc/config/ugate && echo "✓" || echo "✗"

# Web UI
echo -n "Web UI: "
curl -s http://$DEVICE:8888/api/status > /dev/null && echo "✓" || echo "✗"

echo "Done!"
```

## File Sizes

| Component | Size |
|-----------|------|
| Binary | ~500KB |
| Config | ~1KB |
| Init script | ~300B |
| **Total** | **~500KB** |

**Flash usage:** ~5% of 16MB (plenty of space)

## Backup & Restore

### Backup

```bash
DEVICE=192.168.2.171

# Backup binary + config
ssh root@$DEVICE "tar czf /tmp/ugate-backup.tar.gz \
  /usr/bin/ugate \
  /etc/config/ugate \
  /etc/init.d/ugate" \
  && scp root@$DEVICE:/tmp/ugate-backup.tar.gz ./ugate-backup.tar.gz

# Or just config
scp root@$DEVICE:/etc/config/ugate ./ugate-config.backup
```

### Restore

```bash
# Restore from backup
tar xzf ugate-backup.tar.gz -C /
/etc/init.d/ugate restart

# Or restore config only
scp ugate-config.backup root@$DEVICE:/etc/config/ugate
ssh root@$DEVICE /etc/init.d/ugate restart
```

## Rollback

If new version has issues:

```bash
# Keep old binary
ssh root@$DEVICE cp /usr/bin/ugate /usr/bin/ugate.new
ssh root@$DEVICE cp /usr/bin/ugate.old /usr/bin/ugate  # if backup exists
ssh root@$DEVICE /etc/init.d/ugate restart
```

Or recompile previous version and redeploy.

## Performance Notes

- **Build time:** 2-5 min (incremental: 30 sec)
- **Deploy time:** 1-2 min (transfer + init)
- **Service startup:** 5-8 sec (MIPS is slow)
- **Binary load:** <10MB RAM
