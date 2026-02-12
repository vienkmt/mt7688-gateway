use crate::config::Config;
use crate::network_config::{NetworkConfig, NetworkMode, NetworkStatus};

/// Escape HTML special characters
fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

/// Render the configuration page HTML with current config values
pub fn render_config_page(
    config: &Config,
    net_config: &NetworkConfig,
    net_status: &NetworkStatus,
    saved: bool,
    net_errors: &[String],
) -> String {
    let mqtt_checked = if config.mqtt.enabled { "checked" } else { "" };
    let tls_checked = if config.mqtt.tls { "checked" } else { "" };
    let http_checked = if config.http.enabled { "checked" } else { "" };
    let uart_checked = if config.uart.enabled { "checked" } else { "" };
    let baud = config.uart.baudrate;
    let baud_options = [9600, 19200, 38400, 57600, 115200, 230400]
        .iter()
        .map(|&b| {
            let sel = if b == baud { "selected" } else { "" };
            format!(r#"<option value="{}" {}>{}</option>"#, b, sel, b)
        })
        .collect::<Vec<_>>()
        .join("");
    // Network config
    let dhcp_checked = if net_config.mode == NetworkMode::Dhcp { "checked" } else { "" };
    let static_checked = if net_config.mode == NetworkMode::Static { "checked" } else { "" };
    let static_display = if net_config.mode == NetworkMode::Static { "block" } else { "none" };
    let status_color = if net_status.is_up { "#4CAF50" } else { "#f44336" };
    let status_text = if net_status.is_up { "UP" } else { "DOWN" };

    let saved_msg = if saved && net_errors.is_empty() {
        r#"<div class="toast">Đã lưu cấu hình!</div>"#.to_string()
    } else if !net_errors.is_empty() {
        let errs: String = net_errors.iter().map(|e| format!("<li>{}</li>", html_escape(e))).collect();
        format!(r#"<div class="toast" style="background:#f8d7da;color:#721c24"><ul style="margin:0;padding-left:20px;text-align:left">{}</ul></div>"#, errs)
    } else {
        String::new()
    };

    format!(
        r#"<!DOCTYPE html>
<html><head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>MT7688 Cấu hình</title>
<style>
* {{ box-sizing: border-box; }}
body {{
    background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
    min-height: 100vh;
    display: flex;
    justify-content: center;
    align-items: center;
    margin: 0;
    padding: 20px;
    font-family: 'Segoe UI', system-ui, sans-serif;
}}
.card {{
    background: #fff;
    border-radius: 16px;
    padding: 32px;
    box-shadow: 0 20px 60px rgba(0,0,0,0.3);
    max-width: 480px;
    width: 100%;
}}
h1 {{ margin: 0 0 24px; font-size: 22px; color: #333; text-align: center; }}
h3 {{ margin: 20px 0 12px; color: #667eea; font-size: 14px; text-transform: uppercase; letter-spacing: 1px; }}
.row {{ display: flex; align-items: center; margin: 10px 0; }}
.row label {{ flex: 0 0 100px; color: #666; font-size: 14px; }}
.row input[type=text], .row input[type=number], .row select {{
    flex: 1;
    padding: 10px 12px;
    border: 1px solid #ddd;
    border-radius: 8px;
    font-size: 14px;
    outline: none;
}}
.row input:focus, .row select:focus {{ border-color: #667eea; }}
input[type=checkbox] {{ width: 18px; height: 18px; accent-color: #667eea; }}
.btn {{
    display: block;
    width: 100%;
    margin-top: 24px;
    padding: 14px;
    background: linear-gradient(135deg, #667eea, #764ba2);
    color: #fff;
    border: none;
    border-radius: 8px;
    font-size: 16px;
    font-weight: 500;
    cursor: pointer;
}}
.btn:hover {{ opacity: 0.9; }}
.back {{
    display: block;
    text-align: center;
    margin-top: 16px;
    color: #667eea;
    text-decoration: none;
}}
.toast {{
    background: #d4edda;
    color: #155724;
    padding: 12px;
    border-radius: 8px;
    margin-bottom: 16px;
    text-align: center;
}}
</style>
</head>
<body>
<div class="card">
<h1>Cấu hình RMS7688</h1>
{saved_msg}
<form method="POST" action="/config">
<h3>MQTT</h3>
<div class="row"><label>Bật:</label><input type="checkbox" name="mqtt_enabled" {mqtt_en}></div>
<div class="row"><label>Broker:</label><input type="text" name="mqtt_broker" value="{mqtt_broker}"></div>
<div class="row"><label>Cổng:</label><input type="number" name="mqtt_port" value="{mqtt_port}"></div>
<div class="row"><label>TLS:</label><input type="checkbox" name="mqtt_tls" {mqtt_tls}></div>
<div class="row"><label>Topic:</label><input type="text" name="mqtt_topic" value="{mqtt_topic}"></div>
<div class="row"><label>Client ID:</label><input type="text" name="mqtt_client_id" value="{mqtt_client_id}"></div>

<h3>HTTP POST</h3>
<div class="row"><label>Bật:</label><input type="checkbox" name="http_enabled" {http_en}></div>
<div class="row"><label>URL:</label><input type="text" name="http_url" value="{http_url}"></div>

<h3>UART</h3>
<div class="row"><label>Bật:</label><input type="checkbox" name="uart_enabled" {uart_en}></div>
<input type="hidden" name="uart_port" value="{uart_port}">
<div class="row"><label>Baudrate:</label><select name="uart_baudrate">{baud_opts}</select></div>

<h3>Chung</h3>
<div class="row"><label>Chu kỳ:</label><input type="number" name="interval" value="{interval}" min="1" max="3600"> <span style="margin-left:8px;color:#666">giây</span></div>

<h3>Mạng WAN (eth0.2) <span style="padding:2px 8px;border-radius:10px;font-size:11px;color:white;background:{status_color}">{status_text}</span></h3>
<div class="row">
<label>Chế độ:</label>
<div style="display:flex;gap:16px">
<label style="flex:0"><input type="radio" name="net_mode" value="dhcp" {dhcp_checked} onclick="toggleStatic(false)"> DHCP</label>
<label style="flex:0"><input type="radio" name="net_mode" value="static" {static_checked} onclick="toggleStatic(true)"> Tĩnh</label>
</div>
</div>
<div id="static-fields" style="display:{static_display}">
<div class="row"><label>Địa chỉ IP:</label><input type="text" name="net_ipaddr" id="net_ipaddr" value="{net_ipaddr}" placeholder="192.168.1.100"></div>
<div class="row"><label>Subnet:</label><input type="text" name="net_netmask" id="net_netmask" value="{net_netmask}" placeholder="255.255.255.0"></div>
<div class="row"><label>Gateway:</label><input type="text" name="net_gateway" id="net_gateway" value="{net_gateway}" placeholder="192.168.1.1"></div>
<div class="row"><label>DNS chính:</label><input type="text" name="net_dns1" id="net_dns1" value="{net_dns1}" placeholder="8.8.8.8"></div>
<div class="row"><label>DNS phụ:</label><input type="text" name="net_dns2" id="net_dns2" value="{net_dns2}" placeholder="8.8.4.4"></div>
</div>
<div style="font-size:12px;color:#888;margin-top:8px">Hiện tại: {current_ip} | GW: {current_gw}</div>

<button type="submit" class="btn">Lưu cấu hình</button>
</form>
<a href="/" class="back">← Quay lại giám sát</a>
</div>
<script>
var liveStatus = {{ip:"{live_ip}",netmask:"{live_netmask}",gateway:"{live_gw}",dns1:"{live_dns1}",dns2:"{live_dns2}"}};
function toggleStatic(show) {{
  document.getElementById('static-fields').style.display = show ? 'block' : 'none';
  if (show && !document.getElementById('net_ipaddr').value) {{
    document.getElementById('net_ipaddr').value = liveStatus.ip;
    document.getElementById('net_netmask').value = liveStatus.netmask || '255.255.255.0';
    document.getElementById('net_gateway').value = liveStatus.gateway;
    document.getElementById('net_dns1').value = liveStatus.dns1;
    document.getElementById('net_dns2').value = liveStatus.dns2;
  }}
}}
</script>
</body></html>"#,
        saved_msg = saved_msg,
        mqtt_en = mqtt_checked,
        mqtt_broker = html_escape(&config.mqtt.broker),
        mqtt_port = config.mqtt.port,
        mqtt_tls = tls_checked,
        mqtt_topic = html_escape(&config.mqtt.topic),
        mqtt_client_id = html_escape(&config.mqtt.client_id),
        http_en = http_checked,
        http_url = html_escape(&config.http.url),
        uart_en = uart_checked,
        uart_port = html_escape(&config.uart.port),
        baud_opts = baud_options,
        interval = config.general.interval_secs,
        // Network config
        status_color = status_color,
        status_text = status_text,
        dhcp_checked = dhcp_checked,
        static_checked = static_checked,
        static_display = static_display,
        net_ipaddr = html_escape(&net_config.ipaddr),
        net_netmask = html_escape(&net_config.netmask),
        net_gateway = html_escape(&net_config.gateway),
        net_dns1 = html_escape(&net_config.dns_primary),
        net_dns2 = html_escape(&net_config.dns_secondary),
        current_ip = if net_status.ip.is_empty() { "-" } else { &net_status.ip },
        current_gw = if net_status.gateway.is_empty() { "-" } else { &net_status.gateway },
        // Live status for JS auto-fill
        live_ip = html_escape(&net_status.ip),
        live_netmask = html_escape(&net_status.netmask),
        live_gw = html_escape(&net_status.gateway),
        live_dns1 = html_escape(net_status.dns.first().unwrap_or(&String::new())),
        live_dns2 = html_escape(net_status.dns.get(1).unwrap_or(&String::new())),
    )
}
