//! Network configuration for eth0.2 (WAN) interface
//! Supports DHCP and Static IP modes via OpenWrt UCI

use crate::uci::Uci;
use std::process::Command;

#[derive(Clone, Debug, PartialEq)]
pub enum NetworkMode {
    Dhcp,
    Static,
}

impl NetworkMode {
    pub fn as_str(&self) -> &'static str {
        match self {
            NetworkMode::Dhcp => "dhcp",
            NetworkMode::Static => "static",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s {
            "static" => NetworkMode::Static,
            _ => NetworkMode::Dhcp,
        }
    }
}

#[derive(Clone, Debug)]
pub struct NetworkConfig {
    pub mode: NetworkMode,
    pub ipaddr: String,
    pub netmask: String,
    pub gateway: String,
    pub dns_primary: String,
    pub dns_secondary: String,
}

impl Default for NetworkConfig {
    fn default() -> Self {
        Self {
            mode: NetworkMode::Dhcp,
            ipaddr: String::new(),
            netmask: "255.255.255.0".to_string(),
            gateway: String::new(),
            dns_primary: String::new(),
            dns_secondary: String::new(),
        }
    }
}

impl NetworkConfig {
    /// Load current config from UCI
    pub fn load_from_uci() -> Self {
        let mode = Uci::get("network.wan.proto")
            .map(|s| NetworkMode::from_str(&s))
            .unwrap_or(NetworkMode::Dhcp);

        let ipaddr = Uci::get("network.wan.ipaddr").unwrap_or_default();
        let netmask = Uci::get("network.wan.netmask").unwrap_or_else(|_| "255.255.255.0".to_string());
        let gateway = Uci::get("network.wan.gateway").unwrap_or_default();

        // DNS is space-separated in UCI
        let dns = Uci::get("network.wan.dns").unwrap_or_default();
        let dns_parts: Vec<&str> = dns.split_whitespace().collect();
        let dns_primary = dns_parts.first().map(|s| s.to_string()).unwrap_or_default();
        let dns_secondary = dns_parts.get(1).map(|s| s.to_string()).unwrap_or_default();

        Self { mode, ipaddr, netmask, gateway, dns_primary, dns_secondary }
    }

    /// Save config to UCI and apply with ifdown/ifup
    pub fn save_to_uci(&self) -> Result<(), String> {
        // Set protocol
        Uci::set("network.wan.proto", self.mode.as_str())?;

        match self.mode {
            NetworkMode::Static => {
                Uci::set("network.wan.ipaddr", &self.ipaddr)?;
                Uci::set("network.wan.netmask", &self.netmask)?;
                Uci::set("network.wan.gateway", &self.gateway)?;

                // Combine DNS servers
                let dns = if self.dns_secondary.is_empty() {
                    self.dns_primary.clone()
                } else {
                    format!("{} {}", self.dns_primary, self.dns_secondary)
                };
                if !dns.is_empty() {
                    Uci::set("network.wan.dns", &dns)?;
                }
            }
            NetworkMode::Dhcp => {
                // Remove static-only options
                let _ = Uci::delete("network.wan.ipaddr");
                let _ = Uci::delete("network.wan.netmask");
                let _ = Uci::delete("network.wan.gateway");
                let _ = Uci::delete("network.wan.dns");
            }
        }

        // Commit and apply
        Uci::commit("network")?;
        Self::restart_interface()?;
        Ok(())
    }

    /// Restart WAN interface without full network restart
    fn restart_interface() -> Result<(), String> {
        let _ = Command::new("ifdown").arg("wan").output();
        let output = Command::new("ifup").arg("wan").output()
            .map_err(|e| format!("ifup failed: {}", e))?;

        if output.status.success() {
            Ok(())
        } else {
            Err(String::from_utf8_lossy(&output.stderr).trim().to_string())
        }
    }
}

/// Live interface status from system commands
#[derive(Clone, Debug, Default)]
pub struct NetworkStatus {
    pub ip: String,
    pub netmask: String,
    pub gateway: String,
    pub dns: Vec<String>,
    pub is_up: bool,
}

impl NetworkStatus {
    /// Get current interface status from ip/route commands
    pub fn get_current() -> Self {
        let mut status = Self::default();

        // Get IP from: ip addr show eth0.2
        if let Ok(output) = Command::new("ip").args(["addr", "show", "eth0.2"]).output() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            status.is_up = stdout.contains("state UP");

            // Parse inet line: "inet 192.168.1.100/24 ..."
            for line in stdout.lines() {
                let line = line.trim();
                if line.starts_with("inet ") {
                    if let Some(addr) = line.split_whitespace().nth(1) {
                        let parts: Vec<&str> = addr.split('/').collect();
                        status.ip = parts.first().map(|s| s.to_string()).unwrap_or_default();
                        if let Some(cidr) = parts.get(1) {
                            status.netmask = cidr_to_netmask(cidr.parse().unwrap_or(24));
                        }
                    }
                    break;
                }
            }
        }

        // Get gateway from: ip route
        if let Ok(output) = Command::new("ip").args(["route"]).output() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            for line in stdout.lines() {
                if line.starts_with("default via ") {
                    if let Some(gw) = line.split_whitespace().nth(2) {
                        status.gateway = gw.to_string();
                    }
                    break;
                }
            }
        }

        // Get DNS from resolv.conf (use std::fs instead of cat for safety)
        if let Ok(content) = std::fs::read_to_string("/tmp/resolv.conf.d/resolv.conf.auto") {
            for line in content.lines() {
                if line.starts_with("nameserver ") {
                    if let Some(ns) = line.split_whitespace().nth(1) {
                        status.dns.push(ns.to_string());
                    }
                }
            }
        }

        status
    }
}

/// Convert CIDR prefix to dotted netmask
fn cidr_to_netmask(cidr: u8) -> String {
    let mask: u32 = if cidr >= 32 { !0 } else { !0 << (32 - cidr) };
    format!(
        "{}.{}.{}.{}",
        (mask >> 24) & 0xff,
        (mask >> 16) & 0xff,
        (mask >> 8) & 0xff,
        mask & 0xff
    )
}

// === Validation Functions ===

/// Validate IPv4 address format
pub fn is_valid_ipv4(ip: &str) -> bool {
    let parts: Vec<&str> = ip.split('.').collect();
    parts.len() == 4 && parts.iter().all(|p| p.parse::<u8>().is_ok())
}

/// Convert dotted netmask to CIDR prefix
fn netmask_to_cidr(mask: &str) -> Option<u8> {
    let parts: Vec<u8> = mask.split('.').filter_map(|p| p.parse().ok()).collect();
    if parts.len() != 4 {
        return None;
    }
    let mask_u32 = ((parts[0] as u32) << 24)
        | ((parts[1] as u32) << 16)
        | ((parts[2] as u32) << 8)
        | (parts[3] as u32);

    // Check contiguous 1s
    let leading = mask_u32.leading_ones();
    let trailing = mask_u32.trailing_zeros();
    if leading + trailing == 32 {
        Some(leading as u8)
    } else {
        None
    }
}

/// Check if netmask is valid (contiguous 1s followed by 0s)
pub fn is_valid_netmask(mask: &str) -> bool {
    netmask_to_cidr(mask).is_some()
}

/// Parse IPv4 to u32
fn ip_to_u32(ip: &str) -> Option<u32> {
    let parts: Vec<u8> = ip.split('.').filter_map(|p| p.parse().ok()).collect();
    if parts.len() != 4 {
        return None;
    }
    Some(
        ((parts[0] as u32) << 24)
            | ((parts[1] as u32) << 16)
            | ((parts[2] as u32) << 8)
            | (parts[3] as u32),
    )
}

/// Check if IP conflicts with LAN (10.10.10.0/24)
pub fn conflicts_with_lan(ip: &str) -> bool {
    if let Some(ip_u32) = ip_to_u32(ip) {
        let lan_network = ip_to_u32("10.10.10.0").unwrap();
        let lan_mask = !0u32 << 8; // /24
        (ip_u32 & lan_mask) == (lan_network & lan_mask)
    } else {
        false
    }
}

/// Check if gateway is in same subnet as IP
pub fn gateway_in_subnet(ip: &str, mask: &str, gw: &str) -> bool {
    let ip_u32 = match ip_to_u32(ip) {
        Some(v) => v,
        None => return false,
    };
    let gw_u32 = match ip_to_u32(gw) {
        Some(v) => v,
        None => return false,
    };
    let cidr = match netmask_to_cidr(mask) {
        Some(v) => v,
        None => return false,
    };
    let mask_u32 = if cidr >= 32 { !0 } else { !0u32 << (32 - cidr) };
    (ip_u32 & mask_u32) == (gw_u32 & mask_u32)
}

/// Validate complete network config, return list of errors
pub fn validate_config(config: &NetworkConfig) -> Result<(), Vec<String>> {
    if config.mode == NetworkMode::Dhcp {
        return Ok(()); // No validation needed for DHCP
    }

    let mut errors = Vec::new();

    if !is_valid_ipv4(&config.ipaddr) {
        errors.push("Invalid IP address format".to_string());
    }

    if !is_valid_netmask(&config.netmask) {
        errors.push("Invalid subnet mask".to_string());
    }

    if !is_valid_ipv4(&config.gateway) {
        errors.push("Invalid gateway format".to_string());
    }

    if is_valid_ipv4(&config.ipaddr) && conflicts_with_lan(&config.ipaddr) {
        errors.push("IP conflicts with LAN (10.10.10.0/24)".to_string());
    }

    if is_valid_ipv4(&config.ipaddr)
        && is_valid_netmask(&config.netmask)
        && is_valid_ipv4(&config.gateway)
        && !gateway_in_subnet(&config.ipaddr, &config.netmask, &config.gateway)
    {
        errors.push("Gateway not in same subnet".to_string());
    }

    if !config.dns_primary.is_empty() && !is_valid_ipv4(&config.dns_primary) {
        errors.push("Invalid primary DNS".to_string());
    }

    if !config.dns_secondary.is_empty() && !is_valid_ipv4(&config.dns_secondary) {
        errors.push("Invalid secondary DNS".to_string());
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}
