mod config;
mod html_config;
mod html_template;
mod http_publisher;
mod mqtt_publisher;
mod system_info;
mod time_sync;
mod uart_reader;

use config::{AppState, Config, GeneralConfig, HttpConfig, MqttConfig, UartConfig};
use std::sync::Arc;

fn main() {
    // Sync system clock before TLS (needs correct time for cert validation)
    time_sync::sync_time();

    let state = Arc::new(AppState::new());

    // Create UART → publisher channels (bounded to prevent OOM on 64MB device)
    let (mqtt_uart_tx, mqtt_uart_rx) = std::sync::mpsc::sync_channel::<String>(128);
    let (http_uart_tx, http_uart_rx) = std::sync::mpsc::sync_channel::<String>(128);

    // Start publishers in background (with UART receivers)
    mqtt_publisher::start_background(Arc::clone(&state), mqtt_uart_rx);
    http_publisher::start_background(Arc::clone(&state), http_uart_rx);

    // Start UART reader (sends to both publishers)
    uart_reader::start_background(Arc::clone(&state), mqtt_uart_tx, http_uart_tx);

    let addr = "0.0.0.0:8888";
    let server = match tiny_http::Server::http(addr) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Failed to bind {}: {}", addr, e);
            std::process::exit(1);
        }
    };

    println!("V3S System Monitor running on http://{}", addr);

    for mut request in server.incoming_requests() {
        let url = request.url().to_string();
        let is_post = *request.method() == tiny_http::Method::Post;

        let response = if url == "/" {
            let info = system_info::SystemInfo::collect();
            let html = html_template::render_page(&info);
            tiny_http::Response::from_string(html).with_header(
                tiny_http::Header::from_bytes(&b"Content-Type"[..], &b"text/html; charset=utf-8"[..]).unwrap(),
            )
        } else if url == "/config" && is_post {
            let mut body = String::new();
            let _ = request.as_reader().read_to_string(&mut body);
            let new_config = parse_config_form(&body);
            state.update(new_config);
            let config = state.get();
            let html = html_config::render_config_page(&config, true);
            tiny_http::Response::from_string(html).with_header(
                tiny_http::Header::from_bytes(&b"Content-Type"[..], &b"text/html; charset=utf-8"[..]).unwrap(),
            )
        } else if url == "/config" {
            let config = state.get();
            let html = html_config::render_config_page(&config, false);
            tiny_http::Response::from_string(html).with_header(
                tiny_http::Header::from_bytes(&b"Content-Type"[..], &b"text/html; charset=utf-8"[..]).unwrap(),
            )
        } else {
            tiny_http::Response::from_string("404 Not Found")
                .with_status_code(tiny_http::StatusCode(404))
        };
        let _ = request.respond(response);
    }
}

/// Parse URL-encoded form body into Config
fn parse_config_form(body: &str) -> Config {
    let params: Vec<(String, String)> = body
        .split('&')
        .filter_map(|pair| {
            let mut parts = pair.splitn(2, '=');
            Some((url_decode(parts.next()?), url_decode(parts.next().unwrap_or(""))))
        })
        .collect();

    Config {
        mqtt: MqttConfig {
            enabled: has_key(&params, "mqtt_enabled"),
            broker: get_val(&params, "mqtt_broker"),
            port: get_val(&params, "mqtt_port").parse().unwrap_or(8883),
            tls: has_key(&params, "mqtt_tls"),
            topic: get_val(&params, "mqtt_topic"),
            client_id: get_val(&params, "mqtt_client_id"),
        },
        http: HttpConfig {
            enabled: has_key(&params, "http_enabled"),
            url: get_val(&params, "http_url"),
        },
        general: GeneralConfig {
            interval_secs: get_val(&params, "interval").parse().unwrap_or(3),
        },
        uart: UartConfig {
            enabled: has_key(&params, "uart_enabled"),
            port: get_val(&params, "uart_port"),
            baudrate: get_val(&params, "uart_baudrate").parse().unwrap_or(115200),
        },
    }
}

fn get_val(params: &[(String, String)], key: &str) -> String {
    params.iter().find(|(k, _)| k == key).map(|(_, v)| v.clone()).unwrap_or_default()
}

fn has_key(params: &[(String, String)], key: &str) -> bool {
    params.iter().any(|(k, _)| k == key)
}

fn url_decode(s: &str) -> String {
    let s = s.replace('+', " ");
    let mut out = String::with_capacity(s.len());
    let mut chars = s.chars();
    while let Some(c) = chars.next() {
        if c == '%' {
            let hex: String = chars.by_ref().take(2).collect();
            if let Ok(b) = u8::from_str_radix(&hex, 16) {
                out.push(b as char);
            }
        } else {
            out.push(c);
        }
    }
    out
}
