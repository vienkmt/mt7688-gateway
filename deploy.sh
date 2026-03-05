#!/bin/bash
# Deploy to OpenWrt MT7688 (192.168.2.168)
# Usage: ./deploy.sh [build] to build first

BINARY="target/mipsel-unknown-linux-musl/release/vgateway"
HOST="root@192.168.2.168"
REMOTE="/usr/bin/vgateway"

[[ "$1" == "build" ]] && { echo "Building..."; ./build.sh || exit 1; }
[ ! -f "$BINARY" ] && echo "Build first: ./build.sh" && exit 1

echo "Stopping old process..."
ssh "$HOST" "killall vgateway v3s v3s-system-monitor 2>/dev/null; sleep 1"

echo "Uploading..."
scp -O "$BINARY" "$HOST:$REMOTE" && \
ssh "$HOST" "chmod +x $REMOTE && /etc/init.d/vgateway restart"
sleep 2
ssh "$HOST" "ps | grep -v grep | grep vgateway && echo '=== Running ===' || logread | tail -10"
