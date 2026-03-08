//! WiFi configuration handlers
//! 3 chế độ: STA (client), AP (access point), STA+AP (repeater)
//! UCI wifi-iface: 'wwan' = STA, 'default_radio0' = AP

use crate::uci::Uci;
use crate::web_api::{json_err, json_escape, json_resp, jval, Resp};
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
                    json_escape(&ssid), signal, json_escape(&enc)
                ));
            }
            ssid = rest.trim_matches('"').to_string();
            signal = 0;
            enc.clear();
        } else if let Some(rest) = line.strip_prefix("Signal: ") {
            signal = rest.split_whitespace().next()
                .and_then(|s| s.parse().ok()).unwrap_or(0);
        } else if let Some(rest) = line.strip_prefix("Encryption: ") {
            enc = rest.to_string();
        }
    }
    if !ssid.is_empty() {
        networks.push(format!(
            r#"{{"ssid":"{}","signal":{},"encryption":"{}"}}"#,
            json_escape(&ssid), signal, json_escape(&enc)
        ));
    }

    json_resp(&format!(r#"{{"networks":[{}]}}"#, networks.join(",")))
}

/// GET /api/wifi/status — trạng thái đầy đủ cả STA + AP
pub fn handle_status() -> Resp {
    // Detect current mode from UCI disabled flags
    let sta_disabled = Uci::get("wireless.wwan.disabled").unwrap_or_default() == "1";
    let ap_disabled = Uci::get("wireless.default_radio0.disabled").unwrap_or_default() == "1";
    let mode = match (sta_disabled, ap_disabled) {
        (true, true) => "off",
        (false, true) => "sta",
        (true, false) => "ap",
        (false, false) => "sta_ap",
    };

    // STA config + runtime
    let sta_ssid = Uci::get("wireless.wwan.ssid").unwrap_or_default();
    let sta_key = Uci::get("wireless.wwan.key").unwrap_or_default();
    let sta_enc = Uci::get("wireless.wwan.encryption").unwrap_or_default();

    let sta_out = Command::new("iwinfo").args(["phy0-sta0", "info"]).output()
        .ok().map(|o| String::from_utf8_lossy(&o.stdout).to_string()).unwrap_or_default();
    let sta_connected = sta_out.contains("ESSID:") && !sta_out.contains("ESSID: unknown");
    let sta_live_ssid = sta_out.lines()
        .find_map(|l| l.trim().strip_prefix("ESSID: "))
        .map(|s| s.trim_matches('"').to_string()).unwrap_or_default();
    let sta_signal = parse_signal(&sta_out);
    let sta_ip = get_iface_ip("phy0-sta0");

    // AP config + runtime
    let ap_ssid = Uci::get("wireless.default_radio0.ssid").unwrap_or_default();
    let ap_enc = Uci::get("wireless.default_radio0.encryption").unwrap_or_default();
    let ap_key = Uci::get("wireless.default_radio0.key").unwrap_or_default();
    let ap_channel = Uci::get("wireless.radio0.channel").unwrap_or_default();

    let ap_out = Command::new("iwinfo").args(["phy0-ap0", "info"]).output()
        .ok().map(|o| String::from_utf8_lossy(&o.stdout).to_string()).unwrap_or_default();
    let ap_active = ap_out.contains("ESSID:") && !ap_out.contains("ESSID: unknown");

    json_resp(&format!(
        r#"{{"mode":"{}","sta":{{"connected":{},"ssid":"{}","config_ssid":"{}","config_key":"{}","config_enc":"{}","signal":{},"ip":"{}"}},"ap":{{"active":{},"ssid":"{}","encryption":"{}","key":"{}","channel":"{}"}}}}"#,
        mode,
        sta_connected, json_escape(if sta_connected { &sta_live_ssid } else { "" }),
        json_escape(&sta_ssid), json_escape(&sta_key), json_escape(&sta_enc), sta_signal, sta_ip,
        ap_active, json_escape(&ap_ssid), json_escape(&ap_enc),
        json_escape(&ap_key), json_escape(&ap_channel),
    ))
}

/// POST /api/wifi/mode — chuyển chế độ WiFi
/// body: {"mode":"sta|ap|sta_ap", "sta":{...}, "ap":{...}}
pub fn handle_set_mode(body: &str) -> Resp {
    let mode = match jval(body, "mode") {
        Some(m) if m == "sta" || m == "ap" || m == "sta_ap" || m == "off" => m,
        _ => return json_err(400, "mode must be sta, ap, sta_ap, or off"),
    };

    match mode.as_str() {
        "off" => {
            Uci::set("wireless.wwan.disabled", "1").ok();
            Uci::set("wireless.default_radio0.disabled", "1").ok();
        }
        "sta" => {
            Uci::set("wireless.wwan.disabled", "0").ok();
            Uci::set("wireless.default_radio0.disabled", "1").ok();
            set_sta_config(body);
        }
        "ap" => {
            Uci::set("wireless.wwan.disabled", "1").ok();
            Uci::set("wireless.default_radio0.disabled", "0").ok();
            set_ap_config(body);
        }
        "sta_ap" => {
            Uci::set("wireless.wwan.disabled", "0").ok();
            Uci::set("wireless.default_radio0.disabled", "0").ok();
            set_sta_config(body);
            set_ap_config(body);
        }
        _ => unreachable!(),
    }

    json_resp(r#"{"ok":true,"draft":true,"message":"saved to RAM, apply to persist"}"#)
}

/// POST /api/wifi/connect — kết nối STA tới SSID
pub fn handle_connect(body: &str) -> Resp {
    let ssid = match jval(body, "ssid") {
        Some(s) if !s.is_empty() => s,
        _ => return json_err(400, "missing ssid"),
    };
    let password = jval(body, "password").unwrap_or_default();
    let encryption = jval(body, "encryption").unwrap_or_else(|| "psk2".into());

    Uci::set("wireless.wwan.disabled", "0").ok();
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

// --- helpers ---

fn set_sta_config(body: &str) {
    if let Some(ssid) = jval(body, "sta_ssid") {
        if !ssid.is_empty() {
            Uci::set("wireless.wwan.ssid", &ssid).ok();
        }
    }
    let pwd = jval(body, "sta_password").unwrap_or_default();
    if pwd.is_empty() {
        Uci::set("wireless.wwan.encryption", "none").ok();
        Uci::delete("wireless.wwan.key").ok();
    } else {
        Uci::set("wireless.wwan.encryption", "psk2").ok();
        Uci::set("wireless.wwan.key", &pwd).ok();
    }
}

fn set_ap_config(body: &str) {
    if let Some(ssid) = jval(body, "ap_ssid") {
        if !ssid.is_empty() {
            Uci::set("wireless.default_radio0.ssid", &ssid).ok();
        }
    }
    let pwd = jval(body, "ap_password").unwrap_or_default();
    if pwd.is_empty() {
        Uci::set("wireless.default_radio0.encryption", "none").ok();
        Uci::delete("wireless.default_radio0.key").ok();
    } else {
        Uci::set("wireless.default_radio0.encryption", "psk2").ok();
        Uci::set("wireless.default_radio0.key", &pwd).ok();
    }
    if let Some(ch) = jval(body, "ap_channel") {
        Uci::set("wireless.radio0.channel", &ch).ok();
    }
}

fn parse_signal(output: &str) -> i32 {
    output.lines().find_map(|l| {
        l.trim().strip_prefix("Signal: ")
            .and_then(|r| r.split_whitespace().next())
            .and_then(|s| s.parse::<i32>().ok())
    }).unwrap_or(0)
}

fn get_iface_ip(iface: &str) -> String {
    Command::new("ip").args(["-4", "addr", "show", iface]).output()
        .ok().and_then(|o| {
            String::from_utf8_lossy(&o.stdout).lines()
                .find_map(|l| l.trim().strip_prefix("inet ")
                    .and_then(|r| r.split('/').next()).map(String::from))
        }).unwrap_or_default()
}
