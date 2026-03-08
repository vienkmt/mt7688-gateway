# Phase 1: Backend — toolbox.rs Module

## Context Links
- [WebSocket handler](../../ugate/src/web/ws.rs) — broadcast channel pattern
- [Web mod.rs](../../ugate/src/web/mod.rs) — shared helpers (json_resp, json_err, json_escape, jval)
- [Server routes](../../ugate/src/web/server.rs) — route wiring pattern

## Overview
- **Priority:** High (core feature)
- **Status:** Pending
- **Description:** Create `ugate/src/web/toolbox.rs` (~120 lines) implementing 3 network diagnostic tools that stream output via WS broadcast

## Key Insights

1. **WS broadcast reuse**: UART data flows via `ws_manager.broadcast_tx.send(json_string)` — toolbox does the same with `{"type":"toolbox",...}` messages
2. **Blocking context**: server.rs runs in `spawn_blocking`, all handlers are sync — child process `stdout` reading is naturally blocking, fits perfectly
3. **No new deps**: `std::process::Command` + `BufReader` for line-by-line reading
4. **OpenWrt busybox**: `ping -c 10`, `traceroute`, `nslookup` all available

## Architecture

```
handle_run(body, ws_manager) -> Response
  1. Parse JSON: {tool, target}
  2. Validate target (safe chars only)
  3. Check RUNNING AtomicBool — reject if already running
  4. Set RUNNING = true
  5. Build command args based on tool
  6. std::thread::spawn:
     a. Spawn child process with stdout piped
     b. BufReader::lines() — for each line:
        - broadcast: {"type":"toolbox","line":"<escaped>"}
     c. Wait for child exit
     d. broadcast: {"type":"toolbox","done":true,"code":<exit_code>}
     e. Set RUNNING = false
  7. Return {"ok":true,"tool":"ping"}

handle_stop() -> Response
  1. Send kill signal to stored child PID
  2. Return {"ok":true}
```

## Requirements

### Functional
- `POST /api/toolbox/run` — start a tool: `{"tool":"ping|traceroute|nslookup","target":"host"}`
- `POST /api/toolbox/stop` — kill running tool
- Stream each stdout line as `{"type":"toolbox","line":"..."}`
- Send `{"type":"toolbox","done":true,"code":0}` when process exits
- Only 1 tool can run at a time

### Non-Functional
- Target validation: reject any char not in `[a-zA-Z0-9.\-:]`
- Max 200 lines output (kill process after limit)
- 60-second timeout (kill process if still running)
- No heap allocation for command building beyond string formatting

## Security Considerations
- **Command injection prevention**: `is_safe_target()` validates target contains only safe chars (alphanumeric, dot, hyphen, colon for IPv6)
- **Resource exhaustion**: line limit (200) + timeout (60s) + single-run guard
- **Auth required**: routes behind existing auth check in server.rs

## Related Code Files

### Create
- `ugate/src/web/toolbox.rs` (~120 lines)

### Modify
- `ugate/src/web/mod.rs` — add `pub mod toolbox;`

## Implementation Steps

### Step 1: Create `ugate/src/web/toolbox.rs`

```rust
// Key structures and functions:

use std::io::BufRead;
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use crate::web::ws::WsManager;

static RUNNING: AtomicBool = AtomicBool::new(false);

/// Validate target: only alphanumeric, dots, hyphens, colons
fn is_safe_target(s: &str) -> bool {
    !s.is_empty()
        && s.len() <= 253
        && s.chars().all(|c| c.is_ascii_alphanumeric() || c == '.' || c == '-' || c == ':')
}

pub fn handle_run(body: &str, ws_manager: &Arc<WsManager>) -> super::Resp {
    // 1. Parse tool + target from body using jval()
    // 2. Validate tool is one of: ping, traceroute, nslookup
    // 3. Validate target with is_safe_target()
    // 4. Check/set RUNNING atomic
    // 5. Build command: ping -c 10, traceroute -m 20, nslookup
    // 6. Clone broadcast_tx, spawn thread to run process
    // 7. Return ok response
}

pub fn handle_stop() -> super::Resp {
    // Set a STOP flag that the reader thread checks
    // The running thread will kill the child process
}
```

### Step 2: Thread body pseudocode

```
fn run_tool(cmd, args, broadcast_tx):
    let child = Command::new(cmd).args(args).stdout(Stdio::piped()).stderr(Stdio::piped()).spawn()
    let reader = BufReader::new(child.stdout)
    let start = Instant::now()
    let mut lines = 0

    for line in reader.lines():
        if lines >= 200 || start.elapsed() > 60s || STOP.load():
            child.kill()
            break
        broadcast: {"type":"toolbox","line":"<json_escape(line)>"}
        lines += 1

    let code = child.wait().code().unwrap_or(-1)
    broadcast: {"type":"toolbox","done":true,"code":<code>}
    RUNNING.store(false)
```

### Step 3: Register module in mod.rs

Add `pub mod toolbox;` to `ugate/src/web/mod.rs`.

## Todo List

- [ ] Create `ugate/src/web/toolbox.rs` with handle_run, handle_stop, is_safe_target
- [ ] Add `pub mod toolbox;` to `ugate/src/web/mod.rs`
- [ ] Verify `cargo check` passes

## Success Criteria
- `cargo check --target mipsel-unknown-linux-musl` passes
- handle_run spawns process, streams lines via broadcast, respects limits
- handle_stop kills running process
- is_safe_target rejects shell metacharacters
