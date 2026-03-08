//! Bọc lệnh UCI (Unified Configuration Interface) cho OpenWrt
//! Gọi CLI `uci` để đọc/ghi file cấu hình /etc/config/*
//! Dùng cho tất cả thao tác config: get, set, delete, commit

use std::process::Command;

pub struct Uci;

impl Uci {
    /// Get a UCI value: `uci get <key>`
    pub fn get(key: &str) -> Result<String, String> {
        let output = Command::new("uci")
            .args(["get", key])
            .output()
            .map_err(|e| format!("uci exec failed: {}", e))?;

        if output.status.success() {
            Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
        } else {
            Err(String::from_utf8_lossy(&output.stderr).trim().to_string())
        }
    }

    /// Set a UCI value: `uci set <key>=<value>`
    pub fn set(key: &str, value: &str) -> Result<(), String> {
        let arg = format!("{}={}", key, value);
        let output = Command::new("uci")
            .args(["set", &arg])
            .output()
            .map_err(|e| format!("uci exec failed: {}", e))?;

        if output.status.success() {
            Ok(())
        } else {
            Err(String::from_utf8_lossy(&output.stderr).trim().to_string())
        }
    }

    /// Delete a UCI option: `uci delete <key>`
    pub fn delete(key: &str) -> Result<(), String> {
        let output = Command::new("uci")
            .args(["delete", key])
            .output()
            .map_err(|e| format!("uci exec failed: {}", e))?;

        if output.status.success() || output.status.code() == Some(1) {
            Ok(())
        } else {
            Err(String::from_utf8_lossy(&output.stderr).trim().to_string())
        }
    }

    /// Get a UCI list value (space/newline-separated)
    pub fn get_list(key: &str) -> Vec<String> {
        Command::new("uci")
            .args(["get", key])
            .output()
            .ok()
            .filter(|o| o.status.success())
            .map(|o| {
                String::from_utf8_lossy(&o.stdout)
                    .split_whitespace()
                    .map(String::from)
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Add to UCI list: `uci add_list <key>=<value>`
    pub fn add_list(key: &str, value: &str) -> Result<(), String> {
        let arg = format!("{}={}", key, value);
        let output = Command::new("uci")
            .args(["add_list", &arg])
            .output()
            .map_err(|e| format!("uci exec failed: {}", e))?;
        if output.status.success() {
            Ok(())
        } else {
            Err(String::from_utf8_lossy(&output.stderr).trim().to_string())
        }
    }

    /// Revert uncommitted changes: `uci revert <config>`
    pub fn revert(config: &str) -> Result<(), String> {
        let output = Command::new("uci")
            .args(["revert", config])
            .output()
            .map_err(|e| format!("uci exec failed: {}", e))?;
        if output.status.success() {
            Ok(())
        } else {
            Err(String::from_utf8_lossy(&output.stderr).trim().to_string())
        }
    }

    /// Check if there are uncommitted changes: `uci changes <config>`
    pub fn has_changes(config: &str) -> bool {
        Command::new("uci")
            .args(["changes", config])
            .output()
            .ok()
            .map(|o| !o.stdout.is_empty())
            .unwrap_or(false)
    }

    /// Get list of changed sections: `uci changes <config>`
    /// Returns unique section names (e.g. ["lan", "wan"] from "network.lan.proto=...")
    pub fn changed_sections(config: &str) -> Vec<String> {
        let output = Command::new("uci")
            .args(["changes", config])
            .output()
            .ok();
        let mut sections = Vec::new();
        if let Some(o) = output {
            let text = String::from_utf8_lossy(&o.stdout);
            for line in text.lines() {
                // Format: "network.lan.proto='static'" or "-network.wan.dns"
                let line = line.trim_start_matches('-');
                if let Some(rest) = line.strip_prefix(&format!("{}.", config)) {
                    if let Some(section) = rest.split('.').next() {
                        let section = section.split('=').next().unwrap_or(section);
                        let s = section.to_string();
                        if !sections.contains(&s) {
                            sections.push(s);
                        }
                    }
                }
            }
        }
        sections
    }

    /// Commit changes: `uci commit <config>`
    pub fn commit(config: &str) -> Result<(), String> {
        let output = Command::new("uci")
            .args(["commit", config])
            .output()
            .map_err(|e| format!("uci exec failed: {}", e))?;

        if output.status.success() {
            Ok(())
        } else {
            Err(String::from_utf8_lossy(&output.stderr).trim().to_string())
        }
    }
}
