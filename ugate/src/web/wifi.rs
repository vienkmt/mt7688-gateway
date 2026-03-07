//! WiFi configuration handlers
//! Quét, kết nối, ngắt, trạng thái WiFi qua iwinfo + UCI

use crate::uci::Uci;
use crate::web::{json_err, json_escape, json_resp, jval, Resp};
use std::process::Command;

/// GET /api/wifi/scan
pub fn handle_scan() -> Resp {
    let output = Command::new("iwinfo").args(["phy0-sta0", "scan"]).output();
    let stdout = match output {
        Ok(o) if o.status.success() => String::from_utf8_lossy(&o.stdout).to_string(),
        _ => return json_resp(r#"{"networks":[]}"#),
    };

    let mut networks = Vec::new();
    let mut ssid = String::new();
    let mut signal: i32 = 0;
    let mut enc = String::new();

    for line in stdout.lines() {
        let line = line.trim();
        if let Some(rest) = line.strip_prefix("ESSID: ") {
            if !ssid.is_empty() {
                networks.push(format!(
                    r#"{{"ssid":"{}","signal":{},"encryption":"{}"}}"#,
                    json_escape(&ssid),
                    signal,
                    json_escape(&enc)
                ));
            }
            ssid = rest.trim_matches('"').to_string();
            signal = 0;
            enc.clear();
        } else if let Some(rest) = line.strip_prefix("Signal: ") {
            signal = rest
                .split_whitespace()
                .next()
                .and_then(|s| s.parse().ok())
                .unwrap_or(0);
        } else if let Some(rest) = line.strip_prefix("Encryption: ") {
            enc = rest.to_string();
        }
    }
    if !ssid.is_empty() {
        networks.push(format!(
            r#"{{"ssid":"{}","signal":{},"encryption":"{}"}}"#,
            json_escape(&ssid),
            signal,
            json_escape(&enc)
        ));
    }

    json_resp(&format!(r#"{{"networks":[{}]}}"#, networks.join(",")))
}

/// GET /api/wifi/status — kết hợp UCI config + iwinfo runtime
pub fn handle_status() -> Resp {
    // UCI config — wifi-iface 'wwan' là STA (client), 'default_radio0' là AP
    let cfg_ssid = Uci::get("wireless.wwan.ssid").unwrap_or_default();
    let cfg_enc = Uci::get("wireless.wwan.encryption").unwrap_or_default();
    let cfg_disabled = Uci::get("wireless.wwan.disabled").unwrap_or_default();

    // Runtime status từ iwinfo
    let stdout = Command::new("iwinfo")
        .args(["phy0-sta0", "info"])
        .output()
        .ok()
        .map(|o| String::from_utf8_lossy(&o.stdout).to_string())
        .unwrap_or_default();

    let connected = stdout.contains("ESSID:") && !stdout.contains("ESSID: unknown");
    let live_ssid = stdout
        .lines()
        .find_map(|l| l.trim().strip_prefix("ESSID: "))
        .map(|s| s.trim_matches('"').to_string())
        .unwrap_or_default();
    let signal = stdout
        .lines()
        .find_map(|l| {
            l.trim()
                .strip_prefix("Signal: ")
                .and_then(|r| r.split_whitespace().next())
                .and_then(|s| s.parse::<i32>().ok())
        })
        .unwrap_or(0);
    let ip = get_iface_ip("phy0-sta0");

    json_resp(&format!(
        r#"{{"connected":{},"ssid":"{}","signal":{},"ip":"{}","config_ssid":"{}","config_enc":"{}","disabled":{}}}"#,
        connected,
        json_escape(if connected { &live_ssid } else { &cfg_ssid }),
        signal,
        ip,
        json_escape(&cfg_ssid),
        json_escape(&cfg_enc),
        cfg_disabled == "1",
    ))
}

/// POST /api/wifi/connect
pub fn handle_connect(body: &str) -> Resp {
    let ssid = match jval(body, "ssid") {
        Some(s) if !s.is_empty() => s,
        _ => return json_err(400, "missing ssid"),
    };
    let password = jval(body, "password").unwrap_or_default();
    let encryption = jval(body, "encryption").unwrap_or_else(|| "psk2".into());

    // Set trên wifi-iface 'wwan' (STA/client mode)
    if let Err(e) = Uci::set("wireless.wwan.ssid", &ssid) {
        return json_err(500, &e);
    }
    Uci::set("wireless.wwan.key", &password).ok();
    Uci::set("wireless.wwan.encryption", &encryption).ok();
    json_resp(r#"{"ok":true,"draft":true,"message":"saved to RAM, apply to persist"}"#)
}

/// POST /api/wifi/disconnect
pub fn handle_disconnect() -> Resp {
    Uci::set("wireless.wwan.ssid", "").ok();
    Uci::delete("wireless.wwan.key").ok();
    json_resp(r#"{"ok":true,"draft":true}"#)
}

fn get_iface_ip(iface: &str) -> String {
    Command::new("ip")
        .args(["-4", "addr", "show", iface])
        .output()
        .ok()
        .and_then(|o| {
            String::from_utf8_lossy(&o.stdout)
                .lines()
                .find_map(|l| {
                    l.trim()
                        .strip_prefix("inet ")
                        .and_then(|r| r.split('/').next())
                        .map(String::from)
                })
        })
        .unwrap_or_default()
}
