# Phase 6: Integration & Testing

**Priority:** High
**Status:** pending
**Effort:** 2 days
**Depends on:** All previous phases

## Context

Final integration, end-to-end testing, performance validation, và deployment preparation.

## Objectives

1. Full system integration test
2. Performance benchmarks
3. Stress testing
4. Documentation
5. Deploy script

## Test Scenarios

### 1. UART Data Flow

```
MCU → UART → ugate → [MQTT, HTTP, TCP, WebSocket]
```

Test cases:
- [ ] Single message flow
- [ ] Burst messages (100/sec)
- [ ] Large messages (1KB)
- [ ] Newline variations (\n, \r\n)

### 2. Command Flow

```
[WebSocket, TCP, MQTT] → ugate → [UART TX, GPIO]
```

Test cases:
- [ ] GPIO ON/OFF via WebSocket
- [ ] GPIO TOGGLE via TCP
- [ ] UART TX via MQTT
- [ ] Concurrent commands

### 3. Configuration

Test cases:
- [ ] Config save via API
- [ ] Config reload
- [ ] UCI persistence
- [ ] Network config change

### 4. Authentication

Test cases:
- [ ] Login success
- [ ] Login failure
- [ ] Session persistence
- [ ] Unauthorized access blocked

### 5. WebSocket

Test cases:
- [ ] Connect/disconnect
- [ ] Auto-reconnect
- [ ] Max connections limit
- [ ] Broadcast to all clients

### 6. TCP Channel

Test cases:
- [ ] Server accepts multiple clients
- [ ] Client reconnects on disconnect
- [ ] Bidirectional data flow
- [ ] Both mode simultaneous

## Performance Benchmarks

| Metric | Target | Test Method |
|--------|--------|-------------|
| Binary size | <1.2MB | `ls -l` |
| Startup time | <2s | Measure boot to HTTP ready |
| Memory usage | <20MB | `/proc/{pid}/status` |
| UART latency | <10ms | Timestamp comparison |
| WS latency | <100ms | Round-trip time |
| GPIO response | <50ms | LED toggle timing |
| HTTP throughput | >50 req/s | `wrk` benchmark |

## Stress Tests

### 1. 24-hour stability test

```bash
# On device, run for 24h
./ugate &
while true; do
  curl -s http://localhost:8889/api/status
  sleep 60
done
```

Monitor:
- Memory growth (leak detection)
- CPU usage
- Error logs

### 2. Connection storm

```bash
# Simulate 50 WebSocket connections
for i in {1..50}; do
  wscat -c ws://10.10.10.1:8889/ws &
done
```

### 3. High-frequency UART

```bash
# Send 1000 messages/sec from MCU simulator
while true; do
  echo "data=$(date +%s)" > /dev/ttyS2
  sleep 0.001
done
```

## Implementation Steps

### 1. Create test script

```bash
#!/bin/bash
# test.sh - Integration test runner

HOST=${1:-10.10.10.1}
PORT=8889

echo "Testing ugate on $HOST:$PORT"

# Health check
curl -sf http://$HOST:$PORT/api/status || { echo "FAIL: Health check"; exit 1; }
echo "PASS: Health check"

# Login
curl -sf -X POST http://$HOST:$PORT/api/login -d '{"password":"test"}' || { echo "FAIL: Login"; exit 1; }
echo "PASS: Login"

# Config API
curl -sf http://$HOST:$PORT/api/config || { echo "FAIL: Get config"; exit 1; }
echo "PASS: Get config"

# GPIO (if hardware available)
curl -sf -X POST http://$HOST:$PORT/api/gpio/0 -d '{"state":"toggle"}' || echo "SKIP: GPIO"

echo "All tests passed!"
```

### 2. Create deploy script

```bash
#!/bin/bash
# deploy.sh

TARGET=${1:-root@10.10.10.1}
BINARY="target/mipsel-unknown-linux-musl/release/ugate"

# Build
cross +nightly build --target mipsel-unknown-linux-musl --release -p ugate || exit 1

# Check size
SIZE=$(ls -l $BINARY | awk '{print $5}')
echo "Binary size: $SIZE bytes"
if [ $SIZE -gt 1258291 ]; then
  echo "WARNING: Binary exceeds 1.2MB"
fi

# Deploy
scp $BINARY $TARGET:/tmp/ugate
ssh $TARGET 'chmod +x /tmp/ugate && /etc/init.d/ugate stop 2>/dev/null; cp /tmp/ugate /usr/bin/ugate && /etc/init.d/ugate start'

echo "Deployed successfully"
```

### 3. Create init.d script

```bash
#!/bin/sh /etc/rc.common
# /etc/init.d/ugate

START=99
STOP=10

USE_PROCD=1

start_service() {
    procd_open_instance
    procd_set_param command /usr/bin/ugate
    procd_set_param respawn
    procd_set_param stdout 1
    procd_set_param stderr 1
    procd_close_instance
}
```

### 4. Create default UCI config

```
# /etc/config/ugate

config mqtt
    option enabled '0'
    option broker 'mqtt.example.com'
    option port '8883'
    option tls '1'
    option topic 'iot/gateway'
    option client_id 'ugate-001'

config http
    option enabled '0'
    option url 'https://api.example.com/data'

config tcp
    option enabled '0'
    option mode 'server'
    option server_port '9000'
    option client_host ''
    option client_port '0'

config uart
    # device hardcoded: /dev/ttyS1
    option baudrate '115200'
    option buffer_size '512'
    # Protocol: none | frame | modbus
    option protocol 'none'
    # Gap-based (none/modbus)
    option gap_ms '10'
    # Frame mode
    option frame_length '16'
    option frame_timeout '100'
    option tag_enabled '0'
    option tag_head '0x00'
    option tag_tail '0x0A'
    # Modbus mode
    option slave_addr '0x01'

config gpio
    option chip 'gpiochip0'
    list output '11'
    list output '12'
    list output '14'
    list output '15'
    option heartbeat '44'
    option heartbeat_interval '500'

config web
    option port '8888'
    option password 'admin'
    option max_ws_connections '8'

config general
    option interval_secs '3'
```

## Documentation Updates

- [ ] Update README.md with usage
- [ ] Update docs/system-architecture.md
- [ ] Create docs/api-reference.md
- [ ] Create docs/deployment-guide.md

## Files to Create

| File | Description |
|------|-------------|
| scripts/test.sh | Integration test |
| scripts/deploy.sh | Deploy script |
| configs/ugate.init | OpenWrt init.d |
| configs/ugate | Default UCI config |

## Todo

- [ ] Create test.sh
- [ ] Create deploy.sh
- [ ] Create init.d script
- [ ] Create default config
- [ ] Run UART data flow tests
- [ ] Run command flow tests
- [ ] Run config tests
- [ ] Run auth tests
- [ ] Run WebSocket tests
- [ ] Run TCP tests
- [ ] Measure binary size
- [ ] Measure memory usage
- [ ] Run 24h stability test
- [ ] Update documentation

## Success Criteria

- [ ] All test scenarios pass
- [ ] Binary <1.2MB
- [ ] Memory <20MB
- [ ] 24h test stable
- [ ] Documentation complete
- [ ] Deploy script works

## Deliverables

1. Working firmware binary
2. Default configuration
3. OpenWrt init script
4. Test scripts
5. Updated documentation
