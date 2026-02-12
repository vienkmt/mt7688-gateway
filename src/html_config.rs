use crate::config::Config;

/// Render the configuration page HTML with current config values
pub fn render_config_page(config: &Config, saved: bool) -> String {
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
    let saved_msg = if saved {
        r#"<div class="toast">Đã lưu cấu hình! Đang kết nối lại...</div>"#
    } else {
        ""
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

<button type="submit" class="btn">Lưu cấu hình</button>
</form>
<a href="/" class="back">← Quay lại giám sát</a>
</div>
</body></html>"#,
        saved_msg = saved_msg,
        mqtt_en = mqtt_checked,
        mqtt_broker = config.mqtt.broker,
        mqtt_port = config.mqtt.port,
        mqtt_tls = tls_checked,
        mqtt_topic = config.mqtt.topic,
        mqtt_client_id = config.mqtt.client_id,
        http_en = http_checked,
        http_url = config.http.url,
        uart_en = uart_checked,
        uart_port = config.uart.port,
        baud_opts = baud_options,
        interval = config.general.interval_secs,
    )
}
