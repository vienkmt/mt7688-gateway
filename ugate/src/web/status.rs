//! Thu thập trạng thái hệ thống và thống kê các kênh
//! SharedStats dùng atomic counters, chia sẻ giữa tất cả tasks
//! StatusCollector đọc /proc/* để lấy thông tin hệ thống

use std::sync::atomic::{AtomicU32, AtomicU8, Ordering};

/// Bộ đếm atomic chia sẻ giữa UART, MQTT, TCP, GPIO tasks
pub struct SharedStats {
    pub uart_rx_bytes: AtomicU32,
    pub uart_rx_frames: AtomicU32,
    pub uart_tx_bytes: AtomicU32,
    pub uart_tx_frames: AtomicU32,
    pub uart_failed: AtomicU32,
    pub mqtt_published: AtomicU32,
    pub mqtt_failed: AtomicU32,
    pub mqtt_state: AtomicU8, // 0=disabled, 1=disconnected, 2=connected
    pub tcp_connections: AtomicU8,
    pub tcp_state: AtomicU8, // 0=disabled, 1=disconnected, 2=connected
    pub http_state: AtomicU8, // 0=disabled, 1=active, 2=error
    pub http_sent: AtomicU32,
    pub http_failed: AtomicU32,
    pub gpio_states: [AtomicU8; 4],
}

impl SharedStats {
    pub fn new() -> Self {
        Self {
            uart_rx_bytes: AtomicU32::new(0),
            uart_rx_frames: AtomicU32::new(0),
            uart_tx_bytes: AtomicU32::new(0),
            uart_tx_frames: AtomicU32::new(0),
            uart_failed: AtomicU32::new(0),
            mqtt_published: AtomicU32::new(0),
            mqtt_failed: AtomicU32::new(0),
            mqtt_state: AtomicU8::new(0),
            tcp_connections: AtomicU8::new(0),
            tcp_state: AtomicU8::new(0),
            http_state: AtomicU8::new(0),
            http_sent: AtomicU32::new(0),
            http_failed: AtomicU32::new(0),
            gpio_states: [
                AtomicU8::new(0),
                AtomicU8::new(0),
                AtomicU8::new(0),
                AtomicU8::new(0),
            ],
        }
    }

    /// Thu thập trạng thái thành JSON string (không dùng serde_json)
    pub fn to_status_json(&self, config: &crate::config::Config) -> String {
        let uptime = read_uptime();
        let (ram_used, ram_total) = read_mem_info();
        let cpu = read_cpu_percent();

        format!(
            r#"{{"type":"status","version":"{}","uptime":"{}","cpu":{},"ram_used":{},"ram_total":{},"uart":{{"rx_bytes":{},"rx_frames":{},"tx_bytes":{},"tx_frames":{},"failed":{},"config":"{} 8N1"}},"mqtt":{{"enabled":{},"state":"{}","published":{},"failed":{}}},"http":{{"enabled":{},"state":"{}","sent":{},"failed":{}}},"tcp":{{"enabled":{},"state":"{}","connections":{}}},"gpio":[{},{},{},{}]}}"#,
            env!("CARGO_PKG_VERSION"),
            uptime,
            cpu,
            ram_used,
            ram_total,
            self.uart_rx_bytes.load(Ordering::Relaxed),
            self.uart_rx_frames.load(Ordering::Relaxed),
            self.uart_tx_bytes.load(Ordering::Relaxed),
            self.uart_tx_frames.load(Ordering::Relaxed),
            self.uart_failed.load(Ordering::Relaxed),
            config.uart.baudrate,
            config.mqtt.enabled,
            state_str(self.mqtt_state.load(Ordering::Relaxed)),
            self.mqtt_published.load(Ordering::Relaxed),
            self.mqtt_failed.load(Ordering::Relaxed),
            config.http.enabled,
            state_str(self.http_state.load(Ordering::Relaxed)),
            self.http_sent.load(Ordering::Relaxed),
            self.http_failed.load(Ordering::Relaxed),
            config.tcp.enabled,
            state_str(self.tcp_state.load(Ordering::Relaxed)),
            self.tcp_connections.load(Ordering::Relaxed),
            self.gpio_states[0].load(Ordering::Relaxed) != 0,
            self.gpio_states[1].load(Ordering::Relaxed) != 0,
            self.gpio_states[2].load(Ordering::Relaxed) != 0,
            self.gpio_states[3].load(Ordering::Relaxed) != 0,
        )
    }
}

fn state_str(s: u8) -> &'static str {
    match s {
        0 => "disabled",
        1 => "waiting",
        2 => "connected",
        _ => "unknown",
    }
}

/// Đọc uptime từ /proc/uptime
fn read_uptime() -> String {
    std::fs::read_to_string("/proc/uptime")
        .ok()
        .and_then(|s| s.split_whitespace().next().map(String::from))
        .and_then(|s| s.parse::<f64>().ok())
        .map(format_uptime)
        .unwrap_or_else(|| "0m".into())
}

fn format_uptime(secs: f64) -> String {
    let s = secs as u64;
    let d = s / 86400;
    let h = (s % 86400) / 3600;
    let m = (s % 3600) / 60;
    let sec = s % 60;
    if d > 0 {
        format!("{}d {}h {}m {}s", d, h, m, sec)
    } else if h > 0 {
        format!("{}h {}m {}s", h, m, sec)
    } else {
        format!("{}m {}s", m, sec)
    }
}

/// Đọc thông tin RAM từ /proc/meminfo (đơn vị MB)
fn read_mem_info() -> (u16, u16) {
    let content = std::fs::read_to_string("/proc/meminfo").unwrap_or_default();
    let mut total = 0u64;
    let mut free = 0u64;
    for line in content.lines() {
        if line.starts_with("MemTotal:") {
            total = parse_meminfo_kb(line);
        } else if line.starts_with("MemAvailable:") {
            free = parse_meminfo_kb(line);
        }
    }
    let total_mb = (total / 1024) as u16;
    let used_mb = ((total.saturating_sub(free)) / 1024) as u16;
    (used_mb, total_mb)
}

fn parse_meminfo_kb(line: &str) -> u64 {
    line.split_whitespace()
        .nth(1)
        .and_then(|s| s.parse().ok())
        .unwrap_or(0)
}

/// Đọc CPU usage từ /proc/stat (xấp xỉ, snapshot)
fn read_cpu_percent() -> u8 {
    let content = std::fs::read_to_string("/proc/loadavg").unwrap_or_default();
    content
        .split_whitespace()
        .next()
        .and_then(|s| s.parse::<f32>().ok())
        .map(|load| (load * 100.0).min(100.0) as u8)
        .unwrap_or(0)
}
