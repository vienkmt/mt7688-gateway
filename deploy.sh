#!/bin/bash
# Deploy to OpenWrt MT7688 (192.168.2.188)
# Usage: ./deploy.sh [-b] to build first

BINARY="target/mipsel-unknown-linux-musl/release/v3s-system-monitor"
HOST="root@192.168.2.188"
REMOTE="/tmp/v3s-system-monitor"

[[ "$1" == "build" ]] && { echo "Building..."; ./build.sh || exit 1; }
[ ! -f "$BINARY" ] && echo "Build first: ./build.sh" && exit 1

echo "Stopping old process..."
ssh "$HOST" "killall v3s-system-monitor 2>/dev/null; sleep 1"

echo "Uploading..."
scp -O "$BINARY" "$HOST:$REMOTE" && \
ssh "$HOST" "chmod +x $REMOTE && $REMOTE > /tmp/v3s.log 2>&1 &"
sleep 1
ssh "$HOST" "ps | grep v3s-system && echo '=== Running ===' || cat /tmp/v3s.log"
