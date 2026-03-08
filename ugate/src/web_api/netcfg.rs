//! Network configuration handlers
//! LAN/WAN config, NTP settings, routing, interface metrics

use crate::uci::Uci;
use crate::web_api::{is_safe_identifier, is_valid_ipv4, json_err, json_resp, jval, Resp};
use std::process::Command;

/// GET /api/network — lấy config LAN/WAN
pub fn handle_get_network() -> Resp {
    let iface_json = |name: &str| -> String {
        let prefix = format!("network.{}", name);
        format!(
            r#"{{"name":"{}","proto":"{}","ipaddr":"{}","netmask":"{}","gateway":"{}","metric":"{}","dns":{}}}"#,
            name,
            uci_get(&format!("{}.proto", prefix)),
            uci_get(&format!("{}.ipaddr", prefix)),
            uci_get(&format!("{}.netmask", prefix)),
            uci_get(&format!("{}.gateway", prefix)),
            uci_get(&format!("{}.metric", prefix)),
            json_str_array(&Uci::get_list(&format!("{}.dns", prefix))),
        )
    };
    let wwan_metric = uci_get("network.wwan.metric");
    json_resp(&format!(
        r#"{{"lan":{},"wan":{},"wwan_metric":"{}"}}"#,
        iface_json("lan"),
        iface_json("wan"),
        wwan_metric,
    ))
}

/// POST /api/network — set config LAN/WAN
pub fn handle_set_network(body: &str) -> Resp {
    let iface = match jval(body, "interface") {
        Some(s) if s == "lan" || s == "wan" => s,
        _ => return json_err(400, "interface must be lan or wan"),
    };
    let proto = jval(body, "proto").unwrap_or_else(|| "dhcp".into());
    let prefix = format!("network.{}", iface);

    if proto == "static" {
        let ipaddr = jval(body, "ipaddr").unwrap_or_default();
        let netmask = jval(body, "netmask").unwrap_or_else(|| "255.255.255.0".into());
        let gateway = jval(body, "gateway").unwrap_or_default();
        if !ipaddr.is_empty() && !is_valid_ipv4(&ipaddr) {
            return json_err(400, "invalid ipaddr");
        }
        if !gateway.is_empty() && !is_valid_ipv4(&gateway) {
            return json_err(400, "invalid gateway");
        }
        Uci::set(&format!("{}.proto", prefix), "static").ok();
        if !ipaddr.is_empty() {
            Uci::set(&format!("{}.ipaddr", prefix), &ipaddr).ok();
        }
        Uci::set(&format!("{}.netmask", prefix), &netmask).ok();
        if !gateway.is_empty() {
            Uci::set(&format!("{}.gateway", prefix), &gateway).ok();
        }
        Uci::delete(&format!("{}.dns", prefix)).ok();
        if let Some(dns_str) = jval(body, "dns") {
            for dns in dns_str.split(',') {
                let dns = dns.trim().trim_matches(|c| c == '[' || c == ']' || c == '"');
                if !dns.is_empty() && is_valid_ipv4(dns) {
                    Uci::add_list(&format!("{}.dns", prefix), dns).ok();
                }
            }
        }
    } else {
        Uci::set(&format!("{}.proto", prefix), "dhcp").ok();
        Uci::delete(&format!("{}.ipaddr", prefix)).ok();
        Uci::delete(&format!("{}.netmask", prefix)).ok();
        Uci::delete(&format!("{}.gateway", prefix)).ok();
    }

    // Chỉ lưu vào RAM (chưa commit), user phải ấn Apply để ghi flash
    json_resp(r#"{"ok":true,"draft":true,"message":"saved to RAM, apply to persist"}"#)
}

/// POST /api/network/apply — commit thay đổi + restart chỉ interface bị thay đổi
pub fn handle_apply() -> Resp {
    let net_sections = Uci::changed_sections("network");
    let has_wifi = Uci::has_changes("wireless");
    let has_sys = Uci::has_changes("system");
    let has_net = !net_sections.is_empty();
    if !has_net && !has_wifi && !has_sys {
        return json_resp(r#"{"ok":true,"message":"no pending changes"}"#);
    }
    if has_net {
        Uci::commit("network").ok();
    }
    if has_wifi {
        Uci::commit("wireless").ok();
    }
    if has_sys {
        Uci::commit("system").ok();
    }
    // netifd reload: chỉ restart interface thay đổi (diff-based, gián đoạn tối thiểu)
    if has_net {
        std::thread::spawn(|| {
            std::thread::sleep(std::time::Duration::from_secs(1));
            Command::new("ubus")
                .args(["call", "network", "reload"])
                .status()
                .ok();
        });
    }
    if has_wifi {
        std::thread::spawn(|| {
            std::thread::sleep(std::time::Duration::from_secs(1));
            Command::new("wifi").arg("reload").status().ok();
        });
    }
    json_resp(r#"{"ok":true,"message":"applied"}"#)
}

/// POST /api/network/revert — huỷ tất cả thay đổi chưa commit
pub fn handle_revert() -> Resp {
    Uci::revert("network").ok();
    Uci::revert("wireless").ok();
    Uci::revert("system").ok();
    json_resp(r#"{"ok":true,"message":"reverted all pending changes"}"#)
}

/// GET /api/network/changes — xem có thay đổi nào chưa commit không
pub fn handle_changes() -> Resp {
    let net = Uci::has_changes("network");
    let wifi = Uci::has_changes("wireless");
    let sys = Uci::has_changes("system");
    json_resp(&format!(
        r#"{{"pending":{},"network":{},"wireless":{},"system":{}}}"#,
        net || wifi || sys,
        net,
        wifi,
        sys
    ))
}

/// GET /api/ntp
pub fn handle_get_ntp() -> Resp {
    json_resp(&format!(
        r#"{{"enabled":{},"servers":{},"timezone":"{}","zonename":"{}"}}"#,
        uci_get("system.ntp.enabled") == "1",
        json_str_array(&Uci::get_list("system.ntp.server")),
        uci_get("system.@system[0].timezone"),
        uci_get("system.@system[0].zonename"),
    ))
}

/// POST /api/ntp
pub fn handle_set_ntp(body: &str) -> Resp {
    Uci::delete("system.ntp.server").ok();
    if let Some(servers_str) = jval(body, "servers") {
        for srv in servers_str.split(',') {
            let srv = srv.trim().trim_matches(|c| c == '[' || c == ']' || c == '"');
            // Validate: chỉ cho phép hostname hợp lệ (alphanumeric, dot, hyphen)
            if !srv.is_empty()
                && srv
                    .chars()
                    .all(|c| c.is_ascii_alphanumeric() || c == '.' || c == '-')
            {
                Uci::add_list("system.ntp.server", srv).ok();
            }
        }
    }
    if let Some(tz) = jval(body, "timezone") {
        Uci::set("system.@system[0].timezone", &tz).ok();
    }
    if let Some(zn) = jval(body, "zonename") {
        Uci::set("system.@system[0].zonename", &zn).ok();
    }
    let enabled = jval(body, "enabled").map(|v| v == "true").unwrap_or(true);
    Uci::set("system.ntp.enabled", if enabled { "1" } else { "0" }).ok();
    Uci::commit("system").ok();
    json_resp(r#"{"ok":true,"message":"NTP config saved"}"#)
}

/// POST /api/ntp/sync — trigger manual sync (non-blocking with timeout)
pub fn handle_ntp_sync() -> Resp {
    let servers = Uci::get_list("system.ntp.server");
    let server = servers.first().map(|s| s.as_str()).unwrap_or("pool.ntp.org");
    let result = Command::new("timeout")
        .args(["5", "ntpd", "-q", "-p", server])
        .status();
    match result {
        Ok(s) if s.success() => json_resp(r#"{"ok":true,"method":"ntp"}"#),
        _ => {
            crate::time_sync::sync_time();
            json_resp(r#"{"ok":true,"method":"http_date"}"#)
        }
    }
}

/// GET /api/routes — parse routing table thành JSON array
pub fn handle_get_routes() -> Resp {
    let output = Command::new("ip")
        .args(["route", "show"])
        .output()
        .ok()
        .map(|o| String::from_utf8_lossy(&o.stdout).to_string())
        .unwrap_or_default();

    let mut routes = Vec::new();
    for line in output.lines() {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.is_empty() {
            continue;
        }
        let dest = parts[0];
        let gateway = parts.iter().position(|&p| p == "via")
            .and_then(|i| parts.get(i + 1))
            .unwrap_or(&"-");
        let dev = parts.iter().position(|&p| p == "dev")
            .and_then(|i| parts.get(i + 1))
            .unwrap_or(&"-");
        let metric = parts.iter().position(|&p| p == "metric")
            .and_then(|i| parts.get(i + 1))
            .unwrap_or(&"-");
        let scope = parts.iter().position(|&p| p == "scope")
            .and_then(|i| parts.get(i + 1))
            .unwrap_or(&"-");
        routes.push(format!(
            r#"{{"dest":"{}","via":"{}","dev":"{}","metric":"{}","scope":"{}"}}"#,
            crate::web_api::json_escape(dest),
            crate::web_api::json_escape(gateway),
            crate::web_api::json_escape(dev),
            metric,
            scope,
        ));
    }
    json_resp(&format!(r#"{{"routes":[{}]}}"#, routes.join(",")))
}

/// POST /api/routes
pub fn handle_add_route(body: &str) -> Resp {
    let name = match jval(body, "name") {
        Some(n) if is_safe_identifier(&n) => n,
        _ => return json_err(400, "invalid or missing name (alphanumeric/underscore only)"),
    };
    let target = jval(body, "target").unwrap_or_default();
    let netmask = jval(body, "netmask").unwrap_or_else(|| "255.255.255.0".into());
    let gateway = jval(body, "gateway").unwrap_or_default();
    let iface = match jval(body, "interface") {
        Some(i) if is_safe_identifier(&i) => i,
        _ => return json_err(400, "invalid interface name"),
    };

    if !is_valid_ipv4(&target) || !is_valid_ipv4(&gateway) || !is_valid_ipv4(&netmask) {
        return json_err(400, "invalid IP address");
    }

    let route_name = format!("route_{}", name);
    Uci::set(&format!("network.{}", route_name), "route").ok();
    Uci::set(&format!("network.{}.interface", route_name), &iface).ok();
    Uci::set(&format!("network.{}.target", route_name), &target).ok();
    Uci::set(&format!("network.{}.netmask", route_name), &netmask).ok();
    Uci::set(&format!("network.{}.gateway", route_name), &gateway).ok();
    // Apply immediately via ip route (CIDR notation)
    let cidr = netmask_to_cidr(&netmask);
    Command::new("ip")
        .args(["route", "add", &format!("{}/{}", target, cidr), "via", &gateway])
        .status()
        .ok();
    json_resp(r#"{"ok":true,"draft":true}"#)
}

/// DELETE /api/routes/{name}
pub fn handle_delete_route(name: &str) -> Resp {
    if !is_safe_identifier(name) {
        return json_err(400, "invalid route name");
    }
    let route_name = format!("route_{}", name);
    Uci::delete(&format!("network.{}", route_name)).ok();
    json_resp(r#"{"ok":true,"draft":true}"#)
}

/// GET /api/wan/discover — dynamic WAN interface discovery from default routes
pub fn handle_wan_discover() -> Resp {
    let output = Command::new("ip")
        .args(["route", "show", "default"])
        .output()
        .ok()
        .map(|o| String::from_utf8_lossy(&o.stdout).to_string())
        .unwrap_or_default();

    let mut ifaces = Vec::new();
    for line in output.lines() {
        let parts: Vec<&str> = line.split_whitespace().collect();
        let dev = parts.iter().position(|&p| p == "dev")
            .and_then(|i| parts.get(i + 1)).copied().unwrap_or("-");
        let via = parts.iter().position(|&p| p == "via")
            .and_then(|i| parts.get(i + 1)).copied().unwrap_or("-");
        let metric = parts.iter().position(|&p| p == "metric")
            .and_then(|i| parts.get(i + 1)).copied().unwrap_or("0");
        let (uci_name, label) = dev_to_uci(dev);
        let uci_metric = uci_get(&format!("network.{}.metric", uci_name));
        ifaces.push(format!(
            r#"{{"dev":"{}","uci":"{}","label":"{}","gateway":"{}","metric":"{}","uci_metric":"{}"}}"#,
            crate::web_api::json_escape(dev), uci_name,
            label, crate::web_api::json_escape(via),
            metric, uci_metric,
        ));
    }
    json_resp(&format!(r#"{{"interfaces":[{}]}}"#, ifaces.join(",")))
}

/// Map device name → (UCI interface name, display label)
fn dev_to_uci(dev: &str) -> (&str, &str) {
    match dev {
        "eth0.2" | "eth0.2@eth0" => ("wan", "ETH WAN"),
        "phy0-sta0" => ("wwan", "WiFi WAN"),
        "br-lan" => ("lan", "LAN"),
        _ => (dev, dev), // future 4G, USB tethering, etc.
    }
}

/// POST /api/interface/metric
pub fn handle_set_metric(body: &str) -> Resp {
    let iface = match jval(body, "interface") {
        Some(s) if is_safe_identifier(&s) => s,
        _ => return json_err(400, "invalid interface name"),
    };
    let metric = match jval(body, "metric").and_then(|v| v.parse::<u32>().ok()) {
        Some(m) => m,
        None => return json_err(400, "invalid metric"),
    };
    Uci::set(&format!("network.{}.metric", iface), &metric.to_string()).ok();
    json_resp(r#"{"ok":true,"draft":true}"#)
}

// --- Helpers ---

fn uci_get(key: &str) -> String {
    Uci::get(key).unwrap_or_default()
}

fn json_str_array(items: &[String]) -> String {
    if items.is_empty() {
        return "[]".into();
    }
    let inner: Vec<String> = items.iter().map(|s| format!("\"{}\"", s)).collect();
    format!("[{}]", inner.join(","))
}

/// Convert netmask "255.255.255.0" → CIDR prefix length 24
fn netmask_to_cidr(mask: &str) -> u8 {
    mask.split('.')
        .filter_map(|p| p.parse::<u8>().ok())
        .map(|b| b.count_ones())
        .sum::<u32>() as u8
}
