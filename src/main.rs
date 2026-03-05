mod config;
mod html_config;
mod html_template;
mod http_publisher;
mod mqtt_publisher;
mod network_config;
mod oled;
mod system_info;
mod time_sync;
mod uart_reader;
mod uci;

use config::{AppState, Config, GeneralConfig, HttpConfig, MqttConfig, UartConfig};
use std::sync::Arc;

const HTTP_PORT: u16 = 8889;

#[tokio::main(flavor = "current_thread")]
async fn main() {
    // Start OLED display loop (async task)
    tokio::spawn(oled::display_loop());

    // Sync system clock before TLS (needs correct time for cert validation)
    time_sync::sync_time();

    let state = Arc::new(AppState::new());

    // Create std mpsc channels for UART → publishers (cross-thread compatible)
    let (mqtt_tx, mqtt_rx) = std::sync::mpsc::channel::<String>();
    let (http_tx, http_rx) = tokio::sync::mpsc::channel::<String>(64);

    // Spawn MQTT in a separate OS thread (rumqttc has issues with tokio on MIPS)
    let mqtt_state = state.clone();
    std::thread::spawn(move || {
        mqtt_publisher::run_sync(mqtt_state, mqtt_rx);
    });

    // Spawn async tasks
    tokio::spawn(uart_reader::run(state.clone(), mqtt_tx, http_tx));
    tokio::spawn(http_publisher::run(state.clone(), http_rx));

    // HTTP server (blocking, run in spawn_blocking)
    let server_state = state.clone();
    tokio::task::spawn_blocking(move || {
        run_http_server(server_state);
    })
    .await
    .unwrap();
}

fn run_http_server(state: Arc<AppState>) {
    let addr = format!("0.0.0.0:{}", HTTP_PORT);
    let server = match tiny_http::Server::http(&addr) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Failed to bind {}: {}", addr, e);
            std::process::exit(1);
        }
    };

    println!("vGateway running on http://{}", addr);

    for mut request in server.incoming_requests() {
        let url = request.url().to_string();
        let is_post = *request.method() == tiny_http::Method::Post;

        let response = if url == "/" {
            let info = system_info::SystemInfo::collect();
            let html = html_template::render_page(&info);
            tiny_http::Response::from_string(html).with_header(
                tiny_http::Header::from_bytes(&b"Content-Type"[..], &b"text/html; charset=utf-8"[..])
                    .unwrap(),
            )
        } else if url == "/config" && is_post {
            let mut body = String::new();
            let _ = request.as_reader().read_to_string(&mut body);
            let new_config = parse_config_form(&body);
            state.update(new_config);
            let net_config = parse_network_form_from_config(&body);
            let net_errors = match network_config::validate_config(&net_config) {
                Ok(()) => match net_config.save_to_uci() {
                    Ok(()) => vec![],
                    Err(e) => vec![e],
                },
                Err(errs) => errs,
            };
            let config = state.get();
            let net_config_display = if net_errors.is_empty() {
                network_config::NetworkConfig::load_from_uci()
            } else {
                net_config
            };
            let net_status = network_config::NetworkStatus::get_current();
            let html =
                html_config::render_config_page(&config, &net_config_display, &net_status, true, &net_errors);
            tiny_http::Response::from_string(html).with_header(
                tiny_http::Header::from_bytes(&b"Content-Type"[..], &b"text/html; charset=utf-8"[..])
                    .unwrap(),
            )
        } else if url == "/config" {
            let config = state.get();
            let net_config = network_config::NetworkConfig::load_from_uci();
            let net_status = network_config::NetworkStatus::get_current();
            let html = html_config::render_config_page(&config, &net_config, &net_status, false, &[]);
            tiny_http::Response::from_string(html).with_header(
                tiny_http::Header::from_bytes(&b"Content-Type"[..], &b"text/html; charset=utf-8"[..])
                    .unwrap(),
            )
        } else if url == "/network" {
            tiny_http::Response::from_string("")
                .with_status_code(tiny_http::StatusCode(302))
                .with_header(tiny_http::Header::from_bytes(&b"Location"[..], &b"/config"[..]).unwrap())
        } else if url == "/api/network" && is_post {
            let mut body = String::new();
            let _ = request.as_reader().read_to_string(&mut body);
            let net_config = parse_network_json(&body);
            let response_json = match network_config::validate_config(&net_config) {
                Ok(()) => match net_config.save_to_uci() {
                    Ok(()) => {
                        let status = network_config::NetworkStatus::get_current();
                        format_network_json(&network_config::NetworkConfig::load_from_uci(), &status, true, &[])
                    }
                    Err(e) => {
                        format_network_json(&net_config, &network_config::NetworkStatus::get_current(), false, &[e])
                    }
                },
                Err(errs) => {
                    format_network_json(&net_config, &network_config::NetworkStatus::get_current(), false, &errs)
                }
            };
            tiny_http::Response::from_string(response_json).with_header(
                tiny_http::Header::from_bytes(&b"Content-Type"[..], &b"application/json"[..]).unwrap(),
            )
        } else if url == "/api/network" {
            let net_config = network_config::NetworkConfig::load_from_uci();
            let status = network_config::NetworkStatus::get_current();
            let json = format_network_json(&net_config, &status, false, &[]);
            tiny_http::Response::from_string(json).with_header(
                tiny_http::Header::from_bytes(&b"Content-Type"[..], &b"application/json"[..]).unwrap(),
            )
        } else {
            tiny_http::Response::from_string("404 Not Found").with_status_code(tiny_http::StatusCode(404))
        };
        let _ = request.respond(response);
    }
}

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

fn json_escape(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
        .replace('\r', "\\r")
        .replace('\t', "\\t")
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

fn parse_network_form_from_config(body: &str) -> network_config::NetworkConfig {
    let params: Vec<(String, String)> = body
        .split('&')
        .filter_map(|pair| {
            let mut parts = pair.splitn(2, '=');
            Some((url_decode(parts.next()?), url_decode(parts.next().unwrap_or(""))))
        })
        .collect();

    let mode = if get_val(&params, "net_mode") == "static" {
        network_config::NetworkMode::Static
    } else {
        network_config::NetworkMode::Dhcp
    };

    network_config::NetworkConfig {
        mode,
        ipaddr: get_val(&params, "net_ipaddr"),
        netmask: get_val(&params, "net_netmask"),
        gateway: get_val(&params, "net_gateway"),
        dns_primary: get_val(&params, "net_dns1"),
        dns_secondary: get_val(&params, "net_dns2"),
    }
}

fn parse_network_json(body: &str) -> network_config::NetworkConfig {
    let get_json_val = |key: &str| -> String {
        let pattern = format!("\"{}\"", key);
        if let Some(pos) = body.find(&pattern) {
            let rest = &body[pos + pattern.len()..];
            if let Some(colon) = rest.find(':') {
                let after_colon = rest[colon + 1..].trim_start();
                if after_colon.starts_with('"') {
                    if let Some(end) = after_colon[1..].find('"') {
                        return after_colon[1..end + 1].to_string();
                    }
                }
            }
        }
        String::new()
    };

    let mode_str = get_json_val("mode");
    let mode = if mode_str == "static" {
        network_config::NetworkMode::Static
    } else {
        network_config::NetworkMode::Dhcp
    };

    network_config::NetworkConfig {
        mode,
        ipaddr: get_json_val("ipaddr"),
        netmask: get_json_val("netmask"),
        gateway: get_json_val("gateway"),
        dns_primary: get_json_val("dns_primary"),
        dns_secondary: get_json_val("dns_secondary"),
    }
}

fn format_network_json(
    config: &network_config::NetworkConfig,
    status: &network_config::NetworkStatus,
    saved: bool,
    errors: &[String],
) -> String {
    let mode_str = config.mode.as_str();
    let dns_arr: String = status
        .dns
        .iter()
        .map(|d| format!("\"{}\"", json_escape(d)))
        .collect::<Vec<_>>()
        .join(",");
    let errors_arr: String = errors
        .iter()
        .map(|e| format!("\"{}\"", json_escape(e)))
        .collect::<Vec<_>>()
        .join(",");

    format!(
        r#"{{"config":{{"mode":"{}","ipaddr":"{}","netmask":"{}","gateway":"{}","dns_primary":"{}","dns_secondary":"{}"}},"status":{{"ip":"{}","netmask":"{}","gateway":"{}","dns":[{}],"is_up":{}}},"saved":{},"errors":[{}]}}"#,
        mode_str,
        json_escape(&config.ipaddr),
        json_escape(&config.netmask),
        json_escape(&config.gateway),
        json_escape(&config.dns_primary),
        json_escape(&config.dns_secondary),
        json_escape(&status.ip),
        json_escape(&status.netmask),
        json_escape(&status.gateway),
        dns_arr,
        status.is_up,
        saved,
        errors_arr
    )
}
