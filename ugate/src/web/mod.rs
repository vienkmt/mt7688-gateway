//! Web server module: HTTP API + WebSocket real-time
//! tiny-http phục vụ REST API và static files
//! tungstenite xử lý WebSocket cho dữ liệu real-time và lệnh điều khiển

pub mod auth;
pub mod netcfg;
pub mod server;
pub mod status;
pub mod wifi;
pub mod ws;

// --- Shared helpers cho API handlers ---

type Resp = tiny_http::Response<std::io::Cursor<Vec<u8>>>;

pub(crate) fn json_resp(json: &str) -> Resp {
    tiny_http::Response::from_string(json).with_header(
        tiny_http::Header::from_bytes(&b"Content-Type"[..], &b"application/json"[..]).unwrap(),
    )
}

pub(crate) fn json_err(code: u16, msg: &str) -> Resp {
    tiny_http::Response::from_string(format!(r#"{{"error":"{}"}}"#, msg))
        .with_status_code(code)
        .with_header(
            tiny_http::Header::from_bytes(&b"Content-Type"[..], &b"application/json"[..]).unwrap(),
        )
}

/// Parse JSON value đơn giản (string hoặc number/bool)
pub(crate) fn jval(json: &str, key: &str) -> Option<String> {
    let pat = format!("\"{}\":", key);
    json.find(&pat).and_then(|pos| {
        let rest = json[pos + pat.len()..].trim_start();
        if rest.starts_with('"') {
            rest[1..].find('"').map(|end| rest[1..1 + end].to_string())
        } else {
            let end = rest
                .find(|c: char| c == ',' || c == '}')
                .unwrap_or(rest.len());
            Some(rest[..end].trim().to_string())
        }
    })
}

/// Validate identifier an toàn cho UCI key paths (chỉ alphanumeric + underscore)
pub(crate) fn is_safe_identifier(s: &str) -> bool {
    !s.is_empty() && s.len() <= 64 && s.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
}

/// Escape string cho JSON output (quotes, backslashes, control chars)
pub(crate) fn json_escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if c.is_control() => {} // skip control chars
            c => out.push(c),
        }
    }
    out
}

/// Validate IPv4 format
pub(crate) fn is_valid_ipv4(s: &str) -> bool {
    let parts: Vec<&str> = s.split('.').collect();
    parts.len() == 4 && parts.iter().all(|p| p.parse::<u8>().is_ok())
}
