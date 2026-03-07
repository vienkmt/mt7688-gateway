//! HTTP server chính dùng tiny-http
//! Phục vụ: REST API (config, login, GPIO), static files (Vue.js), WebSocket upgrade
//! Chạy trong spawn_blocking vì tiny-http là blocking

use crate::commands::Command;
use crate::config::AppState;
use crate::web::auth::SessionManager;
use crate::web::ws::{self, WsManager};
use std::sync::Arc;

/// Static files nhúng trong binary (từ frontend/dist/)
/// Sẽ được thay bằng include_bytes! sau khi build frontend
const INDEX_HTML: &str = include_str!("../embedded_index.html");

/// Chạy HTTP server (blocking — gọi từ spawn_blocking)
pub fn run(
    state: Arc<AppState>,
    ws_manager: Arc<WsManager>,
    session_mgr: Arc<SessionManager>,
) {
    let config = state.get();
    let addr = format!("0.0.0.0:{}", config.web.port);

    let server = match tiny_http::Server::http(&addr) {
        Ok(s) => s,
        Err(e) => {
            log::error!("[HTTP] Không thể bind {}: {}", addr, e);
            return;
        }
    };

    log::info!("[HTTP] ugate đang chạy tại http://{}", addr);

    for mut request in server.incoming_requests() {
        let url = request.url().to_string();
        let method = request.method().clone();

        // WebSocket upgrade (không cần auth cho đơn giản)
        if url == "/ws" {
            handle_ws_upgrade(request, &ws_manager);
            continue;
        }

        // Kiểm tra auth cho API routes (trừ login và static files)
        let needs_auth = url.starts_with("/api/") && url != "/api/login";
        if needs_auth {
            let cookie = request
                .headers()
                .iter()
                .find(|h| h.field.as_str() == "Cookie" || h.field.as_str() == "cookie")
                .map(|h| h.value.as_str().to_string());
            if !session_mgr.check_session(cookie.as_deref()) {
                let _ = request.respond(
                    tiny_http::Response::from_string(r#"{"error":"unauthorized"}"#)
                        .with_status_code(401)
                        .with_header(content_type_json()),
                );
                continue;
            }
        }

        let response = match (method, url.as_str()) {
            // Static files
            (tiny_http::Method::Get, "/") | (tiny_http::Method::Get, "/index.html") => {
                tiny_http::Response::from_string(INDEX_HTML)
                    .with_header(content_type_html())
            }

            // Auth
            (tiny_http::Method::Post, "/api/login") => {
                handle_login(&mut request, &state, &session_mgr)
            }

            // Config API
            (tiny_http::Method::Get, "/api/config") => {
                handle_get_config(&state)
            }
            (tiny_http::Method::Post, "/api/config") => {
                handle_set_config(&mut request, &state)
            }

            // Status API
            (tiny_http::Method::Get, "/api/status") => {
                handle_get_status(&state)
            }

            // GPIO API
            (tiny_http::Method::Post, path) if path.starts_with("/api/gpio/") => {
                handle_gpio(&mut request, path, &ws_manager)
            }

            // Password change
            (tiny_http::Method::Post, "/api/password") => {
                handle_change_password(&mut request, &state)
            }

            _ => {
                tiny_http::Response::from_string(r#"{"error":"not found"}"#)
                    .with_status_code(404)
                    .with_header(content_type_json())
            }
        };

        let _ = request.respond(response);
    }
}

fn handle_ws_upgrade(request: tiny_http::Request, ws_manager: &Arc<WsManager>) {
    // Lấy Sec-WebSocket-Key từ request để tính accept key
    let ws_key = request
        .headers()
        .iter()
        .find(|h| h.field.as_str() == "Sec-WebSocket-Key" || h.field.as_str() == "sec-websocket-key")
        .map(|h| h.value.as_str().to_string());

    let accept_key = match ws_key {
        Some(key) => tungstenite::handshake::derive_accept_key(key.as_bytes()),
        None => return,
    };

    // Gửi 101 với đầy đủ WS handshake headers
    let response = tiny_http::Response::empty(101)
        .with_header(
            tiny_http::Header::from_bytes(&b"Connection"[..], &b"Upgrade"[..]).unwrap(),
        )
        .with_header(
            tiny_http::Header::from_bytes(&b"Sec-WebSocket-Accept"[..], accept_key.as_bytes())
                .unwrap(),
        );

    let stream = request.upgrade("websocket", response);
    let mgr = ws_manager.clone();
    std::thread::spawn(move || {
        ws::handle_websocket(stream, mgr);
    });
}

fn handle_login(
    request: &mut tiny_http::Request,
    state: &AppState,
    session_mgr: &SessionManager,
) -> tiny_http::Response<std::io::Cursor<Vec<u8>>> {
    let body = read_body(request);
    let config = state.get();

    if crate::web::auth::validate_password(&body, &config.web.password) {
        let token = session_mgr.create_session();
        let cookie = format!("session={}; Path=/; HttpOnly", token);
        tiny_http::Response::from_string(r#"{"ok":true}"#)
            .with_header(content_type_json())
            .with_header(
                tiny_http::Header::from_bytes(&b"Set-Cookie"[..], cookie.as_bytes()).unwrap(),
            )
    } else {
        tiny_http::Response::from_string(r#"{"ok":false}"#)
            .with_status_code(401)
            .with_header(content_type_json())
    }
}

fn handle_get_config(state: &AppState) -> tiny_http::Response<std::io::Cursor<Vec<u8>>> {
    let c = state.get();
    let tcp_mode = match c.tcp.mode {
        crate::config::TcpMode::Server => "server",
        crate::config::TcpMode::Client => "client",
        crate::config::TcpMode::Both => "both",
    };
    let frame_mode = match c.uart.frame_mode {
        crate::config::FrameMode::None => "none",
        crate::config::FrameMode::Frame => "frame",
        crate::config::FrameMode::Modbus => "modbus",
    };
    let parity = match c.uart.parity {
        crate::config::Parity::None => "none",
        crate::config::Parity::Even => "even",
        crate::config::Parity::Odd => "odd",
    };
    let http_method = match c.http.method {
        crate::config::HttpMethod::Post => "post",
        crate::config::HttpMethod::Get => "get",
    };
    let json = format!(
        r#"{{"general":{{"device_name":"{}","interval_secs":{}}},"mqtt":{{"enabled":{},"broker":"{}","port":{},"tls":{},"topic":"{}","sub_topic":"{}","client_id":"{}","username":"{}","password":"{}","qos":{}}},"http":{{"enabled":{},"url":"{}","method":"{}"}},"tcp":{{"enabled":{},"mode":"{}","server_port":{},"client_host":"{}","client_port":{}}},"uart":{{"enabled":{},"baudrate":{},"data_bits":{},"parity":"{}","stop_bits":{},"frame_mode":"{}","frame_length":{},"frame_timeout_ms":{},"gap_ms":{}}},"web":{{"port":{}}}}}"#,
        c.general.device_name, c.general.interval_secs,
        c.mqtt.enabled, c.mqtt.broker, c.mqtt.port, c.mqtt.tls,
        c.mqtt.topic, c.mqtt.sub_topic, c.mqtt.client_id, c.mqtt.username, c.mqtt.password, c.mqtt.qos,
        c.http.enabled, c.http.url, http_method,
        c.tcp.enabled, tcp_mode, c.tcp.server_port, c.tcp.client_host, c.tcp.client_port,
        c.uart.enabled, c.uart.baudrate, c.uart.data_bits, parity, c.uart.stop_bits, frame_mode,
        c.uart.frame_length, c.uart.frame_timeout_ms, c.uart.gap_ms,
        c.web.port,
    );
    tiny_http::Response::from_string(json).with_header(content_type_json())
}

fn handle_set_config(
    request: &mut tiny_http::Request,
    state: &AppState,
) -> tiny_http::Response<std::io::Cursor<Vec<u8>>> {
    let body = read_body(request);
    log::info!("[HTTP] Config update: {}", &body[..body.len().min(300)]);

    let mut cfg = state.get();

    // Tách section từ nested JSON: {"mqtt":{...},"http":{...},...}
    // Tìm nội dung bên trong mỗi section object
    let section_body = |section: &str| -> Option<String> {
        let pat = format!("\"{}\":{{", section);
        body.find(&pat).and_then(|pos| {
            let start = pos + pat.len() - 1; // bao gồm '{'
            let mut depth = 0;
            for (i, c) in body[start..].char_indices() {
                match c {
                    '{' => depth += 1,
                    '}' => { depth -= 1; if depth == 0 { return Some(body[start..start + i + 1].to_string()); } }
                    _ => {}
                }
            }
            None
        })
    };

    // Trích giá trị từ JSON fragment
    fn jval(json: &str, key: &str) -> Option<String> {
        let pat = format!("\"{}\":", key);
        json.find(&pat).and_then(|pos| {
            let rest = json[pos + pat.len()..].trim_start();
            if rest.starts_with('"') {
                rest[1..].find('"').map(|end| rest[1..1 + end].to_string())
            } else {
                let end = rest.find(|c: char| c == ',' || c == '}').unwrap_or(rest.len());
                Some(rest[..end].trim().to_string())
            }
        })
    }
    fn jbool(json: &str, key: &str) -> Option<bool> {
        jval(json, key).map(|v| v == "true" || v == "1")
    }

    // General
    if let Some(s) = section_body("general") {
        if let Some(v) = jval(&s, "device_name") { cfg.general.device_name = v; }
        if let Some(v) = jval(&s, "interval_secs").and_then(|v| v.parse().ok()) { cfg.general.interval_secs = v; }
    }

    // MQTT
    if let Some(s) = section_body("mqtt") {
        if let Some(v) = jbool(&s, "enabled") { cfg.mqtt.enabled = v; }
        if let Some(v) = jval(&s, "broker") { cfg.mqtt.broker = v; }
        if let Some(v) = jval(&s, "port").and_then(|v| v.parse().ok()) { cfg.mqtt.port = v; }
        if let Some(v) = jbool(&s, "tls") { cfg.mqtt.tls = v; }
        if let Some(v) = jval(&s, "topic") { cfg.mqtt.topic = v; }
        if let Some(v) = jval(&s, "sub_topic") { cfg.mqtt.sub_topic = v; }
        if let Some(v) = jval(&s, "client_id") { cfg.mqtt.client_id = v; }
        if let Some(v) = jval(&s, "username") { cfg.mqtt.username = v; }
        if let Some(v) = jval(&s, "password") { cfg.mqtt.password = v; }
        if let Some(v) = jval(&s, "qos").and_then(|v| v.parse().ok()) { cfg.mqtt.qos = v; }
    }

    // HTTP
    if let Some(s) = section_body("http") {
        if let Some(v) = jbool(&s, "enabled") { cfg.http.enabled = v; }
        if let Some(v) = jval(&s, "url") { cfg.http.url = v; }
        if let Some(v) = jval(&s, "method") {
            cfg.http.method = if v == "get" { crate::config::HttpMethod::Get } else { crate::config::HttpMethod::Post };
        }
    }

    // TCP
    if let Some(s) = section_body("tcp") {
        if let Some(v) = jbool(&s, "enabled") { cfg.tcp.enabled = v; }
        if let Some(v) = jval(&s, "mode") {
            cfg.tcp.mode = match v.as_str() {
                "client" => crate::config::TcpMode::Client,
                "both" => crate::config::TcpMode::Both,
                _ => crate::config::TcpMode::Server,
            };
        }
        if let Some(v) = jval(&s, "server_port").and_then(|v| v.parse().ok()) { cfg.tcp.server_port = v; }
        if let Some(v) = jval(&s, "client_host") { cfg.tcp.client_host = v; }
        if let Some(v) = jval(&s, "client_port").and_then(|v| v.parse().ok()) { cfg.tcp.client_port = v; }
    }

    // UART
    if let Some(s) = section_body("uart") {
        if let Some(v) = jval(&s, "baudrate").and_then(|v| v.parse().ok()) { cfg.uart.baudrate = v; }
        if let Some(v) = jval(&s, "data_bits").and_then(|v| v.parse().ok()) { cfg.uart.data_bits = v; }
        if let Some(v) = jval(&s, "parity") {
            cfg.uart.parity = match v.as_str() {
                "even" => crate::config::Parity::Even,
                "odd" => crate::config::Parity::Odd,
                _ => crate::config::Parity::None,
            };
        }
        if let Some(v) = jval(&s, "stop_bits").and_then(|v| v.parse().ok()) { cfg.uart.stop_bits = v; }
        if let Some(v) = jval(&s, "frame_mode") {
            cfg.uart.frame_mode = match v.as_str() {
                "frame" => crate::config::FrameMode::Frame,
                "modbus" => crate::config::FrameMode::Modbus,
                _ => crate::config::FrameMode::None,
            };
        }
        if let Some(v) = jval(&s, "frame_length").and_then(|v| v.parse().ok()) { cfg.uart.frame_length = v; }
        if let Some(v) = jval(&s, "frame_timeout_ms").and_then(|v| v.parse().ok()) { cfg.uart.frame_timeout_ms = v; }
        if let Some(v) = jval(&s, "gap_ms").and_then(|v| v.parse().ok()) { cfg.uart.gap_ms = v; }
    }

    // Lưu UCI và cập nhật state (thông báo tới MQTT/UART reconnect)
    cfg.save_to_uci();
    state.update(cfg);

    tiny_http::Response::from_string(r#"{"ok":true}"#).with_header(content_type_json())
}

fn handle_get_status(_state: &AppState) -> tiny_http::Response<std::io::Cursor<Vec<u8>>> {
    // Trả về status cơ bản (status đầy đủ qua WebSocket)
    tiny_http::Response::from_string(r#"{"ok":true,"status":"running"}"#)
        .with_header(content_type_json())
}

fn handle_gpio(
    request: &mut tiny_http::Request,
    path: &str,
    ws_manager: &WsManager,
) -> tiny_http::Response<std::io::Cursor<Vec<u8>>> {
    // Path: /api/gpio/{pin}/{state} ví dụ /api/gpio/1/toggle
    let parts: Vec<&str> = path.trim_start_matches("/api/gpio/").split('/').collect();
    if parts.len() < 2 {
        return tiny_http::Response::from_string(r#"{"error":"invalid path"}"#)
            .with_status_code(400)
            .with_header(content_type_json());
    }

    let pin: u8 = match parts[0].parse() {
        Ok(p) => p,
        Err(_) => {
            return tiny_http::Response::from_string(r#"{"error":"invalid pin"}"#)
                .with_status_code(400)
                .with_header(content_type_json())
        }
    };

    let state = match parts[1] {
        "on" | "1" => crate::commands::GpioState::On,
        "off" | "0" => crate::commands::GpioState::Off,
        "toggle" | "t" => crate::commands::GpioState::Toggle,
        _ => {
            return tiny_http::Response::from_string(r#"{"error":"invalid state"}"#)
                .with_status_code(400)
                .with_header(content_type_json())
        }
    };

    let _ = ws_manager.cmd_tx.send(Command::Gpio { pin, state });
    tiny_http::Response::from_string(r#"{"ok":true}"#).with_header(content_type_json())
}

fn handle_change_password(
    request: &mut tiny_http::Request,
    _state: &AppState,
) -> tiny_http::Response<std::io::Cursor<Vec<u8>>> {
    let _body = read_body(request);
    // TODO: Parse và save password qua UCI
    log::info!("[HTTP] Password change request");
    tiny_http::Response::from_string(r#"{"ok":true}"#).with_header(content_type_json())
}

fn read_body(request: &mut tiny_http::Request) -> String {
    let mut body = String::new();
    let _ = request.as_reader().read_to_string(&mut body);
    body
}

fn content_type_json() -> tiny_http::Header {
    tiny_http::Header::from_bytes(&b"Content-Type"[..], &b"application/json"[..]).unwrap()
}

fn content_type_html() -> tiny_http::Header {
    tiny_http::Header::from_bytes(&b"Content-Type"[..], &b"text/html; charset=utf-8"[..]).unwrap()
}
