# Phase 7: OpenWrt Packaging

**Priority:** Medium
**Status:** pending
**Effort:** 1 day
**Depends on:** Phase 6

## Context

Đóng gói binary thành IPK chuẩn OpenWrt. Một script duy nhất để build & package.

## Deliverables

1. `build-ipk.sh` - Script đóng gói tự động
2. IPK file cài được bằng `opkg install`
3. Service chạy tự động khi boot

## IPK Structure

```
ugate_1.0.0-1_mipsel_24kc.ipk
├── debian-binary          # "2.0"
├── control.tar.gz
│   ├── control            # Package metadata
│   ├── postinst           # After install script
│   ├── prerm              # Before remove script
│   └── conffiles          # Config files to preserve
└── data.tar.gz
    ├── usr/bin/ugate      # Binary
    ├── etc/config/ugate   # UCI config
    └── etc/init.d/ugate   # Init script (procd)
```

## Implementation

### 1. build-ipk.sh

```bash
#!/bin/bash
set -e

PKG_NAME="ugate"
PKG_VERSION="1.0.0"
PKG_RELEASE="1"
PKG_ARCH="mipsel_24kc"

# Paths
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
ROOT_DIR="$(dirname "$SCRIPT_DIR")"
BUILD_DIR="$ROOT_DIR/target/ipk-build"
IPK_DIR="$BUILD_DIR/$PKG_NAME"
OUTPUT_DIR="$ROOT_DIR/target/ipk"

# Clean
rm -rf "$BUILD_DIR" "$OUTPUT_DIR"
mkdir -p "$IPK_DIR"/{CONTROL,usr/bin,etc/config,etc/init.d}
mkdir -p "$OUTPUT_DIR"

# 1. Cross-compile binary
echo "Building $PKG_NAME..."
cd "$ROOT_DIR"
cross +nightly build --target mipsel-unknown-linux-musl --release -p ugate

# 2. Copy binary
cp "$ROOT_DIR/target/mipsel-unknown-linux-musl/release/ugate" "$IPK_DIR/usr/bin/"
chmod 755 "$IPK_DIR/usr/bin/ugate"

# 3. Create control file
cat > "$IPK_DIR/CONTROL/control" << EOF
Package: $PKG_NAME
Version: ${PKG_VERSION}-${PKG_RELEASE}
Architecture: $PKG_ARCH
Maintainer: vienkmt
Description: IoT Gateway - UART to MQTT/HTTP/TCP with WebSocket
Depends: libc
EOF

# 4. Create conffiles
cat > "$IPK_DIR/CONTROL/conffiles" << EOF
/etc/config/ugate
EOF

# 5. Create postinst
cat > "$IPK_DIR/CONTROL/postinst" << 'EOF'
#!/bin/sh
[ -n "$IPKG_INSTROOT" ] && exit 0
/etc/init.d/ugate enable
/etc/init.d/ugate start
EOF
chmod 755 "$IPK_DIR/CONTROL/postinst"

# 6. Create prerm
cat > "$IPK_DIR/CONTROL/prerm" << 'EOF'
#!/bin/sh
[ -n "$IPKG_INSTROOT" ] && exit 0
/etc/init.d/ugate stop
/etc/init.d/ugate disable
EOF
chmod 755 "$IPK_DIR/CONTROL/prerm"

# 7. Create UCI config
cat > "$IPK_DIR/etc/config/ugate" << EOF
config ugate 'main'
    option enabled '1'

config uart 'uart'
    option device '/dev/ttyS1'
    option baudrate '115200'

config mqtt 'mqtt'
    option enabled '0'
    option broker '127.0.0.1'
    option port '1883'
    option username ''
    option password ''
    option client_id 'ugate'
    option topic_publish 'ugate/data'
    option topic_subscribe 'ugate/cmd'
    option qos '0'

config http 'http'
    option enabled '0'
    option url 'http://example.com/api'
    option interval '60'

config tcp 'tcp'
    option enabled '0'
    option mode 'server'
    option server_port '9000'
    option client_host ''

config web 'web'
    option port '8888'
    option password ''

config gpio 'gpio'
    option out1 '0'
    option out2 '1'
    option out3 '2'
    option out4 '3'
    option led '4'
EOF

# 8. Create init.d script (procd)
cat > "$IPK_DIR/etc/init.d/ugate" << 'EOF'
#!/bin/sh /etc/rc.common

START=99
STOP=10
USE_PROCD=1
PROG=/usr/bin/ugate

start_service() {
    local enabled
    config_load ugate
    config_get enabled main enabled 0

    [ "$enabled" = "1" ] || return 0

    procd_open_instance
    procd_set_param command $PROG
    procd_set_param respawn
    procd_set_param stdout 1
    procd_set_param stderr 1
    procd_close_instance
}

service_triggers() {
    procd_add_reload_trigger "ugate"
}

reload_service() {
    stop
    start
}
EOF
chmod 755 "$IPK_DIR/etc/init.d/ugate"

# 9. Build IPK
cd "$BUILD_DIR"
echo "2.0" > debian-binary

# Create control.tar.gz
cd "$IPK_DIR/CONTROL"
tar czf "$BUILD_DIR/control.tar.gz" ./*

# Create data.tar.gz
cd "$IPK_DIR"
tar czf "$BUILD_DIR/data.tar.gz" ./usr ./etc

# Create IPK
cd "$BUILD_DIR"
IPK_FILE="$OUTPUT_DIR/${PKG_NAME}_${PKG_VERSION}-${PKG_RELEASE}_${PKG_ARCH}.ipk"
tar czf "$IPK_FILE" ./debian-binary ./control.tar.gz ./data.tar.gz

echo ""
echo "IPK created: $IPK_FILE"
echo "Size: $(du -h "$IPK_FILE" | cut -f1)"
echo ""
echo "Install: opkg install $IPK_FILE"
echo "Or copy to device: scp $IPK_FILE root@10.10.10.1:/tmp/"
```

## Usage

```bash
# Build IPK
./build-ipk.sh

# Copy to device
scp target/ipk/ugate_1.0.0-1_mipsel_24kc.ipk root@10.10.10.1:/tmp/

# Install on device
ssh root@10.10.10.1 "opkg install /tmp/ugate_*.ipk"

# Check status
ssh root@10.10.10.1 "/etc/init.d/ugate status"
```

## Todo

- [ ] Create build-ipk.sh
- [ ] Test build IPK
- [ ] Test install on device
- [ ] Test service start/stop
- [ ] Test config reload
- [ ] Test upgrade (preserve config)

## Success Criteria

- [ ] IPK builds without error
- [ ] `opkg install` works
- [ ] Service starts on boot
- [ ] Config preserved on upgrade
- [ ] `opkg remove` cleans up

## Notes

- procd tự động restart nếu crash (`respawn`)
- Config `/etc/config/ugate` được preserve khi upgrade
- Binary strip đã làm trong Cargo.toml profile.release
