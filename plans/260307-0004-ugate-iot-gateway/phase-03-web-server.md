# Phase 3: Web Server + WebSocket

**Priority:** High
**Status:** pending
**Effort:** 2 days
**Depends on:** Phase 1

## Context

tiny-http + tungstenite web server (proven stack từ vgateway):
- Static file serving (Vue.js embedded)
- REST API cho config
- WebSocket cho real-time data + commands
- Simple password auth

## Module Structure

```
ugate/src/web/
├── mod.rs
├── server.rs     # tiny-http setup, request routing
├── ws.rs         # tungstenite WebSocket handler
└── auth.rs       # Simple password auth
```

## Implementation Steps

### 1. Create web/auth.rs

```rust
use std::collections::HashMap;

pub fn check_session(cookie: Option<&str>) -> bool {
    cookie.map(|c| c.contains("session=valid")).unwrap_or(false)
}

pub fn validate_password(body: &str, expected: &str) -> bool {
    // Parse from form: password=xxx
    let params: HashMap<_, _> = body.split('&')
        .filter_map(|p| p.split_once('='))
        .collect();
    params.get("password").map(|p| *p == expected).unwrap_or(false)
}

pub fn login_response(success: bool) -> tiny_http::Response<std::io::Cursor<Vec<u8>>> {
    if success {
        tiny_http::Response::from_string("OK")
            .with_header(
                tiny_http::Header::from_bytes(&b"Set-Cookie"[..], &b"session=valid; Path=/; HttpOnly"[..]).unwrap()
            )
    } else {
        tiny_http::Response::from_string("Unauthorized")
            .with_status_code(401)
    }
}
```

### 2. Create web/ws.rs

WebSocket handler với tungstenite:

```rust
use tungstenite::{accept, Message};
use std::net::TcpStream;
use std::sync::Arc;
use std::sync::atomic::{AtomicU8, Ordering};

pub struct WsState {
    pub data_rx: std::sync::mpsc::Receiver<String>,
    pub cmd_tx: std::sync::mpsc::Sender<Command>,
    pub connections: AtomicU8,
    pub max_connections: u8,
}

pub fn handle_websocket(
    stream: TcpStream,
    state: Arc<WsState>,
) {
    if state.connections.load(Ordering::Relaxed) >= state.max_connections {
        return;
    }
    state.connections.fetch_add(1, Ordering::Relaxed);

    let mut ws = match accept(stream) {
        Ok(ws) => ws,
        Err(_) => {
            state.connections.fetch_sub(1, Ordering::Relaxed);
            return;
        }
    };

    // Set non-blocking for multiplexing
    ws.get_ref().set_nonblocking(true).ok();

    loop {
        // Try to receive data to broadcast
        if let Ok(data) = state.data_rx.try_recv() {
            if ws.send(Message::Text(data)).is_err() {
                break;
            }
        }

        // Try to receive commands from client
        match ws.read() {
            Ok(Message::Text(text)) => {
                if let Some(cmd) = parse_json_command(&text) {
                    let _ = state.cmd_tx.send(cmd);
                }
            }
            Ok(Message::Close(_)) => break,
            Err(tungstenite::Error::Io(ref e))
                if e.kind() == std::io::ErrorKind::WouldBlock => {
                std::thread::sleep(std::time::Duration::from_millis(10));
            }
            Err(_) => break,
        }
    }

    state.connections.fetch_sub(1, Ordering::Relaxed);
}
```

### 3. Create web/server.rs

Main HTTP server (pattern từ vgateway):

```rust
use tiny_http::{Server, Request, Response, Method};
use std::sync::Arc;

pub fn run_server(
    state: Arc<AppState>,
    data_tx: std::sync::mpsc::Sender<String>,
    cmd_tx: std::sync::mpsc::Sender<Command>,
) {
    let config = state.get();
    let addr = format!("0.0.0.0:{}", config.web.port);

    let server = Server::http(&addr).expect("Failed to bind");
    println!("ugate running on http://{}", addr);

    for mut request in server.incoming_requests() {
        let url = request.url().to_string();
        let method = request.method().clone();

        // Check auth for protected routes
        if !url.starts_with("/api/login") && !url.starts_with("/ws") {
            let cookie = request.headers()
                .iter()
                .find(|h| h.field.as_str() == "Cookie")
                .map(|h| h.value.as_str());

            if !auth::check_session(cookie) && !url.starts_with("/assets") && url != "/" {
                let _ = request.respond(Response::from_string("Unauthorized").with_status_code(401));
                continue;
            }
        }

        let response = match (method, url.as_str()) {
            (Method::Get, "/") => serve_index(),
            (Method::Get, path) if path.starts_with("/assets/") => serve_asset(path),
            (Method::Post, "/api/login") => handle_login(&mut request, &state),
            (Method::Get, "/api/config") => handle_get_config(&state),
            (Method::Post, "/api/config") => handle_set_config(&mut request, &state),
            (Method::Get, "/api/status") => handle_get_status(),
            (Method::Post, path) if path.starts_with("/api/gpio/") => {
                handle_set_gpio(&mut request, path, &cmd_tx)
            }
            (Method::Get, "/ws") => {
                // Upgrade to WebSocket
                handle_ws_upgrade(request, &state, &data_tx, &cmd_tx);
                continue;
            }
            _ => Response::from_string("Not Found").with_status_code(404),
        };

        let _ = request.respond(response);
    }
}

// Embedded static files
static INDEX_HTML: &[u8] = include_bytes!("../../frontend/dist/index.html");

fn serve_index() -> Response<std::io::Cursor<Vec<u8>>> {
    Response::from_data(INDEX_HTML.to_vec())
        .with_header(tiny_http::Header::from_bytes(&b"Content-Type"[..], &b"text/html"[..]).unwrap())
}

fn handle_ws_upgrade(
    request: Request,
    state: &Arc<AppState>,
    data_tx: &std::sync::mpsc::Sender<String>,
    cmd_tx: &std::sync::mpsc::Sender<Command>,
) {
    // Extract TCP stream and spawn WebSocket handler
    let stream = request.upgrade("websocket", Response::empty(101));
    if let Some(stream) = stream {
        let ws_state = Arc::new(WsState { ... });
        std::thread::spawn(move || ws::handle_websocket(stream, ws_state));
    }
}
```

### 4. Create web/mod.rs

```rust
pub mod server;
pub mod ws;
pub mod auth;

pub use server::run_server;
```

## Files to Create/Modify

| File | Action |
|------|--------|
| ugate/src/web/mod.rs | Create |
| ugate/src/web/server.rs | Create |
| ugate/src/web/ws.rs | Create |
| ugate/src/web/auth.rs | Create |
| ugate/src/main.rs | Modify |

## Dependencies to Add

```toml
tiny_http = "0.12"
tungstenite = "0.21"
```

## Todo

- [ ] Create web/auth.rs
- [ ] Create web/ws.rs
- [ ] Create web/server.rs
- [ ] Create web/mod.rs
- [ ] Wire in main.rs (spawn_blocking)
- [ ] Test login flow
- [ ] Test API endpoints
- [ ] Test WebSocket connection
- [ ] Test WebSocket broadcast

## Success Criteria

- [ ] Login works with password
- [ ] API returns config
- [ ] WebSocket connects
- [ ] WebSocket receives real-time data
- [ ] WebSocket sends commands

## Next Phase

Phase 4: GPIO Control
