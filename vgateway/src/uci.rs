//! UCI (Unified Configuration Interface) wrapper for OpenWrt
//! Wraps `uci` CLI commands to read/write /etc/config/* files

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

        // Ignore "Entry not found" errors (already deleted)
        if output.status.success() || output.status.code() == Some(1) {
            Ok(())
        } else {
            Err(String::from_utf8_lossy(&output.stderr).trim().to_string())
        }
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
